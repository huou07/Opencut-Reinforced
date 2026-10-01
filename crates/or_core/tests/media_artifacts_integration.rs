use or_core::{
    ApplicationRequest, ApplicationResponse, CacheArtifactKind, CacheStoreConfig, JobId,
    JobManagerConfig, MediaArtifactEvent, MediaArtifactEventState, MediaArtifactRequest,
    MediaArtifactRequestState, MediaArtifactService, MediaArtifactServiceConfig, MediaItem,
    MediaSourceRef, ProjectFileSession, ProjectRevision, QueryEnvelope, fingerprint_media_source,
};
use std::{
    fs::{self, FileTimes, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "or-media-artifacts-integration-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn media_path(&self) -> PathBuf {
        self.0.join("generated-test-media.mkv")
    }

    fn project_path(&self) -> PathBuf {
        self.0.join("generated-test-project.orproj")
    }

    fn cache_path(&self) -> PathBuf {
        self.0.join("cache")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires system ffmpeg and ffprobe; hosted Linux CI runs this explicitly"]
fn system_ffmpeg_generates_cached_thumbnail_and_waveform_previews() {
    let directory = TestDirectory::new();
    let media_path = directory.media_path();
    generate_media(&media_path);

    let project_path = directory.project_path();
    let mut session = ProjectFileSession::create_new(&project_path, "Artifact preview")
        .expect("create project for canonical media query");
    let (imported, result) = session.import_media(&media_path).expect("import fixture");
    assert_eq!(result.after_revision, ProjectRevision::new(1));
    let query = QueryEnvelope::media_get(
        session.session().project_id(),
        session.session().project_instance_id(),
        imported.id(),
    );
    let item = match session.handle_application_request(ApplicationRequest::Query(query)) {
        ApplicationResponse::Query(result) => *result.media_item.expect("media.get item"),
        response => panic!("unexpected media.get response: {response:?}"),
    };
    let revision = session.session().project_revision();

    let service = MediaArtifactService::new(MediaArtifactServiceConfig::system_ffmpeg(
        directory.cache_path(),
        JobManagerConfig::new(2, 32, 128).unwrap(),
        CacheStoreConfig::new(8 * 1024 * 1024, 256 * 1024 * 1024).unwrap(),
    ))
    .expect("initialize production artifact service");
    let events = service.subscribe_events();

    let thumbnail = service
        .request_thumbnail(&item)
        .expect("request real thumbnail");
    assert!(matches!(
        thumbnail.state,
        MediaArtifactRequestState::Queued | MediaArtifactRequestState::Running
    ));
    let thumbnail_event = wait_for_success(&events, &thumbnail, CacheArtifactKind::Thumbnail);
    assert!(thumbnail_event.sequence > 0);
    let thumbnail_key = thumbnail.cache_key.expect("thumbnail cache key");
    let thumbnail_bytes = service
        .read_artifact(CacheArtifactKind::Thumbnail, thumbnail_key)
        .unwrap()
        .expect("generated thumbnail cache entry");
    let (thumbnail_width, thumbnail_height) = png_dimensions(&thumbnail_bytes);
    assert!(thumbnail_width > 0 && thumbnail_height > 0);
    assert!(thumbnail_width <= 320 && thumbnail_height <= 320);

    let thumbnail_hit = service
        .request_thumbnail(&item)
        .expect("repeat thumbnail request");
    assert_eq!(thumbnail_hit.state, MediaArtifactRequestState::Ready);
    assert_eq!(thumbnail_hit.cache_key, Some(thumbnail_key));
    assert_eq!(thumbnail_hit.job_id, None, "cache hit must not start a job");

    let waveform = service
        .request_waveform(&item)
        .expect("request real waveform");
    assert!(matches!(
        waveform.state,
        MediaArtifactRequestState::Queued | MediaArtifactRequestState::Running
    ));
    wait_for_success(&events, &waveform, CacheArtifactKind::Waveform);
    let waveform_key = waveform.cache_key.expect("waveform cache key");
    let waveform_bytes = service
        .read_artifact(CacheArtifactKind::Waveform, waveform_key)
        .unwrap()
        .expect("generated waveform cache entry");
    assert_eq!(png_dimensions(&waveform_bytes), (512, 96));
    let waveform_hit = service
        .request_waveform(&item)
        .expect("repeat waveform request");
    assert_eq!(waveform_hit.state, MediaArtifactRequestState::Ready);
    assert_eq!(waveform_hit.cache_key, Some(waveform_key));
    assert_eq!(waveform_hit.job_id, None, "cache hit must not start a job");
    assert_eq!(session.session().project_revision(), revision);

    let changed_path = directory.0.join("same-size-changed-source.mkv");
    let mut changed_bytes = fs::read(&media_path).unwrap();
    let original_size = changed_bytes.len();
    let original_modified = fs::metadata(&media_path).unwrap().modified().unwrap();
    changed_bytes[0] ^= 1;
    fs::write(&changed_path, changed_bytes).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&changed_path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(original_modified))
        .unwrap();
    let changed_metadata = fs::metadata(&changed_path).unwrap();
    assert_eq!(changed_metadata.len(), original_size as u64);
    assert_eq!(changed_metadata.modified().unwrap(), original_modified);
    assert_ne!(
        fingerprint_media_source(&media_path).unwrap(),
        fingerprint_media_source(&changed_path).unwrap(),
        "same-size, same-mtime sampled byte changes must change the fingerprint"
    );

    let changed_uri = url::Url::from_file_path(&changed_path).expect("changed source file URI");
    let changed_item = MediaItem::new(
        item.id(),
        MediaSourceRef::local_file(changed_uri.as_str()).expect("same-size source URI"),
        item.metadata().clone(),
    )
    .unwrap();
    let changed_thumbnail = service
        .request_thumbnail(&changed_item)
        .expect("request changed source fingerprint");
    assert_ne!(changed_thumbnail.cache_key, Some(thumbnail_key));
    if let Some(job_id) = changed_thumbnail.job_id {
        let _ = service.cancel(job_id);
    }
    assert_eq!(session.session().project_revision(), revision);

    service.shutdown();
}

#[test]
#[ignore = "requires system ffmpeg and ffprobe; hosted Linux CI runs this explicitly"]
fn system_ffmpeg_generates_v1_video_only_proxy_and_preserves_vfr_timing() {
    let directory = TestDirectory::new();
    let media_path = directory.media_path();
    generate_vfr_media(&media_path, "testsrc", "OR source title");

    let project_path = directory.project_path();
    let mut session = ProjectFileSession::create_new(&project_path, "Proxy foundation")
        .expect("create project for proxy independence check");
    let (imported, import_result) = session
        .import_media(&media_path)
        .expect("import VFR fixture");
    assert_eq!(import_result.after_revision, ProjectRevision::new(1));
    let item = match session.handle_application_request(ApplicationRequest::Query(
        QueryEnvelope::media_get(
            session.session().project_id(),
            session.session().project_instance_id(),
            imported.id(),
        ),
    )) {
        ApplicationResponse::Query(result) => *result.media_item.expect("media.get item"),
        response => panic!("unexpected media.get response: {response:?}"),
    };
    let project_revision = session.session().project_revision();
    let service = MediaArtifactService::new(MediaArtifactServiceConfig::system_ffmpeg(
        directory.cache_path(),
        JobManagerConfig::new(2, 32, 128).unwrap(),
        CacheStoreConfig::new(8 * 1024 * 1024, 256 * 1024 * 1024).unwrap(),
    ))
    .expect("initialize production proxy service");
    let events = service.subscribe_events();

    let request = service.request_proxy(&item).expect("request Proxy V1");
    let event = wait_for_success(&events, &request, CacheArtifactKind::Proxy);
    assert_eq!(event.kind, CacheArtifactKind::Proxy);
    assert_eq!(event.error_code, None);
    let key = request.cache_key.expect("proxy cache key");
    let proxy_path = directory
        .cache_path()
        .join("proxy")
        .join(&key.to_hex()[..2])
        .join(format!("{}.mkv", key.to_hex()));
    assert!(proxy_path.is_file());

    let proxy_probe = ffprobe_json(&proxy_path);
    let format = proxy_probe.get("format").expect("proxy format metadata");
    let format_name = format["format_name"].as_str().unwrap_or_default();
    assert!(format_name.split(',').any(|name| name == "matroska"));
    let duration = format["duration"]
        .as_str()
        .expect("proxy duration")
        .parse::<f64>()
        .expect("numeric proxy duration");
    assert!(duration > 0.0 && duration < 10.0);

    let streams = proxy_probe["streams"].as_array().expect("proxy streams");
    assert_eq!(
        streams.len(),
        1,
        "proxy contains only its first video stream"
    );
    let video = &streams[0];
    assert_eq!(video["codec_type"], "video");
    assert_eq!(video["codec_name"], "mpeg4");
    assert_eq!(video["pix_fmt"], "yuv420p");
    let width = video["width"].as_u64().expect("proxy width");
    let height = video["height"].as_u64().expect("proxy height");
    assert!(width > 0 && width <= 960 && width.is_multiple_of(2));
    assert!(height > 0 && height <= 540 && height.is_multiple_of(2));
    assert!(proxy_probe["chapters"].as_array().is_none_or(Vec::is_empty));
    for tags in [format.get("tags"), video.get("tags")]
        .into_iter()
        .flatten()
    {
        let names = tags
            .as_object()
            .into_iter()
            .flat_map(|entries| entries.keys())
            .map(|name| name.to_ascii_lowercase())
            .collect::<Vec<_>>();
        assert!(names.iter().all(|name| {
            !["title", "artist", "author", "comment", "location", "gps"].contains(&name.as_str())
        }));
    }

    let source_pts = ffprobe_video_pts(&media_path);
    let proxy_pts = ffprobe_video_pts(&proxy_path);
    assert!(
        source_pts.len() >= 4,
        "fixture has enough frames to prove VFR"
    );
    assert_eq!(proxy_pts.len(), source_pts.len());
    assert!(proxy_pts.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(proxy_pts[0].abs() < 0.002, "proxy starts at PTS zero");
    for (source, proxy) in source_pts.iter().zip(&proxy_pts) {
        let source_relative = source - source_pts[0];
        let proxy_relative = proxy - proxy_pts[0];
        assert!((source_relative - proxy_relative).abs() < 0.02);
    }
    assert!(has_variable_frame_spacing(&proxy_pts));

    let cache_hit = service.request_proxy(&item).expect("repeat proxy request");
    assert_eq!(cache_hit.state, MediaArtifactRequestState::Ready);
    assert_eq!(cache_hit.cache_key, Some(key));
    assert_eq!(cache_hit.job_id, None);
    assert_eq!(
        service.read_artifact(CacheArtifactKind::Proxy, key),
        Err(or_core::MediaArtifactRequestError::ArtifactNotReadable)
    );

    let changed_path = directory.0.join("changed-generated-test-media.mkv");
    generate_vfr_media(&changed_path, "testsrc2", "OR changed source title");
    assert_ne!(
        fingerprint_media_source(&media_path).unwrap(),
        fingerprint_media_source(&changed_path).unwrap()
    );
    let changed_uri = url::Url::from_file_path(&changed_path).expect("changed source URI");
    let changed_item = MediaItem::new(
        item.id(),
        MediaSourceRef::local_file(changed_uri.as_str()).expect("changed source ref"),
        item.metadata().clone(),
    )
    .unwrap();
    let changed = service
        .request_proxy(&changed_item)
        .expect("request changed source proxy");
    assert_ne!(changed.cache_key, Some(key));
    assert!(changed.job_id.is_some());
    wait_for_success(&events, &changed, CacheArtifactKind::Proxy);
    assert_eq!(session.session().project_revision(), project_revision);
    service.shutdown();
}

fn wait_for_success(
    events: &Receiver<MediaArtifactEvent>,
    request: &MediaArtifactRequest,
    kind: CacheArtifactKind,
) -> MediaArtifactEvent {
    let job_id: JobId = request.job_id.expect("generation job id");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "media artifact job timed out");
        match events.recv_timeout(remaining) {
            Ok(event) if event.job_id == job_id => {
                assert_eq!(event.kind, kind);
                assert_eq!(event.state, MediaArtifactEventState::Succeeded);
                assert_eq!(event.error_code, None);
                return event;
            }
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => panic!("media artifact job timed out"),
            Err(RecvTimeoutError::Disconnected) => panic!("artifact event stream closed"),
        }
    }
}

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "artifact must be PNG"
    );
    assert!(bytes.len() >= 24, "PNG contains IHDR dimensions");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}

fn generate_media(path: &Path) {
    let generated = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=64x48:rate=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000",
            "-t",
            "1",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "ffv1",
            "-level",
            "3",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "pcm_s16le",
            "-shortest",
            "-y",
        ])
        .arg(path)
        .output()
        .expect("run CI-installed ffmpeg to create a legal synthetic fixture");
    assert!(
        generated.status.success(),
        "ffmpeg fixture generation failed: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
}

fn generate_vfr_media(path: &Path, video_source: &str, title: &str) {
    let source = format!("{video_source}=size=96x64:rate=4:duration=1.5");
    let generated = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(source)
        .args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"])
        .args([
            "-vf",
            "setpts=N+floor(N/2)",
            "-fps_mode",
            "passthrough",
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "ffv1",
            "-level",
            "3",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "pcm_s16le",
            "-shortest",
            "-metadata",
        ])
        .arg(format!("title={title}"))
        .args(["-metadata", "artist=OR test", "-y"])
        .arg(path)
        .output()
        .expect("run CI-installed ffmpeg to create a tiny VFR fixture");
    assert!(
        generated.status.success(),
        "VFR fixture generation failed: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
}

fn ffprobe_json(path: &Path) -> serde_json::Value {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .expect("run CI-installed ffprobe");
    assert!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid ffprobe JSON")
}

fn ffprobe_video_pts(path: &Path) -> Vec<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_frames",
            "-show_entries",
            "frame=best_effort_timestamp_time",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("run CI-installed ffprobe for frame timing");
    assert!(
        output.status.success(),
        "ffprobe frame inspection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.trim().trim_end_matches(',').parse().ok())
        .collect()
}

fn has_variable_frame_spacing(timestamps: &[f64]) -> bool {
    let mut intervals = timestamps
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    intervals.sort_by(f64::total_cmp);
    intervals
        .first()
        .zip(intervals.last())
        .is_some_and(|(min, max)| max - min > 0.05)
}
