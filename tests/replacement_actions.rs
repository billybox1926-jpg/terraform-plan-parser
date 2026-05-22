use std::{
    env, fs,
    path::{Path, PathBuf},
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
        "terraform_plan_parser_replacement_{name}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_replacement_plan(path: &Path) {
    fs::write(
        path,
        r#"{
  "resource_changes": [
    {
      "type": "aws_instance",
      "name": "delete_create",
      "change": { "actions": ["delete", "create"] }
    },
    {
      "type": "aws_instance",
      "name": "create_delete",
      "change": { "actions": ["create", "delete"] }
    },
    {
      "type": "aws_s3_bucket",
      "name": "unchanged",
      "change": { "actions": ["no-op"] }
    }
  ]
}"#,
    )
    .expect("write replacement plan fixture");
}

#[test]
fn replacement_action_orders_normalize_to_replace_in_csv() {
    let root = temp_dir("csv");
    let plan_file = root.join("plan.json");
    write_replacement_plan(&plan_file);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&plan_file)
        .arg("--format")
        .arg("csv")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "resource_type,resource_name,action\naws_instance,delete_create,replace\naws_instance,create_delete,replace\n"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn include_action_replace_matches_both_replacement_orders() {
    let root = temp_dir("filter");
    let plan_file = root.join("plan.json");
    write_replacement_plan(&plan_file);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&plan_file)
        .arg("--format")
        .arg("csv")
        .arg("--include-action")
        .arg("replace")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("aws_instance,delete_create,replace"));
    assert!(stdout.contains("aws_instance,create_delete,replace"));
    assert!(!stdout.contains("unchanged"));

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn replacement_summary_counts_create_and_delete_totals() {
    let root = temp_dir("summary");
    let plan_file = root.join("plan.json");
    write_replacement_plan(&plan_file);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&plan_file)
        .arg("--no-emoji")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("aws_instance delete_create (replace)"));
    assert!(stdout.contains("aws_instance create_delete (replace)"));
    assert!(stdout.contains("+ 2 to create"));
    assert!(stdout.contains("~ 0 to update"));
    assert!(stdout.contains("- 2 to delete"));

    fs::remove_dir_all(root).expect("remove temp dir");
}
