# Roadmap — terraform-plan-parser

This roadmap summarizes completed foundations, current maintenance priorities, near-term improvements, and longer-term expansion ideas. GitHub Issues remain the source of truth for active scoped work, acceptance criteria, dependencies, labels, and milestones.

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
  - GitHub Actions workflow builds Linux, macOS, and Windows release artifacts with SHA256 checksums.
  - Tracked by #24.
- [x] Expand release support beyond Intel-only builds
  - Release artifacts now cover x86_64 and ARM64 targets for Linux and macOS, plus native Windows x64.
  - Tracked by #102.
- [x] Add Homebrew formula support
  - `brew install billybox1926-jpg/tap/terraform-plan-parser` installs on macOS and Linux.
  - Formula maintained in [homebrew-tap](https://github.com/billybox1926-jpg/homebrew-tap) repository.
  - Tracked by #25.
- [x] Add Windows Scoop support
  - Native Windows x64 installs can use the repo-hosted Scoop manifest.
  - Tracked by #107.
- [x] Fix Windows test harness mock resolution
  - Mock tests now use mock-only PATH to prevent system terraform.exe from being found before mock terraform.bat.
  - Tracked by #100.
- [x] Add cross-platform CI test jobs
  - CI coverage includes Windows and macOS in addition to Ubuntu for portability-sensitive CLI behavior.
  - Tracked by #104.
- [x] Implement plan diffing between two plan files
  - `--compare` flag shows added, removed, and changed resources between two plans.
  - Supports all output formats (text, JSON, CSV, table).
  - Tracked by #26.
- [x] Add Terraform state JSON inventory parsing
  - `--state` and `--state-json` render local Terraform state JSON as inventory rows.
  - Tracked by #27.
- [x] Add security policy
  - `SECURITY.md` with supported versions and responsible disclosure guidance.
  - Tracked by #105.
- [x] Add changelog
  - `CHANGELOG.md` tracks release history and notable changes.
  - Tracked by #106.
- [x] Add README project visual
  - `docs/assets/project-visual.svg` gives the README a lightweight repo-native visual.
  - Tracked by #131.

## Next

- [ ] Add funding metadata after Sponsors setup is approved (#133)
  - Add `.github/FUNDING.yml` only after the GitHub Sponsors profile is live.
  - Keep GitHub Releases as the official source for binaries and checksums.

## Later

These are larger expansion ideas that should become GitHub Issues before implementation.

- [ ] Add additional selectors
  - Support resource name, address, module path, or provider filters.
- [ ] Add config helper commands
  - Provide config generation or validation commands for `.terraform-plan-parser.toml`.
- [ ] Validate Terraform version compatibility
  - Detect unsupported Terraform versions before live plan/show execution.
- [ ] Add CI policy presets
  - Provide named guardrail presets on top of the existing `--fail-on` flag.
