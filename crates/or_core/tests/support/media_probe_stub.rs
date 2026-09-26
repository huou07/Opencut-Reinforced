use std::{
    env,
    fs,
    io::{self, Write},
    path::Path,
    process,
    thread,
    time::Duration,
};

fn main() {
    let args: Vec<_> = env::args_os().collect();
    let input = args
        .windows(2)
        .find(|pair| pair[0] == "-i")
        .map(|pair| Path::new(&pair[1]));
    let Some(input) = input else {
        process::exit(2);
    };
    let Some(name) = input.file_name().and_then(|name| name.to_str()) else {
        process::exit(3);
    };

    match name {
        "sleep.mkv" => {
            if let Some(marker) = env::var_os("OR_FFPROBE_MARKER") {
                let _ = fs::write(marker, b"started");
            }
            thread::sleep(Duration::from_secs(2));
        }
        "oversized.mkv" => {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(&vec![b'x'; 4096]);
            let _ = stdout.flush();
        }
        "oversized-stderr.mkv" => {
            let mut stderr = io::stderr().lock();
            let _ = stderr.write_all(&vec![b'e'; 4096]);
            let _ = stderr.flush();
        }
        "failure.mkv" => {
            let mut stderr = io::stderr().lock();
            let _ = stderr.write_all(&vec![b'e'; 1024]);
            let _ = stderr.flush();
            process::exit(7);
        }
        "path with spaces-媒体.mkv" => {}
        "cli sample café.mkv" => {
            println!(
                r#"{{"format":{{"format_name":"matroska,webm","duration":"1.5"}},"streams":[{{"index":0,"codec_type":"video","codec_name":"ffv1","width":16,"height":16,"pix_fmt":"yuv420p","avg_frame_rate":"24000/1001"}},{{"index":1,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":"48000","channels":2,"channel_layout":"stereo"}}]}}"#
            );
            return;
        }
        _ => process::exit(4),
    }

    println!("{{\"format\":{{}},\"streams\":[]}}");
}
