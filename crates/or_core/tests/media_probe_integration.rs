use or_core::{
    MediaStreamMetadata, ProjectFileSession, ProjectRevision, decode_project, prepare_media_import,
    probe_media_file,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "or-media-probe-integration-{}-{}",
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
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires system ffmpeg and ffprobe; hosted Linux CI runs this explicitly"]
fn ffprobe_inspects_generated_video_and_audio_media() {
    let directory = TestDirectory::new();
    let path = directory.media_path();
    generate_media(&path);

    let metadata = probe_media_file(&path).expect("probe generated media with real ffprobe");
    assert!(metadata.file_size_bytes() > 0);
    assert!(
        metadata
            .duration()
            .is_some_and(|duration| duration.numerator() > 0)
    );
    assert!(metadata.streams().iter().any(|stream| matches!(
        stream,
        MediaStreamMetadata::Video(video) if video.width() == 16 && video.height() == 16
    )));
    assert!(metadata.streams().iter().any(|stream| matches!(
        stream,
        MediaStreamMetadata::Audio(audio) if audio.sample_rate() == Some(48_000)
    )));
}

#[test]
#[ignore = "requires system ffmpeg and ffprobe; hosted Linux CI runs this explicitly"]
fn prepared_import_persists_and_reopens_after_source_goes_offline() {
    let directory = TestDirectory::new();
    let media_path = directory.media_path();
    generate_media(&media_path);

    let prepared = prepare_media_import(&media_path).expect("prepare generated media");
    let prepared_again = prepare_media_import(&media_path).expect("prepare same source again");
    assert_ne!(prepared.id(), prepared_again.id());
    assert!(prepared.source().uri().starts_with("file:///"));
    assert_eq!(prepared.metadata(), &probe_media_file(&media_path).unwrap());

    let project_path = directory.project_path();
    let mut session =
        ProjectFileSession::create_new(&project_path, "Media import").expect("create project file");
    let original_project_bytes = fs::read(&project_path).unwrap();
    let (imported, command) = session.import_media(&media_path).expect("import media");
    assert_ne!(imported.id(), prepared.id());
    assert_eq!(imported.source(), prepared.source());
    assert_eq!(imported.metadata(), prepared.metadata());
    assert_eq!(command.before_revision, ProjectRevision::INITIAL);
    assert_eq!(command.after_revision, ProjectRevision::new(1));
    assert!(session.is_dirty());
    assert_eq!(fs::read(&project_path).unwrap(), original_project_bytes);
    assert!(
        decode_project(&fs::read_to_string(&project_path).unwrap())
            .unwrap()
            .media_items()
            .is_empty()
    );

    session.save().expect("save imported media");
    assert!(!session.is_dirty());
    drop(session);
    fs::remove_file(&media_path).unwrap();

    let reopened = ProjectFileSession::open(&project_path).expect("offline media must not block");
    assert_eq!(reopened.session().project().media_items(), &[imported]);
    assert_eq!(
        reopened.session().project_revision(),
        ProjectRevision::new(1)
    );
}

fn generate_media(path: &std::path::Path) {
    let generated = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=16x16:rate=2",
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
        .expect("run CI-installed ffmpeg to make a legal synthetic fixture");
    assert!(
        generated.status.success(),
        "ffmpeg fixture generation failed: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
}
