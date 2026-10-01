use ffmpeg_the_third as ffmpeg;
use or_media::MatroskaFfv1PcmS16leWriter;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "or-software-export-{}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("could not create export test directory: {error}"),
            }
        }
    }

    fn file(&self) -> PathBuf {
        self.0.join("profile.mkv")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn writer_publishes_matroska_with_ffv1_and_pcm_s16le_streams() {
    ffmpeg::init().unwrap();
    let directory = TestDirectory::new();
    let path = directory.file();
    let mut writer = MatroskaFfv1PcmS16leWriter::create(&path, 4, 2, 24, 1).unwrap();
    let first = vec![255; 4 * 2 * 4];
    let second = vec![64; 4 * 2 * 4];
    writer.write_video_frame(&first, 4, 2, 0).unwrap();
    writer.write_video_frame(&second, 4, 2, 1).unwrap();
    writer.write_audio_frames(&vec![0; 960], 0).unwrap();
    writer.finish().unwrap();

    let input = ffmpeg::format::input(&path).unwrap();
    assert!(
        input
            .format()
            .name()
            .split(',')
            .any(|name| name == "matroska")
    );
    let codecs: Vec<_> = input
        .streams()
        .map(|stream| stream.parameters().id())
        .collect();
    assert!(codecs.contains(&ffmpeg::codec::Id::FFV1));
    assert!(codecs.contains(&ffmpeg::codec::Id::PCM_S16LE));
    assert_eq!(codecs.len(), 2);
}

#[test]
fn dropping_an_incomplete_export_preserves_destination_and_removes_staging() {
    let directory = TestDirectory::new();
    let path = directory.file();
    fs::write(&path, b"existing export").unwrap();
    {
        let _writer = MatroskaFfv1PcmS16leWriter::create(&path, 4, 2, 24, 1).unwrap();
        let staging = fs::read_dir(&directory.0)
            .unwrap()
            .map(Result::unwrap)
            .find(|entry| entry.file_type().unwrap().is_dir())
            .unwrap()
            .path();
        assert!(staging.is_dir());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(staging).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
    assert_eq!(fs::read(&path).unwrap(), b"existing export");
    let remaining: Vec<_> = fs::read_dir(&directory.0)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].path(), path);
}
