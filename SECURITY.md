# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

This is a small Rust CLI tool. If you discover a security concern, we appreciate responsible disclosure.

**Preferred:** Open a [GitHub Issue](https://github.com/billybox1926-jpg/terraform-plan-parser/issues) with a clear description of the concern. For sensitive findings, you may mark the issue as a bug and avoid including exploit details in the initial report.

**What to expect:**

- We will acknowledge receipt within a reasonable timeframe.
- We will investigate and respond with a fix or explanation.
- We will credit you in the changelog if you wish.

## Scope

This project parses Terraform plan JSON output. It does not execute Terraform plans, manage infrastructure, or handle credentials directly. The primary security considerations are:

- Input parsing robustness (malformed JSON/NDJSON)
- Subprocess invocation (calls `terraform` binary when reading `.tfplan` files)
- File path handling

## Best Effort

This is a maintainer-run open-source project. We address security concerns on a best-effort basis and appreciate your patience.
