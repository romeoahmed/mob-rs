# `mob` — ModOrganizer Build Tool (Rust)

A Rust port of the [C++ mob](https://github.com/ModOrganizer2/mob) build automation tool for the [ModOrganizer2](https://github.com/ModOrganizer2/modorganizer) project.

If you want to contribute to a single plugin of MO2 or even write your own plugins, check the [ModOrganizer2 Wiki](https://github.com/ModOrganizer2/modorganizer/wiki).

## Table of contents

- [Quick start](#quick-start)
  - [Building from source](#building-from-source)
- [Prerequisites](#prerequisites)
  - [Qt installation](#qt-installation)
  - [Visual Studio](#visual-studio)
  - [Python](#python)
  - [7-Zip](#7-zip)
  - [Transifex CLI (optional)](#transifex-cli-optional)
- [Changing options](#changing-options)
  - [TOML files](#override-options-using-toml-files)
  - [Environment variables](#override-options-using-environment-variables)
  - [Command line](#override-options-using-command-line)
  - [TOML format](#toml-format)
- [Options](#options)
  - [`[global]`](#global)
  - [`[task]`](#task)
  - [`[cmake]`](#cmake)
  - [`[tools]`](#tools)
  - [`[transifex]`](#transifex)
  - [`[versions]`](#versions)
  - [`[paths]`](#paths)
- [Command line](#command-line)
  - [Global options](#global-options)
  - [`build`](#build)
  - [`list`](#list)
  - [`options`](#options-1)
  - [`release`](#release)
  - [`git`](#git)
  - [`pr`](#pr)
  - [`tx`](#tx)
  - [`cmake-config`](#cmake-config)
  - [`inis`](#inis)

## Quick start

Download the latest release from [GitHub Releases](https://github.com/romeoahmed/mob-rs/releases), extract `mob.exe` and `mob.toml` into the same directory, then:

```powershell
mob -d C:\dev\modorganizer build
```

### Building from source

```powershell
# install Rust 1.92.0+ (https://rustup.rs)
rustup install stable

# clone and build mob-rs
git clone https://github.com/romeoahmed/mob-rs
cd mob-rs
cargo build --release

# build MO2
./target/release/mob -d C:\dev\modorganizer build
```

## Prerequisites

### Qt installation

Check `mob.toml` to find out which version of Qt you need.

#### CLI based install using [aqt](https://github.com/miurahr/aqtinstall)

[aqt](https://github.com/miurahr/aqtinstall) is a CLI installer for Qt. It makes installing Qt extremely quick and painless, and doesn't require a login. Check the [documentation](https://aqtinstall.readthedocs.io/en/latest/installation.html) to install **aqt** itself.

When using **aqt**, you can choose which modules to install but we recommend installing all of them in case of changes.

```powershell
# you can also use -m all to install all modules
aqt install-qt --outputdir "C:\Qt" windows desktop ${QT_VERSION} win64_msvc2022_64 `
  -m qtwebengine qtimageformats qtpositioning qtserialport qtwebchannel qtwebsockets
```

#### Manual installation

- Install Qt ([Installer](https://download.qt.io/official_releases/online_installers/qt-unified-windows-x64-online.exe)) and select these components:
  - MSVC 2022 64-bit
  - Additional Libraries:
    - Qt WebEngine (display nexus pages)
    - Qt Image Formats (display images in image tab and preview)
    - Qt Positioning (required by QtWebEngine)
    - Qt Serial Port (required by Qt Core)
    - Qt WebChannel (required by QtWebEngine)
    - Qt WebSockets (Nexus api/download)
  - Optional:
    - Qt Source Files
    - Qt Debug Files

### Visual Studio

- Install Visual Studio 2022 ([Installer](https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=Community&channel=Release&version=VS2022&source=VSLandingPage&cid=2030&passive=false))

  - Desktop development with C++
  - Desktop .NET desktop development (needed by OMOD and FOMOD installers)
  - Individual Components:
    - .Net Framework 4.8 SDK
    - .Net Framework 4.7.2 targeting pack
    - Windows Universal C Runtime
    - C++ ATL for latest v143 build Tools (x86 & x64)
    - C++ /CLI support for v143 build Tools (Latest)
    - Windows 11 SDK (get latest)
    - C++ Build Tools core features
    - Git for Windows
    - CMake tools for Windows

### Python

- Install [Python 3.12](https://www.python.org/downloads/) (required by PyQt, sip, and other MO2 build dependencies)
  - Make sure to check **Add python.exe to PATH** during installation

### 7-Zip

- Install [7-Zip](https://www.7-zip.org/) (used to extract and create archives)
  - Make sure `7z.exe` is in your PATH, or set the path in `mob.toml` under `[tools]`

### Transifex CLI (optional)

- Install the [Transifex CLI](https://github.com/transifex/cli) (`tx`) if you need to pull translations
  - Make sure `tx` is in your PATH, or set the path in `mob.toml` under `[tools]`
  - Set the `TX_TOKEN` environment variable or configure `transifex.key` in `mob.toml`

## Changing options

`mob` has three ways of setting options: TOML files, environment variables, or the command line.

### Override options using TOML files

`mob` builds a list of available TOML files in order of priority. Higher numbers override lower numbers:

1. The master TOML `mob.toml` in the directory where `mob.exe` lives (required).
2. Any files set in `MOBINI` (separated by semicolons).
3. Another `mob.toml` in the current directory.
4. Files given with `--ini`.

Use `mob inis` to see the list of config files in order. If `--no-default-inis` is given, `mob` will skip 1) and 2). The first config file it finds after that is considered the master.

### Override options using environment variables

Environment variables prefixed with `MOB_` are mapped to configuration keys using `_` as separator:

```powershell
$env:MOB_GLOBAL_DRY = "true"          # → global.dry = true
$env:MOB_PATHS_PREFIX = "C:\dev\mo2"  # → paths.prefix = "C:\dev\mo2"
$env:MOB_TASK_MO_ORG = "MyOrg"        # → task.mo_org = "MyOrg"
```

Environment variables override TOML files but are overridden by CLI options.

### Override options using command line

Any option can be overridden from the command line with `-s task:section/key=value`, where `task:` is optional. Some options have shortcuts, such as `--dry` for `-s global/dry=true` and `-l5` for `-s global/output_log_level=5`. See `mob options` for the list of options.

### TOML format

`mob` uses [TOML](https://toml.io) for configuration (the C++ version uses INI files). Inside the TOML file are `[sections]` and `key = value` pairs. The `[task]` section is special because it can be overridden for specific tasks using `[tasks.<name>]`. Any value under a `[tasks.usvfs]` table will only apply to the task named `usvfs`. Glob patterns like `installer_*` are also supported.

```toml
[task]
git_shallow = true

[tasks.usvfs]
git_shallow = false # override for usvfs only
```

The list of available tasks can be seen with `mob list`. See [Task names](#task-names).

## Options

### `[global]`

| Option               | Type | Description                                                                                                                                                                                      |
| -------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `dry`                | bool | Whether filesystem operations are simulated. Note that many operations will fail and that the build process will most probably not complete. This is mostly useful to get a dump of the options. |
| `redownload`         | bool | For `build`, re-downloads archives even if they already exist.                                                                                                                                   |
| `reextract`          | bool | For `build`, re-extracts archives even if the target directory already exists, in which case it is deleted first.                                                                                |
| `output_log_level`   | 0–6  | The log level for stdout: 0=silent, 1=errors, 2=warnings, 3=info (default), 4=debug, 5=trace, 6=dump.                                                                                            |
| `file_log_level`     | 0–6  | The log level for the log file. Default: 5 (trace).                                                                                                                                              |
| `log_file`           | path | The path to a log file. Default: `mob.log`.                                                                                                                                                      |
| `ignore_uncommitted` | bool | When `--redownload` or `--reextract` is given, directories controlled by git will be deleted even if they contain uncommitted changes.                                                           |

### `[task]`

Options for individual tasks. Can be overridden per-task via `[tasks.<name>]`, where `<name>` is the name of a task (see `mob list`), `super` for all MO tasks, or a glob like `installer_*`.

| Option           | Type   | Description                                                                                                                                                                             |
| ---------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `enabled`        | bool   | Whether this task is enabled. Disabled tasks are never built. When specifying task names with `mob build task1 task2...`, all tasks except those given are turned off. Default: `true`. |
| `mo_org`         | string | The organisation name when pulling from GitHub. Only applies to ModOrganizer projects. Default: `"ModOrganizer2"`.                                                                      |
| `mo_branch`      | string | The branch name when pulling from GitHub. Default: `"master"`.                                                                                                                          |
| `mo_fallback`    | string | The fallback branch if `mo_branch` doesn't exist. Empty by default (disabled).                                                                                                          |
| `no_pull`        | bool   | If a repo is already cloned, a `git pull` will be done on it every time `mob build` is run. Set to `true` to never pull.                                                                |
| `configuration`  | enum   | Which configuration to build: `Debug`, `Release`, or `RelWithDebInfo`. Default: `RelWithDebInfo`.                                                                                       |
| `git_url_prefix` | string | The URL prefix for cloning repos. Default: `"https://github.com/"`.                                                                                                                     |
| `git_shallow`    | bool   | When true, clones with `--depth 1`. Default: `true`.                                                                                                                                    |

#### Remote setup

| Option                       | Type   | Description                                                                                   |
| ---------------------------- | ------ | --------------------------------------------------------------------------------------------- |
| `remote_org`                 | string | GitHub organisation for the new origin remote. The URL will be `git@github.com:org/repo.git`. |
| `remote_no_push_upstream`    | bool   | Sets the push URL for `upstream` to `nopushurl` to avoid accidental pushes.                   |
| `remote_push_default_origin` | bool   | Sets `origin` as the default push remote.                                                     |

Example per-task override:

```toml
[tasks.super]
no_pull = true
remote_org = "myuser"
remote_no_push_upstream = true
remote_push_default_origin = true
```

### `[cmake]`

| Option            | Type   | Description                                                                        |
| ----------------- | ------ | ---------------------------------------------------------------------------------- |
| `install_message` | enum   | Value for `CMAKE_INSTALL_MESSAGE`: `always`, `lazy`, or `never`. Default: `never`. |
| `host`            | string | Toolset host configuration (`-T host=XXX`).                                        |

### `[tools]`

Paths to external tools. These are looked up in `PATH` if not absolute.

| Option     | Type | Default        |
| ---------- | ---- | -------------- |
| `7z`       | path | `7z.exe`       |
| `cmake`    | path | `cmake.exe`    |
| `msbuild`  | path | `msbuild.exe`  |
| `tx`       | path | `tx.exe`       |
| `lrelease` | path | `lrelease.exe` |
| `iscc`     | path | `ISCC.exe`     |

### `[transifex]`

| Option    | Type   | Description                                                       |
| --------- | ------ | ----------------------------------------------------------------- |
| `enabled` | bool   | Whether Transifex integration is enabled. Default: `true`.        |
| `key`     | string | Transifex API key.                                                |
| `team`    | string | Team slug. Default: `"mod-organizer-2-team"`.                     |
| `project` | string | Project slug. Default: `"mod-organizer-2"`.                       |
| `url`     | string | Transifex API URL. Default: `"https://app.transifex.com"`.        |
| `minimum` | u8     | Minimum translation completion percentage (0–100). Default: `60`. |

### `[versions]`

| Option       | Type   | Description                                       |
| ------------ | ------ | ------------------------------------------------- |
| `vs_toolset` | string | Visual Studio toolset version. Default: `"14.3"`. |
| `sdk`        | string | Windows SDK version. Default: `"10.0.26100.0"`.   |
| `usvfs`      | string | USVFS version/branch. Default: `"master"`.        |
| `explorerpp` | string | Explorer++ version. Default: `"1.4.0"`.           |

Stylesheet versions are flattened into this section:

```toml
[versions]
ss_paper_lad_6788 = "7.2"
ss_paper_automata_6788 = "3.2"
ss_dark_mode_1809_6788 = "3.0"
```

### `[paths]`

The only path that's required is `prefix`, which is where `mob` will put everything. Within this directory will be `downloads/`, `build/`, and `install/`. Everything else is derived from it.

| Option                 | Type | Description                                                         |
| ---------------------- | ---- | ------------------------------------------------------------------- |
| `prefix`               | path | Main build prefix (required). All other paths are relative to this. |
| `cache`                | path | Download cache directory. Default: `prefix/downloads`.              |
| `build`                | path | Build directory. Default: `prefix/build`.                           |
| `install`              | path | Installation root. Default: `prefix/install`.                       |
| `install_bin`          | path | Binary output. Default: `install/bin`.                              |
| `install_installer`    | path | Installer output. Default: `install/installer`.                     |
| `install_libs`         | path | Library output. Default: `install/lib`.                             |
| `install_pdbs`         | path | PDB output. Default: `install/pdb`.                                 |
| `install_stylesheets`  | path | Stylesheets. Default: `install_bin/stylesheets`.                    |
| `install_licenses`     | path | Licenses. Default: `install_bin/licenses`.                          |
| `install_translations` | path | Translations. Default: `install_bin/translations`.                  |
| `vcpkg`                | path | vcpkg installation path.                                            |
| `qt_install`           | path | Qt installation directory (containing `bin/`, `include/`, etc.).    |
| `qt_bin`               | path | Qt bin directory. Default: `qt_install/bin`.                        |
| `qt_translations`      | path | Qt translations. Default: `qt_install/translations`.                |

## Command line

Do `mob --help` for global options and the list of available commands. Do `mob <command> --help` for more help about a command.

### Global options

| Option                | Description                                                               |
| --------------------- | ------------------------------------------------------------------------- |
| `--ini`, `-i`         | Adds a TOML configuration file. Can be specified multiple times.          |
| `--dry`               | Simulates filesystem operations.                                          |
| `--log-level`, `-l`   | The log level for stdout (0–6).                                           |
| `--file-log-level`    | The log level for the log file. Falls back to `--log-level` if not given. |
| `--log-file`          | Path to the log file.                                                     |
| `--destination`, `-d` | The build directory where `mob` will put everything.                      |
| `--set`, `-s`         | Sets an option: `-s task:section/key=value`.                              |
| `--no-default-inis`   | Does not auto detect config files, only uses `--ini`.                     |

### `build`

Builds tasks. The order in which tasks have to be built is handled by `mob`, but dependencies will not be built automatically when specifying tasks manually. That is, `mob build` will build `python` before `pyqt`, but `mob build pyqt` will not build `python`. Many tasks will be able to run in parallel, but not all, either because they hog the CPU (such as `usvfs`) or because they have dependencies that have to be built first.

If any task fails to build, all the active tasks are aborted as quickly as possible.

#### Task names

Each task has a name, some have more. MO tasks for example have a full name that corresponds to their git repo (such as `modorganizer-game_features`) and a shorter name (such as `game_features`). Both can be used interchangeably. The task name can also be `super`, which refers to all repos hosted on the Mod Organizer GitHub account. Globs can be used, like `installer_*`. See `mob list` for a list of all available tasks.

#### Options for `build`

| Option                             | Description                                                                                                                                                                                                                                       |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--redownload`, `-g`               | Re-downloads files. If a download file is found in `prefix/downloads`, it is never re-downloaded. This will delete the file and download it again.                                                                                                |
| `--reextract`, `-e`                | Deletes the source directory for a task and re-extracts archives. If the directory is controlled by git, deletes it and clones again. If git finds modifications in the directory, the operation is aborted (see `--ignore-uncommitted-changes`). |
| `--reconfigure`, `-c`              | Reconfigures the task by running cmake, configure scripts, etc.                                                                                                                                                                                   |
| `--rebuild`, `-b`                  | Cleans and rebuilds projects.                                                                                                                                                                                                                     |
| `--new`, `-n`                      | Implies all four flags above.                                                                                                                                                                                                                     |
| `--clean-task` / `--no-clean-task` | Sets whether tasks are cleaned. With `--no-clean-task`, the flags above are ignored.                                                                                                                                                              |
| `--fetch-task` / `--no-fetch-task` | Sets whether tasks are fetched. With `--no-fetch-task`, nothing is downloaded, extracted, cloned or pulled.                                                                                                                                       |
| `--build-task` / `--no-build-task` | Sets whether tasks are built. With `--no-build-task`, nothing is ever built or installed.                                                                                                                                                         |
| `--pull` / `--no-pull`             | Whether to pull repos that are already cloned.                                                                                                                                                                                                    |
| `--revert-ts` / `--no-revert-ts`   | Whether to revert `.ts` files before pulling to avoid merge conflicts.                                                                                                                                                                            |
| `--ignore-uncommitted-changes`     | With `--reextract`, ignores repos that have uncommitted changes and deletes the directory.                                                                                                                                                        |
| `--keep-msbuild`                   | Don't terminate `msbuild.exe` instances after building.                                                                                                                                                                                           |
| `<task>...`                        | List of tasks to run, see [Task names](#task-names).                                                                                                                                                                                              |

### `list`

Lists all the available task names. If a task has multiple names, they are all shown.

#### Options for `list`

| Option            | Description                                             |
| ----------------- | ------------------------------------------------------- |
| `--all`, `-a`     | Shows a task tree to see which are built in parallel.   |
| `--aliases`, `-i` | Shows only aliases.                                     |
| `<task>...`       | With `--all`, shows only the tasks that would be built. |

### `options`

Lists all the options after parsing the config files and the command line.

### `release`

Creates a release. Supports two modes: `devbuild` and `official`.

#### `release devbuild`

Creates a development build release. A release is made out of archives:

- Binaries from `prefix/install/bin`
- PDBs from `prefix/install/pdb`
- Sources from various directories in `prefix/build`

The archive filename is `Mod.Organizer-version-suffix-what.7z`, where:

- `version` is taken from `ModOrganizer.exe`, `version.rc`, or from `--version`
- `suffix` is the optional `--suffix` argument
- `what` is either nothing, `src`, or `pdbs`

| Option                 | Description                                                       |
| ---------------------- | ----------------------------------------------------------------- |
| `--bin` / `--no-bin`   | Whether the binary archive is created. Default: yes.              |
| `--pdbs` / `--no-pdbs` | Whether the PDBs archive is created. Default: yes.                |
| `--src` / `--no-src`   | Whether the source archive is created. Default: yes.              |
| `--inst` / `--no-inst` | Whether to copy the installer.                                    |
| `--version-from-exe`   | Retrieves version information from ModOrganizer.exe. Default.     |
| `--version-from-rc`    | Retrieves version information from `modorganizer/src/version.rc`. |
| `--rc <PATH>`          | Overrides the path to `version.rc`.                               |
| `--version <VERSION>`  | Overrides the version string.                                     |
| `--output-dir <PATH>`  | Sets the output directory instead of `prefix/releases`.           |
| `--suffix <SUFFIX>`    | Optional suffix to add to the archive filenames.                  |
| `--force`              | Ignores file size warnings and creates the archive regardless.    |

#### `release official`

Creates an official release from a specific branch.

| Option                 | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| `<BRANCH>`             | Use this branch in the super repos. Required.           |
| `--bin` / `--no-bin`   | Whether the binary archive is created. Default: yes.    |
| `--pdbs` / `--no-pdbs` | Whether the PDBs archive is created. Default: yes.      |
| `--no-installer`       | Skip building the installer task.                       |
| `--output-dir <PATH>`  | Sets the output directory instead of `prefix/releases`. |
| `--force`              | Ignores file size warnings.                             |

### `git`

Various commands to manage the git repos.

#### `git set-remotes`

Renames `origin` to `upstream` and adds a new `origin` with the given info.

| Option                | Description                                              |
| --------------------- | -------------------------------------------------------- |
| `--username`, `-u`    | Git username. Required.                                  |
| `--email`, `-e`       | Git email. Required.                                     |
| `--key`, `-k`         | Path to a PuTTY key.                                     |
| `--no-push`, `-s`     | Disables pushing to `upstream`.                          |
| `--push-origin`, `-p` | Sets the new `origin` as the default push target.        |
| `<path>`              | Only use this repo instead of going through all of them. |

#### `git add-remote`

Adds a new remote to all the git repos.

| Option                | Description                                              |
| --------------------- | -------------------------------------------------------- |
| `--name`, `-n`        | Name of new remote. Required.                            |
| `--username`, `-u`    | Git username. Required.                                  |
| `--key`, `-k`         | Path to a PuTTY key.                                     |
| `--push-origin`, `-p` | Sets this remote as the default push target.             |
| `<path>`              | Only use this repo instead of going through all of them. |

#### `git ignore-ts`

Toggles the `--assume-unchanged` status of all `.ts` files.

```powershell
mob git ignore-ts on   # mark .ts files as unchanged
mob git ignore-ts off  # unmark .ts files
```

#### `git branches`

Lists all git repos that are not on master.

| Option        | Description                                    |
| ------------- | ---------------------------------------------- |
| `--all`, `-a` | Shows all branches, including those on master. |

### `pr`

Applies changes from GitHub pull requests.

```powershell
mob pr find modorganizer/123                           # list affected repos
mob pr pull modorganizer/123 --github-token $TOKEN     # checkout PR branch
mob pr revert modorganizer/123                         # revert to master
```

| Option           | Description                                                   |
| ---------------- | ------------------------------------------------------------- |
| `--github-token` | GitHub API token. Can also be set via `GITHUB_TOKEN` env var. |
| `<OP>`           | Operation: `find`, `pull`, or `revert`.                       |
| `<PR>`           | PR reference, e.g. `modorganizer/123`.                        |

### `tx`

Manages Transifex translations.

#### `tx get`

Initializes a Transifex project and pulls all translation files.

| Option            | Description                                                |
| ----------------- | ---------------------------------------------------------- |
| `--key`, `-k`     | Transifex API key. Can also be set via `TX_TOKEN` env var. |
| `--team`, `-t`    | Transifex team name.                                       |
| `--project`, `-p` | Transifex project name.                                    |
| `--url`, `-u`     | Transifex project URL.                                     |
| `--minimum`, `-m` | Minimum translation threshold (0–100).                     |
| `--force`, `-f`   | Don't check timestamps, re-download all.                   |
| `<PATH>`          | Path that will contain the `.tx` directory.                |

#### `tx build`

Compiles all `.ts` translation files into `.qm` files using `lrelease`.

| Option          | Description                                  |
| --------------- | -------------------------------------------- |
| `<SOURCE>`      | Path containing the translation directories. |
| `<DESTINATION>` | Path that will contain the `.qm` files.      |

### `cmake-config`

Prints CMake configuration variables used by `mob` when building so that you can run your own `cmake`.

```powershell
cmake --preset vs2022-windows `
  ("-DCMAKE_INSTALL_PREFIX=" + (mob cmake-config install-prefix)) `
  ("-DCMAKE_PREFIX_PATH=" + (mob cmake-config prefix-path))
```

| Subcommand       | Description                    |
| ---------------- | ------------------------------ |
| `prefix-path`    | Prints `CMAKE_PREFIX_PATH`.    |
| `install-prefix` | Prints `CMAKE_INSTALL_PREFIX`. |

### `inis`

Shows a list of all the config files that would be loaded, in order of priority. See [TOML files](#override-options-using-toml-files).

## Differences from C++ mob

| Feature             | C++ mob                  | mob-rs                                     |
| ------------------- | ------------------------ | ------------------------------------------ |
| Config format       | INI (`.ini`)             | TOML (`.toml`)                             |
| Per-task overrides  | `[task_name:task]`       | `[tasks.task_name]`                        |
| Release modes       | `devbuild` only          | `devbuild` and `official`                  |
| PR command          | —                        | `mob pr find/pull/revert`                  |
| Translation command | —                        | `mob tx get/build`                         |
| Git backend         | Shell only               | Dual: `gix` (native Rust) + shell fallback |
| HTTP                | libcurl (requires vcpkg) | reqwest (native Rust)                      |
| Build dependencies  | vcpkg + C++ toolchain    | Rust toolchain only                        |
| Async               | Threads                  | tokio (async tasks, parallel `JoinSet`)    |
| Env var overrides   | —                        | `MOB_*` prefix mapping                     |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.
