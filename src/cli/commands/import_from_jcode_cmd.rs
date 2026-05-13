use anyhow::Result;
use std::path::{Path, PathBuf};

struct CopyStats {
    copied: usize,
    skipped: usize,
}

pub fn run_import_from_jcode(force: bool) -> Result<()> {
    let target_home = crate::storage::jcode_dir()?;
    let target_config = crate::storage::app_config_dir()?;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
    let config = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("No config directory"))?;

    let source_home = home.join(".jcode");
    let source_config = config.join("jcode");

    if !source_home.exists() && !source_config.exists() {
        anyhow::bail!("No local jcode state found in ~/.jcode or ~/.config/jcode.");
    }

    let mut copied = 0usize;
    let mut skipped = 0usize;

    // Keep migration explicit and bounded. We intentionally avoid runtime sockets/pids.
    let home_items = ["sessions", "auth", "history", "usage", "builds"];
    for item in home_items {
        let src = source_home.join(item);
        let dst = target_home.join(item);
        let stats = copy_path_if_present(&src, &dst, force)?;
        copied += stats.copied;
        skipped += stats.skipped;
    }

    let config_items = ["config.toml", "provider.env"];
    for item in config_items {
        let src = source_config.join(item);
        let dst = target_config.join(item);
        let stats = copy_path_if_present(&src, &dst, force)?;
        copied += stats.copied;
        skipped += stats.skipped;
    }

    println!(
        "Imported jcode state into {} (copied: {}, skipped: {}, force: {}).",
        crate::product::command_name(),
        copied,
        skipped,
        force
    );
    println!(
        "Migration is one-time and opt-in. Active runtime state remains isolated per product."
    );

    Ok(())
}

fn copy_path_if_present(src: &Path, dst: &Path, force: bool) -> Result<CopyStats> {
    if !src.exists() {
        return Ok(CopyStats {
            copied: 0,
            skipped: 0,
        });
    }
    if src.is_file() {
        copy_file(src, dst, force)
    } else if src.is_dir() {
        copy_dir_recursive(src, dst, force)
    } else {
        Ok(CopyStats {
            copied: 0,
            skipped: 1,
        })
    }
}

fn copy_file(src: &Path, dst: &Path, force: bool) -> Result<CopyStats> {
    if dst.exists() && !force {
        return Ok(CopyStats {
            copied: 0,
            skipped: 1,
        });
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src, dst)?;
    Ok(CopyStats {
        copied: 1,
        skipped: 0,
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path, force: bool) -> Result<CopyStats> {
    std::fs::create_dir_all(dst)?;
    let mut copied = 0usize;
    let mut skipped = 0usize;
    let mut stack: Vec<(PathBuf, PathBuf)> = vec![(src.to_path_buf(), dst.to_path_buf())];
    while let Some((current_src, current_dst)) = stack.pop() {
        for entry in std::fs::read_dir(&current_src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = current_dst.join(entry.file_name());
            let ty = entry.file_type()?;
            if ty.is_dir() {
                std::fs::create_dir_all(&dst_path)?;
                stack.push((src_path, dst_path));
                continue;
            }
            if ty.is_file() {
                if dst_path.exists() && !force {
                    skipped += 1;
                    continue;
                }
                if let Some(parent) = dst_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&src_path, &dst_path)?;
                copied += 1;
            }
        }
    }
    Ok(CopyStats { copied, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
        prev: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let prev = std::env::var_os(key);
            crate::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = self.prev.take() {
                crate::env::set_var(self.key, prev);
            } else {
                crate::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn import_from_jcode_copies_selected_state_into_plus_namespace() {
        let _lock = crate::storage::lock_test_env();
        let root = tempfile::TempDir::new().expect("temp dir");
        let home = root.path().join("home");
        let xdg = root.path().join("xdg");
        std::fs::create_dir_all(&home).expect("mkdir home");
        std::fs::create_dir_all(&xdg).expect("mkdir xdg");

        let _home_guard = EnvGuard::set("HOME", &home);
        let _xdg_guard = EnvGuard::set("XDG_CONFIG_HOME", &xdg);
        let _flavor_guard = EnvGuard::set("JCODE_PRODUCT_FLAVOR", Path::new("jcode-plus"));

        let source_home = home.join(".jcode");
        std::fs::create_dir_all(source_home.join("sessions")).expect("mkdir source sessions");
        std::fs::create_dir_all(source_home.join("auth")).expect("mkdir source auth");
        std::fs::write(source_home.join("sessions").join("s1.json"), b"{\"id\":\"s1\"}")
            .expect("write session");
        std::fs::write(source_home.join("auth").join("token.json"), b"{\"token\":\"abc\"}")
            .expect("write auth");

        run_import_from_jcode(false).expect("import should succeed");

        let target_home = home.join(".jcode-plus");
        assert!(target_home.join("sessions").join("s1.json").exists());
        assert!(target_home.join("auth").join("token.json").exists());
    }

    #[test]
    fn import_from_jcode_respects_force_flag() {
        let _lock = crate::storage::lock_test_env();
        let root = tempfile::TempDir::new().expect("temp dir");
        let home = root.path().join("home");
        let xdg = root.path().join("xdg");
        std::fs::create_dir_all(&home).expect("mkdir home");
        std::fs::create_dir_all(&xdg).expect("mkdir xdg");

        let _home_guard = EnvGuard::set("HOME", &home);
        let _xdg_guard = EnvGuard::set("XDG_CONFIG_HOME", &xdg);
        let _flavor_guard = EnvGuard::set("JCODE_PRODUCT_FLAVOR", Path::new("jcode-plus"));

        let source_home = home.join(".jcode");
        let target_home = home.join(".jcode-plus");
        std::fs::create_dir_all(source_home.join("sessions")).expect("mkdir source sessions");
        std::fs::create_dir_all(target_home.join("sessions")).expect("mkdir target sessions");
        std::fs::write(source_home.join("sessions").join("s1.json"), b"from-source")
            .expect("write source session");
        std::fs::write(target_home.join("sessions").join("s1.json"), b"existing")
            .expect("write existing session");

        run_import_from_jcode(false).expect("import without force should succeed");
        let content = std::fs::read_to_string(target_home.join("sessions").join("s1.json"))
            .expect("read target");
        assert_eq!(content, "existing");

        run_import_from_jcode(true).expect("import with force should succeed");
        let content = std::fs::read_to_string(target_home.join("sessions").join("s1.json"))
            .expect("read target after force");
        assert_eq!(content, "from-source");
    }
}
