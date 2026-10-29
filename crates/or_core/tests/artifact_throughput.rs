//! Measures media-artifact generation throughput at different job-pool widths.
//!
//! Run on a machine with ffmpeg and ffprobe on PATH:
//!
//! ```sh
//! cargo test -p or_core --test artifact_throughput --release -- --ignored --nocapture
//! ```
//!
//! The test generates one short clip per library item, then, for each pool
//! width, waits for every thumbnail and waveform to finish and reports the wall
//! clock. It exists to justify the pool width in
//! `crates/or_app_bridge/src/api/project.rs`; it asserts nothing about the
//! numbers because they are host-specific.

use or_core::{
    CacheStoreConfig, JobManagerConfig, MediaArtifactRequestState, MediaArtifactService,
    MediaArtifactServiceConfig, MediaItem, ProjectFileSession,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "or-artifact-throughput-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
        && Command::new("ffprobe").arg("-version").output().is_ok()
}

const CLIPS: usize = 8;

#[test]
#[ignore = "requires system ffmpeg and ffprobe; measures throughput on the host"]
fn artifact_throughput_by_pool_width() {
    if !ffmpeg_available() {
        eprintln!("skipping: ffmpeg/ffprobe unavailable");
        return;
    }
    let directory = TestDirectory::new();
    let mut session =
        ProjectFileSession::create_new(&directory.0.join("throughput.orproj"), "Throughput")
            .expect("create project");
    let mut items: Vec<MediaItem> = Vec::new();
    for index in 0..CLIPS {
        let path = directory.0.join(format!("clip-{index}.mkv"));
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=640x360:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000",
                "-t",
                "1.5",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:a",
                "aac",
            ])
            .arg(&path)
            .output()
            .expect("run ffmpeg");
        assert!(
            status.status.success(),
            "generate clip failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        let (imported, _) = session.import_media(&path).expect("import clip");
        let item = session_media_item(&mut session, imported.id());
        items.push(item);
    }

    for workers in [1usize, 2, 4, 8] {
        let cache = directory.0.join(format!("cache-{workers}"));
        let service = MediaArtifactService::new(MediaArtifactServiceConfig::system_ffmpeg(
            &cache,
            JobManagerConfig::new(workers, 32, 128).unwrap(),
            CacheStoreConfig::new(8 * 1024 * 1024, 256 * 1024 * 1024).unwrap(),
        ))
        .expect("initialize service");
        let events = service.subscribe_events();
        let started = Instant::now();
        let mut pending = 0usize;
        for (index, item) in items.iter().enumerate() {
            let first = if index % 2 == 0 {
                service.request_thumbnail(item)
            } else {
                service.request_waveform(item)
            }
            .expect("request artifact");
            if is_pending(&first) {
                pending += 1;
            }
            let second = if index % 2 == 0 {
                service.request_waveform(item)
            } else {
                service.request_thumbnail(item)
            }
            .expect("request artifact");
            if is_pending(&second) {
                pending += 1;
            }
        }
        let mut done = 0usize;
        while done < pending {
            let event = events
                .recv_timeout(Duration::from_secs(120))
                .expect("artifact event");
            assert!(
                event.state == or_core::MediaArtifactEventState::Succeeded,
                "artifact failed: {event:?}"
            );
            done += 1;
        }
        let elapsed = started.elapsed();
        println!(
            "workers={workers:>2} artifacts={pending:>2} elapsed={:>8.3}s per-artifact={:>6.1}ms",
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() * 1000.0 / pending as f64
        );
        assert_eq!(pending, CLIPS * 2);
    }
}

fn is_pending(request: &or_core::MediaArtifactRequest) -> bool {
    matches!(
        request.state,
        MediaArtifactRequestState::Queued | MediaArtifactRequestState::Running
    )
}

fn session_media_item(session: &mut ProjectFileSession, id: or_core::MediaId) -> MediaItem {
    use or_core::{ApplicationRequest, ApplicationResponse, QueryEnvelope};
    let query = QueryEnvelope::media_get(
        session.session().project_id(),
        session.session().project_instance_id(),
        id,
    );
    match session.handle_application_request(ApplicationRequest::Query(query)) {
        ApplicationResponse::Query(result) => *result.media_item.expect("media.get item"),
        response => panic!("unexpected media.get response: {response:?}"),
    }
}
