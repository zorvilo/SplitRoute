//! 通过指定源地址执行 ICMP 探测，并核对实际网络出口。
//!
//! # 作者
//! lhy

use crate::{
    application::Network,
    domain::{Adapter, AppResult},
    infrastructure::network::WindowsNetwork,
};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use windows::Win32::{
    Foundation::{GetLastError, HANDLE},
    NetworkManagement::IpHelper::*,
    Networking::WinSock::*,
};

#[cfg(test)]
const PUBLIC_TARGET: Ipv4Addr = Ipv4Addr::new(223, 5, 5, 5);
/// 所选接口及内外网 IPv4 检测目标。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeRequest {
    /// 内网接口标识。
    pub internal_id: String,
    /// 实际内网探测目标，自动模式下使用配置的网关。
    pub internal_gateway: String,
    /// 外网接口标识。
    pub external_id: String,
    /// 外网检测目标的 IPv4 地址。
    pub external_target: String,
}
/// 单次连通探测结果状态。
///
/// # 作者
/// lhy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus {
    /// 目标响应了 ICMP 回显。
    Reachable,
    /// 在超时时间内未收到响应。
    Timeout,
    /// 目标接口或路由当前不可用。
    Unavailable,
    /// 尚未选择对应接口。
    Unconfigured,
    /// 目标地址不适合本次探测。
    InvalidTarget,
    /// 实际路由出口与所选接口不一致。
    WrongExit,
    /// 探测过程中发生系统错误。
    Error,
}
/// 单个目标的状态、延迟及源地址信息。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    /// 单次探测状态。
    pub status: ProbeStatus,
    /// 回显延迟，单位为毫秒；未成功时为空。
    pub latency_ms: Option<u32>,
    /// 本次实际探测的目标地址。
    pub target: String,
    /// 本次探测使用的接口源 IPv4 地址。
    pub source: String,
}
impl ProbeResult {
    /// 构造尚未获得延迟与源地址的探测结果。
    ///
    /// # 作者
    /// lhy
    fn new(status: ProbeStatus, target: &str) -> Self {
        Self {
            status,
            latency_ms: None,
            target: target.into(),
            source: String::new(),
        }
    }
}
/// 同一轮内外网连通检测结果。
///
/// # 作者
/// lhy
#[derive(Serialize)]
pub struct Connectivity {
    /// 内网目标的探测结果。
    pub internal: ProbeResult,
    /// 外网目标的探测结果。
    pub external: ProbeResult,
}
/// 离开作用域时关闭的 ICMP 句柄。
///
/// # 作者
/// lhy
struct Icmp(HANDLE);
impl Drop for Icmp {
    /// 关闭 ICMP 句柄，避免探测资源泄漏。
    ///
    /// # 作者
    /// lhy
    fn drop(&mut self) {
        let _ = unsafe { IcmpCloseHandle(self.0) };
    }
}
/// 构造用于源地址与目标地址查询的 Windows IPv4 地址。
///
/// # 作者
/// lhy
fn socket(ip: Ipv4Addr) -> SOCKADDR_INET {
    let mut a = SOCKADDR_IN {
        sin_family: AF_INET,
        ..Default::default()
    };
    a.sin_addr.S_un.S_addr = u32::from_ne_bytes(ip.octets());
    SOCKADDR_INET { Ipv4: a }
}
/// 解析可用的单播 IPv4 探测目标，排除本地回环等无效地址。
///
/// # 返回
/// 合法的单播 IPv4 地址；非法、未指定、回环、多播或广播地址返回空值。
///
/// # 作者
/// lhy
pub(crate) fn target(text: &str) -> Option<Ipv4Addr> {
    let ip = text.parse::<Ipv4Addr>().ok()?;
    (!ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast() && !ip.is_loopback())
        .then_some(ip)
}
/// 选择接口源地址并核对出口后发送 ICMP 回显请求。
///
/// # 作者
/// lhy
fn probe(adapters: &[Adapter], id: &str, destination: &str) -> ProbeResult {
    if id.is_empty() {
        return ProbeResult::new(ProbeStatus::Unconfigured, destination);
    }
    let Some(adapter) = adapters.iter().find(|a| a.id == id) else {
        return ProbeResult::new(ProbeStatus::Unavailable, destination);
    };
    let Some(destination_ip) = target(destination) else {
        return ProbeResult::new(ProbeStatus::InvalidTarget, destination);
    };
    let Some(source_ip) = adapter
        .addresses
        .first()
        .and_then(|a| a.split('/').next())
        .and_then(|s| s.parse::<Ipv4Addr>().ok())
    else {
        return ProbeResult::new(ProbeStatus::Unavailable, destination);
    };
    let mut result = ProbeResult::new(ProbeStatus::Error, destination);
    result.source = source_ip.to_string();
    if source_ip == destination_ip {
        result.status = ProbeStatus::InvalidTarget;
        return result;
    }
    let source = socket(source_ip);
    let dest = socket(destination_ip);
    let mut route = MIB_IPFORWARD_ROW2::default();
    let mut best_source = SOCKADDR_INET::default();
    // 探测前核对源地址对应的实际出口，避免把其他接口可达误报为所选接口正常。
    let code = unsafe {
        GetBestRoute2(
            None,
            0,
            Some(&source),
            &dest,
            0,
            &mut route,
            &mut best_source,
        )
    };
    if code.0 != 0 {
        result.status = ProbeStatus::Unavailable;
        return result;
    }
    if unsafe { route.InterfaceLuid.Value }.to_string() != id {
        result.status = ProbeStatus::WrongExit;
        return result;
    }
    let Ok(handle) = (unsafe { IcmpCreateFile() }) else {
        return result;
    };
    let handle = Icmp(handle);
    let payload = b"SplitRoute";
    let mut reply = [0u64; 64];
    let count = unsafe {
        IcmpSendEcho2Ex(
            handle.0,
            None,
            None,
            None,
            u32::from_ne_bytes(source_ip.octets()),
            u32::from_ne_bytes(destination_ip.octets()),
            payload.as_ptr().cast(),
            payload.len() as u16,
            None,
            reply.as_mut_ptr().cast(),
            std::mem::size_of_val(&reply) as u32,
            800,
        )
    };
    if count == 0 {
        let error = unsafe { GetLastError() }.0;
        result.status = if error == 11010 {
            ProbeStatus::Timeout
        } else {
            ProbeStatus::Unavailable
        };
        return result;
    }
    let echo = unsafe { &*reply.as_ptr().cast::<ICMP_ECHO_REPLY>() };
    result.status = match echo.Status {
        0 => ProbeStatus::Reachable,
        11010 => ProbeStatus::Timeout,
        _ => ProbeStatus::Unavailable,
    };
    if result.status == ProbeStatus::Reachable {
        result.latency_ms = Some(echo.RoundTripTime);
    }
    result
}
/// 校验探测请求并刷新接口列表，并行探测内外网目标。
///
/// # 作者
/// lhy
pub fn check(request: ProbeRequest) -> AppResult<Connectivity> {
    if request.internal_id.len() > 32
        || request.external_id.len() > 32
        || request.internal_gateway.len() > 64
        || target(&request.external_target).is_none()
    {
        return Err("检测参数无效".into());
    }
    let adapters = WindowsNetwork.adapters()?;
    std::thread::scope(|scope| {
        let internal =
            scope.spawn(|| probe(&adapters, &request.internal_id, &request.internal_gateway));
        let external = probe(&adapters, &request.external_id, &request.external_target);
        Ok(Connectivity {
            internal: internal.join().map_err(|_| "内网检测失败")?,
            external,
        })
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    /// 手动执行真实 ICMP 探测；常规测试跳过，避免自动发送网络探测。
    ///
    /// # 作者
    /// lhy
    #[test]
    #[ignore = "Sends real ICMP probes; run manually"]
    fn manual_live_connectivity() {
        let adapters = WindowsNetwork.adapters().unwrap();
        let online: Vec<_> = adapters.iter().filter(|a| !a.gateway.is_empty()).collect();
        assert!(
            !online.is_empty(),
            "An online adapter with a gateway is required"
        );
        for adapter in online {
            let gateway = probe(&adapters, &adapter.id, &adapter.gateway);
            let internet = probe(&adapters, &adapter.id, &PUBLIC_TARGET.to_string());
            println!(
                "{} — Gateway: {gateway:?}; Internet: {internet:?}",
                adapter.name
            );
            assert_ne!(gateway.status, ProbeStatus::Error);
            assert_ne!(internet.status, ProbeStatus::Error);
        }
    }
    /// 验证非法目标被拒绝，未配置或缺失接口不会发送探测。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn validates_targets_and_does_not_probe_missing_adapters() {
        for bad in [
            "",
            "bad",
            "0.0.0.0",
            "127.0.0.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(target(bad).is_none());
        }
        assert!(target("10.147.109.254").is_some());
        assert_eq!(probe(&[], "", "10.0.0.1").status, ProbeStatus::Unconfigured);
        assert_eq!(probe(&[], "1", "10.0.0.1").status, ProbeStatus::Unavailable);
    }
    /// 验证超长请求在读取网络前被拒绝。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn rejects_oversized_requests_before_network_access() {
        assert!(check(ProbeRequest {
            internal_id: "1".repeat(33),
            internal_gateway: String::new(),
            external_id: String::new(),
            external_target: "223.5.5.5".into()
        })
        .is_err());
    }
}
