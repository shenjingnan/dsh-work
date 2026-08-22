<div align="right">

[简体中文](README.md) | **English**

</div>

<div align="center">
  <img src="docs/public/logo.png" alt="DSHWork Logo" width="300" />

  <p>
    <a href="https://github.com/shenjingnan/dsh-work/releases"><img src="https://img.shields.io/github/v/release/shenjingnan/dsh-work" alt="GitHub Release" /></a>
    <a href="https://crates.io/crates/dsh-work"><img src="https://img.shields.io/crates/v/dsh-work" alt="crates.io version" /></a>
    <a href="https://github.com/shenjingnan/dsh-work/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/shenjingnan/dsh-work/ci.yml?branch=main&label=CI" alt="GitHub Actions CI status" /></a>
    <br />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License: MIT" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.85%2B-dea584?logo=rust" alt="Rust 1.85+" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/Windows-0078D6?logo=data:image/svg%2Bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6bTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6bTEyLjYgMEgyNFYyNEgxMi42eiIvPjwvc3ZnPg%3D%3D" alt="Windows support" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/MacOS-000000?logo=apple&logoColor=white" alt="MacOS support" /></a>
    <a href="#download"><img src="https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black" alt="Linux support" /></a>
  </p>
</div>

**DSHWork** (dsh-work) is the desktop app for [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) (dsh): download, install, double-click — no Node.js setup required.

> 🚧 This project is in early development.

<div align="center">
  <img src="docs/public/screenshot.png" alt="dsh-work desktop app screenshot" width="800" />
</div>

<details>

<summary>✨ Features</summary>

- **Batteries included** — the installer bundles a Node.js runtime and dsh itself; nothing else to install
- **Desktop app** — Tauri 2, single window embedding the dsh Web UI, for MacOS / Windows / Linux
- **Local first** — the dsh service runs on the loopback address; your data never leaves the device
- **Custom title bar** — frameless window with a per-platform custom header
- **CLI skeleton** — clap-based argument parsing with subcommands and shell completion generation
- **Async runtime** — tokio integrated, async/await out of the box
- **Configuration** — TOML config files with `${env.VAR}` environment variable references
- **Dual-layer logging** — tracing-based logs to both file and stderr

</details>

## Download

Click a button below to grab the latest installer for your system:

| System | Chip / Arch | Download |
| --- | --- | --- |
| Windows 10 / 11 | x64 | [![Download](https://img.shields.io/badge/-Download-0078D6?style=for-the-badge&logo=data:image/svg%2Bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6bTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6bTEyLjYgMEgyNFYyNEgxMi42eiIvPjwvc3ZnPg%3D%3D)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Windows_x64.exe) |
| MacOS | Apple Silicon (M1/M2/M3/M4) | [![Download](https://img.shields.io/badge/-Download-8E8E93?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_macOS_arm64.dmg) |
| MacOS | Intel | [![Download](https://img.shields.io/badge/-Download-8E8E93?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_macOS_x64.dmg) |
| Ubuntu / Debian | amd64 | [![Download](https://img.shields.io/badge/-Download-A80030?style=for-the-badge&logo=debian&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_amd64.deb) |
| Fedora / RHEL | x86_64 | [![Download](https://img.shields.io/badge/-Download-294172?style=for-the-badge&logo=fedora&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_x86_64.rpm) |

- Windows: an [MSI installer](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Windows_x64.msi) is available for enterprise deployment; Linux: an [AppImage](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_amd64.AppImage) build runs without installation.
- 🍎 Not sure which Mac chip? Apple menu → About This Mac: "Chip: Apple M…" → arm64; "Processor: Intel…" → x64. If the first launch says the app is damaged, see the fix below.
- Full version history and changelogs: [Releases](https://github.com/shenjingnan/dsh-work/releases).

### MacOS says "DSHWork is damaged and can't be opened"?

The app is not signed or notarized with Apple, so Gatekeeper blocks the first launch with ""DSHWork" is damaged and can't be opened. You should move it to the Trash." — **the app is not actually damaged**:

<div align="center">
  <img src="docs/public/macos-damaged-dialog.png" alt="MacOS dialog claiming DSHWork is damaged" width="360" />
</div>

Two ways to fix it (drag DSHWork into the Applications folder first):

- **Double-click the fixer script (recommended)**: open the downloaded DMG image and double-click 「首次打开修复.command」 inside — it fixes the issue and launches the app automatically;
- **Run the command manually**: open Terminal and run:

  ```bash
  xattr -cr /Applications/DSHWork.app
  ```

After that, DSHWork opens normally.

## Contributing

Issues and PRs are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) (in Chinese) for how the app works, local development, and the project layout.

## License

[MIT](LICENSE)
