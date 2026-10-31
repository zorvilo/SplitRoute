use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

pub type AppResult<T> = Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub interface: String,
    pub network: Ipv4Addr,
    pub prefix: u8,
    pub gateway: Ipv4Addr,
    pub metric: u32,
    pub protocol: i32,
}
impl Route {
    pub fn same_key(&self, other: &Self) -> bool {
        self.interface == other.interface
            && self.network == other.network
            && self.prefix == other.prefix
            && self.gateway == other.gateway
    }
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    pub id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub gateway: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub internal_id: String,
    pub external_id: String,
    pub internal_gateway: String,
    pub external_gateway: String,
    pub cidr: String,
}
fn mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}
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
