# terraform-plan-parser

A lightweight Rust CLI tool that parses Terraform plan JSON output and displays a clean, human-readable summary of planned resource changes.

## Features

- **Colorful summary** — see at a glance what's being created (➕), updated (🔄), deleted (➖), or read (📖)
- **Directory-aware** — point it at any Terraform project directory to run `terraform plan -json`
- **Plan-file aware** — parse pre-generated NDJSON/full JSON plan files with `--plan-file`, or inspect saved `.tfplan` files through `terraform show -json`
- **Zero config** — just run it
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

## Architecture

See [Architecture Notes](docs/architecture.md) for system architecture, data flow, design decisions, and future extension points.
