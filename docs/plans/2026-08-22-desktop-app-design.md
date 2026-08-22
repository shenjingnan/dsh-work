# dsh-work 桌面应用 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 把 dsh-work 从 Rust CLI 骨架演进为 DeepSeek Harness 的桌面应用：安装包内置 Node.js + dsh + pnpm，用户双击即用，无需自备 Node 环境。

**Architecture:** Tauri 2 桌面壳，启动时用内置 Node 以隔离的 `DSH_HOME`（`~/.dsh-work`）拉起 `dsh web --no-open --host 127.0.0.1 --port 0`，从 stdout 解析就绪端口，单窗口加载本地 Web UI；PATH 注入内置 node/pnpm 使插件管理可用。

**Tech Stack:** Rust 1.85+ / Tauri 2 / clap（保留 CLI）/ tokio / Node.js v22（sidecar）/ @deepseek-ai/dsh（预装包）/ pnpm（单文件分发）

---

## 设计定稿（brainstorming 结论）

- **形态**：单窗口嵌入 dsh Web UI（不做原生控制面板，不阻塞首发）
- **运行时策略**：安装包内置 Node 官方单二进制 + 预装 dsh 包 + pnpm 单文件分发版
- **平台**：macOS (arm64/x64)、Windows x64、Linux amd64
- **dsh 版本**：打包时锁定（`dsh-version.txt`），升级 dsh = 发新版应用
- **隔离**：DSH_HOME 用 `~/.dsh-work`，与用户系统已装的 dsh（`~/.dsh`）互不干扰；只监听 127.0.0.1
- **品牌**：logo/icon 已入库 `docs/public/`（透明底 PNG，由 tmp/ 下用户原始图重新生成）

## 阶段 0 验证结果（2026-08-22 已完成）

用本机 dsh 0.1.1-rc.1 + 空 `DSH_HOME=/tmp/dsh-work-spike` 实测：

1. ✅ 空 DSH_HOME 首启自动初始化 `profiles/web`（含 node_modules 符号链接，仅 20K）
2. ✅ 就绪时 stdout 输出 `dsh web: http://127.0.0.1:<port>`，端口可直接解析
3. ✅ `--port 0` 由 OS 分配空闲端口；HTTP 200 返回完整 Web UI（14.5KB HTML）
4. ✅ 启动过程不调用 pnpm（pnpm 仅 `dsh plugin` 子命令需要，`spawnSync("pnpm")` 从 PATH 解析）
5. 结论：主链路零网络依赖；装插件需联网（可接受）

## 目标结构

```
dsh-work/
├── Cargo.toml              # workspace：root CLI 包 + members = ["src-tauri"]
├── src/                    # 现有 CLI（保留，cargo install dsh-work 仍可用）
├── src-tauri/              # Tauri 桌面应用
│   ├── Cargo.toml          # tauri / tauri-plugin-single-instance
│   ├── tauri.conf.json     # externalBin=node, resources=dsh+pnpm
│   ├── build.rs
│   ├── icons/              # tauri icon 由 docs/public/icon 生成
│   ├── frontend/           # 无框架静态加载页（spinner/错误页/重试）
│   └── src/
│       ├── main.rs         # 窗口、tauri commands、单实例
│       ├── runtime.rs      # 定位内置 node/pnpm/dsh，构造子进程 env
│       └── process.rs      # spawn dsh web、stdout 端口解析、监控、清理
├── scripts/
│   ├── fetch-runtime.sh    # 下载锁定版本 Node + pnpm（按目标 triple）
│   └── fetch-dsh.sh        # 按 dsh-version.txt 预装 dsh 到 resources
├── dsh-version.txt         # 例如 0.1.1-rc.1
└── .github/workflows/
    ├── ci.yml              # fmt/clippy/test + tauri build 冒烟
    └── release.yml         # tag 触发 tauri-action 矩阵发 Release
```

---

## 阶段 1：Tauri 骨架

### Task 1: workspace 改造

**Files:**
- Modify: `Cargo.toml`（根包保留，加 `[workspace] members = [".", "src-tauri"]`）

**Step 1:** 根 `Cargo.toml` 顶部加：

```toml
[workspace]
members = [".", "src-tauri"]
resolver = "2"
```

**Step 2:** 验证：`cargo metadata --no-deps` 退出码 0（此时 src-tauri 尚不存在会报错，属预期，先建 Task 2 的最小 src-tauri 再验证）。

### Task 2: 最小 src-tauri 脚手架

**Files:**
- Create: `src-tauri/Cargo.toml`、`src-tauri/build.rs`、`src-tauri/tauri.conf.json`、`src-tauri/src/main.rs`、`src-tauri/frontend/index.html`、`src-tauri/capabilities/default.json`

**Step 1:** `src-tauri/Cargo.toml`：

```toml
[package]
name = "dsh-work-app"
version = "0.1.0"
edition = "2021"

[lib]
name = "dsh_work_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Step 2:** `build.rs` = `fn main() { tauri_build::build() }`；`tauri.conf.json` 关键项：`productName: "dsh-work"`、`identifier: "com.shenjingnan.dsh-work"`、`frontendDist: "../frontend"`（指 src-tauri/frontend）、`app.windows[0]` 1280x800 标题 dsh-work。

**Step 3:** `frontend/index.html`：纯 HTML 加载页（居中 logo + spinner + 状态文字 `<p id="status">`），无构建工具。

**Step 4:** `src/main.rs` 最小窗口 app（`tauri::Builder::default().run(tauri::generate_context!())`）。

**Step 5:** 图标：`cargo install tauri-cli --version ^2 --locked`（若未装），`cargo tauri icon docs/public/icon.jpg -o src-tauri/icons`。

**Step 6:** 验证：`cargo tauri dev` 弹窗显示加载页。提交 `feat: tauri 桌面应用骨架`。

## 阶段 2：运行时管理（TDD：纯函数先测试）

### Task 3: runtime.rs — 资源定位与 env 构造

**Files:** Create `src-tauri/src/runtime.rs`；Test: 同文件 `#[cfg(test)]`

**Step 1:** 失败测试：`parse_dsh_url("dsh web: http://127.0.0.1:53044") == Some("http://127.0.0.1:53044")`；`build_child_path("/res/pnpm", "/res/node", "/usr/bin")` 前缀正确。

**Step 2:** 实现纯函数：`parse_dsh_url(line)`（正则/字符串扫描找 `http://127.0.0.1:\d+`）、`build_child_path(pnpm_dir, node_dir, existing)`。

**Step 3:** `resolve_runtime(paths: &tauri::path::PathResolver) -> RuntimePaths { node, dsh_dir, pnpm_dir }`；**dev 回退**：resources 不存在时回退到系统 PATH 的 node/dsh（开发期免跑 fetch 脚本），返回 enum `Bundled | SystemFallback` 并记日志。

**Step 4:** `cargo test -p dsh-work-app` 通过。提交。

### Task 4: process.rs — dsh 进程生命周期

**Files:** Create `src-tauri/src/process.rs`

**Step 1:** `DshProcess::spawn(runtime, dsh_home) -> Result<DshProcess>`：`Command::new(node).args([bin.js, "web", "--no-open", "--host", "127.0.0.1", "--port", "0"])`，env 注入 `DSH_HOME=~/.dsh-work`、`PATH=build_child_path(...)`；stdout piped，读取线程逐行扫描，`parse_dsh_url` 命中后经 `mpsc`/`OnceCell` 通知就绪 URL。

**Step 2:** 失败路径：60s 未就绪 / 进程提前退出 → `DshError::StartupTimeout / Exited(code)`，stderr 尾 20 行进错误信息。

**Step 3:** `Drop` 实现 kill 子进程（防应用退出残留）；`restart()` 杀旧拉新。

**Step 4:** 单元测试：用 `sh -c 'echo "dsh web: http://127.0.0.1:1"'` 模拟子进程验证解析与超时分支。提交。

### Task 5: 主窗口接线

**Files:** Modify `src-tauri/src/main.rs`、`src-tauri/frontend/index.html`

**Step 1:** setup 里 spawn `DshProcess` 并 `manage()` 状态；tauri commands：`server_url() -> Option<String>`、`restart_server() -> Result<String, String>`。

**Step 2:** 加载页 JS：每 500ms `invoke('server_url')`，拿到后 `location.replace(url)`；超时/失败显示错误 + 「重试」按钮调 `restart_server`；带 `capability` 放开这两个 command。

**Step 3:** `tauri-plugin-single-instance`：重复启动激活已有窗口。

**Step 4:** 验收：`cargo tauri dev` → 加载页 → 自动跳进 dsh Web UI 可正常操作；强杀 dsh 子进程 → 错误页可重试；关窗后 `pgrep -f dsh` 无残留。提交 `feat: 内置 dsh 运行时管理`。

## 阶段 3：打包与分发

### Task 6: fetch 脚本

**Files:** Create `scripts/fetch-runtime.sh`、`scripts/fetch-dsh.sh`、`dsh-version.txt`（写 `0.1.1-rc.1`）

**Step 1:** `fetch-runtime.sh <target-triple>`：下载 Node v22 LTS 官方 tarball（triple→node dist 映射表），只取 `bin/node` → `src-tauri/binaries/node-<triple>[.exe]`；下载 pnpm 单文件分发 `pnpm.cjs` + 写 shim `pnpm`（`#!/usr/bin/env node` + exec pnpm.cjs，chmod +x；Windows 生成 `pnpm.cmd`）→ `src-tauri/resources/pnpm/`。

**Step 2:** `fetch-dsh.sh`：`npm install --prefix src-tauri/resources/dsh @deepseek-ai/dsh@$(cat dsh-version.txt) --omit=dev`，入口解析为 `resources/dsh/node_modules/@deepseek-ai/dsh/lib/bin.js`。

**Step 3:** 本机跑两脚本（arm64），`cargo tauri dev` 走 Bundled 路径验证。提交 `chore: 运行时获取脚本`。

### Task 7: bundle 配置与三平台 CI

**Files:** Modify `src-tauri/tauri.conf.json`；Create `.github/workflows/release.yml`；Modify `.github/workflows/ci.yml`

**Step 1:** `tauri.conf.json`：`bundle.externalBin: ["binaries/node"]`、`bundle.resources: {"resources/dsh": "dsh/", "resources/pnpm": "pnpm/"}`、各平台 bundle targets（dmg / nsis+msi / deb+rpm+appimage）。

**Step 2:** `release.yml`：tag `v*` 触发，matrix = [macos-14(arm64), macos-13(x64), windows-latest, ubuntu-latest]，每 job 先跑对应 triple 的 fetch 脚本再 `tauri-action`；产物命名 `dsh-work_<OS>_<arch>.<ext>`。

**Step 3:** `ci.yml` 保持 fmt/clippy/test，追加 `cargo tauri build --no-bundle` 冒烟。

**Step 4:** 验收：本机 `cargo tauri build` 出 `.app`/`.dmg`，`xattr -cr` 后双击即用（断网验证核心链路）；打 tag 后 CI 出齐 5 个安装包。提交 `feat: 三平台安装包与发布流程`。

## 阶段 4（可选，不阻塞首发）

托盘菜单（显示/退出/重启）、开机自启（tauri-plugin-autostart）、设置页（端口固定/自启开关）、错误页品牌化。

## 已知风险与备注

| 风险 | 缓解 |
|------|------|
| macOS 未签名 Gatekeeper 拦截 | README 提供 `xattr -cr` 说明（同 ZapMomo） |
| Windows SmartScreen | README 说明；长期可申请证书 |
| dsh 升级引入新行为 | dsh-version.txt 锁定，升级走发版流程 + 阶段 0 复验 |
| 本地 DMG 打包失败（bundle_dmg.sh 需 Finder 自动化权限） | 本地环境问题，CI 无此限制；本地可用 `--bundles app` |
| Linux webkitgtk 依赖 | tauri-action 标准流程安装；AppImage 兜底 |
