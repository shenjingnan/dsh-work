# dsh-work

dsh-work 命令行工具。

## 特性

- **CLI 骨架** — 基于 clap 的命令行参数解析，支持子命令和 Shell 补全生成
- **异步运行时** — 集成 tokio，开箱即用的 async/await 支持
- **配置管理** — TOML 格式的配置文件读写，支持 `${env.VAR}` 环境变量引用
- **双层日志** — 基于 tracing 的日志系统，同时输出到文件和 stderr
- **日期时间工具** — 基于 chrono 的常用时间格式转换函数
- **Shell 补全** — 支持 bash / zsh / fish / powershell 自动补全

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

## 开发指南

1. 修改 `Cargo.toml` 中的项目元信息（version, description）
2. 按需调整依赖（`Cargo.toml` 中的可选依赖已注释说明）
3. 在 `src/cli.rs` 中定义你的命令
4. 开始编写业务代码

## 依赖说明

| 分类 | Crate | 用途 |
|------|-------|------|
| 核心 | clap | CLI 参数解析 |
| 核心 | tokio | 异步运行时 |
| 核心 | serde / serde_json / toml | 序列化 |
| 核心 | chrono | 日期时间处理 |
| 核心 | tracing / tracing-subscriber | 日志 |
| 核心 | thiserror / anyhow | 错误处理 |
| 可选 | reqwest | HTTP 客户端（按需引入） |

## 许可

[MIT](LICENSE)
