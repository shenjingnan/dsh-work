//! DSHWork（dsh-work 桌面应用）入口：拉起内置 dsh web 服务，单窗口加载本地 Web UI。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod process;
mod runtime;
mod seed;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use process::{DshHandle, DshStatus};
use runtime::{RuntimePaths, RuntimeSource};
use serde::Serialize;
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{Listener, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

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

/// 前端轮询的响应：四态（就绪 / 重启等待 / 失败 / 启动中）。
#[derive(Serialize)]
struct ServerUrlResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// 服务曾就绪后死亡，正在等待插件市场自重启的替代进程接管原端口。
    restarting: bool,
}

impl ServerUrlResponse {
    fn pending() -> Self {
        Self {
            url: None,
            error: None,
            restarting: false,
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
            restarting: false,
        },
        Some(DshStatus::Restarting(_)) => ServerUrlResponse {
            url: None,
            error: None,
            restarting: true,
        },
        Some(DshStatus::Failed(error)) => ServerUrlResponse {
            url: None,
            error: Some(error),
            restarting: false,
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
    write_server_port(&state.dsh_home, handle.port());
    *guard = Some(handle);
    Ok(())
}

/// 记录本次 dsh web 固定端口的文件（DSH_HOME 下的应用私有标记）。
fn server_port_file(dsh_home: &std::path::Path) -> std::path::PathBuf {
    dsh_home.join(".dsh-work-server-port")
}

/// 持久化端口：应用被强杀（kill -9）无法清理自重启替代进程时，下次启动凭此清理。
fn write_server_port(dsh_home: &std::path::Path, port: u16) {
    if let Err(e) = std::fs::write(server_port_file(dsh_home), port.to_string()) {
        tracing::warn!("记录服务端口失败（不影响运行）: {e}");
    }
}

/// 清理上次运行残留的替代进程：应用退出路径（窗口关闭/SIGTERM）会随 DshHandle
/// 清理端口，但强杀场景只能等下次启动时按记录的端口补杀。
fn clean_stale_server(dsh_home: &std::path::Path) {
    let file = server_port_file(dsh_home);
    if let Ok(text) = std::fs::read_to_string(&file)
        && let Ok(port) = text.trim().parse::<u16>()
    {
        tracing::info!("清理上次运行可能残留的服务端口 {port}");
        process::kill_port_owner(port);
    }
    let _ = std::fs::remove_file(&file);
}

/// 创建主窗口（标题栏分平台处理：Windows 用系统原生，其余平台自定义头部）。
///
/// - macOS：透明标题栏 + 隐藏标题文字，保留系统红绿灯；标题栏区域由系统原生承担拖拽，
///   窗口背景设为白色与 loading 页一致（跳转到 dsh 页面后该区域仍可拖动，无需注入）。
/// - Windows：保留系统原生标题栏（默认 decorations），拖拽/三键/边缘 resize/snap
///   全部交给系统；仅启用注入脚本的下载 toast。
/// - Linux：去掉系统标题栏并注入标记脚本，拖拽条与窗口三键由 titlebar.js 绘制：
///   窗口就绪后跳转到 dsh web 页面（127.0.0.1 随机端口），其 DOM 不受本仓库控制，
///   标题栏透明融入页面（无背景无边框，三键颜色随页面深浅主题自适应），并给页面 html
///   注入等高 padding 让顶部内容完整下移、不被遮挡；IPC 授权见
///   capabilities/remote-dsh.json（URL 模式需带 :* 端口通配）。
/// - 三平台注入 titlebar.js：下载 toast 全平台需要；自定义标题栏部分仅在该 Linux
///   标记存在时启用（脚本内判定，避免 Windows 原生标题栏上再叠一层）。
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
    #[cfg(target_os = "linux")]
    {
        builder = builder
            .decorations(false)
            // 标记脚本先于 titlebar.js 注入（initialization_script 按注册顺序执行）：
            // titlebar.js 三平台注入（下载 toast 需要），自定义标题栏部分仅在该
            // 标记存在时启用，避免 Windows 原生标题栏上再叠一层
            .initialization_script("window.__DSH_LINUX_TITLEBAR__ = true;");
    }
    // 三平台注入 titlebar.js：下载 toast 全平台生效，标题栏绘制由上面的标记分流
    builder = builder.initialization_script(include_str!("titlebar.js"));

    // 下载处理：不注册则 macOS/Linux 完全无法下载（wry 不接管），注册后三平台
    // 统一落系统下载目录，完成后由 titlebar.js 弹 toast（详见 download.rs）。
    builder = builder.on_download({
        let names = Arc::new(download::DownloadNames::default());
        move |webview, event| download::on_download(&names, &webview, event)
    });

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
            // toast 的“打开文件夹”按钮经事件触发（remote 页面调应用自有命令会被 ACL 拒绝）
            let opener_handle = app.handle().clone();
            app.listen_any(download::EVENT_OPEN_DOWNLOADS_DIR, move |_| {
                download::open_downloads_dir(&opener_handle);
            });
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
            // 清理上次强杀可能残留的自重启替代进程（按记录端口补杀）
            clean_stale_server(&dsh_home);
            // 首启种子化内置插件市场（仅安装包内置运行时的场景；开发回退模式无种子，
            // try_seed 返回 Ok(false) 自然跳过）。失败仅告警：dsh 会自行 initProfile，
            // 应用无插件市场但主链路不受影响。
            if runtime.source == RuntimeSource::Bundled {
                match seed::try_seed(&resource_dir, &dsh_home) {
                    Ok(true) => tracing::info!("已种子化内置插件市场（profiles/web）"),
                    Ok(false) => {}
                    Err(e) => {
                        tracing::warn!("插件市场种子化失败，降级为 dsh 自动初始化: {e:#}")
                    }
                }
            }
            let handle = DshHandle::spawn(&runtime, &dsh_home).map_err(|e| {
                tracing::error!("dsh 拉起失败: {e}");
                e
            })?;
            write_server_port(&dsh_home, handle.port());
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
