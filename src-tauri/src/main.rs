//! DSHWork（dsh-work 桌面应用）入口：拉起内置 dsh web 服务，单窗口加载本地 Web UI。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod process;
mod runtime;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use process::{DshHandle, DshStatus};
use runtime::RuntimePaths;
use serde::Serialize;
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

/// dsh 进程句柄的共享容器（应用状态与信号处理器共用）。
type SharedDsh = Arc<Mutex<Option<DshHandle>>>;

/// 供 SIGTERM/SIGINT 处理器清理子进程的全局引用（setup 时填入）。
static EXIT_HOOK: OnceLock<SharedDsh> = OnceLock::new();

/// 应用级状态：dsh 进程句柄 + 运行时路径（重启时复用）。
struct AppState {
    dsh: SharedDsh,
    runtime: RuntimePaths,
    dsh_home: PathBuf,
}

/// 前端轮询的响应：三态（就绪 / 失败 / 启动中）。
#[derive(Serialize)]
struct ServerUrlResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ServerUrlResponse {
    fn pending() -> Self {
        Self {
            url: None,
            error: None,
        }
    }
}

#[tauri::command]
fn server_url(state: tauri::State<'_, AppState>) -> ServerUrlResponse {
    let guard = state.dsh.lock().unwrap();
    match guard.as_ref().map(DshHandle::status) {
        Some(DshStatus::Ready(url)) => ServerUrlResponse {
            url: Some(url),
            error: None,
        },
        Some(DshStatus::Failed(error)) => ServerUrlResponse {
            url: None,
            error: Some(error),
        },
        _ => ServerUrlResponse::pending(),
    }
}

#[tauri::command]
fn restart_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.dsh.lock().unwrap();
    if let Some(old) = guard.take() {
        old.kill();
    }
    let handle = DshHandle::spawn(&state.runtime, &state.dsh_home).map_err(|e| e.to_string())?;
    *guard = Some(handle);
    Ok(())
}

/// 创建主窗口（无原生标题栏，参考 zapmomo 的分平台处理）。
///
/// - macOS：透明标题栏 + 隐藏标题文字，保留系统红绿灯；标题栏区域由系统原生承担拖拽，
///   窗口背景设为白色与 loading 页一致（跳转到 dsh 页面后该区域仍可拖动，无需注入）。
/// - Windows：去掉系统标题栏；同时关 DWM shadow（undecorated+shadow 在 Win10 会被
///   DWM 画成左右底三边黑框），loading 页用 CSS 自绘边框。
/// - Linux：去掉系统标题栏。
/// - 非 macOS：注入 titlebar.js。窗口就绪后跳转到 dsh web 页面（127.0.0.1），其 DOM
///   不受本仓库控制，拖拽条与窗口三键由注入脚本绘制；本地 loading 页自带控件会被跳过。
fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DSHWork")
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 600.0)
        .background_color(tauri::utils::config::Color(255, 255, 255, 255));

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(TitleBarStyle::Transparent)
            .hidden_title(true);
    }
    #[cfg(target_os = "windows")]
    {
        builder = builder.decorations(false).shadow(false);
    }
    #[cfg(target_os = "linux")]
    {
        builder = builder.decorations(false);
    }
    #[cfg(not(target_os = "macos"))]
    {
        builder = builder.initialization_script(include_str!("titlebar.js"));
    }

    builder.build()?;
    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 重复启动时激活已有窗口，不出现第二个实例
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            build_main_window(app)?;
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let runtime = runtime::resolve_runtime(&resource_dir).map_err(|e| {
                tracing::error!("运行时解析失败: {e:#}");
                e
            })?;
            tracing::info!(
                "运行时来源: {:?}, dsh 入口: {}",
                runtime.source,
                runtime.dsh_entry.display()
            );

            let dsh_home = runtime::dsh_home();
            let handle = DshHandle::spawn(&runtime, &dsh_home).map_err(|e| {
                tracing::error!("dsh 拉起失败: {e}");
                e
            })?;
            let dsh: SharedDsh = Arc::new(Mutex::new(Some(handle)));
            // SIGTERM/SIGINT 不走 Tauri 事件循环，单独注册处理器保证子进程被清理
            let _ = EXIT_HOOK.set(Arc::clone(&dsh));
            ctrlc::set_handler(|| {
                if let Some(dsh) = EXIT_HOOK.get()
                    && let Some(handle) = dsh.lock().unwrap().take()
                {
                    handle.kill();
                }
                std::process::exit(0);
            })
            .expect("注册信号处理器失败");
            app.manage(AppState {
                dsh,
                runtime,
                dsh_home,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![server_url, restart_server])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 退出时确保 dsh 子进程被杀死，不残留
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit)
                && let Some(state) = app.try_state::<AppState>()
                && let Some(handle) = state.dsh.lock().unwrap().take()
            {
                handle.kill();
            }
        });
}
