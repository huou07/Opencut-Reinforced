//! Measures 1080p decode throughput on this machine.
//!
//! This is not a correctness gate: it exists so a hardware or performance claim
//! about preview decode can cite a number produced on the machine that runs the
//! editor, instead of an assumption.

use or_core::{MediaSourceRef, ProjectDocument, RationalTime, TimeRange};
use or_media::SoftwareMediaDecoder;
use or_runtime::{
    BudgetLimits, CancellationToken, RenderSnapshot, RuntimeBudgets,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const FIXTURE: &str = "tests/fixtures/big_buck_bunny_1080p_h264_aac.mp4";
const FRAMES: i64 = 90;

fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(uri, "%{byte:02X}").unwrap();
        }
    }
    uri
}

fn budgets() -> RuntimeBudgets {
    RuntimeBudgets::new(
        BudgetLimits::new(8, 64 * 1024 * 1024).unwrap(),
        BudgetLimits::new(8, 64 * 1024 * 1024).unwrap(),
        BudgetLimits::new(8, 64 * 1024 * 1024).unwrap(),
    )
}

#[test]
#[ignore = "measures decode throughput on this machine, not a correctness gate"]
fn decode_1080p_throughput() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let source = MediaSourceRef::local_file(file_uri(&path)).unwrap();
    let project = ProjectDocument::new("decode throughput");
    let project_snapshot = RenderSnapshot::from_project(
        &project,
        TimeRange::new(RationalTime::new(0, 1).unwrap(), RationalTime::new(10, 1).unwrap())
            .unwrap(),
    );
    let _ = project_snapshot;

    let decoder = SoftwareMediaDecoder::new(&source, budgets()).unwrap();
    let cancellation = CancellationToken::new();
    let mut session = decoder.open_video_session(&cancellation).unwrap();
    let started = Instant::now();
    for index in 0..FRAMES {
        let timestamp = RationalTime::new(index * 30, 30).unwrap();
        let decoded = session
            .frame_at(timestamp, &cancellation)
            .unwrap()
            .expect("the fixture covers ten seconds");
        std::hint::black_box(decoded.pixels().len());
    }
    let elapsed = started.elapsed();
    let fps = FRAMES as f64 / elapsed.as_secs_f64();
    eprintln!(
        "OR_DECODE_THROUGHPUT frames={} elapsed_ms={} fps={fps:.1}",
        FRAMES,
        elapsed.as_millis()
    );
}
