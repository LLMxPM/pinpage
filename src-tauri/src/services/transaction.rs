// 功能：以持久提交标记协调站点目录交换、配置保存和启动恢复。
use crate::{models::Settings, state::AppState};
use pinpage_protocol::Failure;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize)]
struct Journal {
    transaction: String,
    had_site: bool,
    pending: Option<Vec<u8>>,
}
pub struct SiteTransaction {
    root: PathBuf,
    journal: Journal,
    finished: bool,
}
impl SiteTransaction {
    /// 调用方持有操作锁；记录先落盘，再移动旧站点，旧副本跨配置提交保留。
    pub fn begin(root: PathBuf) -> Result<Self, Failure> {
        safe(&root)?;
        if root.join("transaction.json").exists() {
            return Err(failed("存在待恢复事务"));
        }
        if root.join("stale").exists() {
            return Err(failed("存在未恢复的旧副本"));
        }
        let journal = Journal {
            transaction: uuid::Uuid::new_v4().to_string(),
            had_site: root.join("site").exists(),
            pending: std::fs::read(root.join(crate::import::PENDING_DATA)).ok(),
        };
        let temporary = root.join("transaction.tmp");
        let mut file = std::fs::File::create(&temporary).map_err(failed)?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec(&journal).map_err(failed)?)
            .and_then(|_| file.sync_all())
            .map_err(failed)?;
        drop(file);
        crate::state::replace_file(&temporary, &root.join("transaction.json")).map_err(failed)?;
        let mut tx = Self {
            root,
            journal,
            finished: false,
        };
        if let Err(e) = tx.swap() {
            tx.rollback()?;
            return Err(e);
        }
        Ok(tx)
    }
    /// 恢复记录存在后才允许交换；失败由 rollback 还原。
    fn swap(&self) -> Result<(), Failure> {
        if self.root.join("stale").exists() {
            return Err(failed("旧副本尚未清理"));
        }
        if self.journal.had_site {
            std::fs::rename(self.root.join("site"), self.root.join("stale")).map_err(failed)?;
        }
        std::fs::rename(self.root.join("incoming"), self.root.join("site")).map_err(failed)
    }
    /// 配置中的同一事务标记是启动恢复时唯一的提交依据。
    pub fn mark(&self, settings: &mut Settings, id: &str) {
        settings
            .site_commits
            .insert(id.into(), self.journal.transaction.clone());
    }
    /// 配置已原子落盘；清理失败保留日志供下次启动重试，不回滚已提交内容。
    pub fn committed(mut self) {
        self.finished = true;
        let _ = cleanup(&self.root);
    }
    /// 仅回滚文件；调用者负责恢复尚未落盘的内存配置。
    pub fn rollback(&mut self) -> Result<(), Failure> {
        rollback(&self.root, &self.journal)?;
        self.finished = true;
        Ok(())
    }
}
impl Drop for SiteTransaction {
    /// 意外返回也保留旧副本；回滚失败不得删除恢复材料。
    fn drop(&mut self) {
        if !self.finished {
            if let Err(e) = self.rollback() {
                eprintln!("{e}");
            }
        }
    }
}
/// 启动时在任何内容窗口或控制入口创建前恢复，未来配置只读时不碰磁盘。
pub fn recover(app: &AppHandle) -> Result<(), Failure> {
    let state = app.state::<AppState>();
    if state.read_only {
        return Ok(());
    }
    let pages = crate::import::pages_root(app).map_err(super::source::native)?;
    if !pages.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(pages).map_err(failed)? {
        let root = entry.map_err(failed)?.path();
        if !root.is_dir() {
            continue;
        }
        safe(&root)?;
        let journal = root.join("transaction.json");
        if !journal.exists() {
            continue;
        }
        let record: Journal =
            serde_json::from_slice(&std::fs::read(&journal).map_err(failed)?).map_err(failed)?;
        let id = root.file_name().unwrap().to_string_lossy();
        let committed = state.settings.lock().unwrap().site_commits.get(id.as_ref())
            == Some(&record.transaction);
        if committed {
            cleanup(&root)?;
        } else {
            rollback(&root, &record)?;
        }
    }
    Ok(())
}
/// 检查所有应用自有目录节点，避免恢复时跟随人为植入的链接。
fn safe(root: &Path) -> Result<(), Failure> {
    for path in [
        root.to_owned(),
        root.join("site"),
        root.join("incoming"),
        root.join("stale"),
    ] {
        if !path.exists() {
            continue;
        }
        let meta = std::fs::symlink_metadata(path).map_err(failed)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err(failed("事务目录含重解析点"));
            }
        }
        if meta.file_type().is_symlink() {
            return Err(failed("事务目录含链接"));
        }
    }
    Ok(())
}
/// 旧副本存在意味着交换至少开始过；若旧站点未移动则保留原站点。
fn rollback(root: &Path, journal: &Journal) -> Result<(), Failure> {
    safe(root)?;
    if root.join("stale").exists() {
        if root.join("site").exists() {
            std::fs::remove_dir_all(root.join("site")).map_err(failed)?;
        }
        std::fs::rename(root.join("stale"), root.join("site")).map_err(failed)?;
    } else if !journal.had_site && root.join("site").exists() {
        std::fs::remove_dir_all(root.join("site")).map_err(failed)?;
    }
    let pending = root.join(crate::import::PENDING_DATA);
    match &journal.pending {
        Some(bytes) => std::fs::write(pending, bytes).map_err(failed)?,
        None => {
            if pending.exists() {
                std::fs::remove_file(pending).map_err(failed)?;
            }
        }
    }
    cleanup(root)
}
/// 只清理固定事务子目录和记录，不遍历用户源码。
fn cleanup(root: &Path) -> Result<(), Failure> {
    safe(root)?;
    for dir in ["stale", "incoming"] {
        let path = root.join(dir);
        if path.exists() {
            std::fs::remove_dir_all(path).map_err(failed)?;
        }
    }
    std::fs::remove_file(root.join("transaction.json")).map_err(failed)
}
/// 回滚失败的记录保持原位，避免误报旧副本已恢复。
fn failed(error: impl ToString) -> Failure {
    Failure::new("transactionFailed", error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rollback_keeps_old_site_and_pending() {
        let root = std::env::temp_dir().join(format!("pinpage-tx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("site")).unwrap();
        std::fs::create_dir_all(root.join("incoming")).unwrap();
        std::fs::write(root.join("site/index.html"), "旧").unwrap();
        std::fs::write(root.join("incoming/index.html"), "新").unwrap();
        let tx = SiteTransaction::begin(root.clone()).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("site/index.html")).unwrap(),
            "新"
        );
        drop(tx);
        assert_eq!(
            std::fs::read_to_string(root.join("site/index.html")).unwrap(),
            "旧"
        );
        assert!(!root.join("transaction.json").exists());
        let _ = std::fs::remove_dir_all(root);
    }
    /// 模拟进程在旧站点让位、新站点转正、配置提交三个边界中断。
    #[test]
    fn interrupted_commit_stages_recover_consistently() {
        for stage in 0..4 {
            let root =
                std::env::temp_dir().join(format!("pinpage-tx-recovery-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(root.join("site")).unwrap();
            std::fs::create_dir_all(root.join("incoming")).unwrap();
            std::fs::write(root.join("site/index.html"), "old").unwrap();
            std::fs::write(root.join("incoming/index.html"), "new").unwrap();
            let journal = Journal {
                transaction: "tx".into(),
                had_site: true,
                pending: None,
            };
            std::fs::write(
                root.join("transaction.json"),
                serde_json::to_vec(&journal).unwrap(),
            )
            .unwrap();
            if stage >= 1 {
                std::fs::rename(root.join("site"), root.join("stale")).unwrap();
            }
            if stage >= 2 {
                std::fs::rename(root.join("incoming"), root.join("site")).unwrap();
            }
            if stage == 3 {
                cleanup(&root).unwrap();
            } else {
                rollback(&root, &journal).unwrap();
            }
            assert_eq!(
                std::fs::read_to_string(root.join("site/index.html")).unwrap(),
                if stage == 3 { "new" } else { "old" }
            );
            assert!(!root.join("transaction.json").exists());
            assert!(!root.join("stale").exists());
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
