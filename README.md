# terraform-plan-parser

A lightweight Rust CLI tool that parses Terraform plan JSON output and displays a clean, human-readable summary of planned resource changes.

## Features

- **Colorful summary** — see at a glance what's being created (➕), updated (🔄), deleted (➖), or read (📖)
- **Directory-aware** — point it at any Terraform project directory to run `terraform plan -json`
- **Plan-input aware** — parse piped Terraform plan JSON from stdin, pre-generated NDJSON/full JSON plan files with `--plan-file`, or saved `.tfplan` files through `terraform show -json`
- **Dry-run mode** — preview the Terraform command or file read that would happen with `--dry-run` without executing Terraform
- **Configurable logging** — keep default output focused on the final summary, or add `--verbose`/`-v` for debug diagnostics
- **Flexible filtering** — narrow results with comma-separated exact or glob patterns such as `--include-type aws_*`, `--exclude-type *_bucket`, or `--include-action cre*`
- **Optional project config** — persist output and filter defaults in `.terraform-plan-parser.toml`
- **Zero config by default** — just run it
- **Cross-platform** — works on Windows, macOS, and Linux

## Prerequisites

- [Rust](https://rustup.rs/) (to build from source)
- [Terraform](https://developer.hashicorp.com/terraform/downloads) (must be in your `PATH` when running live plans or reading `.tfplan` binary files)

## Installation

```bash
# Clone the repo
git clone https://github.com/billybox1926-jpg/terraform-plan-parser.git
cd terraform-plan-parser

# Build and install
cargo install --path .
```

## Usage

Run a live plan for the current directory:

```bash
terraform_plan_parser .
```

Run a live plan for another Terraform project directory:

```bash
terraform_plan_parser some/dir
```

Pipe Terraform JSON output directly into the parser:

```bash
terraform plan -json | terraform_plan_parser
```

Piped stdin takes precedence over `--plan-file` and live Terraform execution. If no stdin data is present, the CLI falls back to the selected plan file or directory.

Parse a pre-generated Terraform plan JSON file without running `terraform plan`:

```bash
terraform_plan_parser --plan-file plan.ndjson
```

`--plan-file` accepts newline-delimited JSON from `terraform plan -json > plan.ndjson` and full JSON from `terraform show -json saved.tfplan > plan.json`. When the provided plan file ends in `.tfplan`, the CLI converts it with `terraform show -json`.

Saved `.tfplan` files can still be passed positionally for backward compatibility:

```bash
terraform_plan_parser saved.tfplan
```

`--plan-file` takes precedence if both a positional directory/file and `--plan-file` are provided.

Preview what the CLI would do without executing Terraform:

```bash
terraform_plan_parser . --dry-run
terraform_plan_parser --plan-file saved.tfplan --dry-run
terraform_plan_parser --plan-file plan.json --dry-run
```

Dry-run mode still validates the selected input path. For live directories it prints the `terraform plan -json -input=false -no-color` command that would run, for saved `.tfplan` files it prints the `terraform show -json` command that would run, and for JSON plan files it reports that the file would be read without any Terraform command.

Enable verbose logging when you need to troubleshoot path resolution, Terraform execution, or plan-file loading:

```bash
terraform_plan_parser . --verbose
terraform_plan_parser --plan-file plan.ndjson -v
terraform_plan_parser . --dry-run --verbose
```

By default, the CLI keeps stdout focused on the final rendered summary. Verbose debug diagnostics are written to stderr so JSON/CSV stdout output stays script-friendly.

## Filtering

Filter flags accept comma-separated values. Exact matches remain supported, and each value may also be a glob pattern using wildcards such as `*` and `?`. Include filters are applied first, then matching exclude filters remove resources from the result.

```bash
# Include only AWS resource types
terraform_plan_parser . --include-type 'aws_*'

# Include instance-like resources while hiding bucket resources
terraform_plan_parser . --include-type '*instance' --exclude-type '*_bucket'

# Shorthand for delete-only safety reviews
terraform_plan_parser . --plan-file plan.ndjson -d

# Action filters also accept glob patterns
terraform_plan_parser . --include-action 'cre*' --exclude-action 'no*'

# Multiple patterns can be comma-separated
terraform_plan_parser . --include-type 'aws_*,google_*' --exclude-action 'delete,replace'
```

Available filter flags:

- `--include-type GLOB[,GLOB]...`
- `--exclude-type GLOB[,GLOB]...`
- `--include-action GLOB[,GLOB]...`
- `-d`, `--only-delete` — shorthand for `--include-action delete`
- `-c`, `--only-create` — shorthand for `--include-action create`
- `-u`, `--only-update` — shorthand for `--include-action update`
- `--exclude-action GLOB[,GLOB]...`

## Configuration file

Add `.terraform-plan-parser.toml` to reuse defaults across local runs and CI jobs. The CLI discovers the file in the current directory or next to the selected input, and `--config PATH` can point at a specific file.

```toml
plan-file = "plan.ndjson"
format = "csv"
no-emoji = true
verbose = false
include-type = ["aws_*"]
exclude-type = ["*_bucket"]
include-action = ["create", "update"]
exclude-action = ["delete"]
```

CLI options override config defaults for `plan-file`, `format`, and filter lists. Boolean options are enabled when either the config value or CLI flag is true. Relative `plan-file` values are resolved from the config file directory.

## Architecture

See the canonical [Architecture Notes](ARCHITECTURE.md) for system architecture, configuration flow, data flow, design decisions, and future extension points.
