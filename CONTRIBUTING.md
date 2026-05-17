# Contributing

Thanks for your interest in `terraform-plan-parser`. This document covers the
small handful of conventions that are easy to miss.

## Development

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo build`

The CI workflow runs each of these on every push and pull request.

## No hidden or bidirectional Unicode characters

CI rejects source files that contain characters from any of these ranges:

- `U+202A`..`U+202E` (bidirectional embedding/override controls)
- `U+2066`..`U+2069` (bidirectional isolate controls)
- `U+200B`..`U+200F` (zero-width spaces, ZWNJ, ZWJ, LRM, RLM)
- `U+FEFF` (byte order mark)

These can be used to make code render differently than it compiles
([Trojan Source][trojan]). Keep source files plain UTF-8 without invisible
control characters. If you genuinely need one in a test fixture, isolate it in
a non-scanned directory (for example, `tests/fixtures/`) so it does not affect
the build.

The check is the `unicode-scan` job in `.github/workflows/ci.yml`.

[trojan]: https://trojansource.codes/
