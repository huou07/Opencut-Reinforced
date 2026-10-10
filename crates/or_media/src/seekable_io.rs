//! Direct FFmpeg I/O over a granted descriptor; never reopen its private inode.
use ffmpeg_the_third::{Error, ffi, format};
use or_runtime::CancellationToken;
use std::{
    ffi::c_void,
    fs::File,
    os::unix::fs::FileExt,
    ptr,
    sync::Arc,
    time::{Duration, Instant},
};

const BUFFER_BYTES: usize = 32 * 1024;
// POSIX values shared by the supported Android/Linux/macOS targets.
const EINVAL: i32 = 22;
const EIO: i32 = 5;
const ENOMEM: i32 = 12;

struct Cursor {
    file: Arc<File>,
    length: i64,
    position: i64,
    cancellation: CancellationToken,
    deadline: Option<Instant>,
}

impl Cursor {
    fn read(&mut self, output: &mut [u8]) -> i32 {
        if self.cancellation.is_cancelled() || self.deadline.is_some_and(|at| Instant::now() >= at)
        {
            return ffi::AVERROR_EXIT;
        }
        let remaining = self.length - self.position;
        if remaining == 0 {
            return ffi::AVERROR_EOF;
        }
        let count = output
            .len()
            .min(BUFFER_BYTES)
            .min(usize::try_from(remaining).unwrap_or(usize::MAX));
        if count == 0 {
            return -EINVAL;
        }
        match self
            .file
            .read_at(&mut output[..count], self.position as u64)
        {
            Ok(0) => ffi::AVERROR_EOF,
            Ok(read) => {
                self.position += read as i64;
                read as i32
            }
            Err(error) => -error.raw_os_error().unwrap_or(EIO),
        }
    }

    fn seek(&mut self, offset: i64, whence: i32) -> i64 {
        if self.cancellation.is_cancelled() || self.deadline.is_some_and(|at| Instant::now() >= at)
        {
            return i64::from(ffi::AVERROR_EXIT);
        }
        let whence = whence & !ffi::AVSEEK_FORCE;
        if whence == ffi::AVSEEK_SIZE {
            return self.length;
        }
        let base = match whence {
            0 => 0,
            1 => self.position,
            2 => self.length,
            _ => return -i64::from(EINVAL),
        };
        let Some(position) = base
            .checked_add(offset)
            .filter(|value| (0..=self.length).contains(value))
        else {
            return -i64::from(EINVAL);
        };
        self.position = position;
        position
    }
}

unsafe extern "C" fn read_packet(opaque: *mut c_void, buffer: *mut u8, bytes: i32) -> i32 {
    if opaque.is_null() || buffer.is_null() || bytes <= 0 {
        return -EINVAL;
    }
    // FFmpeg owns the writable buffer; its callback is serialized for this input.
    let cursor = unsafe { &mut *opaque.cast::<Cursor>() };
    let output = unsafe { std::slice::from_raw_parts_mut(buffer, bytes as usize) };
    cursor.read(output)
}
unsafe extern "C" fn seek(opaque: *mut c_void, offset: i64, whence: i32) -> i64 {
    if opaque.is_null() {
        return -i64::from(EINVAL);
    }
    unsafe { &mut *opaque.cast::<Cursor>() }.seek(offset, whence)
}
unsafe extern "C" fn interrupted(opaque: *mut c_void) -> i32 {
    if opaque.is_null() {
        return 1;
    }
    i32::from(
        unsafe { &*opaque.cast::<Cursor>() }
            .cancellation
            .is_cancelled(),
    )
}

pub(crate) struct CapabilityIo {
    context: *mut ffi::AVIOContext,
    // Stable allocation retained until the format context and AVIO are gone.
    _cursor: Box<Cursor>,
}

impl CapabilityIo {
    pub(crate) fn open(
        file: Arc<File>,
        length: i64,
        cancellation: &CancellationToken,
    ) -> Result<(format::context::Input, Self), Error> {
        Self::open_until(file, length, cancellation, None)
    }

    fn open_until(
        file: Arc<File>,
        length: i64,
        cancellation: &CancellationToken,
        deadline: Option<Instant>,
    ) -> Result<(format::context::Input, Self), Error> {
        let mut cursor = Box::new(Cursor {
            file,
            length,
            position: 0,
            cancellation: cancellation.clone(),
            deadline,
        });
        let opaque = (&mut *cursor as *mut Cursor).cast::<c_void>();
        unsafe {
            let buffer = ffi::av_malloc(BUFFER_BYTES).cast::<u8>();
            if buffer.is_null() {
                return Err(Error::Other { errno: ENOMEM });
            }
            let context = ffi::avio_alloc_context(
                buffer,
                BUFFER_BYTES as i32,
                0,
                opaque,
                Some(read_packet),
                None,
                Some(seek),
            );
            if context.is_null() {
                ffi::av_free(buffer.cast());
                return Err(Error::Other { errno: ENOMEM });
            }
            let owner = Self {
                context,
                _cursor: cursor,
            };
            (*context).seekable = ffi::AVIO_SEEKABLE_NORMAL;
            let mut format_context = ffi::avformat_alloc_context();
            if format_context.is_null() {
                return Err(Error::Other { errno: ENOMEM });
            }
            (*format_context).pb = context;
            (*format_context).flags |= ffi::AVFMT_FLAG_CUSTOM_IO;
            (*format_context).interrupt_callback = ffi::AVIOInterruptCB {
                callback: Some(interrupted),
                opaque,
            };
            let opened = ffi::avformat_open_input(
                &mut format_context,
                ptr::null(),
                ptr::null(),
                ptr::null_mut(),
            );
            if opened < 0 {
                if !format_context.is_null() {
                    ffi::avformat_close_input(&mut format_context);
                }
                return Err(Error::from(opened));
            }
            let mut input = format::context::Input::wrap(format_context);
            let discovered = ffi::avformat_find_stream_info(input.as_mut_ptr(), ptr::null_mut());
            if discovered < 0 {
                drop(input); // CUSTOM_IO keeps pb alive for owner cleanup below.
                return Err(Error::from(discovered));
            }
            Ok((input, owner))
        }
    }
}

pub(crate) fn probe(
    capability: &super::SeekableMediaIoCapability,
) -> Result<Vec<u8>, ffmpeg_the_third::Error> {
    use ffmpeg_the_third as ffmpeg;

    const MAX_STREAMS: usize = 64;
    ffmpeg::init()?;
    let cancellation = CancellationToken::new();
    let (input, _io) = CapabilityIo::open_until(
        Arc::clone(&capability.file),
        capability.length,
        &cancellation,
        Some(Instant::now() + Duration::from_secs(15)),
    )?;
    if input.nb_streams() as usize > MAX_STREAMS {
        return Err(ffmpeg::Error::Other { errno: EINVAL });
    }
    let streams = input
        .streams()
        .map(|stream| {
            let parameters = stream.parameters();
            let codec_name = parameters.id().name();
            let duration = decimal_time(stream.duration(), stream.time_base());
            let mut value = serde_json::json!({
                "index": stream.index(),
                "codec_name": (codec_name != "none").then_some(codec_name),
                "duration": duration,
                "disposition": {
                    "attached_pic": stream.disposition().contains(
                        ffmpeg::format::stream::Disposition::ATTACHED_PIC
                    ),
                },
            });
            let fields = value.as_object_mut().expect("JSON object");
            match parameters.medium() {
                ffmpeg::media::Type::Video => {
                    fields.insert("codec_type".to_owned(), "video".into());
                    fields.insert("width".to_owned(), parameters.width().into());
                    fields.insert("height".to_owned(), parameters.height().into());
                    let rate = stream.avg_frame_rate();
                    fields.insert(
                        "avg_frame_rate".to_owned(),
                        if rate.numerator() > 0 && rate.denominator() > 0 {
                            format!("{}/{}", rate.numerator(), rate.denominator()).into()
                        } else {
                            "0/0".into()
                        },
                    );
                }
                ffmpeg::media::Type::Audio => {
                    fields.insert("codec_type".to_owned(), "audio".into());
                    fields.insert("sample_rate".to_owned(), parameters.sample_rate().into());
                    fields.insert(
                        "channels".to_owned(),
                        parameters.ch_layout().channels().into(),
                    );
                }
                _ => {
                    fields.insert("codec_type".to_owned(), "other".into());
                }
            }
            value
        })
        .collect::<Vec<_>>();
    let format_names = input.format().name();
    let duration = input.duration();
    let duration =
        (duration >= 0).then(|| format!("{}.{:06}", duration / 1_000_000, duration % 1_000_000));
    serde_json::to_vec(&serde_json::json!({
        "format": { "format_name": format_names, "duration": duration },
        "streams": streams,
    }))
    .map_err(|_| ffmpeg::Error::Other { errno: EINVAL })
}

fn decimal_time(value: i64, time_base: ffmpeg_the_third::Rational) -> Option<String> {
    if value < 0 || time_base.numerator() <= 0 || time_base.denominator() <= 0 {
        return None;
    }
    let denominator = i128::from(time_base.denominator());
    let micros = i128::from(value)
        .checked_mul(i128::from(time_base.numerator()))?
        .checked_mul(1_000_000)?
        .checked_add(denominator / 2)?
        .checked_div(denominator)?;
    let micros = u64::try_from(micros).ok()?;
    Some(format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000))
}

impl Drop for CapabilityIo {
    fn drop(&mut self) {
        unsafe {
            // FFmpeg may replace the original av_malloc buffer while probing.
            ffi::av_free((*self.context).buffer.cast());
            (*self.context).buffer = ptr::null_mut();
            ffi::avio_context_free(&mut self.context);
        }
        // _cursor (and its Arc<File>) drops after this method, exactly once.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cursor() -> Cursor {
        let file = File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/tiny.mkv"
        ))
        .unwrap();
        let length = file.metadata().unwrap().len() as i64;
        Cursor {
            file: Arc::new(file),
            length,
            position: 0,
            cancellation: CancellationToken::new(),
            deadline: None,
        }
    }
    #[test]
    fn independent_cursors_preserve_seek_eof_and_checked_bounds() {
        let mut first = cursor();
        let mut second = Cursor {
            file: Arc::clone(&first.file),
            length: first.length,
            position: 0,
            cancellation: CancellationToken::new(),
            deadline: None,
        };
        std::thread::scope(|scope| {
            scope.spawn(|| {
                assert_eq!(first.seek(12, 0), 12);
                let mut output = [0; 4];
                assert_eq!(first.read(&mut output), 4);
                assert_eq!(first.position, 16);
            });
            scope.spawn(|| {
                let mut output = [0; 4];
                assert_eq!(second.read(&mut output), 4);
                assert_eq!(output, [0x1a, 0x45, 0xdf, 0xa3]);
                assert_eq!(second.position, 4);
            });
        });
        assert_eq!(first.seek(0, ffi::AVSEEK_SIZE), first.length);
        assert_eq!(first.seek(0, 2 | ffi::AVSEEK_FORCE), first.length);
        assert_eq!(first.read(&mut [0; 8]), ffi::AVERROR_EOF);
        assert_eq!(first.seek(i64::MAX, 1), -i64::from(EINVAL));
        assert_eq!(first.seek(-1, 0), -i64::from(EINVAL));
        assert_eq!(first.seek(0, 17), -i64::from(EINVAL));
        assert_eq!(first.position, first.length);
    }
    #[test]
    fn cancellation_aborts_reads_size_queries_and_seeks() {
        let mut input = cursor();
        input.cancellation.cancel();
        assert_eq!(input.read(&mut [0; 16]), ffi::AVERROR_EXIT);
        assert_eq!(
            input.seek(0, ffi::AVSEEK_SIZE),
            i64::from(ffi::AVERROR_EXIT)
        );
        assert_eq!(input.seek(0, 0), i64::from(ffi::AVERROR_EXIT));
        assert_eq!(input.position, 0);
    }

    #[test]
    fn expired_deadline_aborts_reads_size_queries_and_seeks() {
        let mut input = cursor();
        input.deadline = Some(Instant::now() - Duration::from_secs(1));
        assert_eq!(input.read(&mut [0; 16]), ffi::AVERROR_EXIT);
        assert_eq!(
            input.seek(0, ffi::AVSEEK_SIZE),
            i64::from(ffi::AVERROR_EXIT)
        );
        assert_eq!(input.seek(0, 0), i64::from(ffi::AVERROR_EXIT));
        assert_eq!(input.position, 0);
    }
}
