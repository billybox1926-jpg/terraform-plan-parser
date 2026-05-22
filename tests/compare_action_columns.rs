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
        "terraform_plan_parser_compare_columns_{name}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_compare_fixtures(root: &Path) -> (PathBuf, PathBuf) {
    let old_plan = root.join("old.ndjson");
    let new_plan = root.join("new.ndjson");

    fs::write(
        &old_plan,
        r#"{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"read"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_lambda_function","resource_name":"worker"},"action":"create"}}
"#,
    )
    .expect("write old plan fixture");

    fs::write(
        &new_plan,
        r#"{"@level":"info","change":{"resource":{"resource_type":"aws_iam_role","resource_name":"reader"},"action":"replace"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_lambda_function","resource_name":"worker"},"action":"update"}}
"#,
    )
    .expect("write new plan fixture");

    (old_plan, new_plan)
}

#[test]
fn compare_csv_uses_real_actions_for_all_row_types() {
    let root = temp_dir("csv");
    let (old_plan, new_plan) = write_compare_fixtures(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg("--compare")
        .arg(&old_plan)
        .arg(&new_plan)
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
        "change_type,resource_type,resource_name,old_action,new_action\nadded,aws_iam_role,reader,,replace\nremoved,aws_s3_bucket,logs,read,\nchanged,aws_lambda_function,worker,create,update\n"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn compare_table_places_real_actions_in_old_and_new_columns() {
    let root = temp_dir("table");
    let (old_plan, new_plan) = write_compare_fixtures(&root);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg("--compare")
        .arg(&old_plan)
        .arg(&new_plan)
        .arg("--format")
        .arg("table")
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
    let header = stdout
        .lines()
        .find(|line| line.contains("Old Action") && line.contains("New Action"))
        .expect("table header should contain action columns");
    let old_action_col = header.find("Old Action").expect("old action column");
    let new_action_col = header.find("New Action").expect("new action column");

    let added = stdout
        .lines()
        .find(|line| line.contains("aws_iam_role") && line.contains("reader"))
        .expect("added row should render");
    assert_eq!(added[old_action_col..new_action_col].trim(), "");
    assert_eq!(added[new_action_col..].trim(), "replace");

    let removed = stdout
        .lines()
        .find(|line| line.contains("aws_s3_bucket") && line.contains("logs"))
        .expect("removed row should render");
    assert_eq!(removed[old_action_col..new_action_col].trim(), "read");
    assert_eq!(removed[new_action_col..].trim(), "");

    let changed = stdout
        .lines()
        .find(|line| line.contains("aws_lambda_function") && line.contains("worker"))
        .expect("changed row should render");
    assert_eq!(changed[old_action_col..new_action_col].trim(), "create");
    assert_eq!(changed[new_action_col..].trim(), "update");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
