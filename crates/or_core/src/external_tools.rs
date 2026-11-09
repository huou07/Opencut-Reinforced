//! The tools OR can use without owning, and how it decides they are usable.
//!
//! An external tool is a separate program: a companion editor, a renderer, a
//! converter. OR never links it and never ships it inside the editor binary.
//! This module is the shared catalog and the probe policy; the caller supplies
//! the command runner, so the same decision runs in a unit test, the app, the
//! CLI and an agent tool.
//!
//! Two rules are encoded here rather than in a screen:
//!
//! * A tool is `Ready` only after its own executable answered. A file on disk
//!   is not a working tool.
//! * Nothing installs, downloads or opens a page on its own. The catalog only
//!   carries the exact command and the official page a person can choose.

use serde::{Deserialize, Serialize};

/// Where a tool can run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

impl Platform {
    /// The platform this build runs on.
    pub const fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::Linux
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::MacOs => "macOS",
            Self::Windows => "Windows",
            Self::Linux => "Linux",
        }
    }
}

/// What a person is trying to make with the tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Composition,
    Animation,
    Vector,
    ThreeD,
    Audio,
    Media,
    Runtime,
}

impl ToolCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Composition => "Video composition",
            Self::Animation => "Animation",
            Self::Vector => "Vector and type",
            Self::ThreeD => "3D",
            Self::Audio => "Audio",
            Self::Media => "Media",
            Self::Runtime => "Runtime",
        }
    }
}

/// A file kind a tool can hand back to OR.
///
/// The list is ordered from most editable to least: a scene or a vector source
/// stays a source, a rendered video is only a rendered video.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    MotionScene,
    Svg,
    Lottie,
    Gltf,
    ImageSequence,
    Subtitle,
    Audio,
    Image,
    Video,
    ProjectManifest,
}

impl OutputKind {
    /// True when the output keeps structure OR can edit as separate objects.
    ///
    /// A flat video is deliberately not editable: it is inserted as media and
    /// its source project is kept beside it.
    pub const fn is_structured(self) -> bool {
        matches!(
            self,
            Self::MotionScene
                | Self::Svg
                | Self::Lottie
                | Self::Gltf
                | Self::ImageSequence
                | Self::Subtitle
                | Self::ProjectManifest
        )
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::MotionScene => "MotionScene",
            Self::Svg => "SVG",
            Self::Lottie => "Lottie",
            Self::Gltf => "glTF",
            Self::ImageSequence => "image sequence",
            Self::Subtitle => "subtitle",
            Self::Audio => "audio",
            Self::Image => "image",
            Self::Video => "video",
            Self::ProjectManifest => "source project",
        }
    }
}

/// One program to run: the executable plus fixed arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl CommandSpec {
    pub const fn new(program: &'static str, args: &'static [&'static str]) -> Self {
        Self { program, args }
    }

    /// The command as a person would type it, for copy-to-clipboard.
    pub fn display(self) -> String {
        let mut text = String::from(self.program);
        for argument in self.args {
            text.push(' ');
            if !argument.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '-' | '_' | '.' | '/' | ':' | '=')
            }) {
                text.push('\'');
                text.push_str(&argument.replace('\'', "'\\''"));
                text.push('\'');
            } else {
                text.push_str(argument);
            }
        }
        text
    }
}

/// How to get the tool on one platform, in the platform's own package manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstallCommand {
    pub platform: Platform,
    pub command: &'static str,
    /// Shown when the command needs a terminal the app will not open for you.
    pub note: &'static str,
}

/// Everything the manager shows about one tool.
///
/// `summary` answers "what do I make with it"; `benefit` answers "why is this
/// worth installing"; `license` and `size_hint` answer "what am I agreeing to".
#[derive(Clone, Debug)]
pub struct ToolDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub category: ToolCategory,
    pub summary: &'static str,
    pub benefit: &'static str,
    pub license: &'static str,
    /// A licence fact that changes whether a person may use it commercially.
    pub license_note: Option<&'static str>,
    pub homepage: &'static str,
    pub install_page: &'static str,
    pub install_commands: &'static [InstallCommand],
    pub size_hint: &'static str,
    pub runtime: &'static str,
    /// The companion GUI executable, when the tool has one.
    pub gui_program: Option<&'static str>,
    /// The probe command: running it must print a version on success.
    pub probe: CommandSpec,
    pub outputs: &'static [OutputKind],
    /// True when the tool is installed per project rather than per machine.
    pub project_scoped: bool,
    pub removable: bool,
    pub platforms: &'static [Platform],
    /// How to take it off this machine, in plain words.
    pub remove_hint: &'static str,
    /// How to get a newer release, in plain words.
    pub update_hint: &'static str,
}

impl ToolDescriptor {
    pub fn supports(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }

    pub fn install_command(&self, platform: Platform) -> Option<InstallCommand> {
        self.install_commands
            .iter()
            .copied()
            .find(|command| command.platform == platform)
    }

    pub fn output_labels(&self) -> Vec<&'static str> {
        self.outputs.iter().map(|output| output.label()).collect()
    }
}

const ALL: &[Platform] = &[Platform::MacOs, Platform::Windows, Platform::Linux];

/// The curated catalog. Order is the order the manager shows.
///
/// This is not every tool OR has ever considered; it is the set whose job,
/// licence and output OR can explain and hand back to a timeline. Libraries
/// that are evaluated but not launched live in the technology review.
pub const TOOLS: &[ToolDescriptor] = &[
    ToolDescriptor {
        id: "ffmpeg",
        name: "FFmpeg",
        category: ToolCategory::Media,
        summary: "Convert, probe and encode the media a project already holds.",
        benefit: "The same build OR ships is available to a person for a one-off conversion, and OR can reuse it for imports and exports.",
        license: "LGPL-2.1-or-later (build-dependent; GPL parts in some builds)",
        license_note: None,
        homepage: "https://ffmpeg.org",
        install_page: "https://ffmpeg.org/download.html",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "brew install ffmpeg",
                note: "Homebrew must already be installed.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "winget install Gyan.FFmpeg",
                note: "Run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo apt install ffmpeg",
                note: "Debian and Ubuntu.",
            },
        ],
        size_hint: "about 80 MB with codecs",
        runtime: "Native executable; no runtime to install",
        gui_program: None,
        probe: CommandSpec::new("ffmpeg", &["-version"]),
        outputs: &[
            OutputKind::Video,
            OutputKind::Audio,
            OutputKind::Image,
            OutputKind::ImageSequence,
        ],
        project_scoped: false,
        removable: false,
        platforms: ALL,
        remove_hint: "Bundled with OR; remove it with the app, or uninstall the system copy the same way you installed it.",
        update_hint: "Update the system copy with the same package manager (brew upgrade ffmpeg, winget upgrade, or sudo apt upgrade ffmpeg).",
    },
    ToolDescriptor {
        id: "node",
        name: "Node.js",
        category: ToolCategory::Runtime,
        summary: "Runs the script-based companions: SVGO, Remotion and Motion Canvas.",
        benefit: "One runtime unlocks the JavaScript motion and vector tools; OR does not bundle a second copy.",
        license: "MIT",
        license_note: None,
        homepage: "https://nodejs.org",
        install_page: "https://nodejs.org/en/download",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "brew install node",
                note: "Homebrew must already be installed.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "winget install OpenJS.NodeJS.LTS",
                note: "Run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo apt install nodejs npm",
                note: "Debian and Ubuntu; a newer release is available from nodejs.org.",
            },
        ],
        size_hint: "about 90 MB",
        runtime: "Native executable; npm is bundled",
        gui_program: None,
        probe: CommandSpec::new("node", &["--version"]),
        outputs: &[],
        project_scoped: false,
        removable: true,
        platforms: ALL,
        remove_hint: "brew uninstall node, winget uninstall OpenJS.NodeJS.LTS, or sudo apt remove nodejs npm.",
        update_hint: "brew upgrade node, winget upgrade OpenJS.NodeJS.LTS, or sudo apt upgrade nodejs.",
    },
    ToolDescriptor {
        id: "svgo",
        name: "SVGO",
        category: ToolCategory::Vector,
        summary: "Shrink an SVG before it becomes a layer: fewer points, same picture.",
        benefit: "Vector artwork keeps its structure and gets smaller; OR imports the optimized SVG and keeps the original beside it.",
        license: "MIT",
        license_note: None,
        homepage: "https://github.com/svg/svgo",
        install_page: "https://github.com/svg/svgo#installation",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "npm install -g svgo",
                note: "Needs Node.js.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "npm install -g svgo",
                note: "Needs Node.js; run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo npm install -g svgo",
                note: "Needs Node.js.",
            },
        ],
        size_hint: "about 1.5 MB",
        runtime: "Node.js",
        gui_program: None,
        probe: CommandSpec::new("svgo", &["--version"]),
        outputs: &[OutputKind::Svg],
        project_scoped: false,
        removable: true,
        platforms: ALL,
        remove_hint: "npm uninstall -g svgo (prefix with sudo on Linux if it was installed globally as root).",
        update_hint: "npm update -g svgo.",
    },
    ToolDescriptor {
        id: "inkscape",
        name: "Inkscape",
        category: ToolCategory::Vector,
        summary: "Draw or repair vector art, then rasterize it at the size the timeline needs.",
        benefit: "A real vector editor for titles and diagrams; its command line turns the SVG into a PNG OR can place as a layer.",
        license: "GPL-2.0-or-later",
        license_note: Some("Run as a separate program; OR does not link GPL code."),
        homepage: "https://inkscape.org",
        install_page: "https://inkscape.org/release/",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "brew install --cask inkscape",
                note: "Large download; the cask installs the app.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "winget install Inkscape.Inkscape",
                note: "Run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo apt install inkscape",
                note: "Debian and Ubuntu.",
            },
        ],
        size_hint: "about 200 MB",
        runtime: "Native executable; GTK runtime is bundled",
        gui_program: Some("inkscape"),
        probe: CommandSpec::new("inkscape", &["--version"]),
        outputs: &[OutputKind::Svg, OutputKind::Image],
        project_scoped: false,
        removable: true,
        platforms: ALL,
        remove_hint: "brew uninstall --cask inkscape, winget uninstall Inkscape.Inkscape, or sudo apt remove inkscape.",
        update_hint: "brew upgrade --cask inkscape, winget upgrade Inkscape.Inkscape, or sudo apt upgrade inkscape.",
    },
    ToolDescriptor {
        id: "blender",
        name: "Blender",
        category: ToolCategory::ThreeD,
        summary: "Build a 3D scene, camera and light, then render frames or a clip for the timeline.",
        benefit: "3D motion graphics without OR growing a renderer; the .blend stays the editable source beside the rendered clip.",
        license: "GPL-3.0-or-later",
        license_note: Some("Run as a separate program; OR does not link GPL code."),
        homepage: "https://www.blender.org",
        install_page: "https://www.blender.org/download/",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "brew install --cask blender",
                note: "Large download.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "winget install BlenderFoundation.Blender",
                note: "Run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo apt install blender",
                note: "Debian and Ubuntu; the Flatpak build is also available.",
            },
        ],
        size_hint: "about 350 MB",
        runtime: "Native executable; Python is bundled",
        gui_program: Some("blender"),
        probe: CommandSpec::new("blender", &["--version"]),
        outputs: &[
            OutputKind::ImageSequence,
            OutputKind::Video,
            OutputKind::Gltf,
        ],
        project_scoped: false,
        removable: true,
        platforms: ALL,
        remove_hint: "brew uninstall --cask blender, winget uninstall BlenderFoundation.Blender, or sudo apt remove blender.",
        update_hint: "brew upgrade --cask blender, winget upgrade BlenderFoundation.Blender, or sudo apt upgrade blender.",
    },
    ToolDescriptor {
        id: "audacity",
        name: "Audacity",
        category: ToolCategory::Audio,
        summary: "Record, clean and arrange a voiceover or a music bed outside the editor.",
        benefit: "A full audio editor for work that is awkward on a timeline; the exported WAV drops straight back in.",
        license: "GPL-2.0-or-later",
        license_note: Some("Run as a separate program; OR does not link GPL code."),
        homepage: "https://www.audacityteam.org",
        install_page: "https://www.audacityteam.org/download/",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "brew install --cask audacity",
                note: "Homebrew must already be installed.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "winget install Audacity.Audacity",
                note: "Run in PowerShell.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "sudo apt install audacity",
                note: "Debian and Ubuntu.",
            },
        ],
        size_hint: "about 120 MB",
        runtime: "Native executable",
        gui_program: Some("audacity"),
        probe: CommandSpec::new("audacity", &["--version"]),
        outputs: &[OutputKind::Audio],
        project_scoped: false,
        removable: true,
        platforms: ALL,
        remove_hint: "brew uninstall --cask audacity, winget uninstall Audacity.Audacity, or sudo apt remove audacity.",
        update_hint: "brew upgrade --cask audacity, winget upgrade Audacity.Audacity, or sudo apt upgrade audacity.",
    },
    ToolDescriptor {
        id: "remotion",
        name: "Remotion",
        category: ToolCategory::Composition,
        summary: "Render a React composition into a video file OR imports as media.",
        benefit: "Teams already building React video can keep that project and render it from OR; the source project is recorded with the clip.",
        license: "Remotion License (source-available; not OSI)",
        license_note: Some(
            "Free for individuals and small teams; a company licence is required for larger commercial use. OR never bundles it.",
        ),
        homepage: "https://www.remotion.dev",
        install_page: "https://www.remotion.dev/docs/",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "npx create-video@latest",
                note: "Per project, not per machine; needs Node.js and a browser download.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "npx create-video@latest",
                note: "Per project, not per machine; needs Node.js and a browser download.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "npx create-video@latest",
                note: "Per project, not per machine; needs Node.js and a browser download.",
            },
        ],
        size_hint: "about 300 MB per project with the browser",
        runtime: "Node.js and a bundled Chromium",
        gui_program: None,
        probe: CommandSpec::new("npx", &["--no-install", "remotion", "--version"]),
        outputs: &[OutputKind::Video, OutputKind::ProjectManifest],
        project_scoped: true,
        removable: true,
        platforms: ALL,
        remove_hint: "Delete the project folder; Remotion is installed per project and OR stores nothing else.",
        update_hint: "Run npm update inside the project.",
    },
    ToolDescriptor {
        id: "motion-canvas",
        name: "Motion Canvas",
        category: ToolCategory::Composition,
        summary: "Animate a code-driven scene in its editor, then render frames or a video.",
        benefit: "A visual editor for generator-based animation; rendered output lands as a clip and the project stays the source.",
        license: "MIT",
        license_note: None,
        homepage: "https://motioncanvas.io",
        install_page: "https://motioncanvas.io/docs/quickstart",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "npm init @motion-canvas@latest",
                note: "Per project, not per machine; needs Node.js.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "npm init @motion-canvas@latest",
                note: "Per project, not per machine; needs Node.js.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "npm init @motion-canvas@latest",
                note: "Per project, not per machine; needs Node.js.",
            },
        ],
        size_hint: "about 150 MB per project",
        runtime: "Node.js",
        gui_program: None,
        probe: CommandSpec::new("npx", &["--no-install", "motion-canvas", "--version"]),
        outputs: &[
            OutputKind::Video,
            OutputKind::ImageSequence,
            OutputKind::ProjectManifest,
        ],
        project_scoped: true,
        removable: true,
        platforms: ALL,
        remove_hint: "Delete the project folder; Motion Canvas is installed per project and OR stores nothing else.",
        update_hint: "Run npm update inside the project.",
    },
    ToolDescriptor {
        id: "lottie",
        name: "Lottie renderer",
        category: ToolCategory::Animation,
        summary: "Draw a Lottie animation with a browser and save it as a video or an image sequence.",
        benefit: "An After Effects animation renders without OR carrying a Lottie runtime: the animation file stays the source, and the render lands as a clip with that file recorded beside it.",
        license: "MIT (puppeteer-lottie, lottie-web)",
        license_note: Some(
            "Runs the animation in a browser OR starts; OR bundles neither the renderer nor a Chromium download.",
        ),
        homepage: "https://github.com/transitive-bullshit/puppeteer-lottie",
        install_page: "https://github.com/transitive-bullshit/puppeteer-lottie#readme",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "npm install puppeteer-lottie",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "npm install puppeteer-lottie",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "npm install puppeteer-lottie",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
        ],
        size_hint: "about 300 MB with the browser download",
        runtime: "Node.js and a Chromium browser",
        gui_program: None,
        probe: CommandSpec::new("npx", &["--no-install", "puppeteer-lottie", "--version"]),
        outputs: &[
            OutputKind::Lottie,
            OutputKind::Video,
            OutputKind::ImageSequence,
        ],
        project_scoped: true,
        removable: true,
        platforms: ALL,
        remove_hint: "Delete the renderer folder; it is installed per project and OR stores nothing else.",
        update_hint: "Run npm update inside the renderer folder.",
    },
    ToolDescriptor {
        id: "gltf",
        name: "glTF renderer",
        category: ToolCategory::ThreeD,
        summary: "Draw a glTF or GLB model with a browser and save it as a video or an image sequence.",
        benefit: "A model exported from Blender or another 3D tool becomes a clip without OR carrying a glTF engine: the model file stays the source and is recorded beside the render.",
        license: "MIT (three.js)",
        license_note: Some(
            "Runs the model in a browser OR starts; OR bundles neither three.js nor a Chromium download.",
        ),
        homepage: "https://threejs.org",
        install_page: "https://threejs.org/docs/#examples/en/loaders/GLTFLoader",
        install_commands: &[
            InstallCommand {
                platform: Platform::MacOs,
                command: "npm install three",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
            InstallCommand {
                platform: Platform::Windows,
                command: "npm install three",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
            InstallCommand {
                platform: Platform::Linux,
                command: "npm install three",
                note: "Per project, not per machine; needs Node.js and a browser.",
            },
        ],
        size_hint: "about 300 MB with the browser download",
        runtime: "Node.js and a Chromium browser",
        gui_program: None,
        probe: CommandSpec::new("npx", &["--no-install", "three", "--version"]),
        outputs: &[
            OutputKind::Gltf,
            OutputKind::Video,
            OutputKind::ImageSequence,
        ],
        project_scoped: true,
        removable: true,
        platforms: ALL,
        remove_hint: "Delete the renderer folder; it is installed per project and OR stores nothing else.",
        update_hint: "Run npm update inside the renderer folder.",
    },
];

/// The catalog entry for an id, if the build carries one.
pub fn tool(id: &str) -> Option<&'static ToolDescriptor> {
    TOOLS.iter().find(|tool| tool.id == id)
}

/// The tools that can run on a platform, in catalog order.
pub fn tools_for(platform: Platform) -> impl Iterator<Item = &'static ToolDescriptor> {
    TOOLS.iter().filter(move |tool| tool.supports(platform))
}

/// What a probe found.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolState {
    /// The executable answered with a version.
    Ready,
    /// The executable is not on PATH.
    Missing,
    /// The executable ran but did not answer like a working tool.
    Failed,
    /// The catalog knows the tool but not on this platform.
    Unsupported,
    /// The tool is installed inside a project, so a machine-wide probe cannot
    /// answer for it.
    ProjectScoped,
}

impl ToolState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Missing => "Not installed",
            Self::Failed => "Needs attention",
            Self::Unsupported => "Not supported here",
            Self::ProjectScoped => "Installed per project",
        }
    }
}

/// One tool's detection result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolStatus {
    pub id: String,
    pub state: ToolState,
    /// The resolved executable, when one was found.
    pub executable: Option<String>,
    /// The version the tool reported, normalized to digits and dots.
    pub version: Option<String>,
    /// One line a person can act on: the error, or what was found.
    pub detail: String,
}

/// The result of running one command.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program. The app, the CLI and tests supply different implementations;
/// the probe policy above never spawns anything itself.
pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, String>;
}

/// Extracts the first version-looking token from a tool's output.
///
/// Handles `v24.19.0`, `ffmpeg version 9.0.1 Copyright`, `Blender 4.3.2` and
/// `Inkscape 1.4 (e7c3feb100, 2024-10-09)`. Returns digits and dots only, so a
/// screen never has to trim a vendor banner itself.
pub fn parse_version(output: &str) -> Option<String> {
    for token in output.split(|character: char| {
        character.is_whitespace() || matches!(character, ',' | ';' | '(' | ')' | '[' | ']')
    }) {
        let token = token.trim_start_matches('v');
        // Take the leading version-shaped run, then stop at a distributor
        // suffix: Debian's `7.1.5-0+deb13u1` must report 7.1.5, not 7.1.
        let token: String = token
            .chars()
            .take_while(|character| character.is_ascii_digit() || *character == '.')
            .collect();
        let token = token.trim_end_matches('.');
        let mut parts = token.split('.');
        let major = parts.next()?;
        if major.is_empty() || !major.chars().all(|character| character.is_ascii_digit()) {
            continue;
        }
        let mut version = String::from(major);
        for (dots, part) in parts.enumerate() {
            if dots == 2 || part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
                break;
            }
            version.push('.');
            version.push_str(part);
        }
        return Some(version);
    }
    None
}

/// Probes one catalog entry through the caller's runner.
pub fn probe_tool(
    descriptor: &ToolDescriptor,
    platform: Platform,
    runner: &dyn CommandRunner,
) -> ToolStatus {
    if descriptor.project_scoped {
        return ToolStatus {
            id: descriptor.id.to_owned(),
            state: ToolState::ProjectScoped,
            executable: None,
            version: None,
            detail: format!(
                "{} is installed inside a project. OR uses it while that project is \
                 open; run the setup command below in the project folder.",
                descriptor.name
            ),
        };
    }

    if !descriptor.supports(platform) {
        return ToolStatus {
            id: descriptor.id.to_owned(),
            state: ToolState::Unsupported,
            executable: None,
            version: None,
            detail: format!(
                "{} is not available on {}.",
                descriptor.name,
                platform.label()
            ),
        };
    }

    let command = descriptor.probe;
    match runner.run(command.program, command.args) {
        Ok(output) => {
            let text = if output.stdout.trim().is_empty() {
                &output.stderr
            } else {
                &output.stdout
            };
            let version = output.success.then(|| parse_version(text)).flatten();
            if let Some(found) = version.as_deref() {
                return ToolStatus {
                    id: descriptor.id.to_owned(),
                    state: ToolState::Ready,
                    executable: Some(command.program.to_owned()),
                    version: Some(found.to_owned()),
                    detail: format!("{} answered.", descriptor.name),
                };
            }
            ToolStatus {
                id: descriptor.id.to_owned(),
                state: ToolState::Failed,
                executable: Some(command.program.to_owned()),
                version,
                detail: first_line(text).unwrap_or_else(|| {
                    format!(
                        "{} ran but did not print a version; the installed build may be incomplete.",
                        descriptor.name
                    )
                }),
            }
        }
        Err(error) => ToolStatus {
            id: descriptor.id.to_owned(),
            state: ToolState::Missing,
            executable: None,
            version: None,
            detail: if error.trim().is_empty() {
                format!("{} was not found on PATH.", descriptor.name)
            } else {
                first_line(&error).unwrap_or_else(|| format!("{} was not found.", descriptor.name))
            },
        },
    }
}

/// Probes every tool the platform supports.
pub fn probe_all(platform: Platform, runner: &dyn CommandRunner) -> Vec<ToolStatus> {
    tools_for(platform)
        .map(|descriptor| probe_tool(descriptor, platform, runner))
        .collect()
}

fn first_line(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    const LIMIT: usize = 240;
    if line.chars().count() <= LIMIT {
        return Some(line.to_owned());
    }
    Some(line.chars().take(LIMIT).collect::<String>() + "…")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRunner {
        answers: Vec<(&'static str, Result<CommandOutput, String>)>,
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &str, _args: &[&str]) -> Result<CommandOutput, String> {
            self.answers
                .iter()
                .find(|(name, _)| *name == program)
                .map(|(_, answer)| answer.clone())
                .unwrap_or_else(|| Err(format!("{program}: command not found")))
        }
    }

    fn answer(stdout: &str) -> Result<CommandOutput, String> {
        Ok(CommandOutput {
            success: true,
            stdout: stdout.to_owned(),
            stderr: String::new(),
        })
    }

    #[test]
    fn catalog_ids_are_unique_and_described() {
        let mut ids: Vec<&str> = TOOLS.iter().map(|tool| tool.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TOOLS.len(), "tool ids must be unique");
        for tool in TOOLS {
            assert!(!tool.summary.is_empty(), "{} has no summary", tool.id);
            assert!(!tool.benefit.is_empty(), "{} has no benefit", tool.id);
            assert!(!tool.license.is_empty(), "{} has no licence", tool.id);
            assert!(
                tool.install_page.starts_with("https://"),
                "{} has no https install page",
                tool.id
            );
            assert!(!tool.platforms.is_empty(), "{} has no platforms", tool.id);
            assert!(
                !tool.project_scoped || tool.removable,
                "{} is project scoped but cannot be removed",
                tool.id
            );
        }
    }

    #[test]
    fn every_platform_has_an_install_command_for_machine_scoped_tools() {
        for tool in TOOLS.iter().filter(|tool| !tool.project_scoped) {
            for platform in tool.platforms {
                assert!(
                    tool.install_command(*platform).is_some(),
                    "{} has no install command for {}",
                    tool.id,
                    platform.label()
                );
            }
        }
    }

    #[test]
    fn version_parsing_handles_real_banners() {
        assert_eq!(parse_version("v24.19.0"), Some("24.19.0".to_owned()));
        assert_eq!(
            parse_version("ffmpeg version 9.0.1 Copyright (c) 2000-2026"),
            Some("9.0.1".to_owned())
        );
        assert_eq!(parse_version("Blender 4.3.2"), Some("4.3.2".to_owned()));
        assert_eq!(
            parse_version("Inkscape 1.4 (e7c3feb100, 2024-10-09)"),
            Some("1.4".to_owned())
        );
        assert_eq!(
            parse_version("ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026"),
            Some("7.1.5".to_owned())
        );
        assert_eq!(parse_version("no digits here"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn a_ready_tool_must_answer_with_a_version() {
        let runner = FakeRunner {
            answers: vec![("ffmpeg", answer("ffmpeg version 9.0.1\n"))],
        };
        let status = probe_tool(tool("ffmpeg").unwrap(), Platform::Linux, &runner);
        assert_eq!(status.state, ToolState::Ready);
        assert_eq!(status.version.as_deref(), Some("9.0.1"));

        let runner = FakeRunner {
            answers: vec![(
                "ffmpeg",
                Ok(CommandOutput {
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                }),
            )],
        };
        let status = probe_tool(tool("ffmpeg").unwrap(), Platform::Linux, &runner);
        assert_eq!(status.state, ToolState::Failed);
    }

    #[test]
    fn a_missing_executable_is_reported_as_missing_not_failed() {
        let runner = FakeRunner { answers: vec![] };
        let status = probe_tool(tool("blender").unwrap(), Platform::MacOs, &runner);
        assert_eq!(status.state, ToolState::Missing);
        assert!(status.detail.contains("not found"));
    }

    #[test]
    fn a_tool_outside_its_platforms_is_unsupported() {
        let mut descriptor = tool("ffmpeg").unwrap().clone();
        descriptor.platforms = &[Platform::MacOs];
        let runner = FakeRunner { answers: vec![] };
        let status = probe_tool(&descriptor, Platform::Linux, &runner);
        assert_eq!(status.state, ToolState::Unsupported);
    }

    #[test]
    fn structured_outputs_are_editable_and_video_is_not() {
        assert!(OutputKind::Svg.is_structured());
        assert!(OutputKind::Gltf.is_structured());
        assert!(!OutputKind::Video.is_structured());
        assert!(!OutputKind::Image.is_structured());
    }

    #[test]
    fn command_display_quotes_arguments_with_spaces() {
        let spec = CommandSpec::new("node", &["-e", "console.log(1)"]);
        assert_eq!(spec.display(), "node -e 'console.log(1)'");
    }
}
