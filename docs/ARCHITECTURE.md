# Architecture Notes — terraform-plan-parser

## Overview

`terraform-plan-parser` is a single-binary Rust CLI tool that wraps `terraform plan -json` or reads pre-generated Terraform plan files, parses Terraform JSON output, optionally filters the resulting changes, and renders the summary as text, JSON, CSV, or a plain table.

`docs/ARCHITECTURE.md` is the canonical architecture document for the project.

## System Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                        User Shell                           │
│  $ terraform_plan_parser [DIRECTORY] [--plan-file PATH]     │
│    [--config PATH] [--dry-run] [--format text|json|csv|table]│
│    [--fail-on ACTION] [--completions SHELL]                 │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Interface Layer                      │
│  • Parse command-line arguments with clap derive macros      │
│  • Accept directory/.tfplan, --plan-file, format, emoji,    │
│    dry-run, verbosity, config, filters, fail-on, and        │
│    completion flags                                         │
│  • Generate shell completions before config/input handling  │
│  • CLI path and filter values override config defaults      │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Configuration Layer                        │
│  • Load .terraform-plan-parser.toml from the current dir or │
│    next to the selected input, unless --config is provided  │
│  • Resolve relative config plan-file values from the config │
│    file directory                                           │
│  • Build effective runtime settings before tracing starts   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Input Resolution Layer                   │
│  • Validate input paths and resolve absolute paths          │
│  • Give --plan-file/config plan-file precedence over the    │
│    positional DIRECTORY                                     │
│  • Classify input as live directory, JSON plan file, or     │
│    saved binary .tfplan file                                │
│  • Short-circuit in --dry-run mode after rendering intent   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Logging Layer                            │
│  • tracing subscriber defaults to info-level final output   │
│  • --verbose/-v or config verbose enables debug diagnostics │
│    on stderr                                                │
│  • warnings/errors use tracing warn!/error! macros          │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Terraform Invocation Layer                 │
│  • Verify terraform is available only for live plans or     │
│    saved .tfplan files                                      │
│  • Execute terraform plan -json -input=false -no-color for  │
│    live directories                                         │
│  • Execute terraform show -json for saved .tfplan files     │
│  • Read JSON/NDJSON plan files without invoking Terraform   │
│  • Exit with code 1 if Terraform/file loading fails         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   JSON Parsing Layer                        │
│  • Stream-read live-plan stdout line-by-line                │
│  • Parse JSON plan files from disk                          │
│  • Parse Terraform NDJSON lines and terraform show JSON via │
│    serde_json                                               │
│  • Extract resource type, resource name, and normalized     │
│    action values                                            │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Filtering Layer                           │
│  • Apply include/exclude filters to resource type and action│
│  • Support exact values and glob wildcards (*, ?)           │
│  • Treat exclude matches as higher priority than includes   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Rendering Layer                           │
│  • Render text, JSON, CSV, or table output                  │
│  • Map text actions to emoji symbols unless disabled        │
│  • Honor quiet mode for text/table summary lines            │
│  • Keep machine-readable JSON/CSV payloads on stdout        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Policy Layer                              │
│  • Evaluate --fail-on against filtered action values        │
│  • Exit with code 1 after rendering matching forbidden plans│
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```text
                 ┌────────────────────────────┐
                 │ CLI args + optional config │
                 └──────────────┬─────────────┘
                                ▼
                         Effective settings
                                │
                                ▼
┌───────────────────┐    ┌──────────────┐    ┌───────────────────┐
│ Terraform project │───▶│ plan -json   │───▶│ NDJSON line parser│
└───────────────────┘    └──────────────┘    └─────────┬─────────┘
                                                        │
┌───────────────────┐    ┌──────────────┐              │
│ saved .tfplan     │───▶│ show -json   │──────────────┤
└───────────────────┘    └──────────────┘              │
                                                        ▼
┌───────────────────┐                            Vec<ResourceChange>
│ JSON/NDJSON file  │───────────────────────────────┬───────────────┘
└───────────────────┘                               ▼
                                               Filters
                                                   │
                                                   ▼
                                               Renderer ─────▶ stdout
                                                   │
                                                   ▼
                                             Fail-on check
```

## Configuration

The CLI supports `.terraform-plan-parser.toml` for persistent defaults. Discovery order is:

1. The explicit `--config PATH`, if provided.
2. `.terraform-plan-parser.toml` in the current working directory.
3. `.terraform-plan-parser.toml` next to the selected positional directory/file or explicit `--plan-file`.

Supported keys use kebab-case TOML names that mirror CLI flags:

```toml
plan-file = "plan.ndjson"
format = "csv"
no-emoji = true
dry-run = false
verbose = false
quiet = false
include-type = ["aws_*"]
exclude-type = ["*_bucket"]
include-action = ["create", "update"]
exclude-action = ["delete"]
fail-on = ["delete"]
```

CLI values take precedence over config defaults for `plan-file`, `format`, and each filter list. Boolean flags are enabled when either the CLI flag or the config value is true. Relative `plan-file` paths from config are resolved relative to the config file directory.

## Module Structure

```text
src/
├── main.rs                         # Entry point + unit tests
│   └── main()                      # Orchestration: CLI → config → input → parse → filter → render → policy
├── cli.rs                          # CLI types, config loading, and settings resolution
│   ├── Cli / ConfigFile / AppSettings / Format / SortBy
│   ├── load_config() / resolve_config_path() / default_config_candidates()
│   ├── app_settings() / resolve_include_action()
│   └── cli_or_config_values() / resolve_config_relative_path()
├── parser.rs                       # Data models, JSON parsing, filtering, and sorting
│   ├── ResourceChange / PlanLine / ShowPlan / TerraformInput / ChangeCounts
│   ├── parse_plan_line() / parse_plan_output() / parse_show_plan_output()
│   ├── filter_changes() / matches_filter() / matches_pattern()
│   ├── sort_resource_changes() / count_actions() / has_fail_on_actions()
│   └── Format enum (shared with CLI layer)
├── renderer.rs                     # All output rendering
│   ├── LevelWriter / OutputWriter  # Tracing log routing (stdout vs stderr)
│   ├── render_text() / render_json() / render_csv() / render_table()
│   ├── render_changes() / render_summary_line() / summary_action_symbols()
│   ├── render_github_step_summary() / append_github_step_summary()
│   ├── write_github_summary_if_enabled() / should_write_github_summary()
│   └── write_rendered_output()
└── terraform.rs                    # Terraform process management
    ├── init_tracing()              # Verbosity and subscriber setup
    ├── terraform_command() / verify_terraform_available()
    ├── render_dry_run()            # Dry-run intent rendering
    ├── load_changes()              # Input dispatch: stdin / directory / plan file / tfplan
    ├── run_terraform_plan() / run_terraform_show()
    ├── resolve_input() / read_piped_stdin()
    ├── resolve_plan_file_input() / resolve_positional_input()
    └── absolutize() / is_tfplan_file()
```

## Key Design Decisions

| Decision | Rationale |
| --- | --- |
| Single binary | Easy distribution; no runtime dependencies beyond Terraform for live plans and `.tfplan` conversion. |
| Zero-config by default | The tool still works without a config file; `.terraform-plan-parser.toml` only provides reusable defaults. |
| CLI precedence | Explicit CLI arguments should be safe for one-off overrides in scripts and CI. |
| Config-relative plan files | A committed project config can point at a local generated plan fixture or CI artifact path predictably. |
| Stream parsing | `terraform plan -json` emits newline-delimited JSON, so live output is parsed line-by-line instead of buffering the whole stream first. |
| Absolute path resolution | Prevents Windows-specific issues where `.current_dir()` behaves unexpectedly with relative paths. |
| Exit codes | `0` means success or no changes; `1` means invalid input, missing Terraform, failed plan/show, unreadable config, parse/load errors, or filtered `--fail-on` matches. |
| Glob filters | Resource type and action filters support exact values plus wildcard patterns while preserving comma-separated CLI behavior. |
| Dry-run short-circuit | `--dry-run` resolves and validates the input source, prints the command or file read that would happen, and exits before Terraform availability checks or plan loading. |
| Completion short-circuit | `--completions` emits the requested shell script before config discovery so completions do not depend on project files. |
| Fail-on guardrails | `--fail-on` is evaluated after include/exclude filters so CI policies apply to the same visible change set users reviewed. |
| Tracing-based logging | The tracing subscriber keeps info-level rendered summaries on stdout, routes warnings/errors/debug diagnostics to stderr, and raises the max level from info to debug when verbose mode is used. |

## Dependencies

| Crate | Purpose |
| --- | --- |
| `clap` | Command-line parsing and help text generation. |
| `clap_complete` | Shell completion generation for bash, elvish, fish, PowerShell, and zsh. |
| `glob` | Wildcard pattern matching for include/exclude filters. |
| `serde` | Derive macros for TOML and JSON deserialization plus JSON serialization. |
| `serde_json` | Terraform JSON parsing and JSON output rendering. |
| `toml` | `.terraform-plan-parser.toml` parsing. |
| `tracing` | Structured application logging macros. |
| `tracing-subscriber` | Runtime log filtering and stdout/stderr formatting. |

`Cargo.toml` is the canonical dependency manifest; `Cargo.lock` captures the resolved application dependency graph.

## Error Handling Strategy

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Config Discovery│────▶│ Read/Parse TOML │────▶│ Early Exit      │
│ (optional)      │     │ (if present)    │     │ (code 1 on err) │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ User Input      │────▶│ Validation      │────▶│ Early Exit      │
│ (args/config)   │     │ (exists/type)   │     │ (code 1 on err) │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Dry-run Flag    │────▶│ Render Intent   │────▶│ Success Exit    │
│ (optional)      │     │ (no Terraform)  │     │ (code 0)        │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Terraform Call  │────▶│ Check Status    │────▶│ Early Exit      │
│ (if required)   │     │ (success?)      │     │ (code 1 on err) │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ JSON Parse      │────▶│ Warn on Invalid │────▶│ Continue Loop   │
│ (per NDJSON)    │     │ NDJSON Lines    │     │ (graceful)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Future Extension Points

- Split the current single-file implementation into focused modules once feature growth justifies it.
- Add resource name, address, module path, or provider filters.
- Add explicit config-generation or config-validation commands.
- Validate Terraform version compatibility before live plan/show execution.
- Add configurable policy presets for common CI/CD create/update/delete guardrails.

## Technology Stack

| Layer | Technology |
| --- | --- |
| Language | Rust (Edition 2021) |
| CLI Args | `clap` derive parser |
| Config Parsing | `toml` + `serde` |
| JSON Parsing | `serde` + `serde_json` |
| Filtering | `glob` |
| Shell Completions | `clap_complete` |
| Process Spawning | `std::process::Command` |
| Logging | `tracing` + `tracing-subscriber` |
| Target Platforms | Windows, macOS, Linux |
