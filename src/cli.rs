use clap::Parser;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::parser::Format;
use crate::terraform::absolutize;

pub const CONFIG_FILE_NAME: &str = ".terraform-plan-parser.toml";

#[derive(Parser)]
#[command(
    name = "terraform_plan_parser",
    version = env!("CARGO_PKG_VERSION"),
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
pub struct Cli {
    /// Terraform project directory or saved .tfplan file to inspect.
    #[arg(default_value = ".")]
    pub directory: String,
    /// Read a pre-generated Terraform plan file instead of running terraform plan.
    ///
    /// Parses NDJSON from `terraform plan -json > plan.json` and full JSON from
    /// `terraform show -json` directly. Saved `.tfplan` files are converted with
    /// `terraform show -json`. Takes precedence over DIRECTORY and config defaults.
    #[arg(long, value_name = "PATH")]
    pub plan_file: Option<PathBuf>,
    /// Read defaults from a specific TOML config file.
    ///
    /// When omitted, the CLI looks for `.terraform-plan-parser.toml` in the
    /// current directory and then next to the selected DIRECTORY/plan file.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Write rendered output to a file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub output_file: Option<PathBuf>,
    #[arg(long, value_enum)]
    pub format: Option<Format>,
    #[arg(long)]
    pub no_emoji: bool,
    /// Print the Terraform command that would run, then exit without executing Terraform.
    #[arg(long)]
    pub dry_run: bool,
    /// Enable verbose diagnostic logging.
    #[arg(short, long)]
    pub verbose: bool,
    /// Suppress the action summary line at the end of text/table output.
    #[arg(short, long)]
    pub quiet: bool,
    /// Omit the header row from CSV output.
    #[arg(long)]
    pub no_header: bool,
    /// Include only resource types matching these comma-separated glob patterns.
    ///
    /// Exact values still work, and wildcards such as `aws_*` or `*instance`
    /// match multiple resource types.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    pub include_type: Vec<String>,
    /// Exclude resource types matching these comma-separated glob patterns.
    ///
    /// Exact values still work, and wildcards such as `aws_*` or `*bucket`
    /// match multiple resource types.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    pub exclude_type: Vec<String>,
    /// Include only actions matching these comma-separated glob patterns.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    pub include_action: Vec<String>,
    /// Shorthand for `--include-action delete` (safety reviews).
    #[arg(short = 'd', long)]
    pub only_delete: bool,

    /// Shorthand to include only create actions.
    #[arg(short = 'c', long)]
    pub only_create: bool,

    /// Shorthand to include only update actions.
    #[arg(short = 'u', long)]
    pub only_update: bool,
    /// Shorthand to include only replace actions.
    #[arg(short = 'r', long)]
    pub only_replace: bool,
    /// Exclude actions matching these comma-separated glob patterns.
    #[arg(long, value_delimiter = ',', value_name = "GLOB[,GLOB]...")]
    pub exclude_action: Vec<String>,
    /// Exit with a non-zero status when the plan contains any of these actions.
    ///
    /// Evaluated after filters are applied. Useful in CI to block destructive plans:
    /// terraform_plan_parser . --fail-on delete
    #[arg(long, value_delimiter = ',', value_name = "ACTION[,ACTION]...")]
    pub fail_on: Vec<String>,
    /// Append a Markdown plan summary to `$GITHUB_STEP_SUMMARY` when that variable is set.
    ///
    /// In GitHub Actions the summary is written automatically when the environment
    /// variable is present; pass this flag to require an explicit opt-in.
    #[arg(long)]
    pub github_summary: bool,
    /// Sort resource changes before rendering (default: plan file order).
    #[arg(long, value_enum)]
    pub sort_by: Option<SortBy>,
    /// Generate shell completion scripts for the given shell, then exit.
    #[arg(long, value_enum, value_name = "SHELL")]
    pub completions: Option<clap_complete::Shell>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    Type,
    Name,
    Action,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConfigFile {
    pub plan_file: Option<PathBuf>,
    pub format: Option<Format>,
    pub no_emoji: Option<bool>,
    pub dry_run: Option<bool>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub no_header: Option<bool>,
    pub include_type: Vec<String>,
    pub exclude_type: Vec<String>,
    pub include_action: Vec<String>,
    pub only_delete: Option<bool>,
    pub only_create: Option<bool>,
    pub only_update: Option<bool>,
    pub only_replace: Option<bool>,
    pub exclude_action: Vec<String>,
    pub fail_on: Vec<String>,
    pub github_summary: Option<bool>,
    pub sort_by: Option<SortBy>,
    pub output_file: Option<PathBuf>,
}

#[derive(Debug)]
pub struct AppSettings {
    pub plan_file: Option<PathBuf>,
    pub format: Format,
    pub no_emoji: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub no_header: bool,
    pub include_type: Vec<String>,
    pub exclude_type: Vec<String>,
    pub include_action: Vec<String>,
    pub exclude_action: Vec<String>,
    pub fail_on: Vec<String>,
    pub github_summary: bool,
    pub sort_by: Option<SortBy>,
    pub output_file: Option<PathBuf>,
}

pub fn load_config(cli: &Cli) -> Result<(ConfigFile, Option<PathBuf>), String> {
    let Some(path) = resolve_config_path(cli)? else {
        return Ok((ConfigFile::default(), None));
    };

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read config file '{}': {error}", path.display()))?;
    let config = toml::from_str::<ConfigFile>(&contents)
        .map_err(|error| format!("Failed to parse config file '{}': {error}", path.display()))?;

    Ok((config, Some(path)))
}

pub fn resolve_config_path(cli: &Cli) -> Result<Option<PathBuf>, String> {
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

pub fn default_config_candidates(cli: &Cli) -> Vec<PathBuf> {
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

pub fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|existing: &PathBuf| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

pub fn resolve_include_action(cli: &Cli, config: &ConfigFile) -> Vec<String> {
    if cli.only_delete || config.only_delete.unwrap_or(false) {
        return vec!["delete".to_string()];
    }
    if cli.only_create || config.only_create.unwrap_or(false) {
        return vec!["create".to_string()];
    }
    if cli.only_update || config.only_update.unwrap_or(false) {
        return vec!["update".to_string()];
    }
    if cli.only_replace || config.only_replace.unwrap_or(false) {
        return vec!["replace".to_string()];
    }
    cli_or_config_values(&cli.include_action, config.include_action.clone())
}

pub fn app_settings(cli: &Cli, config: ConfigFile, config_path: Option<&Path>) -> AppSettings {
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
        no_header: cli.no_header || config.no_header.unwrap_or(false),
        include_type: cli_or_config_values(&cli.include_type, config.include_type),
        exclude_type: cli_or_config_values(&cli.exclude_type, config.exclude_type),
        include_action,
        exclude_action: cli_or_config_values(&cli.exclude_action, config.exclude_action),
        fail_on: cli_or_config_values(&cli.fail_on, config.fail_on),
        github_summary: cli.github_summary || config.github_summary.unwrap_or(false),
        sort_by: cli.sort_by.or(config.sort_by),
        output_file: cli.output_file.clone().or_else(|| {
            config
                .output_file
                .map(|path| resolve_config_relative_path(path, config_path))
        }),
    }
}

pub fn resolve_config_relative_path(path: PathBuf, config_path: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    config_path
        .and_then(Path::parent)
        .map(|parent| parent.join(&path))
        .unwrap_or(path)
}

pub fn cli_or_config_values(cli_values: &[String], config_values: Vec<String>) -> Vec<String> {
    if cli_values.is_empty() {
        config_values
    } else {
        cli_values.to_vec()
    }
}
