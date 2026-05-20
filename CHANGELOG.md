# Changelog

All notable changes to this project are documented in this file.

## [0.1.0] — 2026-05-20

### Added

- **Plan comparison** — `--compare` flag to diff two Terraform plan files and show added, removed, and changed resources (#26)
- **Cross-platform release binaries** — GitHub Actions workflow builds Linux (tar.gz), macOS (tar.gz), and Windows (zip) release artifacts with SHA256 checksums (#24)
- **Homebrew formula** — `brew install billybox1926-jpg/tap/terraform-plan-parser` for macOS and Linux (Intel) (#25)
- **Output file support** — `--output-file` flag to write rendered output to a file (#17)
- **GitHub Actions CI** — fmt, clippy, test, build, and Unicode scan jobs on push/PR (#96)
- **Module split** — refactored `src/main.rs` into `cli.rs`, `parser.rs`, `renderer.rs`, `terraform.rs` (#98)
- **Security policy** — `SECURITY.md` with supported versions and responsible disclosure guidance (#105)

### Changed

- **Roadmap synced** — moved all completed items out of "Current priorities" and "Next" sections
- **Windows test harness** — fixed mock terraform resolution by using mock-only PATH (#100)
- **Homebrew tap** — formula moved to dedicated `billybox1926-jpg/homebrew-tap` repository
- **README updated** — Homebrew install instructions, Windows native install, plan comparison docs

### Fixed

- Replacement summary-count edge case for `create/delete` vs `delete/create` (#82)
- Windows terraform command lookup via `cmd /c` (#92)
- Unicode scan for hidden/bidirectional characters in CI (#78)

### Distribution

- Release v0.1.0 published at https://github.com/billybox1926-jpg/terraform-plan-parser/releases/tag/v0.1.0
- Homebrew tap at https://github.com/billybox1926-jpg/homebrew-tap
