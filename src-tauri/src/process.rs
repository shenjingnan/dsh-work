//! dsh web 子进程的生命周期管理：拉起、就绪探测、失败诊断、退出清理、死亡收养。

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::{RuntimePaths, build_child_path, parse_dsh_url};

/// stderr 环形缓冲容量（行进），用于启动失败时回填诊断信息。
const STDERR_TAIL_LINES: usize = 20;

/// 收养探测窗口：覆盖 dshmarket 自重启 helper 的最长等待（30s）加启动余量。
const ADOPT_TIMEOUT: Duration = Duration::from_secs(35);
/// 收养探测间隔。
const ADOPT_INTERVAL: Duration = Duration::from_millis(500);

/// Windows `CREATE_NO_WINDOW` 进程创建标志：GUI 子系统进程启动控制台子进程
/// （node.exe）时，系统默认为其新建一个可见的命令行窗口，此标志阻止该窗口。
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// dsh web 服务状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DshStatus {
    /// 已拉起，尚未输出就绪地址。
    Starting,
    /// 就绪，附本地服务地址。
    Ready(String),
    /// 曾就绪的服务已死亡，正在等待替代进程接管原端口（插件市场自重启场景）。
    Restarting(String),
    /// 启动失败（或替代进程未在窗口内接管），附诊断信息。
    Failed(String),
}

/// 收养探测参数（生产默认与测试注入两套）。
#[derive(Clone, Copy)]
struct AdoptConfig {
    timeout: Duration,
    interval: Duration,
}

impl Default for AdoptConfig {
    fn default() -> Self {
        Self {
            timeout: ADOPT_TIMEOUT,
            interval: ADOPT_INTERVAL,
        }
    }
}

struct Shared {
    status: Mutex<DshStatus>,
    ready: Condvar,
    stderr_tail: Mutex<VecDeque<String>>,
    /// stderr 线程排空标志（含条件变量）：诊断尾部的读取与其写入跨线程同步。
    stderr_done: Mutex<bool>,
    stderr_done_cv: Condvar,
    /// dsh web 监听端口（固定，自重启的替代进程会重放同一端口）。
    port: u16,
    /// 收养探测的取消标志：kill 时置位，探测线程随之退出。
    adopt_cancelled: AtomicBool,
    adopt: AdoptConfig,
}

/// 等待 stderr 线程排空（进程退出后 stderr 必 EOF，正常必达；超时仅防御僵死）。
fn wait_stderr_drained(shared: &Shared) {
    let mut done = shared.stderr_done.lock().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !*done {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            tracing::warn!("等待 stderr 排空超时，诊断尾部可能不完整");
            return;
        }
        let (guard, _) = shared.stderr_done_cv.wait_timeout(done, remaining).unwrap();
        done = guard;
    }
}

/// 一个运行中的 dsh web 进程。析构时自动杀死子进程；若服务已被插件市场的
/// 自重启替代进程接管，退出清理同样覆盖该端口。
pub struct DshHandle {
    child: Mutex<Child>,
    shared: Arc<Shared>,
}

/// 在回环地址上挑一个当前空闲的端口。
///
/// dshmarket 的自重启机制原样重放启动 argv：只有固定端口，替代进程才会回到
/// 同一端口，壳才能探测收养（`--port 0` 随机端口会让替代进程漂移到新端口）。
/// bind 后即释放，与子进程 bind 之间存在微小竞态窗口，失败由就绪探测兜底。
pub fn pick_free_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    Ok(listener.local_addr()?.port())
}

/// 杀死当前监听 `port` 的进程（尽力而为，用于清理自重启的替代进程）。
///
/// 替代进程由 dshmarket 的 detached helper 拉起，不是本应用的子进程，只能按
/// 端口反查 PID：unix 走 lsof，Windows 走 netstat 解析。查无占用或工具缺失
/// 均静默返回（端口本就无人监听是常态）。
pub fn kill_port_owner(port: u16) {
    let pids: Vec<u32> = {
        #[cfg(unix)]
        {
            let Ok(output) = Command::new("lsof")
                .args(["-ti", &format!("tcp:{port}"), "-sTCP:LISTEN"])
                .output()
            else {
                return;
            };
            if !output.status.success() {
                return; // lsof 以非零退出表示无监听者
            }
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect()
        }
        #[cfg(windows)]
        {
            let mut command = Command::new("netstat");
            command
                .args(["-ano", "-p", "TCP"])
                .creation_flags(CREATE_NO_WINDOW);
            let Ok(output) = command.output() else { return };
            let needle = format!("127.0.0.1:{port}");
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| {
                    let cols: Vec<_> = line.split_whitespace().collect();
                    cols.len() >= 5
                        && cols[0] == "TCP"
                        && cols[1] == needle
                        && cols[3].eq_ignore_ascii_case("LISTENING")
                })
                .filter_map(|line| line.split_whitespace().nth(4).and_then(|p| p.parse().ok()))
                .collect()
        }
    };
    for pid in pids {
        // 永不杀自己：测试用本进程 listener 模拟替代进程，且防御生产环境下
        // 端口反查意外命中本应用的任何边界情形
        if pid == std::process::id() {
            continue;
        }
        // 按端口反查到的监听者只可能是 dsh web（回环 + 应用私有端口约定），
        // 误杀风险集中在端口被无关进程复用的窗口期，仅记录告警。
        match kill_process(pid) {
            Ok(()) => tracing::info!("已终止端口 {port} 的接管进程（pid {pid}）"),
            Err(e) => tracing::warn!("终止端口 {port} 的接管进程（pid {pid}）失败: {e}"),
        }
    }
}

/// 跨平台终止进程（非子进程只能按 PID 发信号）。
#[cfg(unix)]
fn kill_process(pid: u32) -> std::io::Result<()> {
    // SIGTERM 与 dshmarket 自重启杀宿主的方式一致，给进程清理机会
    let status = Command::new("kill").arg(pid.to_string()).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("kill 退出码 {status}")))
    }
}

/// Windows：taskkill /PID（force 以处理无窗口控制台进程）。
#[cfg(windows)]
fn kill_process(pid: u32) -> std::io::Result<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("taskkill 退出码 {status}")))
    }
}

impl DshHandle {
    /// 以解析出的运行时拉起 `dsh web --no-open --host 127.0.0.1 --port <固定>`。
    ///
    /// 端口固定是插件市场自重启的前提（见 [`pick_free_port`]）。成功后通过
    /// [`DshHandle::port`] 获取实际端口。
    pub fn spawn(runtime: &RuntimePaths, dsh_home: &Path) -> std::io::Result<Self> {
        let port = pick_free_port()?;
        let args = [
            runtime.dsh_entry.to_string_lossy().into_owned(),
            "web".into(),
            "--no-open".into(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
        ];
        let path = build_child_path(&runtime.extra_path_dirs, std::env::var_os("PATH"));
        Self::spawn_inner(
            &runtime.node,
            &args,
            dsh_home,
            path,
            port,
            AdoptConfig::default(),
        )
    }

    /// dsh web 的固定监听端口。
    pub fn port(&self) -> u16 {
        self.shared.port
    }

    /// 可注入 program/args 的内部实现，便于测试（用 sh 模拟子进程）。
    #[allow(clippy::too_many_arguments)]
    fn spawn_inner(
        program: &Path,
        args: &[String],
        dsh_home: &Path,
        path: std::ffi::OsString,
        port: u16,
        adopt: AdoptConfig,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(dsh_home)?;
        let mut command = Command::new(program);
        command
            .args(args)
            .env("DSH_HOME", dsh_home)
            .env("PATH", path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Windows：桌面应用为 GUI 子系统（无控制台），拉起 node（控制台程序）时系统
        // 会为其新建可见命令行窗口；stdout/stderr 均已管道重定向，隐藏窗口不影响输出。
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let mut child = command.spawn()?;

        let shared = Arc::new(Shared {
            status: Mutex::new(DshStatus::Starting),
            ready: Condvar::new(),
            stderr_tail: Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LINES + 1)),
            stderr_done: Mutex::new(false),
            stderr_done_cv: Condvar::new(),
            port,
            adopt_cancelled: AtomicBool::new(false),
            adopt,
        });

        // stdout：扫描就绪地址
        let stdout = child.stdout.take().expect("stdout 已 piped");
        let shared_out = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut ready_url: Option<String> = None;
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(url) = parse_dsh_url(&line) {
                    let mut status = shared_out.status.lock().unwrap();
                    *status = DshStatus::Ready(url.clone());
                    shared_out.ready.notify_all();
                    ready_url = Some(url);
                    // 就绪后继续读，避免管道写满阻塞子进程
                }
            }
            // 进程退出（stdout EOF）。从未就绪 → 启动失败；曾就绪 → 进入收养等待
            // （dshmarket 自重启：detached helper 会在同一端口拉起替代进程）。
            // 先等 stderr 线程排空（进程退出后 stderr 随之 EOF，必有终点）：诊断
            // 尾部的读取与 stderr 写入跨线程，无同步会读到空尾。
            wait_stderr_drained(&shared_out);
            let mut status = shared_out.status.lock().unwrap();
            match (ready_url, &*status) {
                (None, DshStatus::Starting) => {
                    let tail = shared_out.stderr_tail.lock().unwrap();
                    let detail = if tail.is_empty() {
                        "dsh 进程在就绪前退出，且无 stderr 输出".to_string()
                    } else {
                        format!(
                            "dsh 进程在就绪前退出：\n{}",
                            tail.iter().cloned().collect::<Vec<_>>().join("\n")
                        )
                    };
                    *status = DshStatus::Failed(detail);
                    shared_out.ready.notify_all();
                }
                (Some(url), DshStatus::Ready(current)) if *current == url => {
                    *status = DshStatus::Restarting(url);
                    shared_out.ready.notify_all();
                    drop(status);
                    spawn_adopt_probe(Arc::clone(&shared_out));
                }
                _ => {}
            }
        });

        // stderr：环形缓冲留作诊断
        let stderr = child.stderr.take().expect("stderr 已 piped");
        let shared_err = Arc::clone(&shared);
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                tracing::debug!(target: "dsh", "{}", line);
                let mut tail = shared_err.stderr_tail.lock().unwrap();
                if tail.len() >= STDERR_TAIL_LINES {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
            let mut done = shared_err.stderr_done.lock().unwrap();
            *done = true;
            shared_err.stderr_done_cv.notify_all();
        });

        Ok(Self {
            child: Mutex::new(child),
            shared,
        })
    }

    /// 当前状态快照。
    pub fn status(&self) -> DshStatus {
        self.shared.status.lock().unwrap().clone()
    }

    /// 阻塞等待就绪，超时或失败返回 Err。
    ///
    /// 当前生产路径走前端轮询 `server_url`，此方法供测试与未来的同步等待场景使用。
    #[allow(dead_code)]
    pub fn wait_ready(&self, timeout: Duration) -> Result<String, String> {
        let deadline = Instant::now() + timeout;
        let mut status = self.shared.status.lock().unwrap();
        loop {
            match &*status {
                DshStatus::Ready(url) => return Ok(url.clone()),
                DshStatus::Failed(e) => return Err(e.clone()),
                DshStatus::Starting | DshStatus::Restarting(_) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err(format!(
                            "dsh web 启动超时（{}s 内未就绪）",
                            timeout.as_secs()
                        ));
                    }
                    let (guard, _) = self.shared.ready.wait_timeout(status, remaining).unwrap();
                    status = guard;
                }
            }
        }
    }

    /// 杀死子进程并回收；若端口已被自重启的替代进程接管，一并清理。
    pub fn kill(&self) {
        self.shared.adopt_cancelled.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().unwrap();
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
        // child 死后端口释放，此时端口上的监听者只可能是替代进程（收养态）
        kill_port_owner(self.shared.port);
    }
}

/// 探测原端口是否被 dshmarket 自重启的替代进程接管。
///
/// 探测成功（TCP 可连）则状态回到 `Ready`（同端口即同 URL，前端无感）；
/// 超时转 `Failed`；[`Shared::adopt_cancelled`] 置位（kill）则静默退出。
fn spawn_adopt_probe(shared: Arc<Shared>) {
    std::thread::spawn(move || {
        let addr = SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, shared.port));
        let deadline = Instant::now() + shared.adopt.timeout;
        loop {
            if shared.adopt_cancelled.load(Ordering::SeqCst) {
                return;
            }
            if Instant::now() >= deadline {
                let mut status = shared.status.lock().unwrap();
                if matches!(*status, DshStatus::Restarting(_)) {
                    *status = DshStatus::Failed(
                        "dsh 服务死亡后未在等待窗口内恢复（插件市场自重启未完成）".to_string(),
                    );
                    shared.ready.notify_all();
                }
                return;
            }
            let connected = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok();
            if connected {
                let mut status = shared.status.lock().unwrap();
                if let DshStatus::Restarting(url) = &*status {
                    let url = url.clone();
                    tracing::info!("端口 {} 已被替代进程接管，服务恢复", shared.port);
                    *status = DshStatus::Ready(url);
                    shared.ready.notify_all();
                }
                return;
            }
            std::thread::sleep(shared.adopt.interval);
        }
    });
}

impl Drop for DshHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 快速收养配置：测试不等生产默认的 35s 窗口。
    fn quick_adopt() -> AdoptConfig {
        AdoptConfig {
            timeout: Duration::from_secs(2),
            interval: Duration::from_millis(50),
        }
    }

    fn sh_handle_with(script: &str, port: u16, adopt: AdoptConfig) -> DshHandle {
        let home =
            std::env::temp_dir().join(format!("dsh-work-test-{}-{port}", std::process::id()));
        DshHandle::spawn_inner(
            Path::new("sh"),
            &["-c".to_string(), script.to_string()],
            &home,
            std::env::var_os("PATH").unwrap_or_default(),
            port,
            adopt,
        )
        .expect("spawn sh")
    }

    fn sh_handle(script: &str) -> DshHandle {
        let port = pick_free_port().expect("pick port");
        sh_handle_with(script, port, quick_adopt())
    }

    #[test]
    fn ready_when_url_printed() {
        let handle = sh_handle("echo 'dsh web: http://127.0.0.1:12345'; sleep 5");
        let url = handle.wait_ready(Duration::from_secs(5)).unwrap();
        assert_eq!(url, "http://127.0.0.1:12345");
        assert_eq!(handle.status(), DshStatus::Ready(url));
    }

    #[test]
    fn failed_when_process_exits_early() {
        let handle = sh_handle("echo 'boom: something broke' >&2; exit 1");
        let err = handle.wait_ready(Duration::from_secs(5)).unwrap_err();
        assert!(
            err.contains("boom: something broke"),
            "错误应包含 stderr 尾部: {err}"
        );
    }

    #[test]
    fn timeout_when_never_ready() {
        let handle = sh_handle("sleep 30");
        let err = handle.wait_ready(Duration::from_millis(200)).unwrap_err();
        assert!(err.contains("超时"), "应为超时错误: {err}");
    }

    #[test]
    fn kill_terminates_child() {
        let handle = sh_handle("echo 'dsh web: http://127.0.0.1:1'; sleep 30");
        handle.wait_ready(Duration::from_secs(5)).unwrap();
        handle.kill();
        assert!(handle.child.lock().unwrap().try_wait().unwrap().is_some());
    }

    #[test]
    fn spawn_inner_creates_dsh_home() {
        let home = std::env::temp_dir().join(format!("dsh-work-test-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let port = pick_free_port().expect("pick port");
        let handle = DshHandle::spawn_inner(
            Path::new("sh"),
            &["-c".to_string(), "exit 0".to_string()],
            &home,
            std::env::var_os("PATH").unwrap_or_default(),
            port,
            quick_adopt(),
        )
        .unwrap();
        assert!(home.exists());
        drop(handle);
        let _ = std::fs::remove_dir_all(home);
        let _ = PathBuf::new();
    }

    #[test]
    fn pick_free_port_binds_then_frees() {
        let port = pick_free_port().expect("pick port");
        // 选出的端口应可立即绑定（未被长占）
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port));
        assert!(listener.is_ok(), "端口 {port} 应可绑定");
    }

    #[test]
    fn recovers_when_replacement_takes_over_port() {
        // 模拟 dshmarket 自重启：宿主打印就绪后退出，随后同端口出现替代监听者，
        // 状态应经历 Ready → Restarting → Ready（同 URL）
        let port = pick_free_port().expect("pick port");
        let url = format!("http://127.0.0.1:{port}");
        let handle = sh_handle_with(
            &format!("echo 'dsh web: {url}'; sleep 0.3; exit 0"),
            port,
            quick_adopt(),
        );
        assert_eq!(handle.wait_ready(Duration::from_secs(5)).unwrap(), url);

        // 等宿主退出进入 Restarting
        let deadline = Instant::now() + Duration::from_secs(3);
        while !matches!(handle.status(), DshStatus::Restarting(_)) {
            assert!(Instant::now() < deadline, "应进入 Restarting 状态");
            std::thread::sleep(Duration::from_millis(20));
        }

        // 替代进程接管同端口（模拟 detached helper 拉起的新 dsh web）
        let replacement =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).expect("替代进程监听原端口");
        let server = std::thread::spawn(move || {
            for stream in replacement.incoming() {
                drop(stream);
            }
        });

        // 探测线程应发现端口复活并回到 Ready（URL 不变）
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let DshStatus::Ready(recovered) = handle.status() {
                assert_eq!(recovered, url, "收养后 URL 应保持不变");
                break;
            }
            assert!(Instant::now() < deadline, "应回到 Ready 状态");
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(handle); // kill：同时验证清理路径不 panic
        drop(server);
    }

    #[test]
    fn fails_when_no_replacement_within_window() {
        // 宿主就绪后死亡且无人接管：快速探测窗口内应转 Failed。
        // Ready 窗口极短（就绪行后立即退出），不能依赖 wait_ready 撞上 Ready，
        // 轮询观察状态收敛到 Failed 即可。
        let port = pick_free_port().expect("pick port");
        let url = format!("http://127.0.0.1:{port}");
        let handle = sh_handle_with(
            &format!("echo 'dsh web: {url}'; exit 0"),
            port,
            AdoptConfig {
                timeout: Duration::from_millis(600),
                interval: Duration::from_millis(50),
            },
        );
        let _ = url;

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let DshStatus::Failed(err) = handle.status() {
                assert!(err.contains("窗口"), "失败信息应说明等待窗口: {err}");
                break;
            }
            assert!(Instant::now() < deadline, "应转为 Failed 状态");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn kill_port_owner_is_noop_on_free_port() {
        // 无人监听的端口：静默返回（lsof 非零退出路径），不 panic 不误杀
        let port = pick_free_port().expect("pick port");
        kill_port_owner(port);
    }
}
