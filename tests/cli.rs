//! End-to-end CLI tests running the real binary.

use std::process::Command;

fn sonda(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sonda"))
        .args(args)
        .output()
        .expect("failed to spawn sonda")
}

#[test]
fn default_scan_outputs_contract_json() {
    let out = sonda(&[]);
    assert!(out.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout is not valid JSON");
    assert_eq!(value["command"], "scan");
    assert!(value["generated_at"].is_string());
    assert!(value["os"].is_object());
    assert!(value["cpu"].is_object());
    assert!(value["memory"].is_object());
    assert!(value["pci_devices"].is_array());
    assert!(value["errors"].is_array());
    // Nullability policy: without root the DMI section stays null.
    let dmi = &value["dmi"];
    if dmi["available"] == serde_json::json!(false) {
        assert_eq!(dmi["board_vendor"], serde_json::json!(null));
    }
}

#[test]
fn compact_flag_emits_single_line_json() {
    let out = sonda(&["--compact"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(value["command"], "scan");
}

#[test]
fn summary_mode_prints_human_lines() {
    let out = sonda(&["--summary"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("GPU:"));
    assert!(stdout.contains("Kernel:"));
}
