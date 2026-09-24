use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1)) {
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

fn run(args: impl IntoIterator<Item = OsString>) -> Result<String, String> {
    let args: Vec<_> = args.into_iter().collect();

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
    "Usage: or <version|health|capabilities> [--json]\n       or --help"
}
