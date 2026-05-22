# Contributing

Thanks for your interest in `terraform-plan-parser`. This document covers the small handful of conventions that are easy to miss, and helps first-time contributors get started quickly.

## First-Time Contributor Quickstart

1. **Clone the repository**
```bash
git clone https://github.com/billybox1926-jpg/terraform-plan-parser.git
cd terraform-plan-parser
```

2. **Build and run checks**
```bash
cargo fmt --check      # Ensure formatting is correct
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo test             # Run unit/integration tests
cargo build            # Verify the project builds
```

3. **Follow Unicode policy**
- Do not include hidden or bidirectional Unicode characters in source files.
- If needed for a test fixture, place them in `tests/fixtures/` to avoid build failures.
- CI runs a `unicode-scan` to enforce this.

4. **Make your change**
- Make edits, add tests, or update docs as needed.
- Commit changes using conventional commit messages (e.g., `feat:`, `fix:`, `docs:`).

5. **Push and open a Pull Request**
- Push to a feature branch.
- Open a PR referencing relevant issues.
- CI will automatically run checks; fix any issues before requesting review.

## Development Checks

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build`

The CI workflow runs each of these on every push and pull request.

## Release build notes

The release workflow builds Linux, macOS, and Windows artifacts from tag pushes. Linux ARM64 uses the `aarch64-unknown-linux-gnu` Rust target and installs `gcc-aarch64-linux-gnu` on the Ubuntu runner.

When changing release workflow logic, keep the Linux ARM64 build step wired to the matching cross-linker:

```yaml
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc
```

That target-specific Cargo environment variable avoids relying on host linker defaults during cross-compilation.

## No hidden or bidirectional Unicode characters

CI rejects source files that contain characters from any of these ranges:

- `U+202A`..`U+202E` (bidirectional embedding/override controls)
- `U+2066`..`U+2069` (bidirectional isolate controls)
- `U+200B`..`U+200F` (zero-width spaces, ZWNJ, ZWJ, LRM, RLM)
- `U+FEFF` (byte order mark)

These can be used to make code render differently than it compiles ([Trojan Source][trojan]). Keep source files plain UTF-8 without invisible control characters. If you genuinely need one in a test fixture, isolate it in a non-scanned directory (for example, `tests/fixtures/`) so it does not affect the build.

The check is the `unicode-scan` job in `.github/workflows/ci.yml`.

[trojan]: https://trojansource.codes/
