<div align="right">

**简体中文** | [English](README.en.md)

</div>

<div align="center">
  <img src="docs/public/logo.png" alt="dsh-work Logo" width="300" />

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

**dsh-work** 是 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（dsh）的桌面应用：下载安装后双击即可使用，无需自己准备 Node.js 环境。

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

CI 会为 macOS（Apple Silicon / Intel）、Windows（x64）、Linux 构建安装包，发布在 [Releases](https://github.com/shenjingnan/dsh-work/releases)。

CLI 也已发布到 crates.io：

```bash
cargo install dsh-work
```

## 贡献

欢迎通过 [Issue](https://github.com/shenjingnan/dsh-work/issues) 和 PR 参与贡献。工作原理、本地开发与项目结构见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可

[MIT](LICENSE)
