# Terraform Plan Parser

![Maintained](https://img.shields.io/badge/Maintained-yes-brightgreen)
![Local checks](https://img.shields.io/badge/Local%20checks-documented-brightgreen)
![Rust](https://img.shields.io/badge/Rust-2021-orange)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)

A lightweight Rust CLI that turns Terraform plan JSON into clean summaries for local reviews, CI logs, and release guardrails.

`terraform-plan-parser` reads live Terraform plans, piped JSON, pre-generated JSON/NDJSON files, or saved `.tfplan` files, then renders the planned resource changes as human-readable text, JSON, CSV, or an aligned terminal table.

## Features

- **Colorful summary** — see at a glance what's being created (➕), updated (🔄), deleted (➖), or read (📖)
- **Directory-aware** — point it at any Terraform project directory to run `terraform plan -json`
- **Plan-input aware** — parse piped Terraform plan JSON from stdin, pre-generated NDJSON/full JSON plan files with `--plan-file`, or saved `.tfplan` files through `terraform show -json`
- **State inventory support** — parse Terraform state JSON from `--state`/`--state-json` and export inventory rows with the same JSON/CSV/table tooling
- **Dry-run mode** — preview the Terraform command or file read that would happen with `--dry-run` without executing Terraform
- **Configurable logging** — keep default output focused on the final summary, or add `--verbose`/`-v` for debug diagnostics
- **Flexible filtering** — narrow results with comma-separated exact or glob patterns such as `--include-type aws_*`, `--exclude-type *_bucket`, or `--include-action cre*`
- **CI guardrails** — fail a pipeline when filtered plans include risky actions with `--fail-on delete`
- **Shell completions** — generate completion scripts for bash, elvish, fish, PowerShell, or zsh with `--completions`
- **Optional project config** — persist output, filter, and CI defaults in `.terraform-plan-parser.toml`
- **Zero config by default** — just run it
- **Plan comparison** — diff two Terraform plans with `--compare old.json new.json` to see added, removed, and changed resources
- **Cross-platform** — works on Windows, macOS, and Linux

## Prerequisites

- [Rust](https://rustup.rs/) (to build from source)
- [Terraform](https://developer.hashicorp.com/terraform/downloads) (must be in your `PATH` when running live plans or reading `.tfplan` binary files)

## Installation

### Scoop (Windows)

Install directly from the repository manifest:

```powershell
scoop install https://raw.githubusercontent.com/billybox1926-jpg/terraform-plan-parser/main/scoop/terraform-plan-parser.json
```

This installs the native Windows x64 release ZIP from GitHub Releases. Scoop support currently targets Windows x64 only; Chocolatey and winget can be evaluated separately later.

### Homebrew (macOS/Linux)

```bash
brew tap billybox1926-jpg/tap
brew install terraform-plan-parser
```

To upgrade:

```bash
brew upgrade terraform-plan-parser
```

The Homebrew formula is maintained in the [homebrew-tap](https://github.com/billybox1926-jpg/homebrew-tap) repository. Windows users should use the native release ZIP below; Homebrew on Windows is only relevant inside WSL or another Linux-style environment.

**Homebrew platform support:**
- macOS: Intel (x86_64) and Apple Silicon (ARM64)
- Linux: Intel (x86_64) and ARM64
- Windows: WSL/Linux-style environments only

**Note:** ARM64 artifacts are available in the release workflow but SHA256 checksums in the Homebrew formula will be updated at the next tagged release. Until then, ARM users can download release artifacts directly from GitHub Releases or build from source.

### Pre-built binaries

Download the latest release for your platform from the [Releases page](https://github.com/billybox1926-jpg/terraform-plan-parser/releases). GitHub Releases are the source of truth for downloadable binaries and checksums.

Available artifacts:
- `terraform_plan_parser-linux-x64.tar.gz` — Linux (x86_64)
- `terraform_plan_parser-linux-arm64.tar.gz` — Linux (ARM64)
- `terraform_plan_parser-macos-x64.tar.gz` — macOS (Intel)
- `terraform_plan_parser-macos-arm64.tar.gz` — macOS (Apple Silicon)
- `terraform_plan_parser-windows-x64.zip` — native Windows (x86_64)
- `SHA256SUMS` — checksums for all artifacts

**Linux (x64):**
```bash
curl -LO https://github.com/billybox1926-jpg/terraform-plan-parser/releases/latest/download/terraform_plan_parser-linux-x64.tar.gz
tar xzf terraform_plan_parser-linux-x64.tar.gz
sudo mv terraform_plan_parser /usr/local/bin/
```

**Linux (ARM64):**
```bash
curl -LO https://github.com/billybox1926-jpg/terraform-plan-parser/releases/latest/download/terraform_plan_parser-linux-arm64.tar.gz
tar xzf terraform_plan_parser-linux-arm64.tar.gz
sudo mv terraform_plan_parser /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -LO https://github.com/billybox1926-jpg/terraform-plan-parser/releases/latest/download/terraform_plan_parser-macos-x64.tar.gz
tar xzf terraform_plan_parser-macos-x64.tar.gz
sudo mv terraform_plan_parser /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -LO https://github.com/billybox1926-jpg/terraform-plan-parser/releases/latest/download/terraform_plan_parser-macos-arm64.tar.gz
tar xzf terraform_plan_parser-macos-arm64.tar.gz
sudo mv terraform_plan_parser /usr/local/bin/
```

**Windows (x64):**
```powershell
# Download terraform_plan_parser-windows-x64.zip from the Releases page.
# Extract it, then add the extracted folder to your PATH.
```

Verify the download against the `SHA256SUMS` file included with each release.

### Build from source

Requires [Rust](https://rustup.rs/).

```bash
git clone https://github.com/billybox1926-jpg/terraform-plan-parser.git
cd terraform-plan-parser
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

Parse a local Terraform state JSON file as an inventory:

```bash
terraform_plan_parser --state terraform.tfstate
terraform_plan_parser --state-json state.json --format csv
terraform state pull > state.json
terraform_plan_parser --state-json state.json --include-action managed
```

State parsing reads local JSON state only. It does not fetch remote state itself; use `terraform state pull` first when your backend is remote. State resources are rendered as inventory rows where `resource_type` is the Terraform type, `resource_name` includes module and instance index information when present, and `action` contains the state resource mode such as `managed` or `data`. Plan comparison and drift detection are still plan-focused and do not compare state files yet.

Preview what the CLI would do without executing Terraform:

```bash
terraform_plan_parser . --dry-run
terraform_plan_parser --plan-file saved.tfplan --dry-run
terraform_plan_parser --plan-file plan.json --dry-run
terraform_plan_parser --state terraform.tfstate --dry-run
```

Dry-run mode still validates the selected input path. For live directories it prints the `terraform plan -json -input=false -no-color` command that would run, for saved `.tfplan` files it prints the `terraform show -json` command that would run, and for JSON plan or state files it reports that the file would be read without any Terraform command.

Enable verbose logging when you need to troubleshoot path resolution, Terraform execution, or plan-file loading:

```bash
terraform_plan_parser . --verbose
terraform_plan_parser --plan-file plan.ndjson -v
terraform_plan_parser . --dry-run --verbose
```

By default, the CLI keeps stdout focused on the final rendered summary. Verbose debug diagnostics are written to stderr so JSON/CSV stdout output stays script-friendly.

Choose an output format when you need machine-readable output or aligned terminal tables:

```bash
terraform_plan_parser . --format text
terraform_plan_parser . --format json
terraform_plan_parser . --format csv
terraform_plan_parser . --format table
```

Write rendered output to a file (useful for CI artifacts) with `--output-file`:

```bash
terraform_plan_parser . --format json --output-file plan-output.json
```

Use `--no-emoji` when plain text output is preferred, or `--quiet`/`-q` to suppress the final action-count summary in text and table output:

```bash
terraform_plan_parser . --no-emoji
terraform_plan_parser . --format table --quiet
```

Generate shell completions and write the script wherever your shell expects it:

```bash
terraform_plan_parser --completions bash > /etc/bash_completion.d/terraform_plan_parser
terraform_plan_parser --completions zsh > _terraform_plan_parser
```

Supported completion shells are `bash`, `elvish`, `fish`, `powershell`, and `zsh`.

## CLI reference

| Option | Description |
| --- | --- |
| `[DIRECTORY]` | Terraform project directory or saved `.tfplan` file to inspect. Defaults to the current directory. |
| `--plan-file PATH` | Read a pre-generated NDJSON/full JSON plan file, or convert a saved `.tfplan` file with `terraform show -json`. |
| `--state PATH` | Read a local Terraform state JSON file and render an inventory. |
| `--state-json PATH` | Alias for `--state`, useful when reading output from `terraform state pull`. |
| `--compare PATH,PATH` | Compare two plan files and show added, removed, and changed resources. Accepts NDJSON, JSON, or `.tfplan` files. |
| `--config PATH` | Read defaults from a specific `.terraform-plan-parser.toml` file instead of auto-discovering one. |
| `--output-file PATH` | Write rendered output to a file instead of stdout. |
| `--format text|json|csv|table` | Choose text, JSON, CSV, or aligned table output. |
| `--no-emoji` | Render text/table summaries without emoji symbols. |
| `--dry-run` | Validate the selected input and print the Terraform command or file read that would happen, without loading a plan. |
| `--verbose`, `-v` | Enable debug diagnostics on stderr. |
| `--quiet`, `-q` | Suppress the action summary line in text/table output. |
| `--include-type GLOB[,GLOB]...` | Keep only resource types matching exact values or glob patterns. |
| `--exclude-type GLOB[,GLOB]...` | Remove resource types matching exact values or glob patterns. |
| `--include-action GLOB[,GLOB]...` | Keep only actions matching exact values or glob patterns. |
| `--exclude-action GLOB[,GLOB]...` | Remove actions matching exact values or glob patterns. |
| `--fail-on ACTION[,ACTION]...` | Exit non-zero when filtered results contain one of the listed actions. |
| `--completions bash|elvish|fish|powershell|zsh` | Generate a shell completion script and exit. |
| `--help`, `-h` | Print help text. |

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

## CI guardrails

Use `--fail-on ACTION[,ACTION]...` to make the command exit non-zero when the parsed, filtered plan still contains one of the listed actions. This is evaluated after include/exclude filters, which lets CI jobs block only the subset of changes they care about.

```bash
# Fail if any visible change deletes a resource
terraform_plan_parser . --fail-on delete

# Ignore data reads and fail on destructive replacement-style actions
terraform_plan_parser . --exclude-action 'read,noop' --fail-on 'delete,replace'
```

## Plan comparison

Use `--compare` to diff two Terraform plan files and see what changed:

```bash
# Compare two plan files
terraform_plan_parser --compare old-plan.json new-plan.json

# Compare with JSON output for CI/reporting
terraform_plan_parser --compare old-plan.json new-plan.json --format json

# Compare with CSV output
terraform_plan_parser --compare old-plan.json new-plan.json --format csv

# Compare .tfplan binary files (requires terraform in PATH)
terraform_plan_parser --compare old.tfplan new.tfplan

# Write diff output to a file
terraform_plan_parser --compare old.json new.json --format json --output-file diff.json
```

The diff output shows:
- **Added** — resources present in the new plan but not the old
- **Removed** — resources present in the old plan but not the new
- **Changed** — resources with different actions between plans (e.g., `create → update`)

All output formats (text, JSON, CSV, table) are supported. Resources are matched by `(resource_type, resource_name)` and sorted deterministically.

## Configuration file

Add `.terraform-plan-parser.toml` to reuse defaults across local runs and CI jobs. The CLI discovers the file in the current directory or next to the selected input, and `--config PATH` can point at a specific file.

A complete copy/pasteable example is available at [`examples/terraform-plan-parser.toml`](examples/terraform-plan-parser.toml).

```toml
plan-file = "plan.ndjson"
format = "csv"
output-file = "plan-output.csv"
no-emoji = true
dry-run = false
verbose = false
quiet = false
no-header = false

include-type = ["aws_*"]
exclude-type = ["*_bucket"]
include-action = ["create", "update"]
exclude-action = ["delete"]

only-delete = false
only-create = false
only-update = false
only-replace = false

fail-on = ["delete"]
github-summary = false
sort-by = "type"
```

Supported config keys are `plan-file`, `state-file`, `format`, `output-file`, `no-emoji`, `dry-run`, `verbose`, `quiet`, `no-header`, `include-type`, `exclude-type`, `include-action`, `exclude-action`, `only-delete`, `only-create`, `only-update`, `only-replace`, `fail-on`, `github-summary`, and `sort-by`.

`format` accepts `text`, `json`, `csv`, or `table`. `sort-by` accepts `type`, `name`, or `action`. Filter lists accept exact values or glob patterns. The `only-*` keys are shorthand action filters.

Configuration keys use kebab-case TOML names, such as `plan-file` and `sort-by`, not Rust snake_case field names. CLI options override config defaults for `plan-file`, `state-file`, `output-file`, `format`, and filter lists. Boolean options are enabled when either the config value or CLI flag is true. Relative `plan-file`, `state-file`, and `output-file` values are resolved from the config file directory.

## Acknowledgements

This project is maintained by BillyBox1926 and developed with an AI-assisted workflow. Contributor credit is preserved through Git history and pull request records.
## Project management

This repo treats issues as an active project-management layer. Issue templates, mirrored dependency notes, label taxonomy, and maintainer workflow guidance keep the tracker readable for contributors and future maintainers.

- [Maintainer Workflow](docs/MAINTAINER_WORKFLOW.md) explains issue intake, triage, dependency mirroring, milestones, pull requests, and closing standards.
- [Issue Label Taxonomy](docs/ISSUE_LABELS.md) documents the label system and recommended label combinations.
- [Contributing](docs/CONTRIBUTING.md) covers local checks and collaboration expectations.
- [Roadmap](docs/ROADMAP.md) tracks completed and planned capability areas.
- [Suggestions](docs/suggestions.json) tracks lightweight maintenance notes and future-task ideas.
- [Security](SECURITY.md) covers supported versions and how to report vulnerabilities.
- [Changelog](CHANGELOG.md) tracks release history and notable changes.

## Architecture

See the canonical [Architecture Notes](docs/ARCHITECTURE.md) for system architecture, configuration flow, data flow, design decisions, and future extension points.
