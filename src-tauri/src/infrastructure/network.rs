//! Windows IP Helper 网络实现，读取网卡与路由并执行临时路由修改。
//!
//! # 作者
//! lhy

use crate::{
    application::Network,
    domain::{Adapter, AppResult, Route},
};
use std::{net::Ipv4Addr, ptr};
use windows::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_NOT_FOUND, WIN32_ERROR},
    NetworkManagement::{IpHelper::*, Ndis::*},
    Networking::WinSock::*,
};

/// 基于 Windows 系统接口的网络实现。
///
/// # 作者
/// lhy
pub struct WindowsNetwork;
/// 将 Windows 错误码转换为包含原因的应用结果。
///
/// # 作者
/// lhy
fn check(code: WIN32_ERROR) -> AppResult<()> {
    if code.0 == 0 {
        Ok(())
    } else {
        Err(format!(
            "Windows 网络操作失败（{}）：{}",
            code.0,
            code.to_hresult().message()
        ))
    }
}
/// 按 Windows IPv4 联合体布局构造地址，保持字节序一致。
///
/// # 作者
/// lhy
fn socket(ip: Ipv4Addr) -> SOCKADDR_INET {
    let mut v = SOCKADDR_IN {
        sin_family: AF_INET,
        ..Default::default()
    };
    v.sin_addr.S_un.S_addr = u32::from_ne_bytes(ip.octets());
    SOCKADDR_INET { Ipv4: v }
}
/// 从 IPv4 地址联合体读取地址并转换字节序。
///
/// # 安全
/// 调用方必须确认联合体保存的是有效 IPv4 地址。
///
/// # 作者
/// lhy
unsafe fn ip(socket: SOCKADDR_INET) -> Ipv4Addr {
    Ipv4Addr::from(socket.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes())
}
/// 校验领域路由并转换为 Windows 路由写入结构。
///
/// # 作者
/// lhy
fn row(route: &Route) -> AppResult<MIB_IPFORWARD_ROW2> {
    route.validate()?;
    let mut r = MIB_IPFORWARD_ROW2::default();
    unsafe { InitializeIpForwardEntry(&mut r) };
    r.InterfaceLuid.Value = route.interface.parse().map_err(|_| "无效网卡标识")?;
    r.DestinationPrefix.Prefix = socket(route.network);
    r.DestinationPrefix.PrefixLength = route.prefix;
    r.NextHop = socket(route.gateway);
    r.Metric = route.metric;
    r.Protocol = NL_ROUTE_PROTOCOL(route.protocol);
    Ok(r)
}
/// 离开作用域时释放的 Windows 路由表指针。
///
/// # 作者
/// lhy
struct Table(*mut MIB_IPFORWARD_TABLE2);
impl Drop for Table {
    /// 释放系统分配的路由表内存。
    ///
    /// # 作者
    /// lhy
    fn drop(&mut self) {
        unsafe { FreeMibTable(self.0.cast()) }
    }
}
impl Network for WindowsNetwork {
    /// 读取当前开机标识，用于阻止重启后执行旧撤销记录。
    ///
    /// # 作者
    /// lhy
    fn boot_id(&self) -> AppResult<String> {
        use windows::Wdk::System::SystemInformation::{
            NtQuerySystemInformation, SYSTEM_INFORMATION_CLASS,
        };
        // Windows 10/11 的开机环境信息以本次启动的 GUID 开头。
        let mut data = [0u64; 4];
        let mut size = 0;
        let result = unsafe {
            NtQuerySystemInformation(
                SYSTEM_INFORMATION_CLASS(90),
                data.as_mut_ptr().cast(),
                32,
                &mut size,
            )
        };
        if result.0 < 0 || size < 16 {
            return Err("无法读取本次开机标识，已停止路由操作".into());
        }
        Ok(format!("{:016x}{:016x}", data[0], data[1]))
    }

    /// 读取当前可用的 IPv4 网卡信息。
    ///
    /// # 返回
    /// 状态为启用且具有 IPv4 地址的非回环接口列表。
    ///
    /// # 错误
    /// 系统读取失败或多次扩容后列表仍变化时返回错误。
    ///
    /// # 作者
    /// lhy
    fn adapters(&self) -> AppResult<Vec<Adapter>> {
        let mut size = 16384u32;
        // 使用 u64 缓冲区满足 Windows 网卡结构的内存对齐要求。
        let mut buffer = vec![0u64; size as usize / 8];
        for _ in 0..4 {
            let result = unsafe {
                GetAdaptersAddresses(
                    AF_INET.0 as u32,
                    GAA_FLAG_INCLUDE_GATEWAYS,
                    None,
                    Some(buffer.as_mut_ptr().cast()),
                    &mut size,
                )
            };
            if result == ERROR_BUFFER_OVERFLOW.0 {
                buffer.resize((size as usize).div_ceil(8), 0);
                continue;
            }
            if result == 232 {
                return Ok(vec![]);
            }
            check(WIN32_ERROR(result))?;
            let mut out = Vec::new();
            let mut next = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            unsafe {
                while let Some(a) = next.as_ref() {
                    next = a.Next;
                    if a.OperStatus != IfOperStatusUp || a.IfType == 24 {
                        continue;
                    }
                    let mut addresses = Vec::new();
                    let mut u = a.FirstUnicastAddress;
                    while let Some(v) = u.as_ref() {
                        u = v.Next;
                        if !v.Address.lpSockaddr.is_null()
                            && (*v.Address.lpSockaddr).sa_family == AF_INET
                        {
                            let addr = &*v.Address.lpSockaddr.cast::<SOCKADDR_IN>();
                            addresses.push(format!(
                                "{}/{}",
                                Ipv4Addr::from(addr.sin_addr.S_un.S_addr.to_ne_bytes()),
                                v.OnLinkPrefixLength
                            ));
                        }
                    }
                    if addresses.is_empty() {
                        continue;
                    }
                    let mut gateway = String::new();
                    let mut g = a.FirstGatewayAddress;
                    while let Some(v) = g.as_ref() {
                        g = v.Next;
                        if !v.Address.lpSockaddr.is_null()
                            && (*v.Address.lpSockaddr).sa_family == AF_INET
                        {
                            gateway = Ipv4Addr::from(
                                (*v.Address.lpSockaddr.cast::<SOCKADDR_IN>())
                                    .sin_addr
                                    .S_un
                                    .S_addr
                                    .to_ne_bytes(),
                            )
                            .to_string();
                            break;
                        }
                    }
                    out.push(Adapter {
                        id: a.Luid.Value.to_string(),
                        name: a.FriendlyName.to_string().map_err(|e| e.to_string())?,
                        addresses,
                        gateway,
                    });
                }
            }
            return Ok(out);
        }
        Err("网卡列表正在变化，请稍后刷新".into())
    }
    /// 读取系统当前 IPv4 路由表。
    ///
    /// # 作者
    /// lhy
    fn routes(&self) -> AppResult<Vec<Route>> {
        let mut raw = ptr::null_mut();
        unsafe { check(GetIpForwardTable2(AF_INET, &mut raw))? };
        let table = Table(raw);
        if table.0.is_null() {
            return Err("Windows 未返回路由表".into());
        }
        unsafe {
            let rows = std::slice::from_raw_parts(
                ptr::addr_of!((*raw).Table).cast::<MIB_IPFORWARD_ROW2>(),
                (*raw).NumEntries as usize,
            );
            Ok(rows
                .iter()
                .map(|r| Route {
                    interface: r.InterfaceLuid.Value.to_string(),
                    network: ip(r.DestinationPrefix.Prefix),
                    prefix: r.DestinationPrefix.PrefixLength,
                    gateway: ip(r.NextHop),
                    metric: r.Metric,
                    protocol: r.Protocol.0,
                })
                .collect())
        }
    }
    /// 新增指定的临时 IPv4 路由。
    ///
    /// # 作者
    /// lhy
    fn add(&mut self, r: &Route) -> AppResult<()> {
        unsafe { check(CreateIpForwardEntry2(&row(r)?)) }
    }
    /// 移除指定的临时 IPv4 路由。
    ///
    /// # 作者
    /// lhy
    fn remove(&mut self, r: &Route) -> AppResult<()> {
        let result = unsafe { DeleteIpForwardEntry2(&row(r)?) };
        if result == ERROR_NOT_FOUND {
            Ok(())
        } else {
            check(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 只读验证真实开机标识、网卡和路由的结构合法性。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn reads_real_network_without_modifying() {
        let n = WindowsNetwork;
        assert_eq!(n.boot_id().unwrap(), n.boot_id().unwrap());
        let a = n.adapters().unwrap();
        let r = n.routes().unwrap();
        for a in a {
            a.id.parse::<u64>().unwrap();
        }
        for r in r {
            r.validate().unwrap();
        }
    }
    /// 验证 Windows IPv4 地址转换的字节序往返一致。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn ipv4_byte_order_roundtrip() {
        let a = Ipv4Addr::new(10, 9, 127, 254);
        assert_eq!(unsafe { ip(socket(a)) }, a);
    }
}
