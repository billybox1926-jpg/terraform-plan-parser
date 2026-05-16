use serde_json::Value;
use std::{env, path::Path, process::Command};

fn print_help() {
    println!("terraform_plan_parser - summarize terraform plan -json output\n");
    println!("Usage:");
    println!("  terraform_plan_parser [DIRECTORY]\n");
    println!("Options:");
    println!("  -h, --help    Show this help message");
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
    for (res_type, res_name, action) in resource_changes {
        let symbol = match action.as_str() {
            "create" => "➕",
            "update" => "🔄",
            "delete" => "➖",
            "read" => "📖",
            _ => "•",
        };
        println!("{} {} {} ({})", symbol, res_type, res_name, action);
    }
}
