# Contributing to mob-rs

Thank you for your interest in contributing to mob-rs! This document covers the development workflow, code standards, and submission process.

## Getting Started

### Prerequisites

- **Rust 1.92.0+** via [rustup](https://rustup.rs)
- **Git**
- **Windows** (mob-rs targets Windows exclusively)

Optional (for running mob itself, not for developing it):

- Visual Studio 2022 with C++ workload
- Qt (see [README](README.md#prerequisites) for details)

### Setup

```powershell
git clone https://github.com/romeoahmed/mob-rs
cd mob-rs
cargo build
```

### Verify your setup

```powershell
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
cargo nextest run          # preferred
cargo test --all           # also works (CI uses this)
```

## Development Workflow

### Branch Strategy

1. Fork the repository
2. Create a feature branch from `main`: `git checkout -b my-feature`
3. Make your changes, committing along the way
4. Push to your fork: `git push origin my-feature`
5. Open a pull request against `main`

### Before Submitting a PR

Run these checks locally — CI enforces all of them:

```powershell
cargo fmt --all                      # format code
cargo clippy --all -- -D warnings    # zero warnings required
cargo nextest run                    # run all tests
```

If your changes affect snapshot tests:

```powershell
cargo insta review    # inspect snapshot diffs
```

Accept only intentional changes and commit the updated `.snap` files.

## Code Standards

This project enforces strict coding conventions documented in [AGENTS.md](AGENTS.md). Key rules:

### Zero Tolerance

| Rule            | Forbidden                      | Required                           |
| --------------- | ------------------------------ | ---------------------------------- |
| Zero Panic      | `.unwrap()`, `.expect()`       | `.with_context(\|\| format!(...))` |
| Determinism     | `HashMap`                      | `BTreeMap`, `IndexMap`             |
| No Placeholders | `todo!()`, `println!("debug")` | Functional code only               |
| No Suppressions | `#[allow(...)]`                | Fix the underlying issue           |

### API Design

- **Private by default** — start with no `pub`, widen only when needed
- **Opaque types** — hide fields, expose behavior through methods (`field()` getter, `with_field()` builder)
- **Honest signatures** — don't wrap infallible functions in `Result`, don't wrap always-present values in `Option`
- **No re-exports** — import from the exact source module, never `pub use` in `mod.rs`

### Error Handling

- Application code: `anyhow` with `.with_context(|| format!(...))`
- Library boundaries: `thiserror` with `#[derive(Error)]`
- Never use bare `?` without context on fallible operations
- Never panic in `Drop` implementations

### Style

- `cargo fmt` with default settings (no `.rustfmt.toml`)
- Clippy with `all`, `pedantic`, and `nursery` lints enabled (configured in `Cargo.toml`)
- Prefer enums over boolean flags
- Prefer `let-else` over nested `if let`
- Use `flume::bounded` channels, never unbounded

## Testing

### Structure

- **Unit tests**: `mod.rs` + separate `tests.rs` per module
- **Integration tests**: `tests/integration_*.rs`
- **Snapshots**: `tests/snapshots/*.snap` using [insta](https://insta.rs)

### Running Tests

```powershell
cargo nextest run                              # all tests (preferred)
cargo nextest run -E 'test(config)'            # filter by name
cargo test --all                               # standard runner (CI)
cargo test --doc                               # doc tests only
```

### Snapshot Tests

We use `insta` for snapshot testing. When a snapshot changes:

1. Run the failing test — it will show the diff
2. Review with `cargo insta review`
3. Accept intentional changes, reject regressions
4. Commit updated `.snap` files alongside your code changes

### Writing Tests

- Use `tempfile` for filesystem isolation
- Use `wiremock` for HTTP mocking
- Keep unit tests next to the code (in `tests.rs` or `mod tests`)
- Never put `#[cfg(test)]` helpers in non-test files — move them into `tests.rs` or `mod tests {}`

## Project Layout

```plain
src/
  main.rs             # entry point
  cli/                # clap argument parsing
  cmd/                # command implementations (build, release, git, pr, tx)
  config/             # TOML config loading, validation, path resolution
  core/               # env, process, job objects, VS detection
  error/              # error types
  git/                # git operations (gix + shell backends)
  logging/            # tracing setup
  net.rs              # HTTP downloader
  task/               # task system (manager, registry, tools, task implementations)
  utility/            # filesystem, encoding helpers
tests/
  integration_*.rs    # integration tests
  snapshots/          # insta snapshot files
```

## CI

CI runs on `windows-2025-vs2026` and checks:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all -- -D warnings`
3. `cargo test --all`

All three must pass for a PR to be merged.

## Reporting Issues

Please use the [issue templates](https://github.com/romeoahmed/mob-rs/issues/new/choose) to report bugs or request features. Include:

- Your Rust version (`rustc --version`)
- Your `mob` version or commit hash
- Steps to reproduce (for bugs)
- Expected vs actual behavior

## License

By contributing, you agree that your contributions will be licensed under [GPL-3.0-or-later](LICENSE).
