# 贡献指南

感谢关注 dsh-work！欢迎通过 [Issue](https://github.com/shenjingnan/dsh-work/issues) 和 PR 参与贡献。

## 环境要求

- Rust 1.85+（版本见 `rust-toolchain.toml`）
- CLI 开发仅需 Rust 工具链
- 桌面应用开发无需本机安装 Node.js — 运行时与 dsh 由脚本拉取（见下文）

## 工作原理

桌面应用把 dsh 运行所需的一切打包进安装包，用户完全不需要接触 Node.js：

1. **内置 dsh** — `scripts/fetch-dsh.sh` 按 `dsh-version.txt` 锁定的版本把 `@deepseek-ai/dsh` 预装到 `src-tauri/resources/dsh`，作为 Tauri resources 进入安装包
2. **内置运行时** — `scripts/fetch-runtime.sh` 按平台拉取 Node.js 二进制（sidecar）与 pnpm 打入安装包
3. **启动流程** — 应用拉起内置 Node 运行 `dsh web`（监听回环地址端口），等待服务就绪后在单窗口中加载本地 Web UI
4. **生命周期** — 关闭窗口或收到 SIGTERM/SIGINT 时自动清理 dsh 子进程

## 本地开发

```bash
# CLI
cargo run -- config                  # 运行（greet --name World 可演示参数用法）
cargo test                           # 运行测试
cargo fmt --check && cargo clippy -- -D warnings && cargo test   # 完整检查（提交前建议通过）

# 桌面应用
./scripts/fetch-dsh.sh                # 拉取内置 dsh
./scripts/fetch-runtime.sh <triple>   # 拉取 Node 运行时 + pnpm（如 aarch64-apple-darwin）
cargo tauri dev                       # 启动桌面应用（需已安装 tauri-cli）
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

## 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <description>
```

常用 type：`feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `chore`。

分支命名：`feature/xxx`、`fix/xxx`、`docs/xxx`、`refactor/xxx`。
