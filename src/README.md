# Terraform Plan Parser

A lightweight Rust CLI tool that runs `terraform plan -json` in any Terraform project and displays a clean summary of planned resource changes.

## Features

- Accepts a target directory as a command‑line argument (defaults to current directory).
- Validates directory existence – prints a clear error instead of panicking.
- Displays resource type, name, and action (`create`, `update`, `delete`, `read`) with emoji symbols.
- Supports `--help` flag.
- Returns exit code `0` on success, `1` on error.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021 or later)
- [Terraform](https://developer.hashicorp.com/terraform/downloads) (v1.3+ recommended)

### Build from source

```bash
git clone https://github.com/yourusername/terraform-plan-parser.git
cd terraform-plan-parser
cargo build --release
