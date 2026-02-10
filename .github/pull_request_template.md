<!--
Please read CONTRIBUTING.md before submitting.
CI runs on windows-2025-vs2026 and will check fmt, clippy, and tests.
-->

## What does this PR do?

<!-- Describe the change and why it's needed. -->

## Related issue

<!-- Link the issue this addresses, e.g. "Closes #123". Leave empty if none. -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactoring (no behavior change)
- [ ] Documentation
- [ ] CI / tooling

## Checklist

- [ ] `cargo fmt --all` (no diffs)
- [ ] `cargo clippy --all -- -D warnings` (zero warnings)
- [ ] `cargo nextest run` or `cargo test --all` (all pass)
- [ ] If snapshots changed: reviewed with `cargo insta review` and committed `.snap` files
- [ ] Added or updated tests for new behavior
- [ ] Self-reviewed the diff before submitting
