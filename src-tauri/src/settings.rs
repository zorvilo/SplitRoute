//! 偏好设置的校验、持久化及桌面设置命令。
//!
//! # 作者
//! lhy

use crate::infrastructure::startup;
use crate::{
    domain::AppResult,
    infrastructure::storage::{atomic, FileJournal},
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
/// 界面、窗口行为和连通检测偏好。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    /// 界面语言代码。
    pub language: String,
    /// 浅色、深色或跟随系统的主题选项。
    pub theme: String,
    /// 关闭窗口时是否隐藏到托盘，空值表示每次询问。
    pub close_to_tray: Option<bool>,
    /// 是否在当前用户登录后启动软件。
    pub auto_start: bool,
    /// 自定义内网检测目标；空字符串表示使用网关。
    #[serde(default)]
    pub internal_probe_target: String,
    /// 外网检测目标的 IPv4 地址。
    #[serde(default = "default_external_target")]
    pub external_probe_target: String,
}
/// 提供外网探测目标的默认 IPv4 地址。
///
/// # 作者
/// lhy
fn default_external_target() -> String {
    "223.5.5.5".into()
}
impl Default for Preferences {
    /// 构造首次运行偏好，默认询问关闭方式且不启用登录启动。
    ///
    /// # 作者
    /// lhy
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "system".into(),
            close_to_tray: None,
            auto_start: false,
            internal_probe_target: String::new(),
            external_probe_target: default_external_target(),
        }
    }
}
impl Preferences {
    /// 校验语言、主题和内外网单播 IPv4 检测目标。
    ///
    /// # 作者
    /// lhy
    pub fn validate(&self) -> AppResult<()> {
        if !["zh-CN", "zh-TW", "en", "ja"].contains(&self.language.as_str())
            || !["light", "dark", "system"].contains(&self.theme.as_str())
        {
            return Err("无效的语言或主题设置".into());
        }
        if (!self.internal_probe_target.is_empty()
            && crate::infrastructure::connectivity::target(&self.internal_probe_target).is_none())
            || crate::infrastructure::connectivity::target(&self.external_probe_target).is_none()
        {
            return Err("检测目标必须是有效的单播 IPv4 地址".into());
        }
        Ok(())
    }
    /// 读取用户偏好并校验，无文件时使用首次运行默认值。
    ///
    /// # 作者
    /// lhy
    pub fn load() -> AppResult<Self> {
        let path = FileJournal::new()?.root.join("preferences.json");
        if !path.try_exists().map_err(|e| e.to_string())? {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let p: Self = serde_json::from_slice(&bytes).map_err(|e| format!("设置文件损坏：{e}"))?;
        p.validate()?;
        Ok(p)
    }
    /// 校验后以临时文件替换方式保存偏好。
    ///
    /// # 作者
    /// lhy
    pub fn save(&self) -> AppResult<()> {
        self.validate()?;
        atomic(
            &FileJournal::new()?.root.join("preferences.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
}
/// 通过互斥锁保护的共享偏好状态。
///
/// # 作者
/// lhy
pub struct PreferencesState(pub Mutex<Preferences>);
/// 读取共享偏好，并以计划任务实际状态校正开机启动选项。
///
/// # 作者
/// lhy
#[tauri::command]
pub async fn get_preferences(app: tauri::AppHandle) -> AppResult<Preferences> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<PreferencesState>();
        let actual = startup::enabled()?;
        let mut p = state.0.lock().map_err(|_| "设置状态异常")?;
        p.auto_start = actual;
        Ok(p.clone())
    })
    .await
    .map_err(|e| e.to_string())?
}
/// 校验并保存偏好，持久化失败时尝试回滚登录启动修改。
///
/// # 错误
/// 校验、登录启动更新或文件写入失败时返回错误；任务回滚失败时同时保留两个原因。
///
/// # 作者
/// lhy
#[tauri::command]
pub async fn save_preferences(preferences: Preferences, app: tauri::AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        preferences.validate()?;
        let state = app.state::<PreferencesState>();
        let mut current = state.0.lock().map_err(|_| "设置状态异常")?;
        let actual = current.auto_start;
        if preferences.auto_start != actual {
            startup::set_enabled(preferences.auto_start)?;
        }
        if let Err(error) = preferences.save() {
            if preferences.auto_start != actual {
                if let Err(rollback) = startup::set_enabled(actual) {
                    return Err(format!("{error}；登录启动回滚失败：{rollback}"));
                }
            }
            return Err(error);
        }
        *current = preferences.clone();
        drop(current);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.set_theme(match preferences.theme.as_str() {
                "light" => Some(tauri::Theme::Light),
                "dark" => Some(tauri::Theme::Dark),
                _ => None,
            });
        }
        crate::desktop::update_menu(&app, &preferences.language).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
/// 仅打开预定义的项目链接，拒绝任意目标。
///
/// # 作者
/// lhy
#[tauri::command]
pub fn open_project_link(target: String) -> AppResult<()> {
    crate::desktop::open_project_link(target)
}
/// 读取固定仓库的最新正式发布，区分无发布和请求失败。
///
/// # 作者
/// lhy
#[tauri::command]
pub async fn check_update() -> AppResult<crate::infrastructure::releases::UpdateInfo> {
    crate::infrastructure::releases::check_update().await
}
#[cfg(test)]
mod tests {
    use super::*;
    /// 验证默认偏好和探测目标边界，拒绝域名与非法 IPv4。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn validates_probe_targets() {
        let mut p = Preferences::default();
        assert!(!p.auto_start);
        assert!(p.validate().is_ok());
        p.internal_probe_target = "10.1.2.3".into();
        assert!(p.validate().is_ok());
        for bad in [
            "example.com",
            "127.0.0.1",
            "224.0.0.1",
            "999.1.1.1",
            "0.0.0.0",
        ] {
            p.external_probe_target = bad.into();
            assert!(p.validate().is_err());
        }
        p.external_probe_target = "1.1.1.1".into();
        assert!(p.validate().is_ok());
    }
    /// 验证首次运行默认值，并拒绝不支持的语言。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn validates_preferences() {
        let mut p = Preferences::default();
        assert_eq!(p.close_to_tray, None);
        assert!(!p.auto_start);
        assert!(p.validate().is_ok());
        p.language = "other".into();
        assert!(p.validate().is_err());
    }

    /// 验证三种关闭方式可持久化，原有布尔设置仍保持原意。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn round_trips_close_behavior() {
        for close_to_tray in [None, Some(true), Some(false)] {
            let preferences = Preferences {
                close_to_tray,
                ..Preferences::default()
            };
            let json = serde_json::to_value(preferences).unwrap();
            assert_eq!(json["closeToTray"], serde_json::json!(close_to_tray));
            let saved: Preferences = serde_json::from_value(json).unwrap();
            assert_eq!(saved.close_to_tray, close_to_tray);
        }
    }
}
