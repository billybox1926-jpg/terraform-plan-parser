use std::{fs, path::Path, process::Command};

#[test]
fn writes_github_step_summary_when_env_is_set() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plan.ndjson");
    let dir = std::env::temp_dir().join(format!(
        "terraform_plan_parser_github_summary_{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    let summary_path = dir.join("step-summary.md");

    let status = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&fixture)
        .arg("--no-emoji")
        .env("PATH", "")
        .env("GITHUB_STEP_SUMMARY", &summary_path)
        .status()
        .expect("run terraform_plan_parser");

    assert!(status.success());
    let written = fs::read_to_string(&summary_path).expect("read summary");
    assert!(written.contains("## Terraform plan summary"));
    assert!(written.contains("### Resource changes"));
    let _ = fs::remove_dir_all(dir);
}
