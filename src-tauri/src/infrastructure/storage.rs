//! 配置与撤销记录的校验读取及落盘替换。
//!
//! # 作者
//! lhy

use crate::{
    application::{Journal, Recovery},
    domain::{AppResult, Config},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
};

/// 应用本地目录中的配置与撤销记录存储。
///
/// # 作者
/// lhy
pub struct FileJournal {
    /// 当前用户的应用数据目录。
    pub root: PathBuf,
}
/// 带格式版本的撤销记录文件封装。
///
/// # 作者
/// lhy
#[derive(Serialize, Deserialize)]
struct Record {
    /// 撤销记录的格式版本号。
    version: u32,
    /// 撤销记录内容；空值表示无待撤销记录。
    record: Option<Recovery>,
}
impl FileJournal {
    /// 定位当前用户的应用数据目录，不存在时创建目录。
    ///
    /// # 作者
    /// lhy
    pub fn new() -> AppResult<Self> {
        let root = std::env::var_os("LOCALAPPDATA").ok_or("无法定位本地配置目录")?;
        let root = PathBuf::from(root).join("SplitRoute");
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }
    /// 读取保存的分流配置，文件不存在时返回空结果。
    ///
    /// # 作者
    /// lhy
    pub fn config(&self) -> AppResult<Option<Config>> {
        let p = self.root.join("config.json");
        if !p.try_exists().map_err(|e| e.to_string())? {
            return Ok(None);
        }
        serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?)
            .map(Some)
            .map_err(|e| format!("配置文件损坏：{e}"))
    }
    /// 以临时文件替换方式保存有效分流配置。
    ///
    /// # 作者
    /// lhy
    pub fn save_config(&self, c: &Config) -> AppResult<()> {
        atomic(
            &self.root.join("config.json"),
            &serde_json::to_vec_pretty(c).map_err(|e| e.to_string())?,
        )
    }
}
/// 先写入并同步临时文件，再以写穿透方式替换目标文件。
///
/// # 参数
/// - `path`：目标文件路径。
/// - `bytes`：需要完整落盘的文件内容。
///
/// # 错误
/// 创建、写入、同步或替换文件失败时返回错误。
///
/// # 作者
/// lhy
pub(crate) fn atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let temp = path.with_extension("tmp");
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp)
        .map_err(|e| format!("无法写入恢复/配置记录：{e}"))?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| format!("无法提交恢复/配置记录：{e}"))
}
use std::os::windows::ffi::OsStrExt;
impl Journal for FileJournal {
    /// 读取并校验当前格式的撤销记录，拒绝超大、损坏或非法内容。
    ///
    /// # 作者
    /// lhy
    fn load(&self) -> AppResult<Option<Recovery>> {
        let path = self.root.join("recovery.json");
        if !path.try_exists().map_err(|e| e.to_string())? {
            return Ok(None);
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("撤销记录过大，已停止操作".into());
        }
        let stored: Record = serde_json::from_slice(&bytes)
            .map_err(|e| format!("撤销记录格式错误，已停止操作：{e}"))?;
        if stored.version != 3 {
            return Err("不支持的撤销记录格式".into());
        }
        if let Some(r) = &stored.record {
            if r.boot_id.is_empty() || r.changes.len() > 512 || r.adapters.len() != 2 {
                return Err("撤销记录内容无效".into());
            }
            for c in &r.changes {
                c.route.validate()?;
            }
        }
        Ok(stored.record)
    }
    /// 按当前格式保存撤销记录或清空记录内容。
    ///
    /// # 作者
    /// lhy
    fn save(&mut self, record: Option<&Recovery>) -> AppResult<()> {
        atomic(
            &self.root.join("recovery.json"),
            &serde_json::to_vec_pretty(&Record {
                version: 3,
                record: record.cloned(),
            })
            .map_err(|e| e.to_string())?,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    /// 验证文件记录写入读取一致，损坏内容被拒绝。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn durable_roundtrip_and_corruption() {
        let root = std::env::temp_dir().join(format!("splitroute-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut j = FileJournal { root: root.clone() };
        assert!(j.load().unwrap().is_none());
        let (c, a) = crate::domain::tests::fixture();
        let route = crate::domain::plan(&c, &a).unwrap().remove(0);
        let r = Recovery {
            boot_id: "testboot".into(),
            config: c.clone(),
            adapters: a,
            changes: vec![crate::application::Change {
                route: route.clone(),
                added: true,
            }],
        };
        j.save(Some(&r)).unwrap();
        assert_eq!(j.load().unwrap().unwrap().changes[0].route, route);
        j.save_config(&c).unwrap();
        assert_eq!(j.config().unwrap().unwrap().cidr, c.cidr);
        j.save(None).unwrap();
        assert!(j.load().unwrap().is_none());
        fs::write(root.join("recovery.json"), "broken").unwrap();
        assert!(j.load().is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
