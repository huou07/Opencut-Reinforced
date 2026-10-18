//! Bounded command and project-file latency on a two-hundred-clip project.
//!
//! `#[ignore]`d by default: `scripts/bench_quality.py` runs it, and the ordinary
//! test suite stays fast. One machine-readable line (`OR_BENCH_JSON=...`) carries
//! every distribution so the budget script reads numbers instead of scraping.
//!
//! What this lane does: it builds a real two-hundred-clip project through the
//! canonical command path, then times the commands a user or an agent feels --
//! create, import, add a track, add a clip, trim, split, undo, and the reads a
//! seek performs -- plus whole-file project open and save. Times are wall clock
//! in a debug build on this host, which the report states; they are a regression
//! guard for OR's own code, not a claim about a release build's speed.
use or_core::{
    ApplicationRequest, ApplicationResponse, ClipId, CommandEnvelope, MediaId, ProjectFileSession,
    QueryEnvelope, RationalTime, TimeRange, TimelineTrimEdge, TrackId, TrackKind,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};

/// The product roadmap's own scale for this budget: a project at 200 clips is
/// where a naive implementation starts to hurt.
const CLIPS: usize = 200;
/// Iterations per measured command. 40 samples give a stable p95 without making
/// the lane slow enough that nobody runs it.
const ITERATIONS: usize = 40;

#[test]
#[ignore]
fn command_latency_on_a_two_hundred_clip_project() {
    let Some(media) = fixture_media() else {
        report(&format!(
            "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
            json_string(
                "no media fixture: set OR_BENCH_MEDIA to a small probeable file, or run \
                 from a checkout that has crates/or_media/tests/fixtures/tiny.mkv"
            )
        ));
        return;
    };
    let workspace = temp_workspace();
    let mut measurements: Vec<(String, Vec<f64>)> = Vec::new();

    // --- create ----------------------------------------------------------
    let mut create_samples = Vec::with_capacity(ITERATIONS);
    let mut fixture: Option<ProjectFileSession> = None;
    for index in 0..ITERATIONS {
        let path = workspace.join(format!("create-{index}.orproj"));
        let started = Instant::now();
        let session = ProjectFileSession::create_new(&path, "command latency");
        create_samples.push(elapsed_ms(started));
        match session {
            Ok(session) => {
                if fixture.is_none() {
                    fixture = Some(session);
                }
            }
            Err(error) => {
                unverified(&format!("project create failed: {error}"));
                return;
            }
        }
    }
    measurements.push(("create".to_owned(), create_samples));

    let Some(mut session) = fixture else {
        unverified("no project session was created");
        return;
    };
    // The session's own path, not a guessed one: `create_new` owns the filename.
    let project_path = session.project_path().to_path_buf();

    // --- import ----------------------------------------------------------
    // The project refuses a second item for the same source, which is correct
    // product behaviour: import each iteration from its own copy so the
    // measurement is a real import and not a rejection.
    let mut sources = Vec::with_capacity(ITERATIONS);
    for index in 0..ITERATIONS {
        let copy = workspace.join(format!("source-{index}.mkv"));
        if let Err(error) = fs::copy(&media, &copy) {
            unverified(&format!("could not stage source copy {index}: {error}"));
            return;
        }
        sources.push(copy);
    }
    let mut import_samples = Vec::with_capacity(ITERATIONS);
    let mut media_id: Option<MediaId> = None;
    for source in &sources {
        let started = Instant::now();
        match session.import_media(source) {
            Ok((item, _)) => {
                import_samples.push(elapsed_ms(started));
                media_id = Some(item.id());
            }
            Err(error) => {
                unverified(&format!("media import failed: {error}"));
                return;
            }
        }
    }
    measurements.push(("import".to_owned(), import_samples));
    let Some(media_id) = media_id else {
        unverified("no media item was imported");
        return;
    };

    // --- add track, then build the 200-clip project ----------------------
    let mut track_samples = Vec::with_capacity(ITERATIONS);
    let mut track_id: Option<TrackId> = None;
    for _ in 0..ITERATIONS {
        let id = TrackId::generate();
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::add_timeline_track(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                id,
                TrackKind::Audio,
            ),
        ));
        if !succeeded(&done) {
            unverified("timeline track add failed");
            return;
        }
        track_samples.push(elapsed_ms(started));
        track_id.get_or_insert(id);
    }
    measurements.push(("add_track".to_owned(), track_samples));
    let Some(track_id) = track_id else {
        unverified("no track was added");
        return;
    };

    // A one-second source is enough: the clips tile the timeline by their own
    // starts, so 200 of them exercise the walk the trim and split pay for.
    let source = match TimeRange::new(time(0, 1), time(1, 4)) {
        Ok(range) => range,
        Err(error) => {
            unverified(&format!("invalid source range: {error}"));
            return;
        }
    };
    let mut clip_ids = Vec::with_capacity(CLIPS);
    let mut insert_samples = Vec::with_capacity(CLIPS);
    for index in 0..CLIPS {
        let clip_id = ClipId::generate();
        let start = match RationalTime::new(index as i64, 4) {
            Ok(value) => value,
            Err(error) => {
                unverified(&format!("invalid clip start: {error}"));
                return;
            }
        };
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::insert_timeline_clip(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip_id,
                track_id,
                media_id,
                start,
                source,
            ),
        ));
        if !succeeded(&done) {
            unverified(&format!("clip {index} insert failed"));
            return;
        }
        insert_samples.push(elapsed_ms(started));
        clip_ids.push(clip_id);
    }
    measurements.push(("add_clip".to_owned(), insert_samples));

    // --- overwrite insert ------------------------------------------------
    // The overwrite path deletes the covered clip and inserts the replacement in
    // one transaction, so this row is the cost a user pays when an edit replaces
    // a range instead of refusing to overlap. The span covers the first clip
    // exactly, which keeps the project at 200 clips and the geometry stable.
    let mut overwrite_samples = Vec::with_capacity(ITERATIONS);
    let mut overwritten_id = None;
    for _ in 0..ITERATIONS {
        let clip_id = ClipId::generate();
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::insert_timeline_clip_with_mode(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip_id,
                track_id,
                media_id,
                time(0, 1),
                source,
                or_core::TimelineEditMode::Overwrite,
                None,
            ),
        ));
        if !succeeded(&done) {
            unverified("overwrite insert failed on a project with a covered clip");
            return;
        }
        overwrite_samples.push(elapsed_ms(started));
        overwritten_id = Some(clip_id);
    }
    // The overwrite replaced the first clip, so its id is the one the later trim
    // step must target; the geometry is unchanged because the span is identical.
    if let (Some(clip_id), Some(first)) = (overwritten_id, clip_ids.first_mut()) {
        *first = clip_id;
    }
    measurements.push(("overwrite_insert".to_owned(), overwrite_samples));

    // --- ripple trim ------------------------------------------------------
    // Trimming the first clip's end and pulling the other 199 clips with it is the
    // worst case this command has: the shift touches every clip after the edit. It
    // runs here, where the tile geometry is exactly known (clip i spans i/4 to
    // i/4+1/4), and the loop undoes its own edits afterwards so the trim, split and
    // read rows below see the geometry they expect.
    let mut ripple_samples = Vec::with_capacity(ITERATIONS);
    let mut ripple_edge = match RationalTime::new(1, 4) {
        Ok(value) => value,
        Err(error) => {
            unverified(&format!("invalid ripple edge: {error}"));
            return;
        }
    };
    // 40 iterations of 1/512 s stay well inside the first clip's 1/4 s span, so the
    // loop never reaches its own start (40/512 < 1/4) and the measured edit is
    // always a legal shortening.
    let ripple_step = match RationalTime::new(1, 512) {
        Ok(value) => value,
        Err(error) => {
            unverified(&format!("invalid ripple step: {error}"));
            return;
        }
    };
    for ripple_index in 0..ITERATIONS {
        ripple_edge = match ripple_edge.checked_sub(ripple_step) {
            Ok(value) => value,
            Err(error) => {
                unverified(&format!("ripple edge underflow: {error}"));
                return;
            }
        };
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::trim_timeline_clip_with_ripple(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip_ids[0],
                TimelineTrimEdge::End,
                ripple_edge,
                true,
            ),
        ));
        if !succeeded(&done) {
            unverified(&format!(
                "ripple trim failed at iteration {ripple_index} for edge {}/512 on clip {}",
                ripple_edge.numerator(),
                clip_ids[0],
            ));
            return;
        }
        ripple_samples.push(elapsed_ms(started));
    }
    for _ in 0..ITERATIONS {
        let done =
            session.handle_application_request(ApplicationRequest::Command(CommandEnvelope::undo(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
            )));
        if !succeeded(&done) {
            unverified("undo after the ripple row failed");
            return;
        }
    }
    measurements.push(("ripple_trim".to_owned(), ripple_samples));

    // --- trim ------------------------------------------------------------
    let mut trim_samples = Vec::with_capacity(ITERATIONS);
    for index in 0..ITERATIONS {
        let clip_index = index % clip_ids.len();
        let clip = clip_ids[clip_index];
        // Each clip spans (2i/8, (2i+2)/8) after insertion, so trimming its start
        // to (2i+1)/8 shortens it inside its own source range. A time before the
        // clip's own start is correctly refused, and must not be measured.
        let to = trim_target(clip_index);
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::trim_timeline_clip(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip,
                TimelineTrimEdge::Start,
                to,
            ),
        ));
        if !succeeded(&done) {
            unverified(&format!("clip trim failed for {clip}"));
            return;
        }
        trim_samples.push(elapsed_ms(started));
    }
    measurements.push(("trim".to_owned(), trim_samples));

    // --- split -----------------------------------------------------------
    let mut split_samples = Vec::with_capacity(ITERATIONS);
    for index in 0..ITERATIONS {
        let clip_index = index % clip_ids.len();
        let clip = clip_ids[clip_index];
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::split_timeline_clip(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip,
                ClipId::generate(),
                // Strictly inside the trimmed span ((4i+2)/16, (4i+4)/16).
                RationalTime::new((4 * clip_index + 3) as i64, 16).expect("valid rational time"),
            ),
        ));
        if !succeeded(&done) {
            // Keep going: one unusable instant must not fail the lane, but it
            // must not be counted either.
            continue;
        }
        split_samples.push(elapsed_ms(started));
    }
    if split_samples.is_empty() {
        unverified("no clip could be split");
        return;
    }
    measurements.push(("split".to_owned(), split_samples));

    // --- undo and redo ---------------------------------------------------
    let mut undo_samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        let done =
            session.handle_application_request(ApplicationRequest::Command(CommandEnvelope::undo(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
            )));
        if !succeeded(&done) {
            unverified("history undo failed on a project with history");
            return;
        }
        undo_samples.push(elapsed_ms(started));
    }
    measurements.push(("undo".to_owned(), undo_samples));

    // --- the reads a seek performs ---------------------------------------
    let mut summary_samples = Vec::with_capacity(ITERATIONS);
    let mut clips_samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let envelope = QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: session.session().project_id(),
            project_instance_id: session.session().project_instance_id(),
            arguments: serde_json::json!({}),
        };
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Query(envelope));
        if !succeeded(&done) {
            unverified("project summary query failed");
            return;
        }
        summary_samples.push(elapsed_ms(started));

        let envelope = QueryEnvelope::timeline_clips_v2(
            session.session().project_id(),
            session.session().project_instance_id(),
            track_id,
            0,
            100,
        );
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Query(envelope));
        if !succeeded(&done) {
            unverified("timeline clips query failed");
            return;
        }
        clips_samples.push(elapsed_ms(started));
    }
    measurements.push(("seek_summary".to_owned(), summary_samples));
    measurements.push(("seek_clip_page".to_owned(), clips_samples));

    // --- project file open and save --------------------------------------
    let mut save_samples = Vec::with_capacity(ITERATIONS);
    for index in 0..ITERATIONS {
        // Alternate: mutate, then save, so every save has real work to publish.
        let clip_index = index % clip_ids.len();
        let clip = clip_ids[clip_index];
        // Trim the end back to the same interior instant, so every save
        // publishes a real change and the project file keeps its size.
        let _ = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::trim_timeline_clip(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                clip,
                TimelineTrimEdge::End,
                trim_target(clip_index),
            ),
        ));
        let started = Instant::now();
        if let Err(error) = session.save() {
            unverified(&format!("project save failed: {error}"));
            return;
        }
        save_samples.push(elapsed_ms(started));
    }
    measurements.push(("project_save".to_owned(), save_samples));

    // --- audio clean-up ---------------------------------------------------
    // The only row that measures a helper process: its cost is dominated by
    // starting ffmpeg and running the filter, not by OR's own code, so it is
    // reported as its own figure rather than compared with the command rows. The
    // fixture is generated here because it must be audio only and short.
    match measure_audio_cleanup(&workspace) {
        Ok(samples) => measurements.push(("audio_cleanup".to_owned(), samples)),
        Err(reason) => {
            // Like the media fixture, this row's prerequisite is the host: without a
            // helper there is nothing to measure, so the lane says so instead of
            // reporting a number it did not take.
            unverified(&reason);
            return;
        }
    }

    let mut open_samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        match ProjectFileSession::open(&project_path) {
            Ok(opened) => {
                open_samples.push(elapsed_ms(started));
                drop(opened);
            }
            Err(error) => {
                unverified(&format!("project open failed: {error}"));
                return;
            }
        }
    }
    measurements.push(("project_open".to_owned(), open_samples));

    // --- track reorder ----------------------------------------------------
    // Moving a track is a vector removal and insertion, so its cost is the track
    // count rather than the clip count. It runs here, before the move rows above,
    // and swaps the same two tracks back and forth so the geometry the other rows
    // use is unchanged.
    let mut track_move_samples = Vec::with_capacity(ITERATIONS);
    let second_track = TrackId::generate();
    let added = session.handle_application_request(ApplicationRequest::Command(
        CommandEnvelope::add_timeline_track(
            session.session().project_id(),
            session.session().project_instance_id(),
            session.session().project_revision(),
            second_track,
            or_core::TrackKind::Audio,
        ),
    ));
    if !succeeded(&added) {
        unverified("could not add the second track the reorder row needs");
        return;
    }
    for index in 0..ITERATIONS {
        let to = index % 2;
        let started = Instant::now();
        let done = session.handle_application_request(ApplicationRequest::Command(
            CommandEnvelope::move_timeline_track(
                session.session().project_id(),
                session.session().project_instance_id(),
                session.session().project_revision(),
                track_id,
                to,
            ),
        ));
        if !succeeded(&done) {
            unverified("track move failed on a project with two tracks");
            return;
        }
        track_move_samples.push(elapsed_ms(started));
    }
    // Leave the track where the other rows expect it.
    let _ = session.handle_application_request(ApplicationRequest::Command(
        CommandEnvelope::move_timeline_track(
            session.session().project_id(),
            session.session().project_instance_id(),
            session.session().project_revision(),
            track_id,
            0,
        ),
    ));
    measurements.push(("track_move".to_owned(), track_move_samples));

    // --- slip, roll and slide --------------------------------------------
    // The three clip-level moves that do not change where a clip sits as a whole.
    // They run last, on the tiled 200-clip project, so their offsets cannot disturb
    // the geometry the earlier rows depend on. Each moves by 1/512 s, which is small
    // enough that forty iterations never reach a boundary: the clips are a quarter
    // second long and the project's clips are adjacent by construction, which is
    // exactly what roll and slide require.
    let slip_step = match RationalTime::new(1, 512) {
        Ok(value) => value,
        Err(error) => {
            unverified(&format!("invalid slip step: {error}"));
            return;
        }
    };
    // The trim row trimmed the first ITERATIONS clips' starts and never undid them,
    // so the tiling only still holds past that range; the middle clip of a triple
    // that was never trimmed is the one these rows use.
    let pivot = ITERATIONS * 2;
    if clip_ids.len() > pivot + 2 {
        let mut slip_samples = Vec::with_capacity(ITERATIONS);
        let mut roll_samples = Vec::with_capacity(ITERATIONS);
        let mut slide_samples = Vec::with_capacity(ITERATIONS);
        for _ in 0..ITERATIONS {
            let started = Instant::now();
            let done = session.handle_application_request(ApplicationRequest::Command(
                CommandEnvelope::slip_timeline_clip(
                    session.session().project_id(),
                    session.session().project_instance_id(),
                    session.session().project_revision(),
                    clip_ids[pivot],
                    slip_step,
                ),
            ));
            if !succeeded(&done) {
                unverified("slip failed on a project with tiled clips");
                return;
            }
            slip_samples.push(elapsed_ms(started));

            let started = Instant::now();
            let done = session.handle_application_request(ApplicationRequest::Command(
                CommandEnvelope::roll_timeline_clip(
                    session.session().project_id(),
                    session.session().project_instance_id(),
                    session.session().project_revision(),
                    clip_ids[pivot],
                    slip_step,
                ),
            ));
            if !succeeded(&done) {
                unverified("roll failed on a project with adjacent clips");
                return;
            }
            roll_samples.push(elapsed_ms(started));

            let started = Instant::now();
            let done = session.handle_application_request(ApplicationRequest::Command(
                CommandEnvelope::slide_timeline_clip(
                    session.session().project_id(),
                    session.session().project_instance_id(),
                    session.session().project_revision(),
                    clip_ids[pivot],
                    slip_step,
                ),
            ));
            if !succeeded(&done) {
                unverified("slide failed on a project with adjacent clips");
                return;
            }
            slide_samples.push(elapsed_ms(started));
        }
        measurements.push(("slip".to_owned(), slip_samples));
        measurements.push(("roll".to_owned(), roll_samples));
        measurements.push(("slide".to_owned(), slide_samples));
    }

    let clip_count = clip_ids.len();
    let rows = measurements
        .iter()
        .map(|(name, samples)| distribution(name, samples))
        .collect::<Vec<_>>()
        .join(",");
    report(&format!(
        concat!(
            "{{\"result\":\"MEASURED\",",
            "\"clips\":{clips},\"iterations\":{iterations},",
            "\"project_bytes\":{bytes},",
            "\"build\":\"debug\",",
            "\"includes\":{includes},",
            "\"rows\":[{rows}]}}"
        ),
        clips = clip_count,
        iterations = ITERATIONS,
        bytes = fs::metadata(&project_path)
            .map(|meta| meta.len())
            .unwrap_or(0),
        includes = json_string(
            "canonical command path and project file I/O, in-process, debug build on this host"
        ),
        rows = rows,
    ));
    let _ = fs::remove_dir_all(&workspace);
}

/// Generates a short noisy audio fixture and times the clean-up over it.
///
/// Three seconds of white noise with a tone through it, written by the same helper
/// the clean-up uses, so the row needs no committed media: an audio-only fixture
/// short enough to keep the lane's wall clock bounded.
fn measure_audio_cleanup(workspace: &Path) -> Result<Vec<f64>, String> {
    // The resolver's fallback is the *name* `ffmpeg`, which `is_file` cannot see, so
    // each candidate is asked for its version instead of being stat-ed.
    let resolved = or_core::ffmpeg_executable();
    let candidates: Vec<PathBuf> = if resolved.is_file() {
        vec![resolved]
    } else {
        vec![resolved, PathBuf::from("ffmpeg")]
    };
    let helper = candidates
        .into_iter()
        .find(|candidate| {
            std::process::Command::new(candidate)
                .arg("-version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            "no ffmpeg helper on this host: the clean-up row needs one for its fixture".to_owned()
        })?;
    let source = workspace.join("cleanup-source.wav");
    let generated_ok = std::process::Command::new(&helper)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "anoisesrc=d=3:c=white:a=0.12:r=48000",
            "-ac",
            "1",
        ])
        .arg(&source)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !generated_ok {
        return Err("the helper could not generate the clean-up fixture".to_owned());
    }

    let mut samples = Vec::with_capacity(ITERATIONS.min(8));
    for index in 0..ITERATIONS.min(8) {
        let destination = workspace.join(format!("cleanup-out-{index}.wav"));
        let started = Instant::now();
        // The floor is derived, which is what the product does, so the row covers
        // the measurement pass as well as the filter run.
        let floor = or_core::measure_noise_floor(&source)
            .map_err(|error| format!("the floor measurement failed: {}", error.code().as_str()))?;
        or_core::clean_audio_file(&source, &destination, floor, 12)
            .map_err(|error| format!("the clean-up failed: {}", error.code().as_str()))?;
        samples.push(elapsed_ms(started));
        let _ = fs::remove_file(&destination);
    }
    Ok(samples)
}

fn fixture_media() -> Option<PathBuf> {
    if let Ok(path) = env::var("OR_BENCH_MEDIA") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let candidate =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../or_media/tests/fixtures/tiny.mkv");
    candidate.is_file().then_some(candidate)
}

fn temp_workspace() -> PathBuf {
    // No new dependency for a temporary directory: one unique path under the
    // system temporary directory, removed at the end of the lane.
    let unique = format!(
        "or-command-latency-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    );
    let path = env::temp_dir().join(unique);
    fs::create_dir_all(&path).expect("create the bench workspace");
    path
}

fn succeeded(response: &ApplicationResponse) -> bool {
    !matches!(response, ApplicationResponse::Error(_))
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

/// The interior instant of the clip at `index`: its span is
/// `(2i/8, (2i+2)/8)`, so `(2i+1)/8` is strictly inside it.
fn trim_target(index: usize) -> RationalTime {
    RationalTime::new((2 * index + 1) as i64, 8).expect("valid rational time")
}

#[allow(dead_code)]
fn time(numerator: i64, denominator: u32) -> RationalTime {
    RationalTime::new(numerator, denominator).expect("valid rational time")
}

fn distribution(name: &str, samples: &[f64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).expect("finite samples"));
    let percentile = |fraction: f64| {
        let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
        sorted[index]
    };
    format!(
        "{{\"id\":{name},\"count\":{count},\"min_ms\":{min:.3},\"p50_ms\":{p50:.3},\
         \"p95_ms\":{p95:.3},\"max_ms\":{max:.3}}}",
        name = json_string(name),
        count = sorted.len(),
        min = sorted[0],
        p50 = percentile(0.50),
        p95 = percentile(0.95),
        max = sorted[sorted.len() - 1],
    )
}

fn unverified(reason: &str) {
    report(&format!(
        "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
        json_string(reason)
    ));
}

fn report(payload: &str) {
    println!("OR_BENCH_JSON={payload}");
}

/// Minimal JSON string escaping, so this lane adds no dependency to report one
/// line of machine-readable output.
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < ' ' => out.push_str(&format!("\\u{:04x}", control as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[allow(dead_code)]
fn parse_uuid(value: &str) -> TrackId {
    TrackId::from_str(value).expect("canonical v4 uuid")
}
