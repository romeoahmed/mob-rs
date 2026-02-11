# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-06

### Fixed

- Prevent `modorganizer-modorganizer` repo name duplication when task name is exactly `"modorganizer"`
- Add missing `/` separator in `InstallerTask::git_url()` (was producing `ModOrganizer2modorganizer-Installer.git`)
- Normalize all `PathsConfig` fields via `components().collect()` to eliminate mixed `/` and `\` separators
- Skip alias names (e.g., `"super"`, `"plugins"`) in task registration — they are config override scopes, not actual buildable tasks
- Expand alias-based config overrides: `[tasks.super]` now correctly applies to all tasks matched by the `super` alias's target patterns (mirrors C++ mob's `[super:task]` semantics)
- Dispatch `translations` and `installer` to their correct task types (`TranslationsTask`, `InstallerTask`) instead of falling through to the catch-all `ModOrganizerTask` which tried to git-clone nonexistent repos
- Skip built-in task names in `register_config_tasks()` to prevent them from getting spurious `modorganizer-*` prefix aliases
- Add Transifex CLI (`tx`) to README.md prerequisites
- `InstallerTask::enabled()` now checks `task_config().enabled` instead of being hardcoded to `cfg!(windows)` — respects `[tasks.installer].enabled = false`
- `TranslationsTask::enabled()` now checks both `task_config().enabled` and `transifex.enabled` — respects `[tasks.translations].enabled = false`
- Register all ~30 default `modorganizer-*` sub-projects (matching C++ mob's `add_tasks()` in `main.cpp`) so they are cloned during `mob build`
- Build tasks in 7 sequential groups with parallel execution within each group, matching C++ mob's dependency ordering exactly

### Added

- Complete Rust port of the [C++ mob](https://github.com/ModOrganizer2/mob) build tool for ModOrganizer2
- TOML-based configuration replacing INI files, with `[global]`, `[task]`, `[cmake]`, `[tools]`, `[transifex]`, `[versions]`, and `[paths]` sections
- Per-task configuration overrides via `[tasks.<name>]` with glob pattern support (e.g. `installer_*`)
- Environment variable overrides with `MOB_` prefix mapping (e.g. `MOB_GLOBAL_DRY` -> `global.dry`)
- Layered config resolution: master TOML -> `MOBINI` env -> local `mob.toml` -> `--ini` files -> CLI `-s` flags
- `build` command with dependency-aware parallel task execution via tokio `JoinSet`
- `list` command to display available tasks with tree view (`--all`) and alias listing (`--aliases`)
- `options` command to dump resolved configuration after all overrides
- `release devbuild` command to create development build archives (binaries, PDBs, sources)
- `release official` command to create official releases from a specific branch
- `git set-remotes` command to configure upstream/origin remotes across all repos
- `git add-remote` command to add a new remote to all repos
- `git ignore-ts` command to toggle `--assume-unchanged` on `.ts` translation files
- `git branches` command to list repos not on the master branch
- `pr find` / `pr pull` / `pr revert` commands for GitHub pull request management
- `tx get` command to initialize Transifex projects and pull translations
- `tx build` command to compile `.ts` translation files into `.qm` via lrelease
- `cmake-config` command to print `CMAKE_PREFIX_PATH` and `CMAKE_INSTALL_PREFIX` for standalone cmake usage
- `inis` command to display config file loading order and priority
- Dual git backend: `gix` (native Rust) for read operations with shell fallback for mutations
- Async I/O via tokio with `JoinSet` for parallel task builds and graceful abort on failure
- HTTP downloads via reqwest, replacing the libcurl/vcpkg dependency
- Visual Studio discovery via `vswhere` and VS environment capture via PowerShell `Enter-VsDevShell`
- Configurable external tool paths (`7z`, `cmake`, `msbuild`, `lrelease`, `tx`, `ISCC`) via `[tools]`
- Structured logging with `tracing` (stdout + file, configurable log levels 0-6)
- Zero-panic error handling with `anyhow` contexts and `thiserror` for library errors
- Deterministic iteration using `BTreeMap` (no `HashMap`)
- Memory allocator override with `mimalloc` v3
- Release profile with thin LTO, symbol stripping, and abort-on-panic
- 367 snapshot tests with `insta` and `nextest`

### Changed

- Config format from INI (`.ini`) to TOML (`.toml`)
- Per-task override syntax from `[task_name:task]` to `[tasks.task_name]`
- Build dependencies from vcpkg + C++ toolchain to Rust toolchain only
- HTTP backend from libcurl (requires vcpkg) to reqwest (native Rust)
- Concurrency model from OS threads to tokio async tasks

### Removed

- `third-party/bin/` directory (embedded `nuget.exe`, `perl`)
- Direct `python.exe` invocation (Python is still required at MO2 build time but is no longer bundled or invoked by mob itself)

[unreleased]: https://github.com/romeoahmed/mob-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/romeoahmed/mob-rs/releases/tag/v0.1.0
