<div align="right">

[简体中文](README.md) | **English**

</div>

<div align="center">
  <img src="docs/public/logo.png" alt="dsh-work Logo" width="300" />

  <p>
    <a href="https://github.com/shenjingnan/dsh-work/releases"><img src="https://img.shields.io/github/v/release/shenjingnan/dsh-work" alt="GitHub Release" /></a>
    <a href="https://crates.io/crates/dsh-work"><img src="https://img.shields.io/crates/v/dsh-work" alt="crates.io version" /></a>
    <a href="https://github.com/shenjingnan/dsh-work/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/shenjingnan/dsh-work/ci.yml?branch=main&label=CI" alt="GitHub Actions CI status" /></a>
    <br />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License: MIT" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.85%2B-dea584?logo=rust" alt="Rust 1.85+" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white" alt="Windows support" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white" alt="macOS support" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black" alt="Linux support" /></a>
  </p>
</div>

**dsh-work** is the desktop app for [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (dsh): download, install, double-click — no Node.js setup required.

> 🚧 This project is in early development. The repository contains both the Tauri 2 desktop app and a Rust CLI skeleton.

<details>

<summary>✨ Features</summary>

- **Batteries included** — the installer bundles a Node.js runtime and dsh itself; nothing else to install
- **Desktop app** — Tauri 2, single window embedding the dsh Web UI, for macOS / Windows / Linux
- **Local first** — the dsh service runs on the loopback address; your data never leaves the device
- **Custom title bar** — frameless window with a per-platform custom header
- **CLI skeleton** — clap-based argument parsing with subcommands and shell completion generation
- **Async runtime** — tokio integrated, async/await out of the box
- **Configuration** — TOML config files with `${env.VAR}` environment variable references
- **Dual-layer logging** — tracing-based logs to both file and stderr

</details>

## Download

Installers for macOS (Apple Silicon / Intel), Windows (x64), and Linux are built by CI and attached to [Releases](https://github.com/shenjingnan/dsh-work/releases).

The CLI is also published to crates.io:

```bash
cargo install dsh-work
```

## How It Works

The desktop app ships everything dsh needs, so users never touch Node.js:

1. **Bundled dsh** — `scripts/fetch-dsh.sh` pre-installs `@deepseek-ai/dsh` (version locked in `dsh-version.txt`) into `src-tauri/resources/dsh`, packed as Tauri resources
2. **Bundled runtime** — `scripts/fetch-runtime.sh` fetches a per-platform Node.js binary (sidecar) and pnpm into the installer
3. **Startup** — the app spawns the bundled Node to run `dsh web` on a loopback port, waits for readiness, then loads the local Web UI in a single window
4. **Lifecycle** — the dsh child process is cleaned up on window close and on SIGTERM/SIGINT

## Quick Start (CLI)

```bash
dsh-work config                  # Show configuration
dsh-work greet --name World      # Greet a user (parameter demo)
dsh-work completion bash         # Generate shell completion (bash/zsh/fish/powershell/elvish)
```

## Development

```bash
# CLI
cargo run -- config
cargo test                            # Run tests
cargo fmt --check && cargo clippy -- -D warnings && cargo test   # Full check

# Desktop app
./scripts/fetch-dsh.sh                # Fetch bundled dsh
./scripts/fetch-runtime.sh <triple>   # Fetch Node runtime + pnpm (e.g. aarch64-apple-darwin)
```

## Project Structure

```
├── Cargo.toml            # Workspace + CLI crate
├── dsh-version.txt       # Bundled dsh version lock
├── src/                  # CLI (clap + tokio)
│   ├── main.rs           # Entry point
│   ├── cli.rs            # CLI command definitions
│   ├── config/           # TOML configuration management
│   ├── logging.rs        # tracing dual-layer logging
│   └── datetime.rs       # Date/time utilities
├── src-tauri/            # Tauri 2 desktop app
│   ├── src/              # App entry, dsh process & runtime management
│   ├── frontend/         # Loading page (polls until the dsh server is ready)
│   ├── resources/        # Bundled dsh + pnpm (fetched by scripts)
│   └── binaries/         # Bundled Node.js runtime (sidecar)
├── scripts/              # fetch-dsh.sh / fetch-runtime.sh
├── tests/                # CLI integration tests
├── .github/workflows/    # CI / release-plz / cargo-dist + tauri-action
└── .githooks/            # Git hooks
```

## License

[MIT](LICENSE)
