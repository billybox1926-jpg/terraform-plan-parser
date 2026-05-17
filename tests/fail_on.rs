use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is after unix epoch")
        .as_nanos();
    dir.push(format!(
        "terraform_plan_parser_{name}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

const DELETE_PLAN: &str = r#"{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}
"#;

#[test]
fn fail_on_delete_exits_non_zero_after_printing_output() {
    let root = temp_dir("fail_on_delete");
    let plan_file = root.join("plan.ndjson");
    fs::write(&plan_file, DELETE_PLAN).expect("write plan fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .current_dir(&root)
        .arg("--plan-file")
        .arg("plan.ndjson")
        .arg("--format")
        .arg("csv")
        .arg("--fail-on")
        .arg("delete")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        !output.status.success(),
        "expected non-zero exit when delete actions are present"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("delete"));

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn fail_on_delete_passes_when_only_create_actions_present() {
    let root = temp_dir("fail_on_create_only");
    let plan_file = root.join("plan.ndjson");
    fs::write(
        &plan_file,
        r#"{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
"#,
    )
    .expect("write plan fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .current_dir(&root)
        .arg("--plan-file")
        .arg("plan.ndjson")
        .arg("--format")
        .arg("csv")
        .arg("--fail-on")
        .arg("delete")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}
