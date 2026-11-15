// 功能：以同级暂存和回滚目录实现 skill 的固定目标替换。
use crate::{files, invalid, Bundle, Manifest, ID, MANIFEST, RECORD, VERSION};
use pinpage_protocol::Failure;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 查询已识别技能的版本；不把任意同名目录识别为已安装。
pub fn status(dir: Option<&Path>) -> Result<Value, Failure> {
    let target = crate::root(dir)?.join(ID);
    files::boundary(&target)?;
    let installed = if target.exists() {
        Some(read_record(&target)?.skill_version)
    } else {
        None
    };
    Ok(json!({"target": target, "installedVersion": installed, "builtinVersion": VERSION}))
}
/// 新装、升级均先校验完整资源与本地修改，显式替换时在用户本地目录保留完整备份。
pub fn install(
    dir: Option<&Path>,
    source: Option<&Path>,
    update: bool,
    replace_modified: bool,
) -> Result<Value, Failure> {
    let bundle = crate::load(source)?;
    let manifest = crate::validate(&bundle)?;
    let root = crate::root(dir)?;
    let target = root.join(ID);
    files::boundary(&target)?;
    let _lock = files::lock(&root)?;
    let mut previous = None;
    let mut backup = None;
    if target.exists() {
        let record = read_record(&target)?;
        previous = Some(record.skill_version.clone());
        let actual = files::read_tree(&target)?;
        let modified = actual
            .get(MANIFEST)
            .is_none_or(|b| serde_json::from_slice::<Manifest>(b).ok().as_ref() != Some(&record))
            || record.files.len() + 1 != actual.len()
            || record
                .files
                .iter()
                .any(|(p, hash)| actual.get(p).is_none_or(|b| crate::digest(b) != *hash));
        if modified && !replace_modified {
            let conflicts: Vec<_> = record
                .files
                .keys()
                .chain(actual.keys())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .filter(|path| {
                    path.as_str() != MANIFEST
                        && record.files.get(*path).is_none_or(|hash| {
                            actual
                                .get(*path)
                                .is_none_or(|bytes| crate::digest(bytes) != *hash)
                        })
                })
                .cloned()
                .collect();
            return Err(Failure::new(
                "skillModified",
                "技能文件已增删或修改；检查后可显式 --replace-modified",
            )
            .detail(json!({"target": target,"conflicts":conflicts})));
        }
        if !modified && actual == bundle {
            return Ok(result(&target, &manifest, previous, false, None));
        }
        if !update {
            return Err(Failure::new(
                "skillTargetConflict",
                "目标已安装其他版本，请使用 skill update",
            ));
        }
        if semver::Version::parse(&manifest.skill_version).map_err(invalid)?
            < semver::Version::parse(&record.skill_version).map_err(invalid)?
        {
            return Err(Failure::new("skillIncompatible", "不允许静默降级 skill"));
        }
        if modified {
            let base =
                std::env::var_os("LOCALAPPDATA").ok_or_else(|| invalid("无法定位备份目录"))?;
            let path = PathBuf::from(base)
                .join("PinPage/skill-backups")
                .join(uuid::Uuid::new_v4().to_string());
            files::boundary(&path)?;
            files::backup_tree(&target, &path)?;
            backup = Some(path);
        }
    } else if update {
        return Err(Failure::new(
            "skillNotInstalled",
            "目标未安装 pinpage-pages，请先 install",
        ));
    }
    std::fs::create_dir_all(&root).map_err(invalid)?;
    let stage = root.join(format!(".pinpage-stage-{}", uuid::Uuid::new_v4()));
    let operation = (|| {
        files::write_tree(&stage, &bundle)?;
        std::fs::write(
            stage.join(RECORD),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .map_err(invalid)?;
        exchange(&stage, &target)?;
        Ok(result(&target, &manifest, previous, true, backup))
    })();
    if stage.exists() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    operation
}
/// 安装记录只承认同一技能身份，不能由 --replace-modified 绕过身份冲突。
fn read_record(target: &Path) -> Result<Manifest, Failure> {
    let conflict = || Failure::new("skillTargetConflict", "目标不是受管理的 pinpage-pages");
    files::regular(&target.join(RECORD)).map_err(|_| conflict())?;
    let record: Manifest =
        serde_json::from_slice(&std::fs::read(target.join(RECORD)).map_err(|_| conflict())?)
            .map_err(|_| conflict())?;
    if record.id != ID {
        return Err(conflict());
    }
    Ok(record)
}
/// 先隐藏旧入口，交换完成才激活新入口；失败恢复旧目录及入口。
fn exchange(stage: &Path, target: &Path) -> Result<(), Failure> {
    files::boundary(target)?;
    let old = target.with_file_name(format!(".pinpage-old-{}", uuid::Uuid::new_v4()));
    let existed = target.exists();
    let had_entry = target.join("SKILL.md").exists();
    if existed {
        if had_entry {
            std::fs::rename(target.join("SKILL.md"), target.join(".skill-entry"))
                .map_err(invalid)?;
        }
        if let Err(e) = std::fs::rename(target, &old) {
            if had_entry {
                let _ = std::fs::rename(target.join(".skill-entry"), target.join("SKILL.md"));
            }
            return Err(invalid(e));
        }
    }
    let operation = std::fs::rename(stage, target)
        .and_then(|_| std::fs::rename(target.join(".skill-entry"), target.join("SKILL.md")));
    if let Err(error) = operation {
        if target.exists() {
            let _ = std::fs::rename(target, stage);
        }
        if existed {
            std::fs::rename(&old, target)
                .and_then(|_| {
                    if had_entry {
                        std::fs::rename(target.join(".skill-entry"), target.join("SKILL.md"))
                    } else {
                        Ok(())
                    }
                })
                .map_err(|e| Failure::new("rollbackFailed", e).detail(json!({"recovery": old})))?;
        }
        return Err(invalid(error));
    }
    if existed {
        let _ = std::fs::remove_dir_all(old);
    }
    Ok(())
}
/// 返回最终绝对路径与更新事实；客户端自行刷新技能发现。
fn result(
    target: &Path,
    manifest: &Manifest,
    previous: Option<String>,
    changed: bool,
    backup: Option<PathBuf>,
) -> Value {
    json!({"target":target,"sourceVersion":manifest.skill_version,"previousVersion":previous,"changed":changed,"backup":backup,"notice":"请刷新智能体技能列表或开启新会话"})
}
/// 导出只替换确切文件或固定子目录，不清空用户指定的导出根目录。
pub fn export(out: &Path, overwrite: bool) -> Result<Value, Failure> {
    let out = std::path::absolute(out).map_err(invalid)?;
    let zip = out
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    let target = if zip { out } else { out.join(ID) };
    files::boundary(&target)?;
    if target.exists() && !overwrite {
        return Err(Failure::new("outputExists", "导出目标已存在"));
    }
    if target.join(RECORD).exists() {
        return Err(Failure::new(
            "skillTargetConflict",
            "导出目标是已安装技能，请选择其他导出目录",
        ));
    }
    if target.exists() && !zip {
        crate::validate(&files::read_tree(&target)?)?;
    }
    let parent = target.parent().ok_or_else(|| invalid("导出路径无父目录"))?;
    std::fs::create_dir_all(parent).map_err(invalid)?;
    let _lock = files::lock(parent)?;
    files::boundary(&target)?;
    if target.exists() && !overwrite {
        return Err(Failure::new("outputExists", "导出目标已存在"));
    }
    let stage = parent.join(format!(".pinpage-export-{}", uuid::Uuid::new_v4()));
    let bundle: Bundle = crate::builtin();
    if zip {
        files::write_zip(&stage, &bundle)?;
        let old = target.with_extension(format!("{}.old", uuid::Uuid::new_v4()));
        if target.exists() {
            std::fs::rename(&target, &old).map_err(invalid)?;
        }
        if let Err(e) = std::fs::rename(&stage, &target) {
            if old.exists() {
                let _ = std::fs::rename(&old, &target);
            }
            let _ = std::fs::remove_file(&stage);
            return Err(invalid(e));
        }
        if old.exists() {
            let _ = std::fs::remove_file(old);
        }
    } else {
        files::write_tree(&stage, &bundle)?;
        exchange(&stage, &target)?;
    }
    Ok(json!({"path":target,"skillVersion":VERSION,"format":if zip {"zip"} else {"directory"}}))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 激活暂存入口失败后，恢复完整旧目录，不残留新旧混合文件。
    #[test]
    fn failed_activation_rolls_back() {
        let root =
            std::env::temp_dir().join(format!("pinpage-skill-rollback-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let target = root.join(ID);
        let stage = root.join("stage");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(target.join("SKILL.md"), "old").unwrap();
        std::fs::write(stage.join("new"), "new").unwrap();
        assert!(exchange(&stage, &target).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "old"
        );
        assert!(!target.join("new").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
