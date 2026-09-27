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

    let changed_item = MediaItem::new(
        item.id(),
        MediaSourceRef::local_file(changed_path.to_string_lossy()).expect("same-size source path"),
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
