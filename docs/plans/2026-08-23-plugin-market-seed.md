# 内置插件市场（dshmarket）种子化方案

日期：2026-08-23
状态：已实施并通过端到端验收

## 背景与目标

DSHWork 的产品原则是"主链路零网络依赖、双击即用"。用户希望开箱即有插件市场，从而能在 dsh Web UI 里一键安装 [awesome-dsh-plugin](https://github.com/awesome-dsh-plugin/awesome-dsh-plugin) 列表收录的社区插件。

- **awesome-dsh-plugin 仓库是精选列表（curated list，CC0），本身不是插件**；真正的市场插件是 npm 包 [`dshmarket`](https://github.com/dsh-market/dsh-market)（MIT，社区维护）。
- 选定策略：**构建期离线预装**——CI 锁定版本把装好 dshmarket 的 profile 打进安装包，首启自动种子化到用户 `DSH_HOME`（`~/.dsh-work`）；老用户绝不覆盖；种子化失败仅告警降级。

## 上游机制结论（读源码验证）

- `dsh plugin --profile web add <pkg>` 是 pnpm 转发器：在 `$DSH_HOME/profiles/web` 里跑 `pnpm add`，声明 `dsh.bundle` 的依赖并入 profile package.json 的 `dsh.profile.bundles` 层列表。
- profile 的 node_modules 为 hoisted 平铺布局（`pnpm-workspace.yaml` 固定 `nodeLinker: hoisted`），**可整体跨机器复制**；官方明确否决了在 profile 文件里嵌机器路径的方案。
- `$DSH_HOME/profiles/node_modules` 是启动器维护的绝对路径符号链接 fallback，**每次启动 `healProfilesModuleFallback` 自愈重建**——绝不能进入种子（被解引用复制会导致 dsh 启动 throw）。
- `cordis.yml` 是 dsh 每次启动从 bundles + `cordis.patch.yml` 重算的派生文件，不入种子。
- dshmarket 在 Web UI 里装其他插件时重新唤起 `node <dsh 入口> plugin --profile web add <pkg>`，入口从 `process.argv[1]` 匹配 bin.js 获得，pnpm 从 PATH 解析——恰好复用桌面应用已注入的内置 node/pnpm，链路自洽。

## Spike 实测记录（2026-08-23，本机 arm64 + dsh 0.1.1-rc.2）

| 观察点 | 结论 |
| --- | --- |
| 种子范围 | 临时 DSH_HOME 里只产生 `profiles/web`，无其他目录 |
| bundle id | 依赖名本身：`dshmarket`；dependencies 记录精确版本 |
| 体积 | 5.7MB（纯 JS：argparse / dshmarket / js-yaml / undici，**无平台原生变体**） |
| 符号链接 | 仅 `node_modules/.bin/js-yaml` 一处 → `cp -RL` 解引用 + `find -type l` 断言零链接 |
| lockfile | 无 `file:`/`link:`/`git+` 本地引用 |
| 首启模拟 | 复制到新 DSH_HOME 后 `dsh web` 直接就绪，HTML 引用 `dshmarket/client.js`（市场 client 注入成功），fallback 自愈为本机安装本体的绝对链接，启动无 pnpm 调用 |
| `.modules.yaml` | 其 `storeDir` 记录生产机的 pnpm store 绝对路径：**保留则跨机首次 `pnpm add` 触发 `ERR_PNPM_UNEXPECTED_STORE` 且无自动恢复；删除后 pnpm 全量重链，首装约 0.6s 且状态稳定** → 决策：构建期剔除该文件 |

## 实施内容

### 阶段 1：构建期种子生产（commit `60b5d47c`）

- `plugins-version.txt`：锁定 `dshmarket@1.18.1`。
- `scripts/fetch-plugins.sh`：用已预装的 dsh 在临时 DSH_HOME 走官方链路安装，产物经防呆校验（依赖/bundle/lockfile 可移植性/node 侧 JSON 断言，不依赖 jq）后 `cp -RL` 拷到 `src-tauri/resources/profile-seed/web`；剔除 `cordis.yml` 与 `node_modules/.modules.yaml`；防御性清理 musl/linux-arm64 变体；断言零符号链接；缓存命中时按包内 manifest 版本复核后跳过。
- `tauri.conf.json` resources 增加 `profile-seed` 映射。
- `release.yml` 在 fetch-dsh 后增加种子缓存（key 含 `plugins-version.txt` + `dsh-version.txt` + 脚本 hash，按 `runner.os` 分线）与获取步骤。
- 脚本踩坑记录：双引号内 `$DEST（`——bash 在 UTF-8 locale 把紧跟变量的多字节字符解析进变量名，`set -u` 下报 unbound；中文括号相邻的变量必须写成 `${DEST}`。

### 阶段 2：首启种子化（commit `1d052002`）

- `src-tauri/src/seed.rs`：`try_seed(resource_dir, dsh_home)`——种子缺失 / `profiles/web/package.json` 已存在 / 目录非空时 `Ok(false)`（绝不覆盖用户数据）；复制到 `DSH_HOME/.profile-seed-tmp` 后 `rename` 原子落位；`copy_tree` 遇符号链接条目即 Err（安装包损坏走降级）。
- `main.rs`：仅 `RuntimeSource::Bundled` 时在 `DshHandle::spawn` 前调用；失败仅 `tracing::warn`，dsh 自行 initProfile，主链路不受影响。
- 6 个单元测试（tempdir + process::id 风格，不引入 tempfile 依赖）。

### 存量 bug：内置 pnpm 缺失 worker.js（commit `bb80370c`）

验收时发现**自首次发布起用户机器上的 `dsh plugin add` 一直是坏的**：

- pnpm 10.3x 把 resolve/fetch 放在 worker 线程执行，worker 脚本按 `__dirname` 相对解析（`pnpm.cjs` 内 `workerScriptPath = join(__dirname, "worker.js")`）。
- `fetch-runtime.sh` 只从 tarball 提取了 `pnpm.cjs`，缺失 `worker.js` 时 `add` 表现为**静默无操作**（`added 0`、不写 package.json、exit 0）或 `Worker pnpm#1 exited with code 1`——CI 未察觉是因为 CI 的 fetch 脚本用的是 action-setup 装的完整 pnpm，`resources/pnpm` 从未在 add 场景被验证过。
- 排查过程的关键对照：系统 pnpm 10.18.2（完整安装）一切正常 vs 内置 pnpm.cjs 10.34.5 静默失败；空目录实验分离出"包/环境无关"后从崩溃堆栈定位到 worker。
- 修复：随行提取 `dist/worker.js` 与 `dist/pnpmrc`；`reflink.*.node` 原生模块为可选优化，暂不随行（缺失时 pnpm 退化为普通复制）。

### 端到端验收（全部通过）

本机 `cargo tauri build --bundles app`（.app 内 pnpm 含 worker.js）：

1. 新用户首启：种子化成功，`cordis.yml` 由 dsh 启动时重算，HTML 引用 `dshmarket/client.js`，fallback 链接指向 .app 内资源；
2. 模拟用户装插件（与 dshmarket 内部一致的调用方式：内置 pnpm + `CI=true`）：`dsh plugin --profile web add @wsgsety/dsh-plugin-manager` 成功，reconcile 将其并入 bundles；
3. 老用户（已有无市场的 profile）启动新构建：profile 未被覆盖、未注入市场；
4. `cargo fmt --check && cargo clippy -p dsh-work-app -- -D warnings && cargo test -p dsh-work-app` 全绿（19 测试）。

## 版本升级维护流程

升级 `dsh-version.txt` 或 `plugins-version.txt` 任一侧后：

1. 重跑 spike（种子生成 → 复制模拟首启 → 内置 pnpm 装插件），确认 dshmarket 与新版 dsh 兼容；
2. CI 缓存 key 含双版本文件 + 脚本 hash，自动失效重建；
3. `pnpm`（fetch-runtime.sh 内 PNPM_VERSION）major 升级时注意 lockfile/store 语义变化。
