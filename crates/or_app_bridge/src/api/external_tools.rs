//! External tools as the app sees them: detected once, described fully, and
//! launched only when a person asks.
//!
//! Detection is `or_tools`; the catalog and the probe policy are
//! `or_core::external_tools`, so the CLI and an agent tool answer with the
//! same facts. This module only shapes the answer for the app. No function
//! here installs, downloads or sends anything.

use or_core::{Platform, ToolDescriptor, ToolState, tool, tools_for};
use or_tools::{launch, statuses};

/// One catalog entry plus what this machine answered.
#[derive(Clone, Debug)]
pub struct ExternalToolView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub benefit: String,
    pub license: String,
    pub license_note: Option<String>,
    pub homepage: String,
    pub install_page: String,
    /// The exact command for this platform, ready to copy.
    pub install_command: Option<String>,
    pub install_note: Option<String>,
    pub size_hint: String,
    pub runtime: String,
    /// The command that opens the companion window, when it has one.
    pub gui_command: Option<String>,
    /// Output kinds, most editable first.
    pub outputs: Vec<String>,
    /// True when the tool is installed into a project, not the machine.
    pub project_scoped: bool,
    pub removable: bool,
    /// How to take it off this machine, in plain words.
    pub remove_hint: String,
    /// How to get a newer release, in plain words.
    pub update_hint: String,
    /// "ready", "missing", "failed" or "unsupported".
    pub state: String,
    pub version: Option<String>,
    pub executable: Option<String>,
    pub detail: String,
    /// True when OR may launch this tool right now.
    pub launchable: bool,
}

/// What happened when a person asked to open a companion.
#[derive(Clone, Debug)]
pub struct ExternalToolLaunch {
    pub launched: bool,
    pub detail: String,
}

fn view(descriptor: &ToolDescriptor, platform: Platform) -> ExternalToolView {
    let status = or_tools::probe(descriptor);
    let install = descriptor.install_command(platform);
    ExternalToolView {
        id: descriptor.id.to_owned(),
        name: descriptor.name.to_owned(),
        category: descriptor.category.label().to_owned(),
        summary: descriptor.summary.to_owned(),
        benefit: descriptor.benefit.to_owned(),
        license: descriptor.license.to_owned(),
        license_note: descriptor.license_note.map(str::to_owned),
        homepage: descriptor.homepage.to_owned(),
        install_page: descriptor.install_page.to_owned(),
        install_command: install.map(|command| command.command.to_owned()),
        install_note: install.map(|command| command.note.to_owned()),
        size_hint: descriptor.size_hint.to_owned(),
        runtime: descriptor.runtime.to_owned(),
        gui_command: descriptor.gui_program.map(str::to_owned),
        outputs: descriptor
            .outputs
            .iter()
            .map(|output| output.label().to_owned())
            .collect(),
        project_scoped: descriptor.project_scoped,
        removable: descriptor.removable,
        remove_hint: descriptor.remove_hint.to_owned(),
        update_hint: descriptor.update_hint.to_owned(),
        state: match status.state {
            ToolState::Ready => "ready",
            ToolState::Missing => "missing",
            ToolState::Failed => "failed",
            ToolState::Unsupported => "unsupported",
            ToolState::ProjectScoped => "project_scoped",
        }
        .to_owned(),
        launchable: status.state == ToolState::Ready && descriptor.gui_program.is_some(),
        version: status.version,
        executable: status.executable,
        detail: status.detail,
    }
}

/// Every tool this build can manage on this machine, already probed.
///
/// The list is short and fixed; it is not a scan of the filesystem. Each probe
/// runs the tool's own version flag, so opening Settings costs a handful of
/// short processes and nothing else.
pub fn list_external_tools() -> Vec<ExternalToolView> {
    let platform = Platform::current();
    tools_for(platform)
        .map(|descriptor| view(descriptor, platform))
        .collect()
}

/// Re-probes one tool, e.g. after a person installed it.
pub fn check_external_tool(id: String) -> Result<ExternalToolView, String> {
    let platform = Platform::current();
    let descriptor = tool(&id).ok_or_else(|| format!("unknown external tool: {id}"))?;
    Ok(view(descriptor, platform))
}

/// Probes everything and returns only the ids that are ready.
///
/// Used by workflows that must not offer a tool that cannot run.
pub fn ready_external_tools() -> Vec<String> {
    statuses(Platform::current())
        .into_iter()
        .filter(|status| status.state == ToolState::Ready)
        .map(|status| status.id)
        .collect()
}

/// Opens a companion tool's own window.
///
/// This is the only function that launches a program without a project file.
/// It requires the tool to have answered its probe first, so a broken install
/// reports why instead of flashing a window and dying.
pub fn launch_external_tool(id: String) -> Result<ExternalToolLaunch, String> {
    let outcome = launch(&id)?;
    Ok(ExternalToolLaunch {
        launched: outcome.launched,
        detail: outcome.detail,
    })
}

#[cfg(test)]
mod tests {
    use super::{check_external_tool, list_external_tools};

    #[test]
    fn the_catalog_is_listed_with_a_state_for_every_entry() {
        let tools = list_external_tools();
        assert!(!tools.is_empty());
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.summary.is_empty());
            assert!(!tool.license.is_empty());
            assert!(
                matches!(
                    tool.state.as_str(),
                    "ready" | "missing" | "failed" | "unsupported" | "project_scoped"
                ),
                "{} had state {}",
                tool.id,
                tool.state
            );
            if tool.state != "ready" {
                assert!(
                    !tool.detail.is_empty(),
                    "{} was not ready and had no detail",
                    tool.id
                );
            }
        }
    }

    #[test]
    fn an_unknown_id_is_an_error_not_a_panic() {
        assert!(check_external_tool("no-such-tool".to_owned()).is_err());
    }
}
