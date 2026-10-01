use std::alloc::{GlobalAlloc, Layout, System};
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use or_audio::{AudioBuffer, AudioClockMessage};
use or_core::{ProjectDocument, RationalTime};
use or_runtime::{CancellationToken, RenderSnapshot};

struct CountingAllocator;

static COUNT_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

// This integration-test binary has one test, so tracked allocations belong to
// the callback invocation rather than concurrent test cases.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: delegates the exact layout to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: pointer and layout came from the system allocator above.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: pointer and layout came from the system allocator above.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn device_callback_does_not_allocate() {
    let buffer =
        AudioBuffer::new(NonZeroUsize::new(4).unwrap(), NonZeroUsize::new(2).unwrap()).unwrap();
    let project = ProjectDocument::new("audio callback test");
    let snapshot = RenderSnapshot::at_time(&project, RationalTime::ZERO).unwrap();
    let clock = AudioClockMessage::new(NonZeroU32::new(48_000).unwrap(), snapshot);
    let (mut producer, mut consumer) = buffer.split(clock);
    let token = CancellationToken::new();
    producer
        .try_push(&[0.25, -0.25, 0.5, -0.5], &token)
        .unwrap();
    let mut output = [0.0; 8];

    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::SeqCst);
    let report = consumer.render_into(&mut output, &token).unwrap();
    COUNT_ALLOCATIONS.store(false, Ordering::SeqCst);

    assert_eq!(ALLOCATION_COUNT.load(Ordering::Relaxed), 0);
    assert_eq!(output, [0.25, -0.25, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(report.underrun_frames, 2);
}
