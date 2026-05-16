# TODO — terraform-plan-parser

## High Priority

- [x] Add proper CLI argument parsing with clap
  - Replaced manual `std::env::args()` with clap derive parsing.
  - Added flags: `--format`, `--dry-run`, `--verbose`, `--filter-action`, and `--plan-file`.
- [x] Add a comprehensive `.gitignore`
  - Rust: `/target`, `**/*.rs.bk`.
  - Terraform: `*.tfstate`, `*.tfstate.*`, `.terraform/`, `.terraform.lock.hcl`.
- [x] Add GitHub Actions CI/CD
  - `.github/workflows/ci.yml`.
  - Run `cargo build`, `cargo test`, `cargo fmt --check`, and `cargo clippy`.
  - Trigger on push and PR to `main`.

## Medium Priority

- [x] Add structured output formats
  - `--format json` for CI/CD integration.
  - `--format csv` for spreadsheet/reporting.
  - Keep current emoji text as default table format.
- [x] Add unit and integration tests
  - Unit tests for JSON parsing logic.
  - Integration tests with `--dry-run` and `--plan-file` to avoid needing real Terraform.

## Low Priority / Future

- [x] Support saved `.tfplan` files
  - `--plan-file plan.tfplan` instead of live `terraform plan -json`.
  - Useful for CI pipelines where plan is generated in a previous step.
- [x] Add logging/tracing
  - Replaced `println!`/`eprintln!` diagnostics with tracing output.
  - Configurable via `--verbose` flag for debug-level output.
- [x] Add filtering capabilities
  - `--filter-type` with glob support, for example `aws_*`.
  - `--filter-action`, for example `create` or `delete`.
- [x] Add configuration file support
  - `.terraform-plan-parser.toml` for persistent filters and defaults.
- [x] Consolidate architecture docs
  - Merged `ARCHITECTURE.md` and `docs/architecture.md` into root `ARCHITECTURE.md` as the canonical doc.
