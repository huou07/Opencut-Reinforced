//! Host-side probing for the external tools `or_core` describes.
//!
//! The catalog and the decision live in `or_core::external_tools` and stay
//! pure. This crate is the only place that spawns a tool: it runs the version
//! flag with a bound, and opens a companion window when a person asks.
//!
//! It exists so the app, the CLI and an agent tool probe the same way instead
//! of each growing its own process handling.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use or_core::{
    CommandOutput, CommandRunner, Platform, ToolDescriptor, ToolState, ToolStatus, probe_all,
    probe_tool, tool,
};

/// How long a probe may take before it is killed.
///
/// A version flag is instant on a working tool. A GUI-first program that
/// ignores the flag is the case this bounds; it is not a render timeout.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

/// How long a person-requested tool run may take before it is killed.
///
/// Renders and conversions legitimately take minutes, so this is a backstop
/// against a hung process, not a probe bound.
pub const RUN_TIMEOUT: Duration = Duration::from_secs(600);

/// Runs one program with a bounded wait and captured output.
pub struct SystemCommandRunner {
    timeout: Duration,
}

impl SystemCommandRunner {
    pub fn new() -> Self {
        Self {
            timeout: PROBE_TIMEOUT,
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for SystemCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{program}: {error}"))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (sender, receiver) = mpsc::channel();
        if let Some(stdout) = stdout {
            let sender = sender.clone();
            std::thread::spawn(move || {
                let mut reader = stdout;
                let mut text = String::new();
                let _ = reader.read_to_string(&mut text);
                let _ = sender.send(("stdout", text));
            });
        }
        if let Some(stderr) = stderr {
            let sender = sender.clone();
            std::thread::spawn(move || {
                let mut reader = stderr;
                let mut text = String::new();
                let _ = reader.read_to_string(&mut text);
                let _ = sender.send(("stderr", text));
            });
        }
        drop(sender);

        let started = Instant::now();
        let mut status = None;
        while started.elapsed() < self.timeout {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    status = Some(exit);
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(error) => return Err(format!("{program}: {error}")),
            }
        }
        let Some(status) = status else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{program} did not answer within {} seconds.",
                self.timeout.as_secs()
            ));
        };

        let mut stdout_text = String::new();
        let mut stderr_text = String::new();
        for (stream, text) in receiver.try_iter() {
            match stream {
                "stdout" => stdout_text.push_str(&text),
                _ => stderr_text.push_str(&text),
            }
        }
        Ok(CommandOutput {
            success: status.success(),
            stdout: stdout_text,
            stderr: stderr_text,
        })
    }
}

/// Runs a ready catalog tool with the arguments a person supplied.
///
/// The catalog is the allow-list: an id outside it is rejected, and a tool that
/// is missing or broken is reported before anything is spawned. Arguments are
/// passed through untouched, never through a shell.
pub fn run_tool(
    id: &str,
    arguments: &[String],
    timeout: Duration,
) -> Result<CommandOutput, String> {
    let descriptor = tool(id).ok_or_else(|| format!("unknown companion tool: {id}"))?;
    let status = probe(descriptor);
    if status.state != ToolState::Ready {
        return Err(format!(
            "{} is not ready: {}",
            descriptor.name, status.detail
        ));
    }
    let runner = SystemCommandRunner::with_timeout(timeout);
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    runner.run(descriptor.probe.program, &borrowed)
}

/// The setup command for a tool on this platform, when one exists.
pub fn setup_command(id: &str, platform: Platform) -> Result<String, String> {
    let descriptor = tool(id).ok_or_else(|| format!("unknown companion tool: {id}"))?;
    let install = descriptor.install_command(platform).ok_or_else(|| {
        format!(
            "{} has no setup command for {}",
            descriptor.name,
            platform_name(platform)
        )
    })?;
    Ok(install.command.to_owned())
}

/// The command that undoes a setup command, when one can be derived.
///
/// `npm install -g x` pairs with `npm uninstall -g x`. Anything else — a system
/// package manager, a vendor installer — is left to the tool's own removal
/// hint, because guessing an uninstall is worse than an honest instruction.
pub fn removal_command(install: &str) -> Option<String> {
    let mut words = install.split_whitespace();
    if words.next()? != "npm" {
        return None;
    }
    let verb = words.next()?;
    if !matches!(verb, "install" | "i" | "add") || words.next()? != "-g" {
        return None;
    }
    let package: Vec<&str> = words.collect();
    if package.is_empty() {
        return None;
    }
    Some(format!("npm uninstall -g {}", package.join(" ")))
}

/// Runs a setup (or removal) command the way a terminal would.
///
/// The catalog states one shell command per platform, so this is the only place
/// in the workspace that hands a string to a shell. It is reached only from an
/// explicit user request (`or tools setup <id>`, or the panel's button), and the
/// id must name a catalog tool, so the allow-list still holds.
pub fn run_setup(id: &str, remove: bool, timeout: Duration) -> Result<CommandOutput, String> {
    let platform = Platform::current();
    let install = setup_command(id, platform)?;
    let command = if remove {
        removal_command(&install).ok_or_else(|| {
            format!(
                "no uninstall command is known for {id}; the removal steps are: {}",
                tool(id)
                    .map(|descriptor| descriptor.remove_hint)
                    .unwrap_or("")
            )
        })?
    } else {
        install
    };
    let mut process = if platform == Platform::Windows {
        let mut process = std::process::Command::new("cmd.exe");
        process.arg("/C").arg(&command);
        process
    } else {
        let mut process = std::process::Command::new("/bin/sh");
        process.arg("-lc").arg(&command);
        process
    };
    let mut child = process
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("{command}: {error}"))?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "{command} did not finish within {} seconds.",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(format!("{command}: {error}")),
        }
    }
    // The child has exited, so both pipes reach end-of-file and the output can
    // be read without waiting on a process that would never answer.
    let output = child
        .wait_with_output()
        .map_err(|error| format!("{command}: {error}"))?;
    Ok(CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::MacOs => "macOS",
        Platform::Windows => "Windows",
        Platform::Linux => "Linux",
    }
}

/// Probes every catalog tool this platform supports.
pub fn statuses(platform: Platform) -> Vec<ToolStatus> {
    probe_all(platform, &SystemCommandRunner::new())
}

/// Probes one tool by id.
pub fn status(id: &str) -> Option<ToolStatus> {
    let descriptor = tool(id)?;
    Some(probe_tool(
        descriptor,
        Platform::current(),
        &SystemCommandRunner::new(),
    ))
}

/// Probes one descriptor on the current platform.
pub fn probe(descriptor: &ToolDescriptor) -> ToolStatus {
    probe_tool(descriptor, Platform::current(), &SystemCommandRunner::new())
}

/// What happened when a person asked to open a companion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchOutcome {
    pub launched: bool,
    pub detail: String,
}

/// Opens a companion tool's own window, after its probe answered.
///
/// A broken install reports why instead of flashing a window and dying.
pub fn launch(id: &str) -> Result<LaunchOutcome, String> {
    let descriptor = tool(id).ok_or_else(|| format!("unknown external tool: {id}"))?;
    let Some(program) = descriptor.gui_program else {
        return Err(format!("{} has no companion window.", descriptor.name));
    };
    let status = probe(descriptor);
    if status.state != ToolState::Ready {
        return Ok(LaunchOutcome {
            launched: false,
            detail: status.detail,
        });
    }

    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        Ok(child) => {
            // The child is the tool, not ours to wait for. The operating system
            // owns it from here.
            let pid = child.id();
            std::mem::forget(child);
            Ok(LaunchOutcome {
                launched: true,
                detail: format!("{} opened (process {pid}).", descriptor.name),
            })
        }
        Err(error) => Ok(LaunchOutcome {
            launched: false,
            detail: format!("{} could not be opened: {error}", descriptor.name),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_entry_answers_with_a_state() {
        let statuses = statuses(Platform::current());
        assert_eq!(
            statuses.len(),
            or_core::tools_for(Platform::current()).count()
        );
        for status in &statuses {
            assert!(!status.id.is_empty());
            assert!(!status.detail.is_empty());
            if status.state == ToolState::Ready {
                assert!(
                    status.version.is_some(),
                    "{} was ready without a version",
                    status.id
                );
            }
        }
    }

    #[test]
    fn an_unknown_tool_is_none_not_a_panic() {
        assert!(status("no-such-tool").is_none());
        assert!(launch("no-such-tool").is_err());
    }
}

#[cfg(test)]
mod run_tests {
    use super::*;

    #[test]
    fn an_unknown_id_is_rejected_without_spawning() {
        let error = run_tool("not-a-tool", &["--version".to_owned()], PROBE_TIMEOUT)
            .expect_err("unknown tools are rejected");
        assert!(error.contains("unknown companion tool"), "{error}");
    }
}

#[cfg(test)]
mod setup_tests {
    use super::*;

    #[test]
    fn an_npm_install_pairs_with_its_own_uninstall() {
        assert_eq!(
            removal_command("npm install -g three").as_deref(),
            Some("npm uninstall -g three")
        );
        assert_eq!(
            removal_command("npm i -g puppeteer-lottie").as_deref(),
            Some("npm uninstall -g puppeteer-lottie")
        );
        assert_eq!(removal_command("npm install puppeteer-lottie"), None);
        assert_eq!(removal_command("sudo apt install blender"), None);
        assert_eq!(removal_command(""), None);
    }

    #[test]
    fn a_catalog_tool_reports_the_setup_command_for_a_platform() {
        let command = setup_command("lottie", Platform::MacOs).expect("lottie has a setup command");
        assert!(command.contains("npm install"), "{command}");
        // The bundled tool still states the system command, for a machine
        // where the package is missing.
        let ffmpeg = setup_command("ffmpeg", Platform::Linux).expect("ffmpeg states one");
        assert!(ffmpeg.contains("ffmpeg"), "{ffmpeg}");
        assert!(setup_command("no-such-tool", Platform::Linux).is_err());
    }

    #[test]
    fn a_shell_command_runs_and_its_output_comes_back() {
        if Platform::current() == Platform::Windows {
            return;
        }
        let output = run_shell_for_test("echo setup-line; exit 4").expect("the shell answered");
        assert!(!output.success);
        assert!(output.stdout.contains("setup-line"), "{output:?}");
    }

    /// `run_setup` reads a catalog command, so this helper exercises only the
    /// shell half with an arbitrary string.
    fn run_shell_for_test(command: &str) -> Result<CommandOutput, String> {
        let output = std::process::Command::new("/bin/sh")
            .arg("-lc")
            .arg(command)
            .output()
            .map_err(|error| error.to_string())?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
