use std::{
    io::{BufRead, BufReader, IsTerminal, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use tracing::Level;

use crate::parser::{
    parse_plan_line, parse_plan_output, parse_show_plan_output, parse_state_output, ResourceChange,
    TerraformInput,
};
use crate::renderer::LevelWriter;

pub fn init_tracing(verbose: bool) {
    let max_level = if verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(max_level)
        .with_writer(LevelWriter)
        .without_time()
        .with_level(false)
        .with_target(false)
        .init();
}

pub fn terraform_command() -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        // Use /d /s /c for safe Windows execution:
        // /d — disables AutoRun registry commands
        // /s — strips outer quotes so arguments parse cleanly
        cmd.arg("/d").arg("/s").arg("/c").arg("terraform");
        cmd
    } else {
        Command::new("terraform")
    }
}

pub fn verify_terraform_available() -> Result<(), String> {
    tracing::debug!("Verifying terraform is available in PATH");
    let output = terraform_command()
        .arg("version")
        .output()
        .map_err(|error| {
            format!("Error: 'terraform' not found in PATH or failed to execute: {error}")
        })?;

    if !output.status.success() {
        return Err(format!(
            "Error: 'terraform version' failed with status {}.",
            output.status
        ));
    }
    Ok(())
}

pub fn render_dry_run(input: &TerraformInput) -> String {
    match input {
        TerraformInput::StdinJson(_) => {
            "Dry run: would read JSON Terraform plan data from stdin. No Terraform command would be executed.\n"
                .to_string()
        }
        TerraformInput::Directory(directory) => format!(
            "Dry run: would execute `terraform plan -json -input=false -no-color` in '{}'.\n",
            directory.display()
        ),
        TerraformInput::JsonPlanFile(plan_file) => format!(
            "Dry run: would read JSON Terraform plan file '{}'. No Terraform command would be executed.\n",
            plan_file.display()
        ),
        TerraformInput::StateFile(state_file) => format!(
            "Dry run: would read Terraform state JSON file '{}'. No Terraform command would be executed.\n",
            state_file.display()
        ),
        TerraformInput::BinaryPlanFile(plan_file) => {
            let current_dir = plan_file.parent().unwrap_or_else(|| Path::new("."));
            format!(
                "Dry run: would execute `terraform show -json {}` in '{}'.\n",
                plan_file.display(),
                current_dir.display()
            )
        }
        TerraformInput::Compare { old, new } => {
            format!(
                "Dry run: would compare plan files '{}' and '{}'.\n",
                old.display(),
                new.display()
            )
        }
    }
}

pub fn load_changes(input: &TerraformInput) -> Result<Vec<ResourceChange>, String> {
    match input {
        TerraformInput::StdinJson(contents) => Ok(parse_plan_output(contents)),
        TerraformInput::Directory(directory) => run_terraform_plan(directory),
        TerraformInput::JsonPlanFile(plan_file) => read_plan_json_file(plan_file),
        TerraformInput::BinaryPlanFile(plan_file) => run_terraform_show(plan_file),
        TerraformInput::StateFile(state_file) => read_state_json_file(state_file),
        TerraformInput::Compare { .. } => {
            Err("Compare mode should use load_and_compare, not load_changes".to_string())
        }
    }
}

/// Load a single plan file (JSON, NDJSON, or .tfplan) and return parsed changes.
pub fn load_plan_file(path: &Path) -> Result<Vec<ResourceChange>, String> {
    if is_tfplan_file(path) {
        run_terraform_show(path)
    } else {
        read_plan_json_file(path)
    }
}

/// Load two plan files and return their diff.
pub fn load_and_compare(
    old_path: &Path,
    new_path: &Path,
) -> Result<crate::parser::PlanDiff, String> {
    let old_changes = load_plan_file(old_path)?;
    let new_changes = load_plan_file(new_path)?;
    Ok(crate::parser::compare_plans(&old_changes, &new_changes))
}

pub fn read_plan_json_file(plan_file: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(path = %plan_file.display(), "Reading Terraform plan JSON file");
    let contents = std::fs::read_to_string(plan_file).map_err(|error| {
        format!(
            "Failed to read Terraform plan file '{}': {error}",
            plan_file.display()
        )
    })?;

    Ok(parse_plan_output(&contents))
}

pub fn read_state_json_file(state_file: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(path = %state_file.display(), "Reading Terraform state JSON file");
    let contents = std::fs::read_to_string(state_file).map_err(|error| {
        format!(
            "Failed to read Terraform state file '{}': {error}",
            state_file.display()
        )
    })?;

    parse_state_output(&contents).map_err(|error| {
        format!(
            "Failed to parse Terraform state JSON file '{}': {error}",
            state_file.display()
        )
    })
}

pub fn run_terraform_plan(directory: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(directory = %directory.display(), "Running terraform plan");
    let mut cmd = terraform_command();

    cmd.arg("plan")
        .arg("-json")
        .arg("-input=false")
        .arg("-no-color")
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    tracing::debug!("Executing: {:?}", cmd);

    let mut child = cmd.spawn().map_err(|error| {
        format!(
            "Failed to execute terraform in '{}': {error}",
            directory.display()
        )
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture terraform stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture terraform stderr".to_string())?;

    let stderr_handle = thread::spawn(move || {
        let mut stderr_output = String::new();
        let mut reader = BufReader::new(stderr);
        reader
            .read_to_string(&mut stderr_output)
            .map(|_| stderr_output)
    });

    let mut resource_changes = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| format!("Failed to read terraform stdout: {error}"))?;
        if let Some(change) = parse_plan_line(&line) {
            resource_changes.push(change);
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("Failed to wait for terraform plan: {error}"))?;
    let stderr = stderr_handle
        .join()
        .map_err(|_| "Failed to join terraform stderr reader".to_string())?
        .map_err(|error| format!("Failed to read terraform stderr: {error}"))?;

    if !status.success() {
        return Err(format!(
            "Terraform plan failed in '{}':\n{}",
            directory.display(),
            stderr
        ));
    }

    Ok(resource_changes)
}

pub fn run_terraform_show(plan_file: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(path = %plan_file.display(), "Running terraform show for saved plan file");
    let current_dir = plan_file.parent().unwrap_or_else(|| Path::new("."));

    let mut cmd = terraform_command();

    let output = cmd
        .arg("show")
        .arg("-json")
        .arg(plan_file)
        .current_dir(current_dir)
        .output()
        .map_err(|error| {
            format!(
                "Failed to execute terraform show for '{}': {error}",
                plan_file.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Terraform show failed for '{}':\n{}",
            plan_file.display(),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_show_plan_output(&stdout).map_err(|error| {
        format!(
            "Failed to parse terraform show JSON for '{}': {error}",
            plan_file.display()
        )
    })
}

pub fn resolve_input(
    settings: &crate::cli::AppSettings,
    directory: &str,
    compare: &Option<Vec<PathBuf>>,
) -> Result<TerraformInput, String> {
    // Compare mode takes precedence
    if let Some(paths) = compare {
        if paths.len() != 2 {
            return Err("Compare mode requires exactly two plan file paths".to_string());
        }
        let old = absolutize(&paths[0]);
        let new = absolutize(&paths[1]);
        if !old.is_file() {
            return Err(format!("Old plan file not found: {}", old.display()));
        }
        if !new.is_file() {
            return Err(format!("New plan file not found: {}", new.display()));
        }
        return Ok(TerraformInput::Compare { old, new });
    }

    if let Some(stdin_contents) = read_piped_stdin()? {
        return Ok(TerraformInput::StdinJson(stdin_contents));
    }

    if let Some(state_file) = &settings.state_file {
        return resolve_state_file_input(state_file);
    }

    if let Some(plan_file) = &settings.plan_file {
        return resolve_plan_file_input(plan_file);
    }

    resolve_positional_input(directory)
}

pub fn read_piped_stdin() -> Result<Option<String>, String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Ok(None);
    }

    let mut contents = String::new();
    stdin
        .lock()
        .read_to_string(&mut contents)
        .map_err(|error| format!("Failed to read Terraform plan JSON from stdin: {error}"))?;

    if contents.trim().is_empty() {
        Ok(None)
    } else {
        tracing::debug!("Reading Terraform plan JSON from stdin");
        Ok(Some(contents))
    }
}

pub fn resolve_state_file_input(path: &Path) -> Result<TerraformInput, String> {
    if !path.exists() {
        return Err(format!(
            "Error: state file not found at \"{}\"\
            \nHint: check the path, or save remote state first with `terraform state pull > terraform.tfstate`.",
            path.display()
        ));
    }

    let abs_path = absolutize(path);
    if !abs_path.is_file() {
        return Err(format!(
            "Error: --state path is not a file: \"{}\"\
            \nHint: pass a local Terraform state JSON file.",
            path.display()
        ));
    }

    Ok(TerraformInput::StateFile(abs_path))
}

pub fn resolve_plan_file_input(path: &Path) -> Result<TerraformInput, String> {
    if !path.exists() {
        return Err(format!(
            "Error: plan file not found at \"{}\"\
            \nHint: check the path and ensure the file exists, or run \
            `terraform plan -json > plan.json` in your project directory.",
            path.display()
        ));
    }

    let abs_path = absolutize(path);
    if !abs_path.is_file() {
        return Err(format!(
            "Error: --plan-file path is not a file: \"{}\"\
            \nHint: pass a JSON/NDJSON plan file or a saved .tfplan file.",
            path.display()
        ));
    }

    if is_tfplan_file(&abs_path) {
        Ok(TerraformInput::BinaryPlanFile(abs_path))
    } else {
        Ok(TerraformInput::JsonPlanFile(abs_path))
    }
}

pub fn resolve_positional_input(path: &str) -> Result<TerraformInput, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let abs_path = absolutize(path);

    if abs_path.is_dir() {
        return Ok(TerraformInput::Directory(abs_path));
    }

    if abs_path.is_file() && is_tfplan_file(&abs_path) {
        return Ok(TerraformInput::BinaryPlanFile(abs_path));
    }

    Err(format!(
        "Path is not a directory or .tfplan file: {}",
        path.display()
    ))
}

pub fn absolutize(path: &Path) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(path)
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
}

pub fn is_tfplan_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "tfplan")
}
