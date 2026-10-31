use crate::infrastructure::startup;
use crate::{
    domain::AppResult,
    infrastructure::storage::{atomic, FileJournal},
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub language: String,
    pub theme: String,
    pub close_to_tray: bool,
    pub auto_start: bool,
    #[serde(default)]
    pub internal_probe_target: String,
    #[serde(default = "default_external_target")]
    pub external_probe_target: String,
}
fn default_external_target() -> String {
    "223.5.5.5".into()
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "system".into(),
            close_to_tray: true,
            auto_start: false,
            internal_probe_target: String::new(),
            external_probe_target: default_external_target(),
        }
    }
}
impl Preferences {
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
    pub fn save(&self) -> AppResult<()> {
        self.validate()?;
        atomic(
            &FileJournal::new()?.root.join("preferences.json"),
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
}
pub struct PreferencesState(pub Mutex<Preferences>);
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
#[tauri::command]
pub fn open_project_link(target: String) -> AppResult<()> {
    crate::desktop::open_project_link(target)
}
#[tauri::command]
pub async fn check_update() -> AppResult<crate::infrastructure::releases::UpdateInfo> {
    crate::infrastructure::releases::check_update().await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_probe_targets() {
        let mut p = Preferences::default();
        assert!(p.close_to_tray);
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
    #[test]
    fn validates_preferences() {
        let mut p = Preferences::default();
        assert!(p.close_to_tray);
        assert!(!p.auto_start);
        assert!(p.validate().is_ok());
        p.language = "other".into();
        assert!(p.validate().is_err());
    }
}
