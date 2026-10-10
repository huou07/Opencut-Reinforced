use ffmpeg_the_third as ffmpeg;
use or_media::{FfmpegSoftwareExportWriter, MatroskaFfv1PcmS16leWriter, SoftwareExportProfile};
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

    fn webm_file(&self) -> PathBuf {
        self.0.join("profile.webm")
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
#[ignore = "run against the codec-enabled FFmpeg packaging profile"]
fn writer_publishes_webm_with_vp9_and_opus_streams() {
    ffmpeg::init().unwrap();
    let directory = TestDirectory::new();
    let path = directory.webm_file();
    let mut writer = FfmpegSoftwareExportWriter::create_with_profile(
        &path,
        SoftwareExportProfile::WebmVp9Opus,
        64,
        48,
        30,
        1,
    )
    .expect("this integration test requires the pinned VP9 and Opus encoders");
    for frame_index in 0..10 {
        let mut rgba = vec![0; 64 * 48 * 4];
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&[200, 100, 50, 255]);
        }
        writer
            .write_video_frame(&rgba, 64, 48, frame_index as u64)
            .unwrap();
    }
    for sample_frame_offset in (0..16_000_u64).step_by(2_048) {
        let frames = (16_000 - sample_frame_offset).min(2_048) as usize;
        let samples = vec![8_000_i16; frames * 2];
        writer
            .write_audio_frames(&samples, sample_frame_offset)
            .unwrap();
    }
    writer.finish().unwrap();

    let mut input = ffmpeg::format::input(&path).unwrap();
    assert!(
        input
            .format()
            .name()
            .split(',')
            .any(|name| name == "matroska,webm" || name == "webm")
    );
    let streams: Vec<_> = input
        .streams()
        .map(|stream| (stream.parameters().id(), stream.index()))
        .collect();
    assert_eq!(streams.len(), 2);
    assert!(
        streams
            .iter()
            .any(|(codec, _)| *codec == ffmpeg::codec::Id::VP9)
    );
    assert!(
        streams
            .iter()
            .any(|(codec, _)| *codec == ffmpeg::codec::Id::OPUS)
    );
    let video_stream = input
        .streams()
        .find(|stream| stream.parameters().id() == ffmpeg::codec::Id::VP9)
        .unwrap();
    let video_index = video_stream.index();
    let mut video_decoder =
        ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .unwrap()
            .decoder()
            .video()
            .unwrap();
    let audio_stream = input
        .streams()
        .find(|stream| stream.parameters().id() == ffmpeg::codec::Id::OPUS)
        .unwrap();
    let audio_index = audio_stream.index();
    let mut audio_decoder =
        ffmpeg::codec::context::Context::from_parameters(audio_stream.parameters())
            .unwrap()
            .decoder()
            .audio()
            .unwrap();
    let mut decoded_video_frames = 0;
    let mut decoded_color = None;
    let mut decoded_audio_frames = 0_usize;
    for (stream, packet) in input.packets().map(Result::unwrap) {
        if stream.index() == video_index {
            video_decoder.send_packet(&packet).unwrap();
            drain_video(
                &mut video_decoder,
                &mut decoded_video_frames,
                &mut decoded_color,
            )
            .unwrap();
        } else if stream.index() == audio_index {
            audio_decoder.send_packet(&packet).unwrap();
            drain_audio(&mut audio_decoder, &mut decoded_audio_frames).unwrap();
        }
    }
    video_decoder.send_eof().unwrap();
    drain_video(
        &mut video_decoder,
        &mut decoded_video_frames,
        &mut decoded_color,
    )
    .unwrap();
    audio_decoder.send_eof().unwrap();
    drain_audio(&mut audio_decoder, &mut decoded_audio_frames).unwrap();
    assert_eq!(decoded_video_frames, 10);
    assert_eq!(decoded_audio_frames, 16_000);
    assert!(matches!(
        decoded_color,
        Some((_, ffmpeg::color::Space::BT709, ffmpeg::color::Range::MPEG))
    ));
    assert!(fs::metadata(&path).unwrap().len() < 1_000_000);
}

fn drain_video(
    decoder: &mut ffmpeg::codec::decoder::Video,
    decoded_frames: &mut usize,
    color: &mut Option<((u8, u8, u8), ffmpeg::color::Space, ffmpeg::color::Range)>,
) -> Result<(), ffmpeg::Error> {
    loop {
        let mut frame = ffmpeg::frame::Video::empty();
        match decoder.receive_frame(&mut frame) {
            Ok(()) => {
                if color.is_none() {
                    let mut scaler = ffmpeg::software::scaling::Context::get(
                        frame.format(),
                        frame.width(),
                        frame.height(),
                        ffmpeg::format::Pixel::RGB24,
                        frame.width(),
                        frame.height(),
                        ffmpeg::software::scaling::Flags::BILINEAR,
                    )?;
                    let mut rgb = ffmpeg::frame::Video::empty();
                    scaler.run(&frame, &mut rgb)?;
                    let pixel = rgb.data(0);
                    let measured = (pixel[0], pixel[1], pixel[2]);
                    for (actual, expected) in [measured.0, measured.1, measured.2]
                        .into_iter()
                        .zip([200_u8, 100_u8, 50_u8])
                    {
                        assert!(
                            actual.abs_diff(expected) <= 18,
                            "decoded WebM RGB sample {measured:?} differs from the source"
                        );
                    }
                    *color = Some((measured, frame.color_space(), frame.color_range()));
                }
                *decoded_frames += 1;
            }
            Err(error) if decoder_drain_complete(error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn drain_audio(
    decoder: &mut ffmpeg::codec::decoder::Audio,
    decoded_frames: &mut usize,
) -> Result<(), ffmpeg::Error> {
    loop {
        let mut frame = ffmpeg::frame::Audio::empty();
        match decoder.receive_frame(&mut frame) {
            Ok(()) => *decoded_frames += frame.samples(),
            Err(error) if decoder_drain_complete(error) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn decoder_drain_complete(error: ffmpeg::Error) -> bool {
    match error {
        ffmpeg::Error::Eof => true,
        ffmpeg::Error::Other { errno } => errno == if cfg!(target_os = "macos") { 35 } else { 11 },
        _ => false,
    }
}

#[test]
fn webm_missing_encoder_preserves_an_existing_destination() {
    ffmpeg::init().unwrap();
    let directory = TestDirectory::new();
    let path = directory.webm_file();
    fs::write(&path, b"existing export").unwrap();
    let result = FfmpegSoftwareExportWriter::create_with_profile(
        &path,
        SoftwareExportProfile::WebmVp9Opus,
        64,
        48,
        30,
        1,
    );
    let error = match result {
        Ok(writer) => {
            drop(writer);
            return;
        }
        Err(error) => error.to_string(),
    };
    assert!(error.contains("no libvpx VP9 encoder") || error.contains("no libopus encoder"));
    assert_eq!(fs::read(&path).unwrap(), b"existing export");
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn webm_rejects_odd_dimensions_before_creating_output() {
    ffmpeg::init().unwrap();
    let directory = TestDirectory::new();
    let path = directory.webm_file();
    let error = FfmpegSoftwareExportWriter::create_with_profile(
        &path,
        SoftwareExportProfile::WebmVp9Opus,
        63,
        48,
        30,
        1,
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("even video dimensions"));
    assert!(!path.exists());
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
fn exporting_over_an_existing_destination_replaces_it_atomically() {
    ffmpeg::init().unwrap();
    let directory = TestDirectory::new();
    let path = directory.file();
    let first = vec![255; 4 * 2 * 4];
    let mut writer = MatroskaFfv1PcmS16leWriter::create(&path, 4, 2, 24, 1).unwrap();
    writer.write_video_frame(&first, 4, 2, 0).unwrap();
    writer.write_audio_frames(&vec![0; 960], 0).unwrap();
    writer.finish().unwrap();
    let replaced_bytes = fs::read(&path).unwrap();

    // Saving over an existing export must succeed on every platform, including
    // Windows where a plain rename refuses to replace the destination, and must
    // never leave a half-written file behind.
    let second = vec![64; 4 * 2 * 4];
    let mut writer = MatroskaFfv1PcmS16leWriter::create(&path, 4, 2, 24, 1).unwrap();
    writer.write_video_frame(&second, 4, 2, 0).unwrap();
    writer.write_video_frame(&second, 4, 2, 1).unwrap();
    writer.write_audio_frames(&vec![0; 960], 0).unwrap();
    writer.finish().unwrap();

    let published = fs::read(&path).unwrap();
    assert_ne!(published, replaced_bytes);
    let input = ffmpeg::format::input(&path).unwrap();
    assert_eq!(input.streams().count(), 2);
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
