use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

#[derive(Parser)]
#[command(name = "terraform_plan_parser")]
struct Cli {
    #[arg(default_value = ".")]
    directory: String,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    #[arg(long)]
    no_emoji: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Format {
    Text,
    Json,
    Csv,
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

fn unknown_value() -> String {
    "unknown".to_string()
}

fn parse_plan_output(stdout: &str) -> Vec<ResourceChange> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<PlanLine>(line).ok())
        .filter_map(|line| {
            let change = line.change?;
            let resource = change.resource?;

            Some(ResourceChange {
                resource_type: resource.resource_type,
                resource_name: resource.resource_name,
                action: change.action.unwrap_or_else(|| "noop".to_string()),
            })
        })
        .collect()
}

fn render_changes(
    resource_changes: &[ResourceChange],
    abs_dir: &Path,
    format: &Format,
    no_emoji: bool,
) {
    match format {
        Format::Text => render_text(resource_changes, abs_dir, no_emoji),
        Format::Json => render_json(resource_changes),
        Format::Csv => render_csv(resource_changes),
    }
}

fn render_text(resource_changes: &[ResourceChange], abs_dir: &Path, no_emoji: bool) {
    if resource_changes.is_empty() {
        let prefix = if no_emoji { "" } else { "✅ " };
        println!(
            "{}No resource changes detected in '{}'.",
            prefix,
            abs_dir.display()
        );
        return;
    }

    let prefix = if no_emoji { "" } else { "📊 " };
    println!("{}Planned changes in '{}':", prefix, abs_dir.display());
    for change in resource_changes {
        let symbol = if no_emoji {
            ""
        } else {
            match change.action.as_str() {
                "create" => "➕ ",
                "update" => "🔄 ",
                "delete" => "➖ ",
                "read" => "📖 ",
                _ => "• ",
            }
        };
        println!(
            "{}{} {} ({})",
            symbol, change.resource_type, change.resource_name, change.action
        );
    }
}

fn render_json(resource_changes: &[ResourceChange]) {
    println!(
        "{}",
        serde_json::to_string_pretty(resource_changes).expect("resource changes serialize to JSON")
    );
}

fn render_csv(resource_changes: &[ResourceChange]) {
    println!("resource_type,resource_name,action");
    for change in resource_changes {
        println!(
            "{},{},{}",
            csv_escape(&change.resource_type),
            csv_escape(&change.resource_name),
            csv_escape(&change.action)
        );
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn main() {
    let cli = Cli::parse();

    let path = Path::new(&cli.directory);
    if !path.exists() {
        eprintln!("Directory does not exist: {}", cli.directory);
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("Path is not a directory: {}", cli.directory);
        std::process::exit(1);
    }

    // Get absolute path to avoid Windows relative-path issues with .current_dir()
    let abs_dir = std::env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join(path);
    let abs_dir = abs_dir.canonicalize().unwrap_or(abs_dir);

    // Verify terraform is available
    if Command::new("terraform").arg("version").output().is_err() {
        eprintln!("Error: 'terraform' not found in PATH. Is Terraform installed?");
        std::process::exit(1);
    }

    let output = Command::new("terraform")
        .arg("plan")
        .arg("-json")
        .arg("-input=false")
        .arg("-no-color")
        .current_dir(&abs_dir)
        .output()
        .unwrap_or_else(|e| {
            eprintln!(
                "Failed to execute terraform in '{}': {}",
                abs_dir.display(),
                e
            );
            std::process::exit(1);
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "Terraform plan failed in '{}':\n{}",
            abs_dir.display(),
            stderr
        );
        std::process::exit(1);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let resource_changes = parse_plan_output(&stdout);
    render_changes(&resource_changes, &abs_dir, &cli.format, cli.no_emoji);
}

#[cfg(test)]
mod tests {
    use super::{csv_escape, parse_plan_output, ResourceChange};

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
    fn escapes_csv_fields_that_need_quotes() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("name,with,commas"), "\"name,with,commas\"");
        assert_eq!(csv_escape("name \"quoted\""), "\"name \"\"quoted\"\"\"");
    }
}
