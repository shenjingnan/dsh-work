//! 首启种子化：把安装包内置的插件市场 profile 种子落位到用户 DSH_HOME。
//!
//! 种子由 `scripts/fetch-plugins.sh` 在构建期生成（`resources/profile-seed/web`，
//! 即一个已装好 dshmarket 的 profiles/web），应用首次启动时若检测到用户尚未
//! 初始化 profile，则整体复制过去，开箱即有插件市场。设计要点：
//!
//! - 绝不覆盖：`profiles/web/package.json` 已存在（老用户 / dsh 已自初始化）或
//!   目录非空时直接跳过，用户数据优先。
//! - 原子落位：先复制到 DSH_HOME 下的点前缀临时目录，再 rename 成
//!   `profiles/web`。dsh 以 package.json 存在与否判定 profile 是否初始化，
//!   半成品目录会卡住判定；中断残留的临时目录下次启动先清理再重试。
//! - 失败降级：任何错误向上返回，由调用方记录告警后照常启动 dsh（其会自行
//!   initProfile，应用无插件市场但仍可用——主链路不依赖种子化成功）。

use std::fs;
use std::path::Path;

/// 种子在资源目录中的位置（tauri.conf.json 映射 resources/profile-seed → profile-seed/）。
const SEED_SUBDIR: &str = "profile-seed/web";
/// 落位过程中的临时目录（DSH_HOME 根下，dsh 不会把它当 profile 枚举）。
const TMP_DIR: &str = ".profile-seed-tmp";

/// 尝试把内置插件市场种子落位到 `<dsh_home>/profiles/web`。
///
/// 返回是否实际完成种子化；种子缺失、profile 已初始化或含用户内容时返回
/// `Ok(false)`，复制失败返回 `Err`（调用方降级处理）。
pub fn try_seed(resource_dir: &Path, dsh_home: &Path) -> anyhow::Result<bool> {
    let seed = resource_dir.join(SEED_SUBDIR);
    if !seed.is_dir() {
        return Ok(false);
    }
    let profile_dir = dsh_home.join("profiles").join("web");
    if profile_dir.join("package.json").exists() {
        return Ok(false);
    }
    // 无 package.json 但非空的目录是用户自建内容（dsh 的 initProfile 只在
    // package.json 缺失时写入，删掉这类目录会丢数据），交由 dsh 自行处理
    if profile_dir.is_dir() && fs::read_dir(&profile_dir)?.next().is_some() {
        return Ok(false);
    }

    let tmp = dsh_home.join(TMP_DIR);
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?; // 上次落位中断的残留
    }
    copy_tree(&seed, &tmp)?;
    fs::create_dir_all(dsh_home.join("profiles"))?;
    // 此处只可能是空目录（非空已被上面的守卫拦截），移除后原子落位
    if profile_dir.is_dir() {
        fs::remove_dir(&profile_dir)?;
    }
    fs::rename(&tmp, &profile_dir)?;
    Ok(true)
}

/// 递归复制目录树（文件用 `fs::copy`，保留可执行位等权限）。
///
/// 种子在构建期已断言零符号链接；运行期遇到符号链接条目说明安装包损坏或
/// 平台资源处理不保真，返回 Err 走降级，避免复制出语义错误的链接副本。
fn copy_tree(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_symlink() {
            anyhow::bail!("种子含符号链接（安装包可能损坏）: {}", from.display());
        }
        if file_type.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)
                .map_err(|e| anyhow::anyhow!("复制 {} 失败: {e}", from.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 每个用例独立的临时目录（对齐 runtime.rs 测试的手写风格，不引入 tempfile）。
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dsh-work-seed-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 最小可用种子：package.json（声明 dshmarket bundle）+ node_modules/dshmarket。
    fn make_seed(root: &Path) {
        let seed = root.join("profile-seed/web");
        fs::create_dir_all(seed.join("node_modules/dshmarket")).unwrap();
        fs::write(
            seed.join("package.json"),
            r#"{ "dependencies": { "dshmarket": "1.18.1" },
                "dsh": { "profile": { "bundles": ["dshmarket"] } } }"#,
        )
        .unwrap();
        fs::write(seed.join("node_modules/dshmarket/client.js"), "// market").unwrap();
    }

    #[test]
    fn seeds_fresh_home() {
        let root = scratch("fresh");
        make_seed(&root);
        let dsh_home = root.join("home");
        fs::create_dir_all(&dsh_home).unwrap();

        assert!(try_seed(&root, &dsh_home).unwrap());
        assert!(dsh_home.join("profiles/web/package.json").is_file());
        assert_eq!(
            fs::read_to_string(dsh_home.join("profiles/web/node_modules/dshmarket/client.js"))
                .unwrap(),
            "// market"
        );
        assert!(!dsh_home.join(TMP_DIR).exists(), "临时目录应已 rename 消失");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_when_profile_initialized() {
        let root = scratch("initialized");
        make_seed(&root);
        let profile = root.join("home/profiles/web");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("package.json"), "{\"user\": true}").unwrap();

        assert!(!try_seed(&root, &root.join("home")).unwrap());
        assert_eq!(
            fs::read_to_string(profile.join("package.json")).unwrap(),
            "{\"user\": true}",
            "已初始化的 profile 绝不覆盖"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_when_seed_missing() {
        let root = scratch("no-seed");
        let dsh_home = root.join("home");
        fs::create_dir_all(&dsh_home).unwrap();

        assert!(!try_seed(&root, &dsh_home).unwrap());
        assert!(!dsh_home.join("profiles/web").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn replaces_empty_profile_dir() {
        let root = scratch("empty-dir");
        make_seed(&root);
        let profile = root.join("home/profiles/web");
        fs::create_dir_all(&profile).unwrap(); // dsh 可能已建骨架目录

        assert!(try_seed(&root, &root.join("home")).unwrap());
        assert!(profile.join("package.json").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_non_empty_dir_without_manifest() {
        let root = scratch("user-content");
        make_seed(&root);
        let profile = root.join("home/profiles/web");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("user-notes.txt"), "用户自建内容").unwrap();

        assert!(!try_seed(&root, &root.join("home")).unwrap());
        assert!(profile.join("user-notes.txt").is_file(), "用户内容不被动");
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_in_seed_fails_without_partial_profile() {
        let root = scratch("symlink");
        make_seed(&root);
        std::os::unix::fs::symlink("/etc/hosts", root.join("profile-seed/web/bad-link")).unwrap();
        let dsh_home = root.join("home");
        fs::create_dir_all(&dsh_home).unwrap();

        assert!(try_seed(&root, &dsh_home).is_err());
        assert!(
            !dsh_home.join("profiles/web").exists(),
            "失败不得留下半成品 profile"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
