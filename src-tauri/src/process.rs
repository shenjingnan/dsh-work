//! dsh web 子进程的生命周期管理：拉起、就绪探测、失败诊断、退出清理。

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::{RuntimePaths, build_child_path, parse_dsh_url};

/// stderr 环形缓冲容量（行进），用于启动失败时回填诊断信息。
const STDERR_TAIL_LINES: usize = 20;

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
    /// 启动失败，附诊断信息（stderr 尾部）。
    Failed(String),
}

struct Shared {
    status: Mutex<DshStatus>,
    ready: Condvar,
    stderr_tail: Mutex<VecDeque<String>>,
}

/// 一个运行中的 dsh web 进程。析构时自动杀死子进程。
pub struct DshHandle {
    child: Mutex<Child>,
    shared: Arc<Shared>,
}

impl DshHandle {
    /// 以解析出的运行时拉起 `dsh web --no-open --host 127.0.0.1 --port 0`。
    pub fn spawn(runtime: &RuntimePaths, dsh_home: &Path) -> std::io::Result<Self> {
        let args = [
            runtime.dsh_entry.to_string_lossy().into_owned(),
            "web".into(),
            "--no-open".into(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            "0".into(),
        ];
        let path = build_child_path(&runtime.extra_path_dirs, std::env::var_os("PATH"));
        Self::spawn_inner(&runtime.node, &args, dsh_home, path)
    }

    /// 可注入 program/args 的内部实现，便于测试（用 sh 模拟子进程）。
    fn spawn_inner(
        program: &Path,
        args: &[String],
        dsh_home: &Path,
        path: std::ffi::OsString,
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
        });

        // stdout：扫描就绪地址
        let stdout = child.stdout.take().expect("stdout 已 piped");
        let shared_out = Arc::clone(&shared);
        std::thread::spawn(move || {
            let mut exited_without_ready = true;
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Some(url) = parse_dsh_url(&line) {
                    let mut status = shared_out.status.lock().unwrap();
                    *status = DshStatus::Ready(url);
                    shared_out.ready.notify_all();
                    exited_without_ready = false;
                    // 就绪后继续读，避免管道写满阻塞子进程
                }
            }
            if exited_without_ready {
                let mut status = shared_out.status.lock().unwrap();
                if matches!(*status, DshStatus::Starting) {
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
                DshStatus::Starting => {
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

    /// 杀死子进程并回收。
    pub fn kill(&self) {
        let mut child = self.child.lock().unwrap();
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
        }
        let _ = child.wait();
    }
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

    fn sh_handle(script: &str) -> DshHandle {
        let home = std::env::temp_dir().join(format!("dsh-work-test-{}", std::process::id()));
        DshHandle::spawn_inner(
            Path::new("sh"),
            &["-c".to_string(), script.to_string()],
            &home,
            std::env::var_os("PATH").unwrap_or_default(),
        )
        .expect("spawn sh")
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
        let handle = DshHandle::spawn_inner(
            Path::new("sh"),
            &["-c".to_string(), "exit 0".to_string()],
            &home,
            std::env::var_os("PATH").unwrap_or_default(),
        )
        .unwrap();
        assert!(home.exists());
        drop(handle);
        let _ = std::fs::remove_dir_all(home);
        let _ = PathBuf::new();
    }
}
