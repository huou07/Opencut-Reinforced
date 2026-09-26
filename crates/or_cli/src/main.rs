mod commands;

use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args
        .first()
        .is_some_and(|argument| is_semantic_command(argument))
    {
        return match commands::run(args) {
            Ok(commands::CommandOutput::Immediate(output)) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Ok(commands::CommandOutput::Serve {
                server,
                startup,
                json,
            }) => {
                println!("{startup}");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                match server.wait() {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        let error = commands::CliError::ipc(error, json);
                        eprintln!("{}", error.render());
                        ExitCode::from(error.exit_code())
                    }
                }
            }
            Err(error) => {
                if error.json() {
                    println!("{}", error.render());
                } else {
                    eprintln!("{}", error.render());
                }
                ExitCode::from(error.exit_code())
            }
        };
    }

    match run_bootstrap(args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run_bootstrap(args: Vec<OsString>) -> Result<String, String> {
    if args.is_empty()
        || (args.len() == 1 && is(&args[0], "--help"))
        || (args.len() == 1 && is(&args[0], "-h"))
    {
        return Ok(usage().to_owned());
    }

    if args.len() > 2 {
        return Err(format!("unexpected argument\n\n{}", usage()));
    }

    let command = &args[0];
    let json = if args.len() == 2 {
        if is(&args[1], "--json") {
            true
        } else {
            return Err(format!(
                "unknown option: {}\n\n{}",
                args[1].to_string_lossy(),
                usage()
            ));
        }
    } else {
        false
    };

    if is(command, "version") {
        let info = or_core::app_info();
        if json {
            serde_json::to_string_pretty(&info).map_err(|error| error.to_string())
        } else {
            Ok(format!("{} {}", info.name, info.version))
        }
    } else if is(command, "health") {
        let health = or_core::health();
        if json {
            serde_json::to_string_pretty(&health).map_err(|error| error.to_string())
        } else {
            Ok(health.status)
        }
    } else if is(command, "capabilities") {
        let capabilities = or_core::capabilities();
        if json {
            serde_json::to_string_pretty(&serde_json::json!({ "capabilities": capabilities }))
                .map_err(|error| error.to_string())
        } else {
            Ok(capabilities
                .iter()
                .map(|capability| format!("{} v{}", capability.id, capability.version))
                .collect::<Vec<_>>()
                .join("\n"))
        }
    } else {
        Err(format!(
            "unknown command: {}\n\n{}",
            command.to_string_lossy(),
            usage()
        ))
    }
}

fn is(value: &OsStr, expected: &str) -> bool {
    value == OsStr::new(expected)
}

fn usage() -> &'static str {
    "Usage:\n  or <version|health|capabilities> [--json]\n  or <commands|queries> [--json]\n  or project summary --file PATH|--attach DESCRIPTOR [--json]\n  or project rename --file PATH|--attach DESCRIPTOR --name NAME [--json]\n  or project save --attach DESCRIPTOR [--json]\n  or history <undo|redo> --attach DESCRIPTOR [--json]\n  or recovery <status|apply|discard> --file PATH [--json]\n  or media probe --file PATH [--json]\n  or media list --project PATH|--attach DESCRIPTOR [--offset N] [--limit N] [--json]\n  or media add --project PATH|--attach DESCRIPTOR --source PATH [--json]\n  or media remove --project PATH|--attach DESCRIPTOR --id MEDIA_ID [--json]\n  or session serve --file PATH [--descriptor PATH] [--json]\n  or session describe --attach DESCRIPTOR [--json]\n  or session shutdown --attach DESCRIPTOR [--discard-unsaved] [--json]\n  or --help"
}

fn is_semantic_command(value: &OsStr) -> bool {
    [
        "commands", "queries", "project", "history", "recovery", "media", "session",
    ]
    .iter()
    .any(|command| is(value, command))
}
