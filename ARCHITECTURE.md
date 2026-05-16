# Architecture Notes — terraform-plan-parser

## Overview

`terraform-plan-parser` is a single-binary Rust CLI tool that wraps `terraform plan -json` or reads pre-generated plan files, parses Terraform JSON output, and prints a human-readable summary of resource changes.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        User Shell                           │
│  $ terraform_plan_parser [DIRECTORY] [--plan-file PATH]                        │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Interface Layer                      │
│  • Parse command-line arguments (directory path, --help)    │
│  • Validate directory exists and is a directory             │
│  • Resolve absolute path (handles Windows relative paths)   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Terraform Invocation Layer                 │
│  • Verify `terraform` is available only for live plans or `.tfplan` files           │
│  • Execute: `terraform plan -json -input=false -no-color`   │
│  • Capture stdout (JSON stream) and stderr                  │
│  • Exit with code 1 if Terraform/file loading fails                      │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   JSON Parsing Layer                        │
│  • Stream-read live-plan stdout line-by-line or parse plan-file contents                          │
│  • Parse each line as JSON via `serde_json`                 │
│  • Extract: `change.resource.resource_type`                 │
│           `change.resource.resource_name`                   │
│           `change.action`                                   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Rendering Layer                           │
│  • Map actions to emoji symbols:                            │
│    create → ➕ | update → 🔄 | delete → ➖ | read → 📖      │
│  • Print formatted summary table                            │
│  • Handle empty state: "✅ No resource changes detected"    │
└─────────────────────────────────────────────────────────────┘
```

## Data Flow

```
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
                                            │  Stdout Render│
                                            │  (emoji + text)│
                                            └───────────────┘
```

## Module Structure

```
src/
├── main.rs              # Single-file application (no submodules)
│   ├── print_help()     # CLI help text
│   └── main()           # Entry point: args → validate → run → parse → render
```

> **Note:** The project is intentionally kept as a single-file CLI for simplicity. As features grow, consider splitting into:
> - `cli.rs` — argument parsing
> - `terraform.rs` — Terraform process management
> - `parser.rs` — JSON deserialization models
> - `renderer.rs` — output formatting

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Single binary** | Easy distribution; no runtime dependencies beyond Terraform |
| **Stream parsing** | `terraform plan -json` emits NDJSON (newline-delimited JSON); we parse line-by-line to avoid loading the entire output into memory |
| **Absolute path resolution** | Prevents Windows-specific issues where `.current_dir()` behaves unexpectedly with relative paths |
| **Exit codes** | `0` = success (or no changes), `1` = error (invalid dir, terraform missing, plan failed) |
| **No config file** | Zero-configuration tool; all behavior is deterministic |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` | Derive macros for JSON deserialization |
| `serde_json` | Runtime JSON parsing |

> `requirements.txt` exists for documentation/reference only. Actual dependency management is via `Cargo.toml`.

## Error Handling Strategy

```
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

1. **Structured output formats** — Add `--format json|csv|table` flags
2. **Filtering** — `--include-type aws_instance` or `--exclude-action read`
3. **Additional plan-source detection** — Keep expanding file/source handling while preserving `--plan-file` precedence
4. **Pre-flight checks** — Validate Terraform version compatibility
5. **CI/CD integration** — Exit with different codes for `create` vs `delete` actions
6. **Configuration file** — `.terraform-plan-parser.toml` for persistent filters

## Technology Stack

| Layer | Technology |
|-------|------------|
| Language | Rust (Edition 2021) |
| JSON Parsing | serde + serde_json |
| Process Spawning | std::process::Command |
| CLI Args | std::env::args (manual parsing) |
| Target Platforms | Windows, macOS, Linux |
