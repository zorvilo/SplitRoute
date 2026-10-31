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
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeRequest {
    pub internal_id: String,
    pub internal_gateway: String,
    pub external_id: String,
    pub external_target: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus {
    Reachable,
    Timeout,
    Unavailable,
    Unconfigured,
    InvalidTarget,
    WrongExit,
    Error,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub status: ProbeStatus,
    pub latency_ms: Option<u32>,
    pub target: String,
    pub source: String,
}
impl ProbeResult {
    fn new(status: ProbeStatus, target: &str) -> Self {
        Self {
            status,
            latency_ms: None,
            target: target.into(),
            source: String::new(),
        }
    }
}
#[derive(Serialize)]
pub struct Connectivity {
    pub internal: ProbeResult,
    pub external: ProbeResult,
}
struct Icmp(HANDLE);
impl Drop for Icmp {
    fn drop(&mut self) {
        let _ = unsafe { IcmpCloseHandle(self.0) };
    }
}
fn socket(ip: Ipv4Addr) -> SOCKADDR_INET {
    let mut a = SOCKADDR_IN {
        sin_family: AF_INET,
        ..Default::default()
    };
    a.sin_addr.S_un.S_addr = u32::from_ne_bytes(ip.octets());
    SOCKADDR_INET { Ipv4: a }
}
pub(crate) fn target(text: &str) -> Option<Ipv4Addr> {
    let ip = text.parse::<Ipv4Addr>().ok()?;
    (!ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast() && !ip.is_loopback())
        .then_some(ip)
}
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
    // Verify the actual source-constrained route before probing; never count another exit as healthy.
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
