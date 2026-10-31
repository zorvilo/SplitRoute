//! 路由领域模型与输入校验，不直接访问 Windows 网络。
//!
//! # 作者
//! lhy

use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// 统一携带可展示错误原因的应用结果。
///
/// # 作者
/// lhy
pub type AppResult<T> = Result<T, String>;

/// 单条 IPv4 路由及其接口、网关和优先级属性。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// 路由所属接口的 Windows LUID 字符串。
    pub interface: String,
    /// 目标 IPv4 网络地址。
    pub network: Ipv4Addr,
    /// 目标网络前缀长度，单位为位。
    pub prefix: u8,
    /// 下一跳 IPv4 地址；零地址表示直连路由。
    pub gateway: Ipv4Addr,
    /// 路由优先级值，不包含接口优先级。
    pub metric: u32,
    /// Windows 路由协议标识。
    pub protocol: i32,
}
impl Route {
    /// 比较接口、目标网段、前缀和网关，不包含优先级等可变属性。
    ///
    /// # 作者
    /// lhy
    pub fn same_key(&self, other: &Self) -> bool {
        self.interface == other.interface
            && self.network == other.network
            && self.prefix == other.prefix
            && self.gateway == other.gateway
    }
    /// 校验恢复路由的接口标识、前缀长度及网络地址主机位。
    ///
    /// # 作者
    /// lhy
    pub fn validate(&self) -> AppResult<()> {
        self.interface
            .parse::<u64>()
            .map_err(|_| "无效的网卡标识")?;
        if self.prefix > 32 || u32::from(self.network) & !mask(self.prefix) != 0 {
            return Err("无效的恢复路由网段".into());
        }
        Ok(())
    }
}
/// 网络接口标识、名称、IPv4 地址与网关信息。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    /// Windows 网络接口 LUID 字符串。
    pub id: String,
    /// 网络接口显示名称。
    pub name: String,
    /// 接口 IPv4 地址及前缀列表。
    pub addresses: Vec<String>,
    /// 下一跳网关；网卡未提供网关时为空字符串。
    pub gateway: String,
}
/// 内外网接口、网关及目标网段配置。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// 内网接口标识。
    pub internal_id: String,
    /// 外网接口标识。
    pub external_id: String,
    /// 内网路由的下一跳 IPv4 网关。
    pub internal_gateway: String,
    /// 外网默认路由的下一跳网关。
    pub external_gateway: String,
    /// 走内网出口的目标 CIDR 网段。
    pub cidr: String,
}
/// 将合法的 IPv4 前缀长度转换为掩码，包含零前缀特例。
///
/// # 作者
/// lhy
fn mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}
/// 解析内网 CIDR，拒绝零前缀、越界前缀和包含主机位的地址。
///
/// # 参数
/// - `text`：带前缀长度的内网网段文本。
///
/// # 返回
/// 解析后的网络地址与前缀；格式非法或含主机位时返回错误。
///
/// # 作者
/// lhy
pub fn cidr(text: &str) -> AppResult<(Ipv4Addr, u8)> {
    let (ip, prefix) = text
        .trim()
        .split_once('/')
        .ok_or("网段格式应为 10.0.0.0/8")?;
    let ip = ip.parse::<Ipv4Addr>().map_err(|_| "无效的 IPv4 网段")?;
    let prefix = prefix.parse::<u8>().map_err(|_| "无效的网段前缀")?;
    if !(1..=32).contains(&prefix) {
        return Err("内网前缀必须在 1 到 32 之间".into());
    }
    if u32::from(ip) & !mask(prefix) != 0 {
        return Err("请输入网络地址，例如 10.0.0.0/8，不能包含主机位".into());
    }
    Ok((ip, prefix))
}
/// 校验内外网接口和直连网关，生成内网路由及外网默认路由计划。
///
/// 仅生成计划，不写入系统；更具体的现有路由仍由系统匹配。
///
/// # 参数
/// - `config`：内外网接口、网关和目标网段。
/// - `adapters`：当前可用接口及其 IPv4 地址。
///
/// # 返回
/// 依次为内网指定网段路由与外网默认路由。
///
/// # 错误
/// 接口相同、接口缺失、网段非法或网关不在直连网段时拒绝生成计划。
///
/// # 作者
/// lhy
pub fn plan(config: &Config, adapters: &[Adapter]) -> AppResult<Vec<Route>> {
    if config.internal_id == config.external_id {
        return Err("内网和外网必须选择不同的网卡".into());
    }
    let (network, prefix) = cidr(&config.cidr)?;
    let mut routes = Vec::new();
    for (id, gateway, net, len, metric) in [
        (
            &config.internal_id,
            &config.internal_gateway,
            network,
            prefix,
            10,
        ),
        (
            &config.external_id,
            &config.external_gateway,
            Ipv4Addr::UNSPECIFIED,
            0,
            0,
        ),
    ] {
        let adapter = adapters
            .iter()
            .find(|a| &a.id == id)
            .ok_or("所选网卡已断开，请刷新网卡")?;
        id.parse::<u64>().map_err(|_| "无效的网卡标识")?;
        let gateway = gateway
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| "网关必须是有效 IPv4 地址")?;
        if gateway.is_unspecified()
            || gateway.is_multicast()
            || gateway.is_broadcast()
            || gateway.is_loopback()
        {
            return Err("网关必须是可用的单播地址".into());
        }
        let on_link = adapter.addresses.iter().any(|a| {
            let Some((ip, bits)) = a.split_once('/') else {
                return false;
            };
            let (Ok(ip), Ok(bits)) = (ip.parse::<Ipv4Addr>(), bits.parse::<u8>()) else {
                return false;
            };
            bits <= 32
                && ip != gateway
                && u32::from(ip) & mask(bits) == u32::from(gateway) & mask(bits)
        });
        if !on_link {
            return Err(format!(
                "网关 {gateway} 不在网卡“{}”的直连网段，请检查设置",
                adapter.name
            ));
        }
        routes.push(Route {
            interface: id.clone(),
            network: net,
            prefix: len,
            gateway,
            metric,
            protocol: 3,
        });
    }
    Ok(routes)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    /// 构造领域规则测试所需的双网卡配置与地址。
    ///
    /// # 作者
    /// lhy
    pub fn fixture() -> (Config, Vec<Adapter>) {
        (
            Config {
                internal_id: "1".into(),
                external_id: "2".into(),
                internal_gateway: "10.9.127.254".into(),
                external_gateway: "192.168.43.1".into(),
                cidr: "10.0.0.0/8".into(),
            },
            vec![
                Adapter {
                    id: "1".into(),
                    name: "内网".into(),
                    addresses: vec!["10.9.127.2/24".into()],
                    gateway: "10.9.127.254".into(),
                },
                Adapter {
                    id: "2".into(),
                    name: "Wi-Fi".into(),
                    addresses: vec!["192.168.43.2/24".into()],
                    gateway: "192.168.43.1".into(),
                },
            ],
        )
    }
    /// 验证网段边界及网关直连条件，拒绝非法配置。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn validates_network_and_gateway() {
        for bad in [
            "10.1.0.0/8",
            "10.0.0.0/0",
            "10.0.0.0/33",
            "bad/8",
            "10.0.0.0/8x",
        ] {
            assert!(cidr(bad).is_err());
        }
        let (mut c, a) = fixture();
        assert!(plan(&c, &a).is_ok());
        c.external_gateway = "172.16.0.1".into();
        assert!(plan(&c, &a).is_err());
        c.external_gateway = "192.168.43.2".into();
        assert!(plan(&c, &a).is_err());
    }
}
