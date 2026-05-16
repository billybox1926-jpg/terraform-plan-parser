# TODO — terraform-plan-parser

## High Priority

- [ ] **Add proper CLI argument parsing with `clap`**
  - Replace manual `std::env::args()` with `clap` (derive feature)
  - Add flags: `--help`, `--version`, `--format`, `--filter-action`, `--no-emoji`
  - Example usage:
    ```bash
    terraform_plan_parser --format json ./my-project
    terraform_plan_parser --filter-action create,update
    terraform_plan_parser --no-emoji
    ```

- [ ] **Add a comprehensive `.gitignore`**
  - Rust: `/target`, `**/*.rs.bk`
  - Terraform: `*.tfstate`, `*.tfstate.*`, `.terraform/`, `.terraform.lock.hcl`
  - Decide on `Cargo.lock` (keep for binaries)

- [ ] **Add GitHub Actions CI/CD**
  - `.github/workflows/ci.yml`
  - Run `cargo build`, `cargo test`, `cargo fmt --check`, `cargo clippy`
  - Trigger on push and PR to `main`

## Medium Priority

- [ ] **Add structured output formats**
  - `--format json` for CI/CD integration
  - `--format csv` for spreadsheet/reporting
  - Keep current emoji text as default

- [ ] **Add unit and integration tests**
  - Unit tests for JSON parsing logic
  - Integration tests with sample `terraform plan -json` output fixtures
  - Mock `terraform` command for testing without real Terraform

- [ ] **Add pre-commit hooks**
  - `cargo fmt`, `cargo clippy`, `cargo test`
  - Use `pre-commit` framework or simple git hooks

## Low Priority / Future

- [ ] **Support saved `.tfplan` files**
  - `--plan-file plan.tfplan` instead of live `terraform plan -json`
  - Useful for CI pipelines where plan is generated in a previous step

- [ ] **Add logging/tracing**
  - Replace `println!`/`eprintln!` with `tracing` or `log` crate
  - Configurable levels: `info`, `warn`, `error`, `debug`

- [ ] **Add filtering capabilities**
  - `--include-type aws_instance`
  - `--exclude-action read`
  - `--only-destructive` (show only delete/update)

- [ ] **Add configuration file support**
  - `.terraform-plan-parser.toml` for persistent filters and defaults

- [ ] **Consolidate architecture docs**
  - Merge `ARCHITECTURE.md` (root) and `docs/architecture.md` into one canonical doc
