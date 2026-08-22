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

> 🚧 项目处于早期开发阶段，仓库同时包含 Tauri 2 桌面应用与 Rust CLI 骨架。

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

## 工作原理

桌面应用把 dsh 运行所需的一切打包进安装包，用户完全不需要接触 Node.js：

1. **内置 dsh** — `scripts/fetch-dsh.sh` 按 `dsh-version.txt` 锁定的版本把 `@deepseek-ai/dsh` 预装到 `src-tauri/resources/dsh`，作为 Tauri resources 进入安装包
2. **内置运行时** — `scripts/fetch-runtime.sh` 按平台拉取 Node.js 二进制（sidecar）与 pnpm 打入安装包
3. **启动流程** — 应用拉起内置 Node 运行 `dsh web`（监听回环地址端口），等待服务就绪后在单窗口中加载本地 Web UI
4. **生命周期** — 关闭窗口或收到 SIGTERM/SIGINT 时自动清理 dsh 子进程

## 快速开始（CLI）

```bash
dsh-work config                  # 显示配置
dsh-work greet --name World      # 向用户问好（演示命令参数用法）
dsh-work completion bash         # 生成 Shell 补全（bash/zsh/fish/powershell/elvish）
```

## 开发

```bash
# CLI
cargo run -- config
cargo test                            # 运行测试
cargo fmt --check && cargo clippy -- -D warnings && cargo test   # 完整检查

# 桌面应用
./scripts/fetch-dsh.sh                # 拉取内置 dsh
./scripts/fetch-runtime.sh <triple>   # 拉取 Node 运行时 + pnpm（如 aarch64-apple-darwin）
```

## 项目结构

```
├── Cargo.toml            # Workspace + CLI crate
├── dsh-version.txt       # 内置 dsh 版本锁定
├── src/                  # CLI（clap + tokio）
│   ├── main.rs           # 入口文件
│   ├── cli.rs            # CLI 命令定义
│   ├── config/           # TOML 配置管理
│   ├── logging.rs        # tracing 双层日志
│   └── datetime.rs       # 日期时间工具
├── src-tauri/            # Tauri 2 桌面应用
│   ├── src/              # 应用入口、dsh 进程与运行时管理
│   ├── frontend/         # 加载页（轮询等待 dsh 服务就绪）
│   ├── resources/        # 内置 dsh + pnpm（由 scripts 拉取）
│   └── binaries/         # 内置 Node.js 运行时（sidecar）
├── scripts/              # fetch-dsh.sh / fetch-runtime.sh
├── tests/                # CLI 集成测试
├── .github/workflows/    # CI / release-plz / cargo-dist + tauri-action
└── .githooks/            # Git hooks
```

## 许可

[MIT](LICENSE)
