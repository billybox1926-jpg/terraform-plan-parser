use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, IsTerminal, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

const CONFIG_FILE_NAME: &str = ".terraform-plan-parser.toml";

#[derive(Clone, Copy)]
struct LevelWriter;

enum OutputWriter {
    Stdout(std::io::Stdout),
    Stderr(std::io::Stderr),
}

impl<'writer> MakeWriter<'writer> for LevelWriter {
    type Writer = OutputWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        OutputWriter::Stderr(std::io::stderr())
    }

    fn make_writer_for(&'writer self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        match *meta.level() {
            Level::INFO => OutputWriter::Stdout(std::io::stdout()),
            _ => OutputWriter::Stderr(std::io::stderr()),
        }
    }
}

impl std::io::Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Stdout(writer) => writer.write(buf),
            Self::Stderr(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Stdout(writer) => writer.flush(),
            Self::Stderr(writer) => writer.flush(),
        }
    }
}

#[derive(Parser)]
#[command(
    name = "terraform_plan_parser",
    after_help = r#"EXAMPLES:
  # Parse a saved JSON plan file
  terraform_plan_parser . --plan-file plan.ndjson --format csv

  # Read plan JSON from stdin
  cat plan.ndjson | terraform_plan_parser . --format table

  # Filter to create actions only
  terraform_plan_parser . --plan-file plan.ndjson --include-action create

  # Install shell completions (bash example)
  terraform_plan_parser --completions bash > /etc/bash_completion.d/terraform_plan_parser
"#
)]
struct Cli {
    /// Terraform project directory or saved .tfplan file to inspect.
    #[arg(default_value = ".")]
    directory: String,
    /// Read a pre-generated Terraform plan file instead of running terraform plan.
    ///
    /// Parses NDJSON from `terraform plan -json > plan.json` and full JSON from
    /// `terraform show -json` directly. Saved `.tfplan` files are converted with
    /// `terraform show -json`. Takes precedence over DIRECTORY and config defaults.
    #[arg(long, value_name = "PATH")]
    plan_file: Option<PathBuf>,
    /// Read defaults from a specific TOML config file.
    ///
    /// When omitted, the CLI looks for `.terraform-plan-parser.toml` in the
    /// current directory and then next to the selected DIRECTORY/plan file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long, value_enum)]
    format: Option<Format>,
    #[arg(long)]
    no_emoji: bool,
    /// Print the Terraform command that would run, then exit without executing Terraform.
    #[arg(long)]
    dry_run: bool,
    /// Enable verbose diagnostic logging.
    #[arg(short, long)]
    verbose: bool,
    /// Suppress the action summary line at the end of text/table output.
    #[arg(short, long)]
    quiet: bool,
    /// Include only resource types matching these comma-separated glob patterns.
    ///
    /// Exact values still work, and wildcards such as `aws_*` or `*instance`
    /// match multiple resource types.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    include_type: Vec<String>,
    /// Exclude resource types matching these comma-separated glob patterns.
    ///
    /// Exact values still work, and wildcards such as `aws_*` or `*bucket`
    /// match multiple resource types.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    exclude_type: Vec<String>,
    /// Include only actions matching these comma-separated glob patterns.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    include_action: Vec<String>,
    /// Shorthand for `--include-action delete` (safety reviews).
    #[arg(short = 'd', long)]
    only_delete: bool,
    /// Exclude actions matching these comma-separated glob patterns.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    exclude_action: Vec<String>,
    /// Exit with a non-zero status when the plan contains any of these actions.
    ///
    /// Evaluated after filters are applied. Useful in CI to block destructive plans:
    /// terraform_plan_parser . --fail-on delete
    #[arg(long, value_delimiter = ',', value_name = "ACTION[,ACTION]...")]
    fail_on: Vec<String>,
    /// Append a Markdown plan summary to `$GITHUB_STEP_SUMMARY` when that variable is set.
    ///
    /// In GitHub Actions the summary is written automatically when the environment
    /// variable is present; pass this flag to require an explicit opt-in.
    #[arg(long)]
    github_summary: bool,
    /// Generate shell completion scripts for the given shell, then exit.
    #[arg(long, value_enum, value_name = "SHELL")]
    completions: Option<Shell>,
}

#[derive(clap::ValueEnum, Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Format {
    Text,
    Json,
    Csv,
    Table,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct ConfigFile {
    plan_file: Option<PathBuf>,
    format: Option<Format>,
    no_emoji: Option<bool>,
    dry_run: Option<bool>,
    verbose: Option<bool>,
    quiet: Option<bool>,
    include_type: Vec<String>,
    exclude_type: Vec<String>,
    include_action: Vec<String>,
    only_delete: Option<bool>,
    exclude_action: Vec<String>,
    fail_on: Vec<String>,
    github_summary: Option<bool>,
}

#[derive(Debug)]
struct AppSettings {
    plan_file: Option<PathBuf>,
    format: Format,
    no_emoji: bool,
    dry_run: bool,
    verbose: bool,
    quiet: bool,
    include_type: Vec<String>,
    exclude_type: Vec<String>,
    include_action: Vec<String>,
    exclude_action: Vec<String>,
    fail_on: Vec<String>,
    github_summary: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ResourceChange {
    resource_type: String,
    resource_name: String,
    action: String,
}

#[derive(Debug, Deserialize)]
struct PlanLine {
    change: Option<PlanChange>,
}

#[derive(Debug, Deserialize)]
struct PlanChange {
    resource: Option<PlanResource>,
    action: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlanResource {
    #[serde(default = "unknown_value")]
    resource_type: String,
    #[serde(default = "unknown_value")]
    resource_name: String,
}

#[derive(Debug, Deserialize)]
struct ShowPlan {
    resource_changes: Option<Vec<ShowResourceChange>>,
}

#[derive(Debug, Deserialize)]
struct ShowResourceChange {
    #[serde(default = "unknown_value", rename = "type")]
    resource_type: String,
    #[serde(default = "unknown_value")]
    name: String,
    change: ShowChange,
}

#[derive(Debug, Deserialize)]
struct ShowChange {
    #[serde(default)]
    actions: Vec<String>,
}

#[derive(Debug)]
enum TerraformInput {
    StdinJson(String),
    Directory(PathBuf),
    JsonPlanFile(PathBuf),
    BinaryPlanFile(PathBuf),
}

impl TerraformInput {
    fn requires_terraform(&self) -> bool {
        matches!(self, Self::Directory(_) | Self::BinaryPlanFile(_))
    }
}

fn unknown_value() -> String {
    "unknown".to_string()
}

fn parse_plan_line(line: &str) -> Option<ResourceChange> {
    let line = match serde_json::from_str::<PlanLine>(line) {
        Ok(line) => line,
        Err(error) => {
            tracing::warn!(%error, line, "Skipping invalid Terraform JSON line");
            return None;
        }
    };
    let change = line.change?;
    let resource = change.resource?;

    Some(ResourceChange {
        resource_type: resource.resource_type,
        resource_name: resource.resource_name,
        action: change.action.unwrap_or_else(|| "noop".to_string()),
    })
}

fn parse_plan_output(stdout: &str) -> Vec<ResourceChange> {
    if stdout.trim_start().starts_with('{') && stdout.contains("\"resource_changes\"") {
        if let Ok(show_changes) = parse_show_plan_output(stdout) {
            return show_changes;
        }
    }

    stdout.lines().filter_map(parse_plan_line).collect()
}

fn parse_show_plan_output(stdout: &str) -> Result<Vec<ResourceChange>, serde_json::Error> {
    let plan = serde_json::from_str::<ShowPlan>(stdout)?;
    Ok(plan
        .resource_changes
        .unwrap_or_default()
        .into_iter()
        .filter_map(|change| {
            let action = action_from_show_actions(&change.change.actions)?;
            Some(ResourceChange {
                resource_type: change.resource_type,
                resource_name: change.name,
                action,
            })
        })
        .collect())
}

fn action_from_show_actions(actions: &[String]) -> Option<String> {
    match actions {
        [] => None,
        [action] if action == "no-op" => None,
        [action] => Some(action.clone()),
        [first, second] if first == "delete" && second == "create" => Some("replace".to_string()),
        _ => Some(actions.join("/")),
    }
}

fn filter_changes(
    resource_changes: Vec<ResourceChange>,
    settings: &AppSettings,
) -> Vec<ResourceChange> {
    resource_changes
        .into_iter()
        .filter(|change| {
            matches_filter(
                &change.resource_type,
                &settings.include_type,
                &settings.exclude_type,
            ) && matches_filter(
                &change.action,
                &settings.include_action,
                &settings.exclude_action,
            )
        })
        .collect()
}

fn matches_filter(value: &str, include: &[String], exclude: &[String]) -> bool {
    (include.is_empty()
        || include
            .iter()
            .any(|pattern| matches_pattern(value, pattern)))
        && !exclude
            .iter()
            .any(|pattern| matches_pattern(value, pattern))
}

fn matches_pattern(value: &str, pattern: &str) -> bool {
    Pattern::new(pattern).map_or_else(|_| pattern == value, |glob| glob.matches(value))
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ChangeCounts {
    create: usize,
    update: usize,
    delete: usize,
}

fn count_actions(resource_changes: &[ResourceChange]) -> ChangeCounts {
    let mut counts = ChangeCounts::default();
    for change in resource_changes {
        match change.action.as_str() {
            "create" => counts.create += 1,
            "update" => counts.update += 1,
            "delete" => counts.delete += 1,
            "replace" => {
                counts.create += 1;
                counts.delete += 1;
            }
            _ => {}
        }
    }
    counts
}

fn summary_action_symbols(no_emoji: bool) -> (&'static str, &'static str, &'static str) {
    if no_emoji {
        ("+", "~", "-")
    } else {
        ("➕", "🔄", "➖")
    }
}

fn render_summary_line(counts: &ChangeCounts, no_emoji: bool) -> String {
    let (create_sym, update_sym, delete_sym) = summary_action_symbols(no_emoji);
    format!(
        "Summary:\n  {create_sym} {} to create\n  {update_sym} {} to update\n  {delete_sym} {} to delete\n",
        counts.create, counts.update, counts.delete
    )
}

fn render_github_step_summary(
    display_path: &Path,
    resource_changes: &[ResourceChange],
    counts: &ChangeCounts,
    no_emoji: bool,
) -> String {
    use std::fmt::Write;

    let (create_sym, update_sym, delete_sym) = summary_action_symbols(no_emoji);
    let mut output = String::new();
    writeln!(output, "## Terraform plan summary").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "**Plan:** `{}`", display_path.display()).unwrap();
    writeln!(output).unwrap();
    writeln!(output, "| | Count |").unwrap();
    writeln!(output, "| --- | ---: |").unwrap();
    writeln!(output, "| {create_sym} Create | {} |", counts.create).unwrap();
    writeln!(output, "| {update_sym} Update | {} |", counts.update).unwrap();
    writeln!(output, "| {delete_sym} Delete | {} |", counts.delete).unwrap();

    if !resource_changes.is_empty() {
        writeln!(output).unwrap();
        writeln!(output, "### Resource changes").unwrap();
        writeln!(output).unwrap();
        writeln!(output, "| Action | Type | Name |").unwrap();
        writeln!(output, "| --- | --- | --- |").unwrap();
        for change in resource_changes {
            writeln!(
                output,
                "| {} | {} | {} |",
                change.action, change.resource_type, change.resource_name
            )
            .unwrap();
        }
    }

    output
}

fn append_github_step_summary(
    summary_path: &str,
    display_path: &Path,
    resource_changes: &[ResourceChange],
    counts: &ChangeCounts,
    no_emoji: bool,
) -> std::io::Result<()> {
    use std::io::Write;

    let markdown = render_github_step_summary(display_path, resource_changes, counts, no_emoji);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(summary_path)?;
    if file.metadata()?.len() > 0 {
        writeln!(file)?;
    }
    write!(file, "{markdown}")?;
    if !markdown.ends_with('\n') {
        writeln!(file)?;
    }
    Ok(())
}

fn should_write_github_summary(settings: &AppSettings) -> bool {
    std::env::var_os("GITHUB_STEP_SUMMARY").is_some() || settings.github_summary
}

fn write_github_summary_if_enabled(
    settings: &AppSettings,
    display_path: &Path,
    resource_changes: &[ResourceChange],
) {
    if !should_write_github_summary(settings) {
        return;
    }

    let Some(summary_path) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
        if settings.github_summary {
            tracing::warn!(
                "--github-summary was set but GITHUB_STEP_SUMMARY is not set; skipping summary"
            );
        }
        return;
    };

    let summary_path = summary_path.to_string_lossy();
    let counts = count_actions(resource_changes);
    if let Err(error) = append_github_step_summary(
        &summary_path,
        display_path,
        resource_changes,
        &counts,
        settings.no_emoji,
    ) {
        tracing::warn!("Failed to write GitHub Actions summary: {error}");
    }
}

fn render_changes(
    resource_changes: &[ResourceChange],
    abs_path: &Path,
    format: &Format,
    no_emoji: bool,
    quiet: bool,
) -> String {
    let counts = count_actions(resource_changes);
    match format {
        Format::Text => render_text(resource_changes, abs_path, no_emoji, quiet, &counts),
        Format::Json => render_json(resource_changes),
        Format::Csv => render_csv(resource_changes),
        Format::Table => render_table(resource_changes, abs_path, no_emoji, quiet, &counts),
    }
}

fn render_text(
    resource_changes: &[ResourceChange],
    abs_path: &Path,
    no_emoji: bool,
    quiet: bool,
    counts: &ChangeCounts,
) -> String {
    let mut output = String::new();
    if resource_changes.is_empty() {
        let prefix = if no_emoji { "" } else { "✅ " };
        output.push_str(&format!(
            "{}No resource changes detected in '{}'.\n",
            prefix,
            abs_path.display()
        ));
        if !quiet {
            output.push_str(&render_summary_line(counts, no_emoji));
        }
        return output;
    }

    let prefix = if no_emoji { "" } else { "📊 " };
    output.push_str(&format!(
        "{}Planned changes in '{}':\n",
        prefix,
        abs_path.display()
    ));
    for change in resource_changes {
        let symbol = if no_emoji {
            match change.action.as_str() {
                "create" => "+ ",
                "update" => "~ ",
                "delete" => "- ",
                "read" => "? ",
                _ => "* ",
            }
        } else {
            match change.action.as_str() {
                "create" => "➕ ",
                "update" => "🔄 ",
                "delete" => "➖ ",
                "read" => "📖 ",
                _ => "• ",
            }
        };
        output.push_str(&format!(
            "{}{} {} ({})\n",
            symbol, change.resource_type, change.resource_name, change.action
        ));
    }
    if !quiet {
        output.push_str(&render_summary_line(counts, no_emoji));
    }
    output
}

fn render_json(resource_changes: &[ResourceChange]) -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(resource_changes).expect("resource changes serialize to JSON")
    )
}

fn render_csv(resource_changes: &[ResourceChange]) -> String {
    let mut output = String::from("resource_type,resource_name,action\n");
    for change in resource_changes {
        output.push_str(&format!(
            "{},{},{}\n",
            csv_escape(&change.resource_type),
            csv_escape(&change.resource_name),
            csv_escape(&change.action)
        ));
    }
    output
}

fn render_table(
    resource_changes: &[ResourceChange],
    abs_path: &Path,
    no_emoji: bool,
    quiet: bool,
    counts: &ChangeCounts,
) -> String {
    if resource_changes.is_empty() {
        let mut output = format!(
            "No resource changes detected in '{}'.\n",
            abs_path.display()
        );
        if !quiet {
            output.push_str(&render_summary_line(counts, no_emoji));
        }
        return output;
    }

    let type_width = resource_changes
        .iter()
        .map(|change| change.resource_type.len())
        .chain(["Resource Type".len()])
        .max()
        .unwrap_or("Resource Type".len());
    let name_width = resource_changes
        .iter()
        .map(|change| change.resource_name.len())
        .chain(["Resource Name".len()])
        .max()
        .unwrap_or("Resource Name".len());
    let action_width = resource_changes
        .iter()
        .map(|change| change.action.len())
        .chain(["Action".len()])
        .max()
        .unwrap_or("Action".len());

    let mut output = format!("Planned changes in '{}':\n", abs_path.display());
    output.push_str(&format!(
        "{:<type_width$}  {:<name_width$}  {:<action_width$}\n",
        "Resource Type", "Resource Name", "Action"
    ));
    output.push_str(&format!(
        "{:-<type_width$}  {:-<name_width$}  {:-<action_width$}\n",
        "", "", ""
    ));

    for change in resource_changes {
        output.push_str(&format!(
            "{:<type_width$}  {:<name_width$}  {:<action_width$}\n",
            change.resource_type, change.resource_name, change.action
        ));
    }

    if !quiet {
        output.push_str(&render_summary_line(counts, no_emoji));
    }

    output
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn load_config(cli: &Cli) -> Result<(ConfigFile, Option<PathBuf>), String> {
    let Some(path) = resolve_config_path(cli)? else {
        return Ok((ConfigFile::default(), None));
    };

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read config file '{}': {error}", path.display()))?;
    let config = toml::from_str::<ConfigFile>(&contents)
        .map_err(|error| format!("Failed to parse config file '{}': {error}", path.display()))?;

    Ok((config, Some(path)))
}

fn resolve_config_path(cli: &Cli) -> Result<Option<PathBuf>, String> {
    if let Some(path) = &cli.config {
        if !path.exists() {
            return Err(format!("Config file does not exist: {}", path.display()));
        }
        let abs_path = absolutize(path);
        if !abs_path.is_file() {
            return Err(format!("Config path is not a file: {}", path.display()));
        }
        return Ok(Some(abs_path));
    }

    for candidate in default_config_candidates(cli) {
        if candidate.is_file() {
            return Ok(Some(absolutize(&candidate)));
        }
    }

    Ok(None)
}

fn default_config_candidates(cli: &Cli) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(CONFIG_FILE_NAME));
    }

    let input_path = cli
        .plan_file
        .as_deref()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&cli.directory));
    let input_config_dir = if input_path.is_dir() {
        Some(input_path)
    } else if input_path.is_file() {
        input_path.parent().map(Path::to_path_buf)
    } else {
        None
    };

    if let Some(config_dir) = input_config_dir {
        candidates.push(config_dir.join(CONFIG_FILE_NAME));
    }

    dedup_paths(candidates)
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|existing: &PathBuf| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

fn resolve_include_action(cli: &Cli, config: &ConfigFile) -> Vec<String> {
    if cli.only_delete || config.only_delete.unwrap_or(false) {
        return vec!["delete".to_string()];
    }
    cli_or_config_values(&cli.include_action, config.include_action.clone())
}

fn app_settings(cli: &Cli, config: ConfigFile, config_path: Option<&Path>) -> AppSettings {
    let include_action = resolve_include_action(cli, &config);
    let plan_file = cli.plan_file.clone().or_else(|| {
        config
            .plan_file
            .map(|path| resolve_config_relative_path(path, config_path))
    });

    AppSettings {
        plan_file,
        format: cli.format.clone().or(config.format).unwrap_or(Format::Text),
        no_emoji: cli.no_emoji || config.no_emoji.unwrap_or(false),
        dry_run: cli.dry_run || config.dry_run.unwrap_or(false),
        verbose: cli.verbose || config.verbose.unwrap_or(false),
        quiet: cli.quiet || config.quiet.unwrap_or(false),
        include_type: cli_or_config_values(&cli.include_type, config.include_type),
        exclude_type: cli_or_config_values(&cli.exclude_type, config.exclude_type),
        include_action,
        exclude_action: cli_or_config_values(&cli.exclude_action, config.exclude_action),
        fail_on: cli_or_config_values(&cli.fail_on, config.fail_on),
        github_summary: cli.github_summary || config.github_summary.unwrap_or(false),
    }
}

fn has_fail_on_actions(resource_changes: &[ResourceChange], fail_on: &[String]) -> bool {
    fail_on.iter().any(|pattern| {
        resource_changes
            .iter()
            .any(|change| matches_pattern(&change.action, pattern))
    })
}

fn resolve_config_relative_path(path: PathBuf, config_path: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    config_path
        .and_then(Path::parent)
        .map(|parent| parent.join(&path))
        .unwrap_or(path)
}

fn cli_or_config_values(cli_values: &[String], config_values: Vec<String>) -> Vec<String> {
    if cli_values.is_empty() {
        config_values
    } else {
        cli_values.to_vec()
    }
}

fn resolve_input(settings: &AppSettings, directory: &str) -> Result<TerraformInput, String> {
    if let Some(stdin_contents) = read_piped_stdin()? {
        return Ok(TerraformInput::StdinJson(stdin_contents));
    }

    if let Some(plan_file) = &settings.plan_file {
        return resolve_plan_file_input(plan_file);
    }

    resolve_positional_input(directory)
}

fn read_piped_stdin() -> Result<Option<String>, String> {
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

fn resolve_plan_file_input(path: &Path) -> Result<TerraformInput, String> {
    if !path.exists() {
        return Err(format!(
            "Error: plan file not found at \"{}\"\n\
             Hint: check the path and ensure the file exists, or run \
             `terraform plan -json > plan.json` in your project directory.",
            path.display()
        ));
    }

    let abs_path = absolutize(path);
    if !abs_path.is_file() {
        return Err(format!(
            "Error: --plan-file path is not a file: \"{}\"\n\
             Hint: pass a JSON/NDJSON plan file or a saved .tfplan file.",
            path.display()
        ));
    }

    if is_tfplan_file(&abs_path) {
        Ok(TerraformInput::BinaryPlanFile(abs_path))
    } else {
        Ok(TerraformInput::JsonPlanFile(abs_path))
    }
}

fn resolve_positional_input(path: &str) -> Result<TerraformInput, String> {
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

fn absolutize(path: &Path) -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(path)
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
}

fn is_tfplan_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "tfplan")
}

fn verify_terraform_available() -> Result<(), String> {
    tracing::debug!("Verifying terraform is available in PATH");
    Command::new("terraform")
        .arg("version")
        .output()
        .map(|_| ())
        .map_err(|_| "Error: 'terraform' not found in PATH. Is Terraform installed?".to_string())
}

fn render_dry_run(input: &TerraformInput) -> String {
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
        TerraformInput::BinaryPlanFile(plan_file) => {
            let current_dir = plan_file.parent().unwrap_or_else(|| Path::new("."));
            format!(
                "Dry run: would execute `terraform show -json {}` in '{}'.\n",
                plan_file.display(),
                current_dir.display()
            )
        }
    }
}

fn load_changes(input: &TerraformInput) -> Result<Vec<ResourceChange>, String> {
    match input {
        TerraformInput::StdinJson(contents) => Ok(parse_plan_output(contents)),
        TerraformInput::Directory(directory) => run_terraform_plan(directory),
        TerraformInput::JsonPlanFile(plan_file) => read_plan_json_file(plan_file),
        TerraformInput::BinaryPlanFile(plan_file) => run_terraform_show(plan_file),
    }
}

fn read_plan_json_file(plan_file: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(path = %plan_file.display(), "Reading Terraform plan JSON file");
    let contents = std::fs::read_to_string(plan_file).map_err(|error| {
        format!(
            "Failed to read Terraform plan file '{}': {error}",
            plan_file.display()
        )
    })?;

    Ok(parse_plan_output(&contents))
}

fn run_terraform_plan(directory: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(directory = %directory.display(), "Running terraform plan");
    let mut child = Command::new("terraform")
        .arg("plan")
        .arg("-json")
        .arg("-input=false")
        .arg("-no-color")
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
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

fn run_terraform_show(plan_file: &Path) -> Result<Vec<ResourceChange>, String> {
    tracing::debug!(path = %plan_file.display(), "Running terraform show for saved plan file");
    let current_dir = plan_file.parent().unwrap_or_else(|| Path::new("."));
    let output = Command::new("terraform")
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

fn init_tracing(verbose: bool) {
    let max_level = if verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(max_level)
        .with_writer(LevelWriter)
        .without_time()
        .with_level(false)
        .with_target(false)
        .init();
}

fn main() {
    let cli = Cli::parse();
    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        return;
    }
    let (config, config_path) = load_config(&cli).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    let settings = app_settings(&cli, config, config_path.as_deref());

    init_tracing(settings.verbose);
    tracing::debug!(config_path = ?config_path, "Configuration loaded");
    tracing::debug!("Verbose logging enabled");

    let input = resolve_input(&settings, &cli.directory).unwrap_or_else(|error| {
        tracing::error!("{error}");
        std::process::exit(1);
    });

    if settings.dry_run {
        tracing::info!("{}", render_dry_run(&input).trim_end());
        return;
    }

    if input.requires_terraform() {
        verify_terraform_available().unwrap_or_else(|error| {
            tracing::error!("{error}");
            std::process::exit(1);
        });
    }

    let resource_changes = load_changes(&input).unwrap_or_else(|error| {
        tracing::error!("{error}");
        std::process::exit(1);
    });
    let resource_changes = filter_changes(resource_changes, &settings);
    let stdin_display_path = Path::new("<stdin>");
    let display_path = match &input {
        TerraformInput::StdinJson(_) => stdin_display_path,
        TerraformInput::Directory(directory) => directory.as_path(),
        TerraformInput::JsonPlanFile(plan_file) | TerraformInput::BinaryPlanFile(plan_file) => {
            plan_file.as_path()
        }
    };

    tracing::info!(
        "{}",
        render_changes(
            &resource_changes,
            display_path,
            &settings.format,
            settings.no_emoji,
            settings.quiet
        )
        .trim_end()
    );

    write_github_summary_if_enabled(&settings, display_path, &resource_changes);

    if has_fail_on_actions(&resource_changes, &settings.fail_on) {
        tracing::error!("Plan contains forbidden actions matching --fail-on criteria");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_settings, append_github_step_summary, count_actions, csv_escape, filter_changes,
        has_fail_on_actions, parse_plan_output, render_csv, render_dry_run,
        render_github_step_summary, render_summary_line, render_table, render_text, ChangeCounts,
        Cli, ConfigFile, Format, ResourceChange, TerraformInput,
    };
    use clap::Parser;
    use std::path::Path;

    #[test]
    fn parses_resource_changes_from_ndjson() {
        let stdout = r#"{"@level":"info","change":{"resource":{"resource_type":"aws_instance","resource_name":"web"},"action":"create"}}
not-json
{"@level":"info","change":{"resource":{"resource_type":"aws_s3_bucket","resource_name":"logs"},"action":"delete"}}
"#;

        assert_eq!(
            parse_plan_output(stdout),
            vec![
                ResourceChange {
                    resource_type: "aws_instance".to_string(),
                    resource_name: "web".to_string(),
                    action: "create".to_string(),
                },
                ResourceChange {
                    resource_type: "aws_s3_bucket".to_string(),
                    resource_name: "logs".to_string(),
                    action: "delete".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parses_saved_plan_json_output() {
        let stdout = r#"{
  "resource_changes": [
    {
      "type": "aws_instance",
      "name": "web",
      "change": { "actions": ["delete", "create"] }
    },
    {
      "type": "aws_s3_bucket",
      "name": "logs",
      "change": { "actions": ["no-op"] }
    }
  ]
}"#;

        assert_eq!(
            parse_plan_output(stdout),
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "replace".to_string(),
            }]
        );
    }

    #[test]
    fn parses_single_ndjson_change_line_as_stream_output() {
        let stdout = r#"{"@level":"info","change":{"resource":{"resource_type":"google_compute_instance","resource_name":"piped"},"action":"delete"}}
"#;

        assert_eq!(
            parse_plan_output(stdout),
            vec![ResourceChange {
                resource_type: "google_compute_instance".to_string(),
                resource_name: "piped".to_string(),
                action: "delete".to_string(),
            }]
        );
    }

    #[test]
    fn ignores_lines_without_resource_changes() {
        let stdout = r#"{"@level":"info","message":"Refreshing state..."}
{"change":{"action":"create"}}
{"change":{"resource":{"resource_type":"aws_instance","resource_name":"web"}}}
"#;

        assert_eq!(
            parse_plan_output(stdout),
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "noop".to_string(),
            }]
        );
    }

    #[test]
    fn filters_changes_by_type_and_action() {
        let cli = Cli::parse_from([
            "terraform_plan_parser",
            "--include-type",
            "aws_instance,aws_s3_bucket",
            "--exclude-type",
            "aws_s3_bucket",
            "--include-action",
            "create",
        ]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "old".to_string(),
                action: "delete".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            }]
        );
    }

    #[test]
    fn filters_resource_types_with_glob_patterns() {
        let cli = Cli::parse_from([
            "terraform_plan_parser",
            "--include-type",
            "aws_*",
            "--exclude-type",
            "*-bucket,*_s3_bucket",
        ]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "google_compute_instance".to_string(),
                resource_name: "app".to_string(),
                action: "create".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            }]
        );
    }

    #[test]
    fn filters_actions_with_glob_patterns() {
        let cli = Cli::parse_from([
            "terraform_plan_parser",
            "--include-action",
            "cre*",
            "--exclude-action",
            "*-before",
        ]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "old".to_string(),
                action: "create-before".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "delete".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            }]
        );
    }

    #[test]
    fn filters_action_include_and_exclude_lists() {
        let cli = Cli::parse_from([
            "terraform_plan_parser",
            "--include-action",
            "create,update,delete",
            "--exclude-action",
            "delete",
        ]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
            ResourceChange {
                resource_type: "aws_security_group".to_string(),
                resource_name: "old".to_string(),
                action: "delete".to_string(),
            },
            ResourceChange {
                resource_type: "aws_iam_role".to_string(),
                resource_name: "reader".to_string(),
                action: "read".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![
                ResourceChange {
                    resource_type: "aws_instance".to_string(),
                    resource_name: "web".to_string(),
                    action: "create".to_string(),
                },
                ResourceChange {
                    resource_type: "aws_s3_bucket".to_string(),
                    resource_name: "logs".to_string(),
                    action: "update".to_string(),
                },
            ]
        );
    }

    #[test]
    fn filters_only_update_actions() {
        let cli = Cli::parse_from(["terraform_plan_parser", "--include-action", "update"]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
            ResourceChange {
                resource_type: "aws_rds_cluster".to_string(),
                resource_name: "db".to_string(),
                action: "delete".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            }]
        );
    }

    #[test]
    fn filters_only_delete_actions() {
        let cli = Cli::parse_from(["terraform_plan_parser", "--include-action", "delete"]);
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
            ResourceChange {
                resource_type: "aws_rds_cluster".to_string(),
                resource_name: "db".to_string(),
                action: "delete".to_string(),
            },
        ];

        assert_eq!(
            filter_changes(changes, &app_settings(&cli, ConfigFile::default(), None)),
            vec![ResourceChange {
                resource_type: "aws_rds_cluster".to_string(),
                resource_name: "db".to_string(),
                action: "delete".to_string(),
            }]
        );
    }

    #[test]
    fn renders_dry_run_for_stdin_without_terraform_command() {
        let output = render_dry_run(&TerraformInput::StdinJson("{}".to_string()));

        assert_eq!(
            output,
            "Dry run: would read JSON Terraform plan data from stdin. No Terraform command would be executed.\n"
        );
    }

    #[test]
    fn renders_csv_and_escapes_fields_that_need_quotes() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("name,with,commas"), "\"name,with,commas\"");
        assert_eq!(csv_escape("name \"quoted\""), "\"name \"\"quoted\"\"\"");
        assert_eq!(
            render_csv(&[ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            }]),
            "resource_type,resource_name,action\naws_instance,web,create\n"
        );
    }

    #[test]
    fn counts_actions_from_filtered_changes() {
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
            ResourceChange {
                resource_type: "aws_rds_cluster".to_string(),
                resource_name: "db".to_string(),
                action: "replace".to_string(),
            },
        ];

        assert_eq!(
            count_actions(&changes),
            ChangeCounts {
                create: 2,
                update: 1,
                delete: 1,
            }
        );
        assert_eq!(
            render_summary_line(&count_actions(&changes), true),
            "Summary:\n  + 2 to create\n  ~ 1 to update\n  - 1 to delete\n"
        );
    }

    #[test]
    fn renders_summary_counts_in_text_output() {
        let changes = vec![
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
        ];
        let counts = count_actions(&changes);
        let output = render_text(&changes, Path::new("/tmp/project"), true, false, &counts);

        assert!(output.contains("aws_instance"));
        assert!(output.contains("Summary:"));
        assert!(output.contains("+ 1 to create"));
        assert!(output.contains("~ 1 to update"));
        assert!(output.contains("- 0 to delete"));
    }

    #[test]
    fn hides_summary_counts_when_quiet() {
        let changes = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let counts = count_actions(&changes);
        let output = render_text(&changes, Path::new("/tmp/project"), true, true, &counts);

        assert!(!output.contains("Summary:"));
    }

    #[test]
    fn renders_table_output() {
        let changes = [ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let counts = count_actions(&changes);
        let output = render_table(&changes, Path::new("/tmp/project"), true, false, &counts);

        assert!(output.contains("Resource Type"));
        assert!(output.contains("aws_instance"));
        assert!(output.contains("create"));
        assert!(output.contains("Summary:"));
        assert!(output.contains("+ 1 to create"));
    }

    #[test]
    fn accepts_dry_run_from_cli() {
        let cli = Cli::parse_from(["terraform_plan_parser", "--dry-run"]);
        assert!(cli.dry_run);
    }

    #[test]
    fn only_delete_shorthand_sets_include_action() {
        let cli = Cli::parse_from(["terraform_plan_parser", "-d"]);
        let settings = app_settings(&cli, ConfigFile::default(), None);
        assert_eq!(settings.include_action, vec!["delete".to_string()]);
    }

    #[test]
    fn renders_dry_run_for_directory_without_loading_changes() {
        let output = render_dry_run(&TerraformInput::Directory(
            Path::new("/tmp/project").to_path_buf(),
        ));

        assert_eq!(
            output,
            "Dry run: would execute `terraform plan -json -input=false -no-color` in '/tmp/project'.\n"
        );
    }

    #[test]
    fn renders_dry_run_for_binary_plan_file() {
        let output = render_dry_run(&TerraformInput::BinaryPlanFile(
            Path::new("/tmp/project/tfplan").to_path_buf(),
        ));

        assert_eq!(
            output,
            "Dry run: would execute `terraform show -json /tmp/project/tfplan` in '/tmp/project'.\n"
        );
    }

    #[test]
    fn renders_dry_run_for_json_plan_file_without_terraform_command() {
        let output = render_dry_run(&TerraformInput::JsonPlanFile(
            Path::new("/tmp/project/plan.json").to_path_buf(),
        ));

        assert_eq!(
            output,
            "Dry run: would read JSON Terraform plan file '/tmp/project/plan.json'. No Terraform command would be executed.\n"
        );
    }

    #[test]
    fn accepts_table_format_from_cli() {
        let cli = Cli::parse_from(["terraform_plan_parser", "--format", "table"]);
        assert!(matches!(cli.format, Some(Format::Table)));
    }

    #[test]
    fn resolve_plan_file_input_reports_missing_file() {
        let error = crate::resolve_plan_file_input(Path::new("./missing-plan.json"))
            .expect_err("missing plan file should fail");

        assert!(error.contains("plan file not found"));
        assert!(error.contains("./missing-plan.json"));
    }

    #[test]
    fn fail_on_matches_delete_actions() {
        let changes = vec![
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "delete".to_string(),
            },
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
        ];
        assert!(has_fail_on_actions(&changes, &["delete".to_string()]));
        assert!(!has_fail_on_actions(&changes, &["update".to_string()]));
        assert!(has_fail_on_actions(
            &changes,
            &["delete".to_string(), "create".to_string()]
        ));
    }

    #[test]
    fn renders_github_step_summary_markdown() {
        let changes = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let counts = count_actions(&changes);
        let summary = render_github_step_summary(Path::new("plan.ndjson"), &changes, &counts, true);

        assert!(summary.contains("## Terraform plan summary"));
        assert!(summary.contains("**Plan:** `plan.ndjson`"));
        assert!(summary.contains("| + Create | 1 |"));
        assert!(summary.contains("| create | aws_instance | web |"));
    }

    #[test]
    fn append_github_step_summary_writes_to_file() {
        let dir = std::env::temp_dir().join(format!(
            "terraform_plan_parser_summary_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let summary_path = dir.join("summary.md");
        let summary_path = summary_path.to_string_lossy();

        let changes = vec![ResourceChange {
            resource_type: "aws_s3_bucket".to_string(),
            resource_name: "logs".to_string(),
            action: "delete".to_string(),
        }];
        let counts = count_actions(&changes);

        append_github_step_summary(
            &summary_path,
            Path::new("plan.ndjson"),
            &changes,
            &counts,
            true,
        )
        .expect("append summary");

        let written = std::fs::read_to_string(dir.join("summary.md")).expect("read summary");
        assert!(written.contains("## Terraform plan summary"));
        assert!(written.contains("| - Delete | 1 |"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
