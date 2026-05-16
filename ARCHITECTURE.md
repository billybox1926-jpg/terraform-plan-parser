# Architecture Notes — terraform-plan-parser

## Overview

`terraform-plan-parser` is a single-binary Rust CLI tool that wraps `terraform plan -json` or reads pre-generated plan files, parses Terraform JSON output, and prints a human-readable summary of resource changes.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        User Shell                           │
│  $ terraform_plan_parser [DIRECTORY] [--plan-file PATH] [--dry-run]             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Interface Layer                      │
│  • Parse command-line arguments (paths, output, filters)    │
│  • Validate input path exists and resolve its plan source   │
│  • Short-circuit in `--dry-run` mode after rendering intent │
│  • Resolve absolute path (handles Windows relative paths)   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Terraform Invocation Layer                 │
│  • Verify `terraform` is available only for live plans or `.tfplan` files │
│  • Execute: `terraform plan -json -input=false -no-color`   │
│  • Execute: `terraform show -json` for saved `.tfplan` files│
│  • Capture stdout (JSON stream) and stderr                  │
│  • Exit with code 1 if Terraform/file loading fails         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   JSON Parsing Layer                        │
│  • Stream-read live stdout or parse plan-file contents      │
│  • Parse Terraform JSON via `serde_json`                    │
│  • Extract resource type, resource name, and action         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                   Filtering Layer                           │
│  • Apply include/exclude filters to type and action         │
│  • Support exact values and glob wildcards (`*`, `?`)       │
│  • Treat exclude matches as higher priority than includes   │
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
                                            │ Glob Filters  │
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
│   ├── Cli              # clap-derived CLI help and argument parsing
│   ├── parse_*          # Terraform JSON deserialization helpers
│   ├── filter_*         # include/exclude exact and glob matching
│   ├── render_*         # text, JSON, CSV, table, and dry-run output
│   └── main()           # entry point: args → input → optional dry-run → parse → filter → render
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
| **Glob filters** | Resource type and action filters support exact values plus wildcard patterns while preserving comma-separated CLI behavior |
| **Dry-run short-circuit** | `--dry-run` resolves and validates the input source, prints the command or file read that would happen, and exits before Terraform availability checks or plan loading |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` | Derive macros for JSON deserialization |
| `serde_json` | Runtime JSON parsing |
| `glob` | Wildcard pattern matching for include/exclude filters |
| `clap` | Command-line parsing and help text generation |

> `requirements.txt` exists for documentation/reference only. Actual dependency management is via `Cargo.toml`.

## Error Handling Strategy

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   User Input    │────▶│   Validation    │────▶│   Early Exit    │
│  (args, path)   │     │  (exists, dir)  │     │   (code 1)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Dry-run Flag  │────▶│  Render Intent  │────▶│  Success Exit   │
│   (optional)    │     │  (no Terraform) │     │   (code 0)      │
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
2. **Filtering enhancements** — expand beyond current glob support if resource names, modules, or tags become filter targets
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
| CLI Args | clap derive parser |
| Target Platforms | Windows, macOS, Linux |
