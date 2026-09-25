use serde_json::Value;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_or"))
        .args(args)
        .output()
        .expect("run or executable")
}

fn json(args: &[&str]) -> Value {
    let output = run(args);
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("valid JSON output")
}

#[test]
fn version_has_human_and_json_output() {
    let human = run(&["version"]);
    assert!(human.status.success());
    assert!(String::from_utf8_lossy(&human.stdout).starts_with("Opencut Reinforced "));

    let value = json(&["version", "--json"]);
    assert_eq!(value["name"], "Opencut Reinforced");
    assert!(!value["version"].as_str().unwrap().is_empty());
    assert_eq!(value["core_api_version"], 1);
}

#[test]
fn health_has_human_and_json_output() {
    let human = run(&["health"]);
    assert!(human.status.success());
    assert_eq!(String::from_utf8_lossy(&human.stdout).trim(), "ok");

    assert_eq!(json(&["health", "--json"])["status"], "ok");
}

#[test]
fn capabilities_json_contains_only_bootstrap_capabilities() {
    let value = json(&["capabilities", "--json"]);
    let ids: Vec<_> = value["capabilities"]
        .as_array()
        .expect("capabilities array")
        .iter()
        .map(|capability| capability["id"].as_str().unwrap())
        .collect();

    assert_eq!(ids, ["core.app_info", "core.health", "core.capabilities"]);
}

#[test]
fn help_and_invalid_input_have_expected_exit_behavior() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("recovery <status|apply|discard>"));

    let unknown = run(&["timeline"]);
    assert!(!unknown.status.success());
    assert!(!String::from_utf8_lossy(&unknown.stderr).contains("panicked"));

    let invalid_option = run(&["health", "--yaml"]);
    assert!(!invalid_option.status.success());
    assert!(!String::from_utf8_lossy(&invalid_option.stderr).contains("panicked"));
}
