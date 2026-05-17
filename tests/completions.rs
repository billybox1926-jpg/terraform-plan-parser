use std::{env, process::Command};

#[test]
fn generates_bash_completions() {
    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg("--completions")
        .arg("bash")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("terraform_plan_parser"));
    assert!(stdout.contains("complete"));
}

#[test]
fn generates_zsh_completions() {
    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg("--completions")
        .arg("zsh")
        .output()
        .expect("run terraform_plan_parser");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("#compdef"));
}

#[test]
fn generates_fish_completions() {
    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg("--completions")
        .arg("fish")
        .output()
        .expect("run terraform_plan_parser");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("terraform_plan_parser"));
}
