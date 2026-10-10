use or_core::{MediaSourceRef, ProjectDocument, RationalTime, TimeRange};
#[cfg(unix)]
use or_media::SeekableMediaIoCapability;
use or_media::{DecodeError, SnapshotQueue, SoftwareMediaDecoder};
use or_runtime::{
    BudgetLimits, CancellationToken, RenderSnapshot, ResourceBudgetMetrics, RuntimeBudgets,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const FIXTURE: &str = "tests/fixtures/tiny.mkv";
const AUDIO_FIXTURE: &str = "tests/fixtures/tiny.wav";
const PHONE_FIXTURE: &str = "tests/fixtures/tiny_h264_aac.mp4";
const BIG_BUCK_BUNNY_FIXTURE: &str = "tests/fixtures/big_buck_bunny_1080p_h264_aac.mp4";
const PHONE_PORTRAIT_FIXTURE: &str = "tests/fixtures/phone_portrait_90_h264_aac.mp4";

fn time(numerator: i64, denominator: u32) -> RationalTime {
    RationalTime::new(numerator, denominator).unwrap()
}

fn snapshot(start: RationalTime, duration: RationalTime) -> RenderSnapshot {
    let project = ProjectDocument::new("generated media fixture");
    RenderSnapshot::from_project(&project, TimeRange::new(start, duration).unwrap())
}

fn budgets() -> RuntimeBudgets {
    RuntimeBudgets::new(
        BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
        BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
        BudgetLimits::new(8, 32 * 1024 * 1024).unwrap(),
    )
}

fn fixture_source() -> MediaSourceRef {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    MediaSourceRef::local_file(file_uri(&path)).unwrap()
}

fn phone_fixture_source() -> MediaSourceRef {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PHONE_FIXTURE);
    MediaSourceRef::local_file(file_uri(&path)).unwrap()
}

fn big_buck_bunny_fixture_source() -> MediaSourceRef {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BIG_BUCK_BUNNY_FIXTURE);
    MediaSourceRef::local_file(file_uri(&path)).unwrap()
}

fn phone_portrait_fixture_source() -> MediaSourceRef {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PHONE_PORTRAIT_FIXTURE);
    MediaSourceRef::local_file(file_uri(&path)).unwrap()
}

fn file_uri(path: &Path) -> String {
    #[cfg(windows)]
    let (prefix, path) = ("file:///", path.to_string_lossy().replace('\\', "/"));
    #[cfg(not(windows))]
    let (prefix, path) = ("file://", path.to_string_lossy().into_owned());

    let mut uri = String::from(prefix);
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(uri, "%{byte:02X}").unwrap();
        }
    }
    uri
}

#[test]
fn software_video_seek_produces_an_owned_exact_time_rgba_frame() {
    let snapshot = snapshot(time(1, 4), time(1, 4));
    let budgets = budgets();
    let decoder = SoftwareMediaDecoder::new(&fixture_source(), budgets.clone()).unwrap();
    let queue = SnapshotQueue::new(snapshot, 4).unwrap();
    let cancellation = CancellationToken::new();

    assert_eq!(
        decoder
            .decode_video(snapshot, &queue, &cancellation)
            .unwrap(),
        1
    );
    assert_eq!(queue.capacity(), 4);
    assert_eq!(queue.len(), 1);
    assert_eq!(
        budgets.decode().metrics(),
        ResourceBudgetMetrics {
            in_flight: 1,
            bytes_in_use: 1024,
            peak_in_flight: 1,
            peak_bytes: 1024,
            successful_acquisitions: 1,
            in_flight_rejections: 0,
            byte_rejections: 0,
        }
    );
    let item = queue.try_pop_current().unwrap().unwrap();
    let frame = item.into_value();
    assert_eq!(frame.descriptor().width(), 16);
    assert_eq!(frame.descriptor().height(), 16);
    assert_eq!(frame.descriptor().timing().timestamp(), time(1, 4));
    assert_eq!(frame.pixels().len(), 16 * 16 * 4);
    assert_eq!(frame.pixels()[3], 255);
    assert_eq!(budgets.decode().bytes_in_use(), 16 * 16 * 4);

    drop(frame);
    assert_eq!(budgets.decode().bytes_in_use(), 0);
    assert_eq!(budgets.decode().metrics().in_flight, 0);
    assert_eq!(budgets.decode().metrics().peak_bytes, 1024);
}

#[cfg(unix)]
#[test]
fn software_decoder_uses_transient_seekable_io_for_a_saf_source() {
    let source = MediaSourceRef::android_saf_document_uri(
        "content://com.android.providers.media.documents/document/video%3A42",
    )
    .unwrap();
    assert!(source.to_file_path().is_err());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let capability =
        SeekableMediaIoCapability::from_file(std::fs::File::open(path).unwrap()).unwrap();
    let decoder = SoftwareMediaDecoder::new_with_seekable_io(capability, budgets()).unwrap();
    let frame = decoder
        .decode_video_frame_at(time(1, 4), &CancellationToken::new())
        .unwrap()
        .unwrap();

    assert_eq!(frame.descriptor().timing().timestamp(), time(1, 4));
    assert_eq!(frame.pixels().len(), 16 * 16 * 4);
}

#[cfg(unix)]
#[test]
fn seekable_saf_probe_returns_metadata_accepted_by_the_core_import_matrix() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let capability =
        SeekableMediaIoCapability::from_file(std::fs::File::open(path).unwrap()).unwrap();
    let size = capability.len();
    let output = or_media::probe_seekable_media(&capability).unwrap();
    let metadata = or_core::parse_media_probe_output(&output, size).unwrap();

    assert!(
        metadata
            .format_names()
            .iter()
            .any(|name| name == "matroska")
    );
    assert_eq!(metadata.streams().len(), 2);
    assert_eq!(metadata.duration().unwrap().numerator(), 1);
    assert_eq!(metadata.duration().unwrap().denominator(), 2);
    match &metadata.streams()[0] {
        or_core::MediaStreamMetadata::Video(video) => {
            assert_eq!(video.codec_name(), Some("ffv1"));
            assert_eq!((video.width(), video.height()), (16, 16));
        }
        _ => panic!("expected fixture video stream"),
    }
    match &metadata.streams()[1] {
        or_core::MediaStreamMetadata::Audio(audio) => {
            assert_eq!(audio.codec_name(), Some("pcm_s16le"));
            assert_eq!(audio.sample_rate(), Some(8_000));
            assert_eq!(audio.channels(), Some(1));
        }
        _ => panic!("expected fixture audio stream"),
    }
}

#[test]
fn software_video_preview_holds_the_preceding_source_presentation_timestamp() {
    let decoder = SoftwareMediaDecoder::new(&fixture_source(), budgets()).unwrap();
    let cancellation = CancellationToken::new();

    let frame = decoder
        .decode_video_frame_at(time(3, 8), &cancellation)
        .unwrap()
        .unwrap();

    assert_eq!(frame.descriptor().timing().timestamp(), time(1, 4));
    assert_eq!(frame.pixels().len(), 16 * 16 * 4);
}

#[test]
fn software_video_session_reuses_sequential_decode_and_seeks_on_discontinuity() {
    let baseline_budgets = budgets();
    let baseline_decoder = SoftwareMediaDecoder::new(&fixture_source(), baseline_budgets).unwrap();
    let cancellation = CancellationToken::new();
    let baseline_started = Instant::now();
    for _ in 0..4 {
        let preceding = baseline_decoder
            .decode_video_frame_at(time(1, 4), &cancellation)
            .unwrap()
            .unwrap();
        let sequential = baseline_decoder
            .decode_video_frame_at(time(3, 10), &cancellation)
            .unwrap()
            .unwrap();
        assert_eq!(preceding.descriptor().timing().timestamp(), time(1, 4));
        assert_eq!(preceding.pixels(), sequential.pixels());
    }
    let baseline_elapsed = baseline_started.elapsed();

    let runtime_budgets = budgets();
    let decoder = SoftwareMediaDecoder::new(&fixture_source(), runtime_budgets.clone()).unwrap();
    let session_started = Instant::now();
    let mut session = decoder.open_video_session(&cancellation).unwrap();
    for _ in 0..4 {
        let preceding = session
            .frame_at(time(1, 4), &cancellation)
            .unwrap()
            .unwrap();
        let sequential = session
            .frame_at(time(3, 10), &cancellation)
            .unwrap()
            .unwrap();
        assert_eq!(preceding.descriptor().timing().timestamp(), time(1, 4));
        assert_eq!(preceding.pixels(), sequential.pixels());
    }
    let session_elapsed = session_started.elapsed();
    assert_eq!(session.metrics().seeks, 4);
    assert_eq!(session.metrics().decoded_frames, 4);
    eprintln!(
        "OR_DECODER_SESSION_MEASUREMENT baseline_opens=8 baseline_seeks=8 baseline_elapsed_us={} session_opens=1 session_seeks={} session_decoded_frames={} session_elapsed_us={} session_peak_decode_bytes={}",
        baseline_elapsed.as_micros(),
        session.metrics().seeks,
        session.metrics().decoded_frames,
        session_elapsed.as_micros(),
        runtime_budgets.decode().metrics().peak_bytes,
    );

    let forward_jump = session
        .frame_at(time(2, 1), &cancellation)
        .unwrap()
        .unwrap();
    assert_eq!(forward_jump.descriptor().timing().timestamp(), time(1, 4));
    assert_eq!(session.metrics().seeks, 4);

    let discontinuous = session
        .frame_at(RationalTime::ZERO, &cancellation)
        .unwrap()
        .unwrap();
    assert_eq!(
        discontinuous.descriptor().timing().timestamp(),
        RationalTime::ZERO
    );
    assert_eq!(session.metrics().seeks, 5);
    assert_eq!(session.metrics().decoded_frames, 6);
    drop((forward_jump, discontinuous, session));
    assert_eq!(runtime_budgets.decode().bytes_in_use(), 0);
}

#[test]
fn cancelled_software_video_seek_returns_before_opening_media() {
    let decoder = SoftwareMediaDecoder::new(&fixture_source(), budgets()).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    assert!(matches!(
        decoder.decode_video_frame_at(time(1, 4), &cancellation),
        Err(DecodeError::Cancelled)
    ));
}

#[test]
fn cancelled_software_video_session_returns_before_opening_media() {
    let missing = std::env::temp_dir().join(format!(
        "or-missing-video-session-{}.mkv",
        std::process::id()
    ));
    let source = MediaSourceRef::local_file(file_uri(&missing)).unwrap();
    let decoder = SoftwareMediaDecoder::new(&source, budgets()).unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    assert!(matches!(
        decoder.open_video_session(&cancellation),
        Err(DecodeError::Cancelled)
    ));
}

#[test]
fn software_audio_seek_resamples_and_clips_to_the_exact_requested_range() {
    let snapshot = snapshot(time(1, 4), time(1, 4));
    let decoder = SoftwareMediaDecoder::new(&fixture_source(), budgets()).unwrap();
    let queue = SnapshotQueue::new(snapshot, 8).unwrap();
    let cancellation = CancellationToken::new();

    assert!(
        decoder
            .decode_audio(snapshot, &queue, &cancellation)
            .unwrap()
            > 0
    );
    let mut sample_frames = 0;
    while let Some(item) = queue.try_pop_current().unwrap() {
        let chunk = item.into_value();
        assert_eq!(chunk.sample_rate(), 48_000);
        assert_eq!(chunk.channels(), 2);
        sample_frames += chunk.sample_frames();
    }
    assert_eq!(sample_frames, 12_000);
}

#[test]
fn software_decoder_reads_audio_only_pcm_wav() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(AUDIO_FIXTURE);
    let source = MediaSourceRef::local_file(file_uri(&path)).unwrap();
    let decoder = SoftwareMediaDecoder::new(&source, budgets()).unwrap();
    let snapshot = snapshot(time(0, 1), time(1, 4));
    let queue = SnapshotQueue::new(snapshot, 16).unwrap();
    let cancellation = CancellationToken::new();
    assert!(
        decoder
            .decode_audio(snapshot, &queue, &cancellation)
            .unwrap()
            > 0
    );
    let mut decoded_sample_frames = 0;
    while let Some(item) = queue.try_pop_current().unwrap() {
        let chunk = item.into_value();
        assert_eq!(chunk.sample_rate(), 48_000);
        assert_eq!(chunk.channels(), 2);
        decoded_sample_frames += chunk.sample_frames();
    }
    assert_eq!(decoded_sample_frames, 12_000);
}

#[test]
fn software_decoder_reads_h264_video_and_aac_audio_from_phone_mp4() {
    let decoder = SoftwareMediaDecoder::new(&phone_fixture_source(), budgets()).unwrap();
    let cancellation = CancellationToken::new();

    let frame = decoder
        .decode_video_frame_at(time(1, 4), &cancellation)
        .unwrap()
        .unwrap();
    assert_eq!(
        (frame.descriptor().width(), frame.descriptor().height()),
        (32, 24)
    );
    assert_eq!(frame.pixels().len(), 32 * 24 * 4);

    let audio_snapshot = snapshot(time(0, 1), time(1, 8));
    let audio_queue = SnapshotQueue::new(audio_snapshot, 8).unwrap();
    assert!(
        decoder
            .decode_audio(audio_snapshot, &audio_queue, &cancellation)
            .unwrap()
            > 0
    );
    let mut decoded_sample_frames = 0;
    while let Some(item) = audio_queue.try_pop_current().unwrap() {
        let chunk = item.into_value();
        assert_eq!(chunk.sample_rate(), 48_000);
        assert_eq!(chunk.channels(), 2);
        decoded_sample_frames += chunk.sample_frames();
    }
    assert_eq!(decoded_sample_frames, 6_000);

    // AAC packets can begin before the requested edit point. Seeking with the
    // codec's priming interval must still produce an exact half-open range.
    let audio_snapshot = snapshot(time(1, 4), time(1, 8));
    let audio_queue = SnapshotQueue::new(audio_snapshot, 8).unwrap();
    assert!(
        decoder
            .decode_audio(audio_snapshot, &audio_queue, &cancellation)
            .unwrap()
            > 0
    );
    let mut decoded_sample_frames = 0;
    while let Some(item) = audio_queue.try_pop_current().unwrap() {
        let chunk = item.into_value();
        assert_eq!(chunk.sample_rate(), 48_000);
        assert_eq!(chunk.channels(), 2);
        decoded_sample_frames += chunk.sample_frames();
    }
    assert_eq!(decoded_sample_frames, 6_000);
}

#[test]
fn software_decoder_reads_real_1080p_h264_and_surround_aac_media() {
    let decoder = SoftwareMediaDecoder::new(&big_buck_bunny_fixture_source(), budgets()).unwrap();
    let cancellation = CancellationToken::new();

    let frame = decoder
        .decode_video_frame_at(time(1, 4), &cancellation)
        .unwrap()
        .unwrap();
    assert_eq!(
        (frame.descriptor().width(), frame.descriptor().height()),
        (1920, 1080)
    );
    assert_eq!(frame.pixels().len(), 1920 * 1080 * 4);
    drop(frame);

    let audio_snapshot = snapshot(time(1, 4), time(1, 8));
    let audio_queue = SnapshotQueue::new(audio_snapshot, 8).unwrap();
    assert!(
        decoder
            .decode_audio(audio_snapshot, &audio_queue, &cancellation)
            .unwrap()
            > 0
    );
    let mut decoded_sample_frames = 0;
    while let Some(item) = audio_queue.try_pop_current().unwrap() {
        let chunk = item.into_value();
        assert_eq!(chunk.sample_rate(), 48_000);
        assert_eq!(chunk.channels(), 2);
        decoded_sample_frames += chunk.sample_frames();
    }
    assert_eq!(decoded_sample_frames, 6_000);
}

#[test]
fn software_decoder_applies_phone_display_rotation_to_h264_video() {
    let decoder = SoftwareMediaDecoder::new(&phone_portrait_fixture_source(), budgets()).unwrap();
    let cancellation = CancellationToken::new();
    let frame = decoder
        .decode_video_frame_at(time(1, 4), &cancellation)
        .unwrap()
        .unwrap();

    assert_eq!(
        (frame.descriptor().width(), frame.descriptor().height()),
        (1080, 1920)
    );
    assert_eq!(frame.pixels().len(), 1080 * 1920 * 4);
}

#[cfg(unix)]
#[test]
fn granted_private_file_decodes_without_reopening_and_clones_have_independent_cursors() {
    use std::{
        fs::{File, Permissions},
        io::Read,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "or-granted-private-{}-{stamp}.mkv",
        std::process::id()
    ));
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    std::fs::copy(fixture, &path).unwrap();
    let file = File::open(&path).unwrap();
    let raw_fd = std::os::fd::AsRawFd::as_raw_fd(&file);
    std::fs::set_permissions(&path, Permissions::from_mode(0o000)).unwrap();
    // Normal unprivileged CI cannot reopen this inode. The existing grant can
    // still read it; this matches provider-owned private Android media.
    assert!(
        File::open(&path).is_err(),
        "Run this permission guard without root privileges"
    );
    #[cfg(target_os = "linux")]
    assert!(File::open(format!("/proc/self/fd/{raw_fd}")).is_err());
    #[cfg(not(target_os = "linux"))]
    let _ = raw_fd;
    let mut direct = &file;
    let mut header = [0; 4];
    direct.read_exact(&mut header).unwrap();
    assert_eq!(header, [0x1a, 0x45, 0xdf, 0xa3]);
    let capability = SeekableMediaIoCapability::from_file(file).unwrap();
    let first = SoftwareMediaDecoder::new_with_seekable_io(capability.clone(), budgets()).unwrap();
    let second = SoftwareMediaDecoder::new_with_seekable_io(capability, budgets()).unwrap();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for _ in 0..8 {
                let frame = first
                    .decode_video_frame_at(time(1, 4), &CancellationToken::new())
                    .unwrap()
                    .unwrap();
                assert_eq!(frame.descriptor().timing().timestamp(), time(1, 4));
                assert_eq!(frame.pixels().len(), 1024);
            }
        });
        scope.spawn(|| {
            for _ in 0..8 {
                let frame = second
                    .decode_video_frame_at(time(3, 4), &CancellationToken::new())
                    .unwrap()
                    .unwrap();
                // tiny.mkv carries frames only through 1/4, so a later target
                // correctly resolves to the greatest frame at or before it;
                // the file-path decoder agrees (see scratch parity probe).
                assert_eq!(frame.descriptor().timing().timestamp(), time(1, 4));
                assert_eq!(frame.pixels().len(), 1024);
            }
        });
    });
    drop(first);
    drop(second);
    std::fs::remove_file(path).unwrap();
}
