<div align="center">
  <img src="docs/public/logo.png" alt="dsh-work Logo" width="300" />

  <p>
    <a href="https://github.com/shenjingnan/dsh-work/releases"><img src="https://img.shields.io/github/v/release/shenjingnan/dsh-work" alt="GitHub Release" /></a>
    <a href="https://github.com/shenjingnan/dsh-work/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/shenjingnan/dsh-work/ci.yml?branch=main&label=CI" alt="GitHub Actions CI 状态" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="License: MIT" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.85%2B-dea584?logo=rust" alt="Rust 1.85+" /></a>
  </p>
</div>

**dsh-work** 是 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（dsh）的桌面应用：下载安装后双击即可使用，无需自己准备 Node.js 环境。

> 🚧 项目处于早期开发阶段，当前仓库为 Rust CLI 骨架，正在向桌面应用形态演进。

## 目标

- **开箱即用** — 安装包内置 Node.js 运行时与 dsh，用户无需安装任何依赖
- **桌面应用形态** — 基于 Tauri，单窗口内嵌 dsh Web UI，支持 macOS / Windows / Linux
- **本地优先** — dsh 服务在本机回环地址运行，数据不出设备

## 当前功能（CLI 骨架）

- **CLI 骨架** — 基于 clap 的命令行参数解析，支持子命令和 Shell 补全生成
- **异步运行时** — 集成 tokio，开箱即用的 async/await 支持
- **配置管理** — TOML 格式的配置文件读写，支持 `${env.VAR}` 环境变量引用
- **双层日志** — 基于 tracing 的日志系统，同时输出到文件和 stderr
- **日期时间工具** — 基于 chrono 的常用时间格式转换函数

## 安装

```bash
cargo install dsh-work
```

## 快速开始

```bash
dsh-work config
dsh-work greet --name World

# 开发
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

## 项目结构

```
├── Cargo.toml           # 项目配置和依赖
├── rust-toolchain.toml  # Rust 工具链版本（1.85）
├── docs/public/         # Logo、图标等品牌资源
├── src/
│   ├── main.rs          # 入口文件
│   ├── lib.rs           # 库入口 + 测试工具
│   ├── cli.rs           # CLI 命令定义
│   ├── config/
│   │   ├── mod.rs       # 配置模块入口
│   │   └── settings.rs  # TOML 配置管理
│   ├── logging.rs       # tracing 双层日志
│   └── datetime.rs      # 日期时间工具
├── tests/               # 集成测试
├── .github/workflows/   # CI/CD
└── .githooks/           # Git hooks
```

## 许可

[MIT](LICENSE)
