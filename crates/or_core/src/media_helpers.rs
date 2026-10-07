//! Packaged-first resolution for the FFmpeg media helper executables.
//!
//! The desktop application archives and CLI packages carry their platform's
//! `ffmpeg`/`ffprobe` programs as siblings of the main application binary (see
//! the 9B1 review in `docs/SECURITY_LICENSING.md`). Import probing and
//! disposable artifact generation resolve through this module so a packaged
//! application never depends on a system executable or a developer `PATH`.
//!
//! Resolution order is the explicit `OR_FFPROBE_PATH`/`OR_FFMPEG_PATH`
//! developer override, then the packaged sibling, then a bare program name for
//! `PATH` lookup in unpackaged development runs. This module contains no FFmpeg
//! binding: it only resolves a path that is spawned with direct process
//! arguments and no shell.

use std::{
    env, ffi,
    path::{Path, PathBuf},
};

/// Developer override for the packaged `ffprobe` program.
pub const FFPROBE_PATH_ENVIRONMENT_VARIABLE: &str = "OR_FFPROBE_PATH";
/// Developer override for the packaged `ffmpeg` program.
pub const FFMPEG_PATH_ENVIRONMENT_VARIABLE: &str = "OR_FFMPEG_PATH";

/// Directory that holds the packaged helpers: the main binary's own directory.
pub fn packaged_helper_directory() -> Option<PathBuf> {
    env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

/// Resolves one helper executable without touching the environment.
///
/// `environment_override` is the already-read `OR_*_PATH` value. The packaged
/// sibling is used only when it exists, so unpackaged development runs keep
/// their `PATH` lookup instead of shadowing it with a missing file.
pub fn resolve_helper_executable(
    binary_name: &str,
    environment_override: Option<ffi::OsString>,
    packaged_directory: Option<&Path>,
) -> PathBuf {
    if let Some(path) = environment_override.filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    let file_name = helper_file_name(binary_name);
    if let Some(directory) = packaged_directory {
        let packaged = directory.join(&file_name);
        if packaged.is_file() {
            return packaged;
        }
    }
    PathBuf::from(file_name)
}

fn helper_file_name(binary_name: &str) -> String {
    if cfg!(windows) {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_owned()
    }
}

/// Resolves the `ffprobe` program: override, packaged sibling, or `PATH`.
pub fn ffprobe_executable() -> PathBuf {
    resolve_helper_executable(
        "ffprobe",
        env::var_os(FFPROBE_PATH_ENVIRONMENT_VARIABLE),
        packaged_helper_directory().as_deref(),
    )
}

/// Resolves the `ffmpeg` program: override, packaged sibling, or `PATH`.
pub fn ffmpeg_executable() -> PathBuf {
    resolve_helper_executable(
        "ffmpeg",
        env::var_os(FFMPEG_PATH_ENVIRONMENT_VARIABLE),
        packaged_helper_directory().as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::{packaged_helper_directory, resolve_helper_executable};
    use std::path::PathBuf;

    fn missing_directory() -> PathBuf {
        std::env::temp_dir().join("or-no-such-helper-directory-9b1")
    }

    #[test]
    fn environment_override_wins_over_a_packaged_sibling() {
        let directory =
            std::env::temp_dir().join(format!("or-helper-override-{}", crate::MediaId::generate()));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("ffprobe"), b"packaged").unwrap();
        let resolved = resolve_helper_executable(
            "ffprobe",
            Some(std::ffi::OsString::from("/developer/ffprobe")),
            Some(&directory),
        );
        assert_eq!(resolved, PathBuf::from("/developer/ffprobe"));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn packaged_sibling_wins_over_path_lookup() {
        let directory =
            std::env::temp_dir().join(format!("or-helper-packaged-{}", crate::MediaId::generate()));
        std::fs::create_dir_all(&directory).unwrap();
        let packaged = directory.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        std::fs::write(&packaged, b"packaged").unwrap();
        let resolved = resolve_helper_executable("ffmpeg", None, Some(&directory));
        assert_eq!(resolved, packaged);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn missing_packaged_sibling_falls_back_to_path_lookup() {
        let resolved = resolve_helper_executable("ffprobe", None, Some(&missing_directory()));
        assert_eq!(resolved, PathBuf::from("ffprobe"));
        let resolved = resolve_helper_executable("ffprobe", None, None);
        assert_eq!(resolved, PathBuf::from("ffprobe"));
    }

    #[test]
    fn empty_override_is_ignored() {
        let resolved = resolve_helper_executable(
            "ffmpeg",
            Some(std::ffi::OsString::from("")),
            Some(&missing_directory()),
        );
        assert_eq!(resolved, PathBuf::from("ffmpeg"));
    }

    #[test]
    fn packaged_directory_is_the_current_executable_directory() {
        let expected = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        assert_eq!(packaged_helper_directory(), Some(expected));
    }
}
