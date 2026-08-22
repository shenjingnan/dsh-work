//! 运行时资源定位：内置 node / dsh / pnpm 的路径解析与子进程环境构造。
//!
//! 打包后资源布局（macOS `.app/Contents/` 为例，其他平台同理）：
//! - `MacOS/node`                — externalBin sidecar，与主可执行文件同目录
//! - `Resources/dsh/node_modules/@deepseek-ai/dsh/lib/bin.js` — 预装 dsh 包
//! - `Resources/pnpm/`           — pnpm 单文件分发 + shim
//!
//! 开发期（`cargo tauri dev`，未跑 fetch 脚本）回退到系统 PATH 上的 node/dsh。

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// dsh 运行时的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSource {
    /// 安装包内置的运行时。
    Bundled,
    /// 开发期回退：使用系统 PATH 上的 node 与 dsh。
    SystemFallback,
}

/// 解析后的运行时路径。
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    /// node 可执行文件路径（回退模式下为裸命令名 "node"，由 PATH 解析）。
    pub node: PathBuf,
    /// dsh 入口 bin.js 路径。
    pub dsh_entry: PathBuf,
    /// 需要 prepend 到子进程 PATH 的目录（pnpm 目录、node 所在目录）。
    pub extra_path_dirs: Vec<PathBuf>,
    /// 运行时来源。
    pub source: RuntimeSource,
}

/// 从 dsh stdout 的一行中解析就绪地址。
///
/// dsh web 就绪时输出形如 `dsh web: http://127.0.0.1:53044`。
pub fn parse_dsh_url(line: &str) -> Option<String> {
    let start = line.find("http://")?;
    let rest = &line[start..];
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let url = rest[..end].trim_end_matches(['/', '.', ',', ';']);
    let authority = url.strip_prefix("http://")?;
    // 要求 host:port 且端口为数字，避免误匹配日志里的其他 URL 片段
    let port = authority.rsplit(':').next()?;
    if authority.contains(':') && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
        Some(url.to_string())
    } else {
        None
    }
}

/// 构造子进程的 PATH：内置 pnpm 目录与 node 目录在最前，系统 PATH 在后。
pub fn build_child_path(extra_dirs: &[PathBuf], existing: Option<OsString>) -> OsString {
    let mut dirs = extra_dirs.to_vec();
    if let Some(existing) = existing {
        dirs.extend(env::split_paths(&existing));
    }
    env::join_paths(dirs).expect("PATH 中含有非法字符")
}

/// dsh 数据目录：与用户系统已装的 dsh（`~/.dsh`）隔离。
pub fn dsh_home() -> PathBuf {
    env::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dsh-work")
}

/// 定位 node sidecar：externalBin 安装为与主可执行文件同目录的 `node`。
fn bundled_node() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let node = dir.join(name);
    node.exists().then_some(node)
}

/// 在 PATH 上查找名为 `name` 的可执行文件。
fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// canonicalize 出子进程可用的绝对路径。
///
/// Windows 上 `fs::canonicalize` 返回 `\\?\` 扩展长度路径，node 等子进程
/// 无法将其作为入口（CJS realpathSync 解析失败），需剥去该前缀。
fn canonicalize_child(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// 解析运行时。`resource_dir` 为 Tauri 的资源目录。
///
/// 内置资源齐全时走 Bundled；否则回退到系统 PATH（开发模式）。
pub fn resolve_runtime(resource_dir: &Path) -> anyhow::Result<RuntimePaths> {
    let dsh_entry = resource_dir.join("dsh/node_modules/@deepseek-ai/dsh/lib/bin.js");
    let pnpm_dir = resource_dir.join("pnpm");

    if let (Some(node), true, true) = (bundled_node(), dsh_entry.exists(), pnpm_dir.exists()) {
        let node_dir = node.parent().map(Path::to_path_buf).unwrap_or_default();
        return Ok(RuntimePaths {
            node,
            dsh_entry: canonicalize_child(&dsh_entry),
            extra_path_dirs: vec![pnpm_dir, node_dir],
            source: RuntimeSource::Bundled,
        });
    }

    // 开发回退：系统 node + PATH 上的 dsh（dsh 可执行文件即 bin.js 的符号链接）
    let dsh_bin = find_on_path(if cfg!(windows) { "dsh.cmd" } else { "dsh" }).ok_or_else(|| {
        anyhow::anyhow!("未找到内置 dsh 资源，且系统 PATH 上也没有 dsh（开发模式请先安装 dsh）")
    })?;
    let dsh_entry = canonicalize_child(&dsh_bin);
    Ok(RuntimePaths {
        node: PathBuf::from("node"),
        dsh_entry,
        extra_path_dirs: Vec::new(),
        source: RuntimeSource::SystemFallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ready_line() {
        assert_eq!(
            parse_dsh_url("dsh web: http://127.0.0.1:53044"),
            Some("http://127.0.0.1:53044".to_string())
        );
    }

    #[test]
    fn parse_ready_line_with_trailing_chars() {
        assert_eq!(
            parse_dsh_url("dsh web: http://127.0.0.1:8080/"),
            Some("http://127.0.0.1:8080".to_string())
        );
        assert_eq!(
            parse_dsh_url("  dsh web: http://127.0.0.1:1234  "),
            Some("http://127.0.0.1:1234".to_string())
        );
    }

    #[test]
    fn reject_lines_without_url() {
        assert_eq!(parse_dsh_url("dsh: initialized profile web"), None);
        assert_eq!(parse_dsh_url(""), None);
    }

    #[test]
    fn reject_url_without_port() {
        assert_eq!(parse_dsh_url("visit http://example.com for docs"), None);
        assert_eq!(parse_dsh_url("http://127.0.0.1:abc"), None);
    }

    #[test]
    fn child_path_prepends_extra_dirs() {
        let extra = vec![PathBuf::from("/res/pnpm"), PathBuf::from("/res/bin")];
        let existing = env::join_paths([PathBuf::from("/usr/bin")]).unwrap();
        let joined = build_child_path(&extra, Some(existing));
        let dirs: Vec<_> = env::split_paths(&joined).collect();
        assert_eq!(dirs[0], PathBuf::from("/res/pnpm"));
        assert_eq!(dirs[1], PathBuf::from("/res/bin"));
        assert_eq!(dirs[2], PathBuf::from("/usr/bin"));
    }

    #[test]
    fn child_path_without_existing() {
        let extra = vec![PathBuf::from("/res/pnpm")];
        let joined = build_child_path(&extra, None);
        let dirs: Vec<_> = env::split_paths(&joined).collect();
        assert_eq!(dirs, vec![PathBuf::from("/res/pnpm")]);
    }

    #[test]
    fn canonicalized_entry_has_no_extended_prefix() {
        let dir =
            std::env::temp_dir().join(format!("dsh-work-canonicalize-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("bin.js");
        std::fs::write(&file, "// test").unwrap();

        let resolved = canonicalize_child(&file);

        let s = resolved.to_string_lossy();
        assert!(
            !s.starts_with(r"\\?\"),
            r"子进程不接受 \\?\ 扩展路径前缀: {s}"
        );
        assert!(resolved.is_absolute(), "应保持绝对路径: {s}");
        assert!(resolved.ends_with("bin.js"), "应仍指向原文件: {s}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dsh_home_is_isolated() {
        let home = dsh_home();
        assert!(home.ends_with(".dsh-work"));
        assert_ne!(home, env::home_dir().unwrap().join(".dsh"));
    }
}
