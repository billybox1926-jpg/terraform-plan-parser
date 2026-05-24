# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

### Added

- **Scoop manifest** — added `scoop/terraform-plan-parser.json` so Windows users can install the existing `v0.1.3` Windows x64 release ZIP through Scoop (#107)

### Changed

- **Windows install docs** — documented direct Scoop install instructions in `README.md` while keeping Chocolatey and winget deferred for later evaluation (#107)
- **Acknowledgements** — added a concise README acknowledgement for the maintainer and AI-assisted workflow (#124)

## [0.1.3] — 2026-05-22

### Changed

- **Package version metadata** — aligned `Cargo.toml` package version with the current release tag at `0.1.3` (#120)

### Distribution

- Release v0.1.3 published at https://github.com/billybox1926-jpg/terraform-plan-parser/releases/tag/v0.1.3
- Release assets continue to include Linux, macOS, Windows, and `SHA256SUMS` artifacts.

## [0.1.2] — 2026-05-22

### Fixed

- **macOS Intel release runner** — updated the macOS x64 release job to use a supported Intel runner instead of the stale `macos-13` runner (#118)

### Distribution

- Release v0.1.2 validated the expanded release artifact path after the macOS runner fix.

## [0.1.1] — 2026-05-22

### Distribution

- Release validation tag used while proving the expanded cross-platform artifact workflow.
- Superseded by v0.1.2 after the macOS x64 runner issue was fixed.

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
