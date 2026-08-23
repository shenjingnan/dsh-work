//! 下载事件处理：dsh 页面里触发的下载统一落盘系统“下载”目录，完成后向页面 toast 通知。
//!
//! 背景：不注册下载处理器时仅 Windows（WebView2 自带下载 UI）可用，macOS/Linux 完全
//! 无法下载（wry 无 handler 时不接管下载）。注册 `on_download` 后三平台统一由
//! wry 落盘到默认下载目录（重名自动加 `(n)` 后缀）；代价是 Windows 失去 WebView2
//! 自带下载气泡，改由注入脚本（titlebar.js）弹 toast 反馈，事件授权见
//! capabilities/remote-dsh.json 的 `core:event:default`。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::webview::DownloadEvent;
use tauri::{Emitter, Manager, Runtime, Webview};

/// toast 事件名：下载完成（Rust → 注入脚本）。
pub const EVENT_DOWNLOAD_FINISHED: &str = "download-finished";
/// toast 事件名：请求打开下载目录（注入脚本 → Rust）。
pub const EVENT_OPEN_DOWNLOADS_DIR: &str = "open-downloads-dir";

/// toast 通用兜底文件名（blob:/data: 的 URL 末段是 uuid/数据片段，不适合展示）。
pub const GENERIC_FILENAME: &str = "下载的文件";

/// toast 载荷：文件名 + 成败。
#[derive(Serialize, Clone)]
pub struct DownloadFinishedPayload {
    pub filename: String,
    pub success: bool,
}

/// Requested 与 Finished 之间的文件名登记表。
///
/// `Requested.destination` 在三平台都携带完整落盘路径（macOS 由 wry 预填为
/// 下载目录 + 网页建议文件名），而 `Finished.path` 在 macOS 恒为 `None`
/// （系统 API 限制），因此在 Requested 时记下文件名、Finished 时按 URL 取回。
#[derive(Default)]
pub struct DownloadNames(Mutex<HashMap<String, String>>);

impl DownloadNames {
    /// 登记一次下载的目标文件名。
    pub fn record(&self, url: &str, destination: &Path) {
        let Some(name) = destination.file_name() else {
            return;
        };
        self.0
            .lock()
            .unwrap()
            .insert(url.to_string(), name.to_string_lossy().into_owned());
    }

    /// 取出并移除该 URL 登记的文件名（下载已结束，无论成败都清理）。
    pub fn take(&self, url: &str) -> Option<String> {
        self.0.lock().unwrap().remove(url)
    }
}

/// 由 Finished 的 url/path 推导 toast 要展示的文件名（登记表优先，未命中时兜底）。
pub fn finished_filename(names: &DownloadNames, url: &tauri::Url, path: Option<&Path>) -> String {
    names
        .take(url.as_str())
        .or_else(|| path.and_then(filename_from_path))
        .unwrap_or_else(|| filename_from_url(url))
}

/// 取路径末段作为文件名。
fn filename_from_path(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}

/// 取 URL 路径末段作为文件名；非 http(s) 或末段为空时回退通用名。
fn filename_from_url(url: &tauri::Url) -> String {
    let last = url.path_segments().and_then(Iterator::last).unwrap_or("");
    match url.scheme() {
        "http" | "https" if !last.is_empty() => last.to_string(),
        _ => GENERIC_FILENAME.to_string(),
    }
}

/// `WebviewWindowBuilder::on_download` 的处理器：始终放行下载且不修改 destination
/// （各平台默认落系统下载目录，重名自动加 `(n)` 后缀），Finished 时向页面广播
/// toast 事件。返回 false 会取消下载，这里永不取消。
pub fn on_download<R: Runtime>(
    names: &DownloadNames,
    webview: &Webview<R>,
    event: DownloadEvent<'_>,
) -> bool {
    match event {
        DownloadEvent::Requested { url, destination } => {
            tracing::info!(url = %url, destination = %destination.display(), "下载开始");
            names.record(url.as_str(), destination);
        }
        DownloadEvent::Finished { url, path, success } => {
            let filename = finished_filename(names, &url, path.as_deref());
            tracing::info!(url = %url, filename = %filename, success, "下载结束");
            if let Err(e) = webview.app_handle().emit(
                EVENT_DOWNLOAD_FINISHED,
                DownloadFinishedPayload { filename, success },
            ) {
                tracing::warn!("下载完成事件发送失败: {e}");
            }
        }
        // DownloadEvent 标注 non_exhaustive，未来新增变体（如进度）同样放行
        _ => {}
    }
    true
}

/// 打开系统下载目录（toast 的“打开文件夹”按钮经 `open-downloads-dir` 事件触发）。
/// 不走自定义 command：dsh 页面是 remote origin，应用自有命令会被 ACL 拒绝，
/// 事件通道（core:event:default）已覆盖双向通信。
pub fn open_downloads_dir<R: Runtime>(app: &tauri::AppHandle<R>) {
    let dir = match app.path().download_dir() {
        Ok(dir) => dir,
        Err(e) => {
            tracing::warn!("解析下载目录失败: {e}");
            return;
        }
    };
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    if let Err(e) = std::process::Command::new(program).arg(&dir).spawn() {
        tracing::warn!("打开下载目录失败（{} {}）: {e}", program, dir.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> tauri::Url {
        tauri::Url::parse(s).expect("测试 URL 需合法")
    }

    #[test]
    fn record_then_take_returns_filename() {
        let names = DownloadNames::default();
        names.record(
            "http://127.0.0.1:1/api/export/a.zip",
            Path::new(r"C:\Users\u\Downloads\a.zip"),
        );
        assert_eq!(
            names.take("http://127.0.0.1:1/api/export/a.zip").as_deref(),
            Some("a.zip")
        );
    }

    #[test]
    fn take_removes_entry_after_first_take() {
        let names = DownloadNames::default();
        names.record("http://h/a", Path::new("/tmp/f.txt"));
        names.take("http://h/a");
        assert_eq!(names.take("http://h/a"), None);
    }

    #[test]
    fn take_unknown_url_is_none() {
        assert_eq!(DownloadNames::default().take("missing"), None);
    }

    #[test]
    fn finished_filename_prefers_recorded_name_over_path_and_url() {
        let names = DownloadNames::default();
        names.record("http://h/a", Path::new("/dl/记录.zip"));
        // 登记的文件名优先于 Finished.path 与 URL 末段
        assert_eq!(
            finished_filename(&names, &url("http://h/a"), Some(Path::new("/dl/other.txt"))),
            "记录.zip"
        );
    }

    #[test]
    fn finished_filename_falls_back_to_path_when_not_recorded() {
        let names = DownloadNames::default();
        assert_eq!(
            finished_filename(
                &names,
                &url("http://h/a"),
                Some(Path::new("/dl/report (1).pdf"))
            ),
            "report (1).pdf"
        );
    }

    #[test]
    fn finished_filename_falls_back_to_url_last_segment() {
        let names = DownloadNames::default();
        assert_eq!(
            finished_filename(&names, &url("https://h/api/exports/session.zip"), None),
            "session.zip"
        );
    }

    #[test]
    fn finished_filename_falls_back_to_generic_for_blob_data_or_empty_segment() {
        let names = DownloadNames::default();
        assert_eq!(
            finished_filename(&names, &url("blob:http://h/8b1a2c3d-uuid"), None),
            GENERIC_FILENAME
        );
        assert_eq!(
            finished_filename(&names, &url("data:text/plain,hello"), None),
            GENERIC_FILENAME
        );
        assert_eq!(
            finished_filename(&names, &url("https://h/api/"), None),
            GENERIC_FILENAME
        );
    }
}
