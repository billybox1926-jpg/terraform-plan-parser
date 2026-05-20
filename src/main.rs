mod cli;
mod parser;
mod renderer;
mod terraform;

use clap::{CommandFactory, Parser};

fn main() {
    let cli = cli::Cli::parse();
    if let Some(shell) = cli.completions {
        let mut cmd = cli::Cli::command();
        let bin_name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        return;
    }
    let (config, config_path) = cli::load_config(&cli).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    let settings = cli::app_settings(&cli, config, config_path.as_deref());

    terraform::init_tracing(settings.verbose);
    tracing::debug!(config_path = ?config_path, "Configuration loaded");
    tracing::debug!("Verbose logging enabled");

    let input =
        terraform::resolve_input(&settings, &cli.directory, &cli.compare).unwrap_or_else(|error| {
            tracing::error!("{error}");
            std::process::exit(1);
        });

    // Handle compare mode
    if let parser::TerraformInput::Compare { old, new } = &input {
        let diff = terraform::load_and_compare(old, new).unwrap_or_else(|error| {
            tracing::error!("{error}");
            std::process::exit(1);
        });
        let rendered = renderer::render_diff(&diff, &settings.format, settings.no_emoji);
        renderer::write_rendered_output(settings.output_file.as_deref(), &rendered).unwrap_or_else(
            |error| {
                tracing::error!("{error}");
                std::process::exit(1);
            },
        );
        return;
    }

    if settings.dry_run {
        tracing::info!("{}", terraform::render_dry_run(&input).trim_end());
        return;
    }

    if input.requires_terraform() {
        terraform::verify_terraform_available().unwrap_or_else(|error| {
            tracing::error!("{error}");
            std::process::exit(1);
        });
    }

    let resource_changes = terraform::load_changes(&input).unwrap_or_else(|error| {
        tracing::error!("{error}");
        std::process::exit(1);
    });
    let mut resource_changes = parser::filter_changes(resource_changes, &settings);
    parser::sort_resource_changes(&mut resource_changes, settings.sort_by);
    let stdin_display_path = std::path::Path::new("");
    let display_path = match &input {
        parser::TerraformInput::StdinJson(_) => stdin_display_path,
        parser::TerraformInput::Directory(directory) => directory.as_path(),
        parser::TerraformInput::JsonPlanFile(plan_file)
        | parser::TerraformInput::BinaryPlanFile(plan_file) => plan_file.as_path(),
        parser::TerraformInput::Compare { new, .. } => new.as_path(),
    };

    let rendered_output = renderer::render_changes(
        &resource_changes,
        display_path,
        &settings.format,
        settings.no_emoji,
        settings.quiet,
        settings.no_header,
    );
    renderer::write_rendered_output(settings.output_file.as_deref(), &rendered_output)
        .unwrap_or_else(|error| {
            tracing::error!("{error}");
            std::process::exit(1);
        });

    renderer::write_github_summary_if_enabled(&settings, display_path, &resource_changes);

    if parser::has_fail_on_actions(&resource_changes, &settings.fail_on) {
        tracing::error!("Plan contains forbidden actions matching --fail-on criteria");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{app_settings, Cli, ConfigFile, SortBy};
    use crate::parser::{
        count_actions, filter_changes, has_fail_on_actions, parse_plan_output,
        sort_resource_changes, ChangeCounts, Format, ResourceChange, TerraformInput,
    };
    use crate::renderer::{
        append_github_step_summary, csv_escape, render_csv, render_github_step_summary,
        render_json, render_summary_line, render_table, render_text, write_rendered_output,
    };
    use crate::terraform::render_dry_run;
    use clap::Parser;
    use std::{fs, path::Path};

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
    fn writes_rendered_output_to_file_when_requested() {
        let temp_file = std::env::temp_dir().join(format!(
            "terraform_plan_parser_output_{}.txt",
            std::process::id()
        ));
        let content = "test output\n";

        write_rendered_output(Some(temp_file.as_path()), content).expect("writes output file");
        assert_eq!(
            fs::read_to_string(&temp_file).expect("read output file"),
            content
        );

        fs::remove_file(temp_file).expect("remove output file");
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
            render_csv(
                &[ResourceChange {
                    resource_type: "aws_instance".to_string(),
                    resource_name: "web".to_string(),
                    action: "create".to_string(),
                }],
                false,
            ),
            "resource_type,resource_name,action\naws_instance,web,create\n"
        );
        assert_eq!(
            render_csv(
                &[ResourceChange {
                    resource_type: "aws_instance".to_string(),
                    resource_name: "web".to_string(),
                    action: "create".to_string(),
                }],
                true,
            ),
            "aws_instance,web,create\n"
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
            "Summary:\n + 2 to create\n ~ 1 to update\n - 1 to delete\n"
        );
    }

    #[test]
    fn counts_create_delete_replacements_as_create_and_delete() {
        let changes = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create/delete".to_string(),
        }];

        assert_eq!(
            count_actions(&changes),
            ChangeCounts {
                create: 1,
                update: 0,
                delete: 1,
            }
        );
        assert_eq!(
            render_summary_line(&count_actions(&changes), true),
            "Summary:\n + 1 to create\n ~ 0 to update\n - 1 to delete\n"
        );
    }

    #[test]
    fn preserves_create_delete_rendering_while_counting_summary_totals() {
        let stdout = r#"{
    "resource_changes": [
        {
            "type": "aws_instance",
            "name": "web",
            "change": { "actions": ["create", "delete"] }
        }
    ]
}"#;

        let changes = parse_plan_output(stdout);

        assert_eq!(
            changes,
            vec![ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create/delete".to_string(),
            }]
        );
        assert_eq!(
            count_actions(&changes),
            ChangeCounts {
                create: 1,
                update: 0,
                delete: 1,
            }
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
    fn render_json_serializes_resource_changes() {
        let changes = vec![
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
        ];
        let output = render_json(&changes);
        assert!(output.contains(r#""resource_type": "aws_instance""#));
        assert!(output.contains(r#""action": "create""#));
        assert!(output.contains(r#""action": "delete""#));
    }

    #[test]
    fn sorts_resource_changes_by_type() {
        let mut changes = vec![
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "logs".to_string(),
                action: "update".to_string(),
            },
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "web".to_string(),
                action: "create".to_string(),
            },
        ];
        sort_resource_changes(&mut changes, Some(SortBy::Type));
        assert_eq!(changes[0].resource_type, "aws_instance");
        assert_eq!(changes[1].resource_type, "aws_s3_bucket");
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
        let error = crate::terraform::resolve_plan_file_input(Path::new("./missing-plan.json"))
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

    // ── Compare mode unit tests ──────────────────────────────────────────────

    #[test]
    fn compare_plans_detects_added_resources() {
        let old = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let new = vec![
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
        ];

        let diff = crate::parser::compare_plans(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].resource_name, "logs");
        assert!(diff.removed.is_empty());
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn compare_plans_detects_removed_resources() {
        let old = vec![
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
        ];
        let new = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];

        let diff = crate::parser::compare_plans(&old, &new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].resource_name, "logs");
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn compare_plans_detects_changed_actions() {
        let old = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let new = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "update".to_string(),
        }];

        let diff = crate::parser::compare_plans(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].old_action, "create");
        assert_eq!(diff.changed[0].new_action, "update");
    }

    #[test]
    fn compare_plans_empty_when_identical() {
        let old = vec![ResourceChange {
            resource_type: "aws_instance".to_string(),
            resource_name: "web".to_string(),
            action: "create".to_string(),
        }];
        let new = old.clone();

        let diff = crate::parser::compare_plans(&old, &new);
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn compare_plans_sorts_output_deterministically() {
        let old = vec![];
        let new = vec![
            ResourceChange {
                resource_type: "aws_s3_bucket".to_string(),
                resource_name: "z_bucket".to_string(),
                action: "create".to_string(),
            },
            ResourceChange {
                resource_type: "aws_instance".to_string(),
                resource_name: "a_instance".to_string(),
                action: "create".to_string(),
            },
        ];

        let diff = crate::parser::compare_plans(&old, &new);
        assert_eq!(diff.added[0].resource_type, "aws_instance");
        assert_eq!(diff.added[1].resource_type, "aws_s3_bucket");
    }
}
