//! 前端路由命令入口，负责权限检查、状态互斥和快照组装。
//!
//! # 作者
//! lhy

use crate::application::Network;
use crate::{
    application::{RoutingStatus, Service},
    domain::{Adapter, AppResult, Config, Route},
    infrastructure::{network::WindowsNetwork, storage::FileJournal},
};
use serde::Serialize;
use std::sync::Mutex;
/// 使用 Windows 网络与文件记录存储的应用服务类型。
///
/// # 作者
/// lhy
pub type AppService = Service<WindowsNetwork, FileJournal>;
/// 命令间共享并通过互斥锁保护的分流服务。
///
/// # 作者
/// lhy
pub struct AppState(pub Mutex<AppService>);
/// 在线程池中执行内外网探测，避免阻塞桌面主线程。
///
/// # 作者
/// lhy
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
/// 前端读取的实际网卡、路由、恢复配置和状态快照。
///
/// # 作者
/// lhy
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// 本次读取的可用 IPv4 接口。
    adapters: Vec<Adapter>,
    /// 本次撤销记录中的原接口，用于断开后保留名称。
    saved_adapters: Vec<Adapter>,
    /// 系统当前 IPv4 路由列表。
    routes: Vec<Route>,
    /// 当前开机是否仍有待撤销的修改。
    pending: bool,
    /// 与实际网卡及路由匹配后的状态。
    routing_status: RoutingStatus,
    /// 保存的有效分流配置。
    config: Option<Config>,
    /// 配置读取异常等需要用户关注的提示。
    warning: Option<String>,
}
/// 读取配置、当前撤销记录和实际网络，组装界面快照。
///
/// # 作者
/// lhy
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
        saved_adapters: record
            .as_ref()
            .map(|r| r.adapters.clone())
            .unwrap_or_default(),
        adapters,
        routes,
        pending: record.is_some(),
        routing_status,
        config,
        warning,
    })
}
/// 检查管理员权限并互斥执行分流，成功后保存有效配置。
///
/// # 作者
/// lhy
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
/// 检查管理员权限并互斥撤销本次记录的修改。
///
/// # 作者
/// lhy
#[tauri::command]
pub fn restore(state: tauri::State<AppState>) -> AppResult<()> {
    crate::infrastructure::privilege::require_admin()?;
    state
        .0
        .lock()
        .map_err(|_| "操作状态异常，请重启应用")?
        .restore()
}
