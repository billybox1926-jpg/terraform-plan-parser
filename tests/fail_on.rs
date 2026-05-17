use std::{path::Path, process::Command};

#[test]
fn fail_on_exits_non_zero_for_matching_filtered_action() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plan.ndjson");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&fixture)
        .arg("--fail-on")
        .arg("update")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("Plan contains forbidden actions matching --fail-on criteria"));
}
