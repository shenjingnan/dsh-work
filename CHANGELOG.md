# Changelog

## [0.1.5](https://github.com/shenjingnan/dsh-work/compare/v0.1.4...v0.1.5) - 2026-08-22

### Added

- *(ci)* Release 发布后自动上传安装包与说明到百度网盘 ([#27](https://github.com/shenjingnan/dsh-work/pull/27))
- *(release)* dmg 注入 Gatekeeper 修复脚本并优化下载入口 ([#23](https://github.com/shenjingnan/dsh-work/pull/23))

### Other

- 忽略 tmp 目录 ([#22](https://github.com/shenjingnan/dsh-work/pull/22))

## [0.1.4](https://github.com/shenjingnan/dsh-work/compare/v0.1.3...v0.1.4) - 2026-08-22

### Added

- *(desktop)* 桌面应用更名 DSHWork 并统一版本号来源 ([#21](https://github.com/shenjingnan/dsh-work/pull/21))
- *(ci)* 统一 Release 资产命名并在正文顶部增加下载表格 ([#19](https://github.com/shenjingnan/dsh-work/pull/19))

### Fixed

- *(ci)* 全平台构建成功后自动发布 release 草稿 ([#17](https://github.com/shenjingnan/dsh-work/pull/17))

### Other

- *(readme)* README 聚焦用户视角，开发内容迁移至 CONTRIBUTING ([#20](https://github.com/shenjingnan/dsh-work/pull/20))

## [0.1.3](https://github.com/shenjingnan/dsh-work/compare/v0.1.2...v0.1.3) - 2026-08-22

### Added

- 支持 macOS 26 Liquid Glass 应用图标 ([#13](https://github.com/shenjingnan/dsh-work/pull/13))

### Fixed

- *(build)* 清除 dsh 依赖的 musl/linux-arm64 原生变体修复 AppImage 打包 ([#16](https://github.com/shenjingnan/dsh-work/pull/16))

### Other

- *(readme)* 新增英文版 README 并同步桌面应用现状 ([#15](https://github.com/shenjingnan/dsh-work/pull/15))

## [0.1.2](https://github.com/shenjingnan/dsh-work/compare/v0.1.1...v0.1.2) - 2026-08-22

### Fixed

- *(ci)* 合并 macOS 构建到 macos-latest 并交叉编译 x86_64 ([#9](https://github.com/shenjingnan/dsh-work/pull/9))

### Other

- 忽略 .claude/worktrees 目录 ([#12](https://github.com/shenjingnan/dsh-work/pull/12))
- *(ci)* release-plz 改用 GitHub App token ([#10](https://github.com/shenjingnan/dsh-work/pull/10))
- 为 GitHub Actions 工作流添加 Rust 构建缓存 ([#7](https://github.com/shenjingnan/dsh-work/pull/7))

## [0.1.1](https://github.com/shenjingnan/dsh-work/compare/v0.1.0...v0.1.1) - 2026-08-22

### Added

- 演进为 DeepSeek Harness 桌面应用（Tauri 2 + 内置运行时） ([#4](https://github.com/shenjingnan/dsh-work/pull/4))

## [0.1.0] - 2026-06-05

### Added

- 项目初始化
- CLI 骨架（clap + tokio）
- 配置管理（TOML 配置读写）
- 双层日志系统（tracing）
- 日期时间工具模块
- CI/CD 配置（GitHub Actions）
- 代码质量工具（fmt, clippy, typos, tarpaulin, codecov）
- Shell 补全生成
