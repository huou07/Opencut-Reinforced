use or_core::{MediaSourceRef, ProjectDocument, RationalTime, TimeRange};
#[cfg(unix)]
use or_media::SeekableMediaIoCapability;
use or_media::{DecodeError, SnapshotQueue, SoftwareMediaDecoder};
use or_runtime::{
    BudgetLimits, CancellationToken, RenderSnapshot, ResourceBudgetMetrics, RuntimeBudgets,
};
use std::path::{Path, PathBuf};

const FIXTURE: &str = "tests/fixtures/tiny.mkv";

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
