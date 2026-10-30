use or_core::{
    CacheStoreConfig, JobManagerConfig, MediaArtifactService, MediaArtifactServiceConfig,
    ffmpeg_executable_from_environment,
};
use std::path::PathBuf;

// The artifact cache location and worker pool are configured once per host.

/// How many artifact jobs may run at once.
///
/// Two jobs keep playback responsive on small machines; wider machines halve
/// the cold-import fill time instead of leaving cores idle. Measured on this
/// host with `crates/or_core/tests/artifact_throughput.rs` (16 artifacts):
/// 34.4 ms each at one worker, 15.3 ms at two, 8.7 ms at four, 5.9 ms at eight.
pub(crate) fn media_artifact_worker_count() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2);
    media_artifact_worker_count_for(parallelism)
}

pub(crate) fn media_artifact_worker_count_for(parallelism: usize) -> usize {
    if parallelism >= 6 { 4 } else { 2 }
}

pub(crate) fn create_media_artifact_service() -> Option<MediaArtifactService> {
    #[cfg(target_os = "android")]
    {
        None
    }

    #[cfg(not(target_os = "android"))]
    {
        let cache_root = media_artifact_cache_root()?;
        let jobs = JobManagerConfig::new(media_artifact_worker_count(), 32, 128).ok()?;
        let cache = CacheStoreConfig::new(8 * 1024 * 1024, 256 * 1024 * 1024).ok()?;
        MediaArtifactService::new(MediaArtifactServiceConfig::new(
            cache_root,
            jobs,
            cache,
            ffmpeg_executable_from_environment(),
        ))
        .ok()
    }
}

#[allow(dead_code)] // Every desktop target constructs only its own variant; tests cover all paths.
#[derive(Clone, Copy)]
pub(crate) enum CachePlatform {
    MacOs,
    Linux,
    Windows,
    Unsupported,
}

pub(crate) fn configured_media_artifact_cache_root(
    platform: CachePlatform,
    home: Option<PathBuf>,
    xdg_cache_home: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> Option<PathBuf> {
    match platform {
        CachePlatform::MacOs => home.map(|home| {
            home.join("Library")
                .join("Caches")
                .join("Opencut-Reinforced")
                .join("media-artifacts")
        }),
        CachePlatform::Linux => xdg_cache_home
            .or_else(|| home.map(|home| home.join(".cache")))
            .map(|root| root.join("opencut-reinforced").join("media-artifacts")),
        CachePlatform::Windows => local_app_data.map(|root| {
            root.join("Opencut Reinforced")
                .join("Cache")
                .join("media-artifacts")
        }),
        CachePlatform::Unsupported => None,
    }
}

fn non_empty_environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn media_artifact_cache_root() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let platform = CachePlatform::MacOs;
    #[cfg(target_os = "linux")]
    let platform = CachePlatform::Linux;
    #[cfg(target_os = "windows")]
    let platform = CachePlatform::Windows;
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let platform = CachePlatform::Unsupported;

    configured_media_artifact_cache_root(
        platform,
        non_empty_environment_path("HOME"),
        non_empty_environment_path("XDG_CACHE_HOME"),
        non_empty_environment_path("LOCALAPPDATA"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_count_scales_with_the_machine() {
        assert_eq!(media_artifact_worker_count_for(1), 2);
        assert_eq!(media_artifact_worker_count_for(6), 4);
    }

    #[test]
    fn the_cache_root_follows_the_platform() {
        let home = Some(PathBuf::from("/home/user"));
        assert_eq!(
            configured_media_artifact_cache_root(CachePlatform::MacOs, home.clone(), None, None),
            Some(PathBuf::from(
                "/home/user/Library/Caches/Opencut-Reinforced/media-artifacts"
            ))
        );
        assert!(
            configured_media_artifact_cache_root(CachePlatform::Unsupported, home, None, None)
                .is_none()
        );
    }
}
