# Roadmap — terraform-plan-parser

This roadmap summarizes completed foundations, current release-readiness priorities, near-term improvements, and longer-term expansion ideas. GitHub Issues remain the source of truth for active scoped work, acceptance criteria, dependencies, labels, and milestones.

## Completed foundations

These items form the current stable base of the project.

- [x] Add proper CLI argument parsing with clap
  - Replaced manual `std::env::args()` handling with clap derive parsing.
  - Added flags for format selection, dry runs, verbosity, filters, `--fail-on`, shell completions, and plan-file input.
- [x] Add a comprehensive `.gitignore`
  - Covers Rust build output, Terraform local state/artifact files, and OS files.
- [x] Add GitHub Actions CI/CD
  - Runs formatting, clippy, tests, build, and hidden/bidirectional Unicode scanning on pushes and pull requests.
- [x] Add structured output formats
  - Supports text, JSON, CSV, and table output for local review and CI/reporting workflows.
- [x] Add unit and integration tests
  - Covers parsing behavior and CLI flows that can run without a real Terraform project.
- [x] Support saved `.tfplan` files
  - Converts saved Terraform plans through `terraform show -json` when needed.
- [x] Add logging/tracing
  - Uses tracing for diagnostics while preserving clean stdout for machine-readable formats.
- [x] Add filtering capabilities
  - Supports include/exclude filters for resource types and actions with glob matching.
- [x] Add configuration file support
  - Supports `.terraform-plan-parser.toml` for reusable defaults.
- [x] Add CI guardrails
  - Supports `--fail-on` so pipelines can fail on blocked actions after filtering.
- [x] Add shell completions
  - Generates completion scripts for bash, elvish, fish, PowerShell, and zsh.
- [x] Consolidate documentation under `docs/`
  - Keeps the root README as the public landing page and stores support docs under `docs/`, including `docs/ARCHITECTURE.md` as the canonical architecture document.
- [x] Add `--output-file` support
  - Write rendered output to a file for CI artifact workflows.
  - Tracked by #17.
- [x] Split the single-file CLI into focused modules
  - `cli.rs`, `parser.rs`, `renderer.rs`, `terraform.rs` — each module owns one concern.
- [x] Fix replacement summary-count edge case
  - Terraform `create/delete` replacements are counted consistently with `delete/create` replacements.
  - Tracked by #82.
- [x] Complete configuration documentation
  - Documented every supported `.terraform-plan-parser.toml` key with a copy/pasteable example config file.
  - Tracked by #74.
- [x] Expand contributor onboarding
  - Added a first-time contributor quickstart and kept local check instructions aligned with CI.
  - Tracked by #83.
- [x] Align GitHub Wiki with canonical repository docs
  - Wiki acts as an operations/navigation surface that links back to README and `docs/`.
  - Tracked by #79.
- [x] Add cross-platform release binaries
  - GitHub Actions workflow builds Linux (tar.gz), macOS (tar.gz), and Windows (zip) release artifacts with SHA256 checksums.
  - Tracked by #24.
- [x] Add Homebrew formula support
  - `brew install billybox1926-jpg/tap/terraform-plan-parser` installs on macOS and Linux.
  - Formula lives in `homebrew/terraform-plan-parser.rb`, sourced from release artifacts.
  - Tracked by #25.

## Next

These items become stronger candidates now that the release-readiness foundation is solid.

## Later

These are larger expansion ideas that need stable foundations first.

- [ ] Implement plan diffing between two plan files
  - Compare two parsed plans and render added, removed, or changed resources.
  - Tracked by #26.
- [ ] Add support for parsing Terraform state files
  - Build toward inventory, drift-analysis, and reporting workflows.
  - Tracked by #27.
- [ ] Add additional selectors
  - Support resource name, address, module path, or provider filters.
- [ ] Add config helper commands
  - Provide config generation or validation commands for `.terraform-plan-parser.toml`.
- [ ] Validate Terraform version compatibility
  - Detect unsupported Terraform versions before live plan/show execution.
- [ ] Add CI policy presets
  - Provide named guardrail presets on top of the existing `--fail-on` flag.
