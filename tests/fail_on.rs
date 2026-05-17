use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = env::temp_dir();

    #[cfg(windows)]
    {
        let dir_str = dir.to_string_lossy();
        if dir_str.starts_with(r"\\?\") {
            dir = PathBuf::from(&dir_str[4..]);
        }
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is after unix epoch")
        .as_nanos();
    dir.push(format!(
        "terraform_plan_parser_fail_on_{name}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[cfg(unix)]
fn write_mock_terraform(bin_dir: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let terraform = bin_dir.join("terraform");
    fs::write(
        &terraform,
        r#"#!/bin/sh
set -eu
case "$1" in
  version)
    echo "Terraform v1.6.0"
    ;;
  plan)
    echo '{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}'
    echo '{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}'
    ;;
  *)
    echo "unexpected terraform command: $*" >&2
    exit 1
    ;;
esac
"#,
    )
    .expect("write mock terraform");
    let mut permissions = fs::metadata(&terraform)
        .expect("read mock metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(terraform, permissions).expect("make mock executable");
}

#[cfg(windows)]
fn write_mock_terraform(bin_dir: &Path) {
    fs::write(
        bin_dir.join("terraform.bat"),
        r#"@echo off
if "%1" == "version" (
  echo Terraform v1.6.0
  exit /b 0
)
if "%1" == "plan" (
  echo {"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
  echo {"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}
  exit /b 0
)
echo unexpected terraform command: %* 1>&2
exit /b 1
"#,
    )
    .expect("write mock terraform");
}

fn prepend_path(bin_dir: &Path) -> String {
    let existing = env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir.to_path_buf()];
    paths.extend(env::split_paths(&existing));
    env::join_paths(paths)
        .expect("join PATH entries")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn fail_on_delete_exits_non_zero() {
    let root = temp_dir("fail_on_delete");
    let bin_dir = root.join("bin");
    let project_dir = root.join("project");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&project_dir).expect("create project dir");
    write_mock_terraform(&bin_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--fail-on")
        .arg("delete")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --fail-on delete matches a delete action"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("forbidden actions"));

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn fail_on_create_exits_non_zero() {
    let root = temp_dir("fail_on_create");
    let bin_dir = root.join("bin");
    let project_dir = root.join("project");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&project_dir).expect("create project dir");
    write_mock_terraform(&bin_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--fail-on")
        .arg("create")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --fail-on create matches a create action"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn fail_on_update_exits_zero_when_no_match() {
    let root = temp_dir("fail_on_update");
    let bin_dir = root.join("bin");
    let project_dir = root.join("project");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&project_dir).expect("create project dir");
    write_mock_terraform(&bin_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--fail-on")
        .arg("update")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "expected zero exit when --fail-on update does not match any action"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn fail_on_comma_separated_actions() {
    let root = temp_dir("fail_on_comma");
    let bin_dir = root.join("bin");
    let project_dir = root.join("project");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&project_dir).expect("create project dir");
    write_mock_terraform(&bin_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--fail-on")
        .arg("delete,replace")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --fail-on delete matches"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}
