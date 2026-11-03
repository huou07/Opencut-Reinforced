fn main() {
    assert!(
        or_media::verify_ffmpeg_runtime(),
        "production or_media FFmpeg initialization or license check failed"
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

    // The Linux delivery build links x264, so it reports GPL-2 there
    // (ADR 0011). Every other target must stay LGPL.
    let license = ffmpeg_the_third::util::license();
    let approved = license == "LGPL version 2.1 or later"
        || (cfg!(target_os = "linux") && license == "GPL version 2 or later");
    assert!(approved, "unexpected FFmpeg license: {license}");
    println!("ffmpeg_license={license}");
}
