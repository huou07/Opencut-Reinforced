fn main() {
    assert!(
        or_media::verify_ffmpeg_runtime(),
        "production or_media FFmpeg initialization or LGPL check failed"
    );

    let versions = unsafe {
        [
            ("libavcodec", ffmpeg_the_third::sys::avcodec_version(), 62),
            ("libavformat", ffmpeg_the_third::sys::avformat_version(), 62),
            ("libavutil", ffmpeg_the_third::sys::avutil_version(), 60),
            (
                "libswresample",
                ffmpeg_the_third::sys::swresample_version(),
                6,
            ),
            ("libswscale", ffmpeg_the_third::sys::swscale_version(), 9),
        ]
    };

    for (name, encoded_version, expected_major) in versions {
        assert_eq!(
            encoded_version >> 16,
            expected_major,
            "{name} ABI major is incompatible"
        );
        println!("{name}_abi_major={expected_major}");
    }

    let license = ffmpeg_the_third::util::license();
    assert_eq!(license, "LGPL version 2.1 or later");
    println!("ffmpeg_license={license}");
}
