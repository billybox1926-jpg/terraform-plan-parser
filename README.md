# terraform-plan-parser

A lightweight Rust CLI tool that parses `terraform plan -json` output and displays a clean, human-readable summary of planned resource changes.

## Features

- **Colorful summary** — see at a glance what's being created (➕), updated (🔄), deleted (➖), or read (📖)
- **Directory-aware** — point it at any Terraform project directory
- **Zero config** — just run it
- **Cross-platform** — works on Windows, macOS, and Linux

## Prerequisites

- [Rust](https://rustup.rs/) (to build from source)
- [Terraform](https://developer.hashicorp.com/terraform/downloads) (must be in your `PATH`)

## Installation

```bash
# Clone the repo
git clone https://github.com/billybox1926-jpg/terraform-plan-parser.git
cd terraform-plan-parser

# Build and install
cargo install --path .
```

## Architecture

See [Architecture Notes](docs/architecture.md) for system architecture, data flow, design decisions, and future extension points.
