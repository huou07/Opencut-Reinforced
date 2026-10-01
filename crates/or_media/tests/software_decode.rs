use or_core::{MediaSourceRef, ProjectDocument, RationalTime, TimeRange};
use or_media::{DecodeError, SnapshotQueue, SoftwareMediaDecoder};
use or_runtime::{
    BudgetLimits, CancellationToken, RenderSnapshot, ResourceBudgetMetrics, RuntimeBudgets,
};
use std::{path::Path, path::PathBuf, sync::Mutex};

const FIXTURE: &str = "tests/fixtures/tiny.mkv";
// ponytail: serialize FFmpeg initialization and decoding until tests have independent resources.
static FFMPEG_TEST_LOCK: Mutex<()> = Mutex::new(());

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
    let _guard = FFMPEG_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
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

#[test]
fn software_video_preview_holds_the_preceding_source_presentation_timestamp() {
    let _guard = FFMPEG_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
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
    let _guard = FFMPEG_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
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
    let _guard = FFMPEG_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
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
