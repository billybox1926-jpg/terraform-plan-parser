# Architecture Notes

## Overview

`terraform-plan-parser` is a single-binary Rust CLI tool that wraps `terraform plan -json` or reads pre-generated plan files, parses Terraform JSON output, and prints a human-readable summary of resource changes.

## System Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                        User Shell                           │
│  $ terraform_plan_parser [DIRECTORY] [--plan-file PATH]                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Interface Layer                      │
│  • Parse command-line arguments with clap derive macros      │
│  • Accept directory/.tfplan, --plan-file, format, emoji, filter flags     │
│  • Validate input exists and resolve --plan-file before positional input │
│  • Resolve absolute path (handles Windows relative paths)   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Terraform Invocation Layer                 │
│  • Verify `terraform` is available only for live plans or `.tfplan` files           │
│  • Execute live plans, read JSON plan files, or run `terraform show -json` for .tfplan  │
│  • Stream live-plan stdout and capture stderr                │
│  • Exit with code 1 if Terraform/file loading fails                      │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   JSON Parsing Layer                        │
│  • Stream-read live-plan stdout line-by-line or parse plan-file contents                          │
│  • Parse each line as JSON via `serde_json`                 │
│  • Extract: `change.resource.resource_type`                 │
│  │         `change.resource.resource_name`                  │
│  │         `change.action`                                  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Rendering Layer                           │
│  • Map actions to emoji symbols:                            │
│  │ create → ➕ | update → 🔄 | delete → ➖ | read → 📖      │
│  • Print text, JSON, CSV, or table output                    │
│  • Handle empty text state: "✅ No resource changes detected"│
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```text
Terraform Project Directory
        │
        ▼
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  terraform    │────▶│  JSON Stream  │────▶│  Rust Parser  │
│  plan -json   │     │  (line-del.)  │     │  (serde_json) │
└───────────────┘     └───────────────┘     └───────┬───────┘
                                                    │
                                                    ▼
                                            ┌───────────────┐
                                            │  Vec<Change>  │
                                            │  (in-memory)  │
                                            └───────┬───────┘
                                                    │
                                                    ▼
                                            ┌───────────────┐
                                            │ Stdout Render │
                                            │ (emoji + text)│
                                            └───────────────┘
```

## Module Structure

```text
src/
├── main.rs                 # Single-file application (no submodules)
│   ├── ResourceChange      # In-memory parsed change model
│   ├── PlanLine structs    # Typed serde models for Terraform JSON lines
│   ├── Cli / Format       # clap-powered CLI arguments and output mode
│   ├── parse_plan_output() # NDJSON parser for Terraform output
│   └── main()              # Entry point: args → validate → run → parse → render
```

The project is intentionally kept as a single-file CLI for simplicity. As features grow, consider splitting it into:

- `cli.rs` — argument parsing
- `terraform.rs` — Terraform process management
- `parser.rs` — JSON deserialization models and `parse_plan_output` tests
- `renderer.rs` — output formatting

## Key Design Decisions

| Decision | Rationale |
| --- | --- |
| Single binary | Easy distribution; no runtime dependencies beyond Terraform. |
| Stream parsing | `terraform plan -json` emits newline-delimited JSON (NDJSON), so parsing line-by-line avoids loading the entire output into memory. |
| Absolute path resolution | Prevents Windows-specific issues where `.current_dir()` behaves unexpectedly with relative paths. |
| Exit codes | `0` = success or no changes, `1` = error such as invalid directory, missing Terraform, or failed plan. |
| No config file | Zero-configuration tool; all behavior is deterministic. |

## Dependencies

| Crate | Purpose |
| --- | --- |
| `serde` | Derive macros for JSON deserialization. |
| `serde_json` | Runtime JSON parsing. |
| `clap` | Command-line argument parsing via derive macros. |

`requirements.txt` exists for documentation/reference only. Actual dependency management is via `Cargo.toml`.

## Error Handling Strategy

```text
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   User Input    │────▶│   Validation    │────▶│   Early Exit    │
│  (args, path)   │     │  (exists, dir)  │     │   (code 1)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Terraform Call │────▶│  Check Status   │────▶│   Early Exit    │
│                 │     │  (success?)     │     │   (code 1)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  JSON Parse     │────▶│  Skip Invalid   │────▶│  Continue Loop  │
│  (per line)     │     │  Lines silently │     │  (graceful)     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Future Extension Points

- **Structured output formats** — add `--format json|csv|table` flags.
- **Filtering** — add filters such as `--include-type aws_instance` or `--exclude-action read`.
- **Additional plan-source detection** — keep expanding file/source handling while preserving `--plan-file` precedence.
- **Pre-flight checks** — validate Terraform version compatibility.
- **CI/CD integration** — exit with different codes for create vs. delete actions.
- **Configuration file** — support `.terraform-plan-parser.toml` for persistent filters.

## Technology Stack

| Layer | Technology |
| --- | --- |
| Language | Rust (Edition 2021) |
| JSON Parsing | `serde` + `serde_json` |
| Process Spawning | `std::process::Command` |
| CLI Args | `clap` derive macros |
| Target Platforms | Windows, macOS, Linux |
