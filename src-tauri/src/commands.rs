use crate::application::Network;
use crate::{
    application::{RoutingStatus, Service},
    domain::{Adapter, AppResult, Config, Route},
    infrastructure::{network::WindowsNetwork, storage::FileJournal},
};
use serde::Serialize;
use std::sync::Mutex;
pub type AppService = Service<WindowsNetwork, FileJournal>;
pub struct AppState(pub Mutex<AppService>);
#[tauri::command]
pub async fn probe_connectivity(
    request: crate::infrastructure::connectivity::ProbeRequest,
) -> AppResult<crate::infrastructure::connectivity::Connectivity> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::infrastructure::connectivity::check(request)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    adapters: Vec<Adapter>,
    routes: Vec<Route>,
    pending: bool,
    routing_status: RoutingStatus,
    config: Option<Config>,
    warning: Option<String>,
}
#[tauri::command]
pub fn snapshot(state: tauri::State<AppState>) -> AppResult<Snapshot> {
    let s = state.0.lock().map_err(|_| "操作状态异常，请重启应用")?;
    let (config, warning) = match s.journal.config() {
        Ok(config) => (config, None),
        Err(error) => (
            None,
            Some(format!("{error}；请重新选择网卡配置，恢复记录不受影响。")),
        ),
    };
    let record = s.current_record()?;
    let config = record.as_ref().map(|r| r.config.clone()).or(config);
    let adapters = s.network.adapters()?;
    let routes = s.network.routes()?;
    let routing_status = s.status(config.as_ref(), &adapters, &routes)?;
    Ok(Snapshot {
        adapters,
        routes,
        pending: record.is_some(),
        routing_status,
        config,
        warning,
    })
}
#[tauri::command]
pub fn enable(config: Config, state: tauri::State<AppState>) -> AppResult<()> {
    crate::infrastructure::privilege::require_admin()?;
    let mut s = state.0.lock().map_err(|_| "操作状态异常，请重启应用")?;
    crate::domain::plan(&config, &s.network.adapters()?)?;
    s.enable(&config)?;
    s.journal
        .save_config(&config)
        .map_err(|e| format!("分流已生效，但配置保存失败：{e}"))
}
#[tauri::command]
pub fn restore(state: tauri::State<AppState>) -> AppResult<()> {
    crate::infrastructure::privilege::require_admin()?;
    state
        .0
        .lock()
        .map_err(|_| "操作状态异常，请重启应用")?
        .restore()
}
