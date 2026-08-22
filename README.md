<div align="right">

**简体中文** | [English](README.en.md)

</div>

<div align="center">
  <img src="docs/public/logo.png" alt="DSHWork Logo" width="300" />

  <p>
    <a href="https://github.com/shenjingnan/dsh-work/releases"><img src="https://img.shields.io/github/v/release/shenjingnan/dsh-work" alt="GitHub Release" /></a>
    <a href="https://crates.io/crates/dsh-work"><img src="https://img.shields.io/crates/v/dsh-work" alt="crates.io 版本" /></a>
    <a href="https://github.com/shenjingnan/dsh-work/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/shenjingnan/dsh-work/ci.yml?branch=main&label=CI" alt="GitHub Actions CI 状态" /></a>
    <br />
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License: MIT" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.85%2B-dea584?logo=rust" alt="Rust 1.85+" /></a>
    <a href="#下载"><img src="https://img.shields.io/badge/Windows-0078D6?logo=windows&logoColor=white" alt="Windows 支持" /></a>
    <a href="#下载"><img src="https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white" alt="macOS 支持" /></a>
    <a href="#下载"><img src="https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black" alt="Linux 支持" /></a>
  </p>
</div>

**DSHWork**（dsh-work）是 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（dsh）的桌面应用：下载安装后双击即可使用，无需自己准备 Node.js 环境。

> 🚧 项目处于早期开发阶段。

<div align="center">
  <img src="docs/public/screenshot.png" alt="dsh-work 桌面应用截图" width="800" />
</div>

<details>

<summary>✨ 特性一览</summary>

- **开箱即用** — 安装包内置 Node.js 运行时与 dsh，用户无需安装任何依赖
- **桌面应用形态** — 基于 Tauri 2，单窗口内嵌 dsh Web UI，支持 macOS / Windows / Linux
- **本地优先** — dsh 服务在本机回环地址运行，数据不出设备
- **自定义标题栏** — 无边框窗口，分平台实现自定义头部
- **CLI 骨架** — 基于 clap 的命令行参数解析，支持子命令和 Shell 补全生成
- **异步运行时** — 集成 tokio，开箱即用的 async/await 支持
- **配置管理** — TOML 格式的配置文件读写，支持 `${env.VAR}` 环境变量引用
- **双层日志** — 基于 tracing 的日志系统，同时输出到文件和 stderr

</details>

## 下载

点击下方按钮直接下载对应系统的最新版安装包（无需登录 GitHub，自动指向最新 Release）：

| 系统 | 芯片 / 架构 | 立即下载 |
| --- | --- | --- |
| Windows 10 / 11 | x64 | [![立即下载](https://img.shields.io/badge/-%E7%AB%8B%E5%8D%B3%E4%B8%8B%E8%BD%BD-0078D6?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Windows_x64.exe) |
| macOS 13+ | Apple Silicon（M1/M2/M3/M4） | [![立即下载](https://img.shields.io/badge/-%E7%AB%8B%E5%8D%B3%E4%B8%8B%E8%BD%BD-8E8E93?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_macOS_arm64.dmg) |
| macOS 13+ | Intel | [![立即下载](https://img.shields.io/badge/-%E7%AB%8B%E5%8D%B3%E4%B8%8B%E8%BD%BD-8E8E93?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_macOS_x64.dmg) |
| Ubuntu / Debian | amd64 | [![立即下载](https://img.shields.io/badge/-%E7%AB%8B%E5%8D%B3%E4%B8%8B%E8%BD%BD-A80030?style=for-the-badge&logo=debian&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_amd64.deb) |
| Fedora / RHEL | x86_64 | [![立即下载](https://img.shields.io/badge/-%E7%AB%8B%E5%8D%B3%E4%B8%8B%E8%BD%BD-294172?style=for-the-badge&logo=fedora&logoColor=white)](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_x86_64.rpm) |

- Windows 企业批量部署可选 [MSI 版](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Windows_x64.msi)；Linux 可选 [AppImage](https://github.com/shenjingnan/dsh-work/releases/latest/download/DSHWork_Linux_amd64.AppImage) 免安装直接运行。
- 🍎 Mac 不确定芯片？左上角  →「关于本机」：显示「芯片：Apple M…」选 arm64，显示「处理器：Intel…」选 x64。首次打开提示「已损坏」？见下方修复说明。
- 完整版本与更新日志见 [Releases](https://github.com/shenjingnan/dsh-work/releases)。

CLI 也已发布到 crates.io：

```bash
cargo install dsh-work
```

### macOS 打开时提示「已损坏」？

应用未经 Apple 签名公证，首次打开会被 Gatekeeper 拦截并提示「"DSHWork" 已损坏，无法打开。你应该将它移到废纸篓。」——**并非真的损坏**：

<div align="center">
  <img src="docs/public/macos-damaged-dialog.png" alt="macOS 提示 DSHWork 已损坏的弹窗" width="360" />
</div>

两种修复方式（先把 DSHWork 拖入 Applications 文件夹）：

- **双击修复脚本（推荐）**：打开下载的 dmg 镜像，双击其中的「首次打开修复.command」，自动完成修复并启动应用；
- **手动执行命令**：打开「终端」（Terminal），粘贴执行：

  ```bash
  xattr -cr /Applications/DSHWork.app
  ```

执行后重新双击 DSHWork 即可正常打开。

## 贡献

欢迎通过 [Issue](https://github.com/shenjingnan/dsh-work/issues) 和 PR 参与贡献。工作原理、本地开发与项目结构见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可

[MIT](LICENSE)
