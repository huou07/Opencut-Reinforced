use or_core::RationalTime;
use or_runtime::{ViewerFrameLease, ViewerTextureAdapter};
use std::{ffi::c_void, slice, sync::OnceLock};

static VIEWER_TEXTURE_ADAPTER: OnceLock<ViewerTextureAdapter> = OnceLock::new();

/// Loads the FFmpeg shared libraries from the packaged app search path.
#[unsafe(no_mangle)]
pub extern "C" fn or_ffmpeg_runtime_check() -> bool {
    or_media::verify_ffmpeg_runtime()
}

fn adapter() -> &'static ViewerTextureAdapter {
    VIEWER_TEXTURE_ADAPTER.get_or_init(ViewerTextureAdapter::default)
}

/// C ABI pixel data held until the Flutter platform release callback.
#[repr(C)]
pub struct OrViewerPixelBuffer {
    pub pixels: *const u8,
    pub width: usize,
    pub height: usize,
    pub release_context: *mut c_void,
}

/// Starts a newer seek/project generation and discards any older queued frame.
#[unsafe(no_mangle)]
pub extern "C" fn or_viewer_advance_generation(generation: u64) -> bool {
    adapter().advance_generation(generation)
}

/// Publishes CPU RGBA pixels from native runtime code; this is not a Dart API.
/// The adapter copies and converts them to premultiplied BGRA before Flutter
/// can request a texture frame.
#[unsafe(no_mangle)]
pub extern "C" fn or_viewer_publish_rgba(
    generation: u64,
    width: u32,
    height: u32,
    pixels: *const u8,
    length: usize,
) -> bool {
    if pixels.is_null() {
        return false;
    }
    let Some(expected_length) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixel_count| pixel_count.checked_mul(4))
        .filter(|byte_count| *byte_count <= 256 * 1024 * 1024)
    else {
        return false;
    };
    if expected_length != length {
        return false;
    }
    let rgba = unsafe { slice::from_raw_parts(pixels, length) };
    adapter()
        .publish_rgba(generation, width, height, RationalTime::ZERO, rgba)
        .is_ok()
}

/// Acquires the latest valid frame for a Flutter pixel-buffer callback.
/// `output->pixels` stays valid until `or_viewer_release_frame` is called.
#[unsafe(no_mangle)]
pub extern "C" fn or_viewer_acquire_latest(output: *mut OrViewerPixelBuffer) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(lease) = adapter().acquire_latest() else {
        return false;
    };
    let lease = Box::new(lease);
    let pixels = lease.pixels().as_ptr();
    let descriptor = lease.descriptor();
    let buffer = OrViewerPixelBuffer {
        pixels,
        width: descriptor.width() as usize,
        height: descriptor.height() as usize,
        release_context: Box::into_raw(lease).cast(),
    };
    unsafe { output.write(buffer) };
    true
}

/// Releases a frame after Flutter has finished consuming its pixel buffer.
#[unsafe(no_mangle)]
pub extern "C" fn or_viewer_release_frame(release_context: *mut c_void) {
    if !release_context.is_null() {
        drop(unsafe { Box::from_raw(release_context.cast::<ViewerFrameLease>()) });
    }
}
