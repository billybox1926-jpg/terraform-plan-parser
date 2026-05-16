use serde::Deserialize;
use std::{env, path::Path, process::Command};

#[derive(Debug, PartialEq, Eq)]
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

fn print_help() {
    println!("terraform_plan_parser - summarize terraform plan -json output\n");
    println!("Usage:");
    println!("  terraform_plan_parser [DIRECTORY]\n");
    println!("Options:");
    println!("  -h, --help    Show this help message");
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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return;
    }

    let dir = if args.len() > 1 { &args[1] } else { "." };

    let path = Path::new(dir);
    if !path.exists() {
        eprintln!("Directory does not exist: {}", dir);
        std::process::exit(1);
    }
    if !path.is_dir() {
        eprintln!("Path is not a directory: {}", dir);
        std::process::exit(1);
    }

    // Get absolute path to avoid Windows relative-path issues with .current_dir()
    let abs_dir = env::current_dir()
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
    let mut resource_changes = Vec::new();

    for line in stdout.lines() {
        if let Ok(json) = serde_json::from_str::<Value>(line) {
            if let Some(change) = json.get("change") {
                if let Some(resource) = change.get("resource") {
                    let resource_type = resource["resource_type"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();
                    let resource_name = resource["resource_name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();
                    let action = change["action"].as_str().unwrap_or("noop").to_string();

                    resource_changes.push((resource_type, resource_name, action));
                }
            }
        }
    }

    if resource_changes.is_empty() {
        println!(
            "✅ No resource changes detected in '{}'.",
            abs_dir.display()
        );
        return;
    }

    println!("📊 Planned changes in '{}':", abs_dir.display());
    for change in resource_changes {
        let symbol = match change.action.as_str() {
            "create" => "➕",
            "update" => "🔄",
            "delete" => "➖",
            "read" => "📖",
            _ => "•",
        };
        println!(
            "{} {} {} ({})",
            symbol, change.resource_type, change.resource_name, change.action
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_plan_output, ResourceChange};

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
}
