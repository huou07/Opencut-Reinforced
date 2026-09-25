#[derive(Clone, Debug)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub core_api_version: u32,
}

#[derive(Clone, Debug)]
pub struct HealthStatus {
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct Capability {
    pub id: String,
    pub version: u32,
}

pub fn app_info() -> AppInfo {
    let info = or_core::app_info();
    AppInfo {
        name: info.name,
        version: info.version,
        core_api_version: info.core_api_version,
    }
}

pub fn health() -> HealthStatus {
    HealthStatus {
        status: or_core::health().status,
    }
}

pub fn capabilities() -> Vec<Capability> {
    or_core::capabilities()
        .into_iter()
        .map(|capability| Capability {
            id: capability.id,
            version: capability.version,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{app_info, capabilities, health};

    #[test]
    fn bridge_values_come_from_the_core() {
        let core_app_info = or_core::app_info();
        let app_info = app_info();
        assert_eq!(app_info.name, core_app_info.name);
        assert_eq!(app_info.version, core_app_info.version);
        assert_eq!(app_info.core_api_version, core_app_info.core_api_version);

        assert_eq!(health().status, or_core::health().status);

        let core_capabilities = or_core::capabilities();
        let capabilities = capabilities();
        assert_eq!(capabilities.len(), core_capabilities.len());
        for (bridged, core) in capabilities.iter().zip(core_capabilities) {
            assert_eq!(bridged.id, core.id);
            assert_eq!(bridged.version, core.version);
        }
    }
}
pub mod project;
