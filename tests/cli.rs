use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_dir(name: &str) -> PathBuf {
    let mut dir = env::temp_dir();

    // On Windows, the `\\?\` prefix breaks PATH lookups for child processes.
    // Stripping it allows the mock `terraform.bat` to be found in the PATH.
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
        "terraform_plan_parser_{name}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
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
  show)
    cat <<'JSON'
{"resource_changes":[{"type":"aws_instance","name":"web","change":{"actions":["delete","create"]}}]}
JSON
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
if "%1" == "show" (
  echo {"resource_changes":[{"type":"aws_instance","name":"web","change":{"actions":["delete","create"]}}]}
  exit /b 0
)
echo unexpected terraform command: %* 1>&2
exit /b 1
"#,
    )
    .expect("write mock terraform");
}

#[test]
fn renders_csv_from_mocked_live_plan() {
    let root = temp_dir("live_plan");
    let bin_dir = root.join("bin");
    let project_dir = root.join("project");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&project_dir).expect("create project dir");
    write_mock_terraform(&bin_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--format")
        .arg("csv")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "resource_type,resource_name,action\naws_instance,web,create\naws_s3_bucket,logs,delete\n"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn renders_json_from_mocked_saved_plan_file() {
    let root = temp_dir("saved_plan");
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    write_mock_terraform(&bin_dir);
    let plan_file = root.join("plan.tfplan");
    fs::write(&plan_file, "mock plan").expect("write plan file");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&plan_file)
        .arg("--format")
        .arg("json")
        .env("PATH", prepend_path(&bin_dir))
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""resource_type": "aws_instance""#));
    assert!(stdout.contains(r#""action": "replace""#));

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn renders_csv_from_plan_file_without_running_terraform() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plan.ndjson");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&fixture)
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
        "resource_type,resource_name,action\naws_instance,fixture_web,create\naws_s3_bucket,fixture_logs,update\n"
    );
}

#[test]
fn renders_csv_from_piped_stdin_before_plan_file_or_terraform() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plan.ndjson");
    let mut child = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .arg("--plan-file")
        .arg(&fixture)
        .arg("--format")
        .arg("csv")
        .env("PATH", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn terraform_plan_parser");

    child
        .stdin
        .as_mut()
        .expect("capture child stdin")
        .write_all(
            br#"{"@level":"info","change":{"resource":{"resource_type":"google_compute_instance","resource_name":"piped"},"action":"delete"}}
"#,
        )
        .expect("write stdin fixture");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "resource_type,resource_name,action\ngoogle_compute_instance,piped,delete\n"
    );
}

#[test]
fn dry_run_reports_live_plan_command_without_running_terraform() {
    let root = temp_dir("dry_run_live_plan");
    let project_dir = root.join("project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--dry-run")
        .env("PATH", "")
        .output()
        .expect("run terraform_plan_parser");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry run: would execute `terraform plan -json -input=false -no-color`"));
    assert!(stdout.contains(&project_dir.display().to_string()));
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn verbose_flag_enables_debug_logging() {
    let root = temp_dir("verbose_logging");
    let project_dir = root.join("project");
    fs::create_dir_all(&project_dir).expect("create project dir");

    let quiet_output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--dry-run")
        .output()
        .expect("run terraform_plan_parser without verbose");

    assert!(
        quiet_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&quiet_output.stderr)
    );
    assert!(String::from_utf8_lossy(&quiet_output.stderr).is_empty());

    let verbose_output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(&project_dir)
        .arg("--dry-run")
        .arg("--verbose")
        .output()
        .expect("run terraform_plan_parser with verbose");

    assert!(
        verbose_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&verbose_output.stderr)
    );
    assert!(String::from_utf8_lossy(&verbose_output.stderr).contains("Verbose logging enabled"));
    assert_eq!(
        String::from_utf8_lossy(&quiet_output.stdout),
        String::from_utf8_lossy(&verbose_output.stdout)
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[test]
fn reads_defaults_and_filters_from_config_file() {
    let root = temp_dir("config_defaults");
    let plan_file = root.join("plan.ndjson");
    fs::write(
        &plan_file,
        r#"{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}
"#,
    )
    .expect("write plan fixture");
    fs::write(
        root.join(".terraform-plan-parser.toml"),
        r#"plan-file = "plan.ndjson"
format = "csv"
include-type = ["aws_*"]
exclude-action = ["delete"]
"#,
    )
    .expect("write config file");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .current_dir(&root)
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
        "resource_type,resource_name,action\naws_instance,web,create\n"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}
#[test]
fn filters_only_create_actions() {
    let root = temp_dir("filter_create_actions");
    let plan_file = root.join("plan.ndjson");

    fs::write(
        &plan_file,
        r#"{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}
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
        .arg("--include-action")
        .arg("create")
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
        "resource_type,resource_name,action\naws_instance,web,create\n"
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

const MIXED_ACTIONS_PLAN: &str = r#"{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"update"}}
{"@level":"info","change":{"resource":{"resource_type":"aws_rds_cluster","resource_name":"db"},"action":"delete"}}
"#;

#[test]
fn filters_only_delete_actions() {
    let root = temp_dir("filter_delete_actions");
fn filters_only_update_actions() {
    let root = temp_dir("filter_update_actions");
fn excludes_actions_even_when_included() {
    let root = temp_dir("filter_exclude_actions");
    let plan_file = root.join("plan.ndjson");

    fs::write(&plan_file, MIXED_ACTIONS_PLAN).expect("write plan fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_terraform_plan_parser"))
        .arg(".")
        .current_dir(&root)
        .arg("--plan-file")
        .arg("plan.ndjson")
        .arg("--format")
        .arg("csv")
        .arg("--include-action")
        .arg("update")
        .arg("create,update,delete")
        .arg("--exclude-action")
        .arg("delete")
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
        "resource_type,resource_name,action\naws_rds_cluster,db,delete\n"
    );

        "resource_type,resource_name,action\naws_s3_bucket,logs,update\n"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("create"));
    assert!(stdout.contains("update"));
    assert!(!stdout.contains("delete"));

    fs::remove_dir_all(root).expect("remove temp dir");
}
