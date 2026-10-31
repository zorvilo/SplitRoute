//! 路由应用流程：根据真实网络判断状态，记录变更并控制启用、撤销和失败回滚。
//!
//! # 作者
//! lhy

use crate::domain::{plan, Adapter, AppResult, Config, Route};
use serde::{Deserialize, Serialize};

/// 网络操作边界，隔离 Windows 实现与测试替身。
///
/// # 作者
/// lhy
pub trait Network {
    /// 读取当前开机标识，用于阻止重启后执行旧撤销记录。
    ///
    /// # 作者
    /// lhy
    fn boot_id(&self) -> AppResult<String>;
    /// 读取当前可用的 IPv4 网卡信息。
    ///
    /// # 作者
    /// lhy
    fn adapters(&self) -> AppResult<Vec<Adapter>>;
    /// 读取系统当前 IPv4 路由表。
    ///
    /// # 作者
    /// lhy
    fn routes(&self) -> AppResult<Vec<Route>>;
    /// 新增指定的临时 IPv4 路由。
    ///
    /// # 作者
    /// lhy
    fn add(&mut self, route: &Route) -> AppResult<()>;
    /// 移除指定的临时 IPv4 路由。
    ///
    /// # 作者
    /// lhy
    fn remove(&mut self, route: &Route) -> AppResult<()>;
}
/// 单条路由变更及其新增或移除方向。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    /// 本条变更涉及的路由。
    pub route: Route,
    /// 是否由本次操作新增；否则为本次移除。
    pub added: bool,
}
/// 绑定本次开机标识的配置、网卡环境与待撤销变更。
///
/// # 作者
/// lhy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    /// 创建记录时的 Windows 开机标识。
    pub boot_id: String,
    /// 保存的有效分流配置。
    pub config: Config,
    /// 启用时保存的两张接口及其 IPv4 环境。
    pub adapters: Vec<Adapter>,
    /// 尚待撤销的路由变更列表。
    pub changes: Vec<Change>,
}
/// 撤销记录存储边界。
///
/// # 作者
/// lhy
pub trait Journal {
    /// 读取存储中的撤销记录。
    ///
    /// # 作者
    /// lhy
    fn load(&self) -> AppResult<Option<Recovery>>;
    /// 保存或清空撤销记录。
    ///
    /// # 作者
    /// lhy
    fn save(&mut self, record: Option<&Recovery>) -> AppResult<()>;
}
/// 根据实际网卡和路由判定的分流状态。
///
/// # 作者
/// lhy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoutingStatus {
    /// 当前配置未生效。
    Inactive,
    /// 实际路由与配置匹配。
    Active,
    /// 本次记录对应的网络环境或路由发生变化。
    Changed,
}
/// 组合网络与记录存储的分流应用服务。
///
/// # 作者
/// lhy
pub struct Service<N, J> {
    /// 执行路由读写的网络实现。
    pub network: N,
    /// 撤销记录存储实现。
    pub journal: J,
}
/// 比较本次记录与当前网卡环境，允许本工具移除默认路由导致的网关缺失。
///
/// # 作者
/// lhy
fn same_environment(record: &Recovery, current: &[Adapter]) -> bool {
    record.adapters.iter().all(|old| {
        current.iter().any(|a| {
            let mut before = old.addresses.clone();
            before.sort();
            let mut after = a.addresses.clone();
            after.sort();
            a.id == old.id
                && before == after
                && (a.gateway == old.gateway
                    || (a.gateway.is_empty()
                        && record.changes.iter().any(|c| {
                            !c.added
                                && c.route.prefix == 0
                                && c.route.interface == old.id
                                && c.route.gateway.to_string() == old.gateway
                        })))
        })
    })
}
impl<N: Network, J: Journal> Service<N, J> {
    /// 只返回当前开机中仍有待撤销变更的记录。
    ///
    /// # 作者
    /// lhy
    pub fn current_record(&self) -> AppResult<Option<Recovery>> {
        let boot = self.network.boot_id()?;
        Ok(self
            .journal
            .load()?
            .filter(|r| r.boot_id == boot && !r.changes.is_empty()))
    }
    /// 判断当前开机是否存在尚未撤销的记录。
    ///
    /// # 作者
    /// lhy
    pub fn pending(&self) -> AppResult<bool> {
        Ok(self.current_record()?.is_some())
    }
    /// 结合实际路由、接口和本次记录判定分流状态。
    ///
    /// # 作者
    /// lhy
    pub fn status(
        &self,
        config: Option<&Config>,
        adapters: &[Adapter],
        routes: &[Route],
    ) -> AppResult<RoutingStatus> {
        let record = self.current_record()?;
        let Some(c) = record.as_ref().map(|r| &r.config).or(config) else {
            return Ok(RoutingStatus::Inactive);
        };
        if let Some(r) = &record {
            if !same_environment(r, adapters) {
                return Ok(RoutingStatus::Changed);
            }
        }
        let Ok(desired) = plan(c, adapters) else {
            return Ok(if record.is_some() {
                RoutingStatus::Changed
            } else {
                RoutingStatus::Inactive
            });
        };
        let active = routes.contains(&desired[0])
            && routes.iter().any(|r| r.same_key(&desired[1]))
            && routes
                .iter()
                .filter(|r| r.prefix == 0)
                .all(|r| r.same_key(&desired[1]));
        Ok(if active {
            RoutingStatus::Active
        } else if record.is_some() {
            RoutingStatus::Changed
        } else {
            RoutingStatus::Inactive
        })
    }
    /// 保存变更意图后写入路由，校验实际状态；失败时尝试回滚。
    ///
    /// 先写记录再修改系统，避免部分成功后丢失撤销依据。
    ///
    /// # 参数
    /// - `config`：本次需要应用的分流配置。
    ///
    /// # 错误
    /// 已有待撤销记录、第三接口默认路由冲突、目标路由已存在或写入校验失败时返回错误。
    ///
    /// # 作者
    /// lhy
    pub fn enable(&mut self, config: &Config) -> AppResult<()> {
        if self.pending()? {
            return Err("请先撤销本次修改再启用".into());
        }
        let adapters = self.network.adapters()?;
        let mut desired = plan(config, &adapters)?;
        let current = self.network.routes()?;
        if let Some(old) = current.iter().find(|r| r.same_key(&desired[1])) {
            desired[1] = old.clone();
        }
        if current
            .iter()
            .any(|r| r.prefix != 0 && r.same_key(&desired[0]))
        {
            return Err("目标内网路由已存在，请检查当前路由".into());
        }
        let mut changes = Vec::new();
        for route in &desired {
            if !current.contains(route) {
                changes.push(Change {
                    route: route.clone(),
                    added: true,
                });
            }
        }
        for route in current.iter().filter(|r| r.prefix == 0) {
            if route.interface != config.internal_id && route.interface != config.external_id {
                return Err("其他网卡存在默认路由，可能正在使用 VPN，请断开后再试".into());
            }
            if route != &desired[1] {
                changes.push(Change {
                    route: route.clone(),
                    added: false,
                });
            }
        }
        let record = Recovery {
            boot_id: self.network.boot_id()?,
            config: config.clone(),
            adapters: adapters
                .into_iter()
                .filter(|a| a.id == config.internal_id || a.id == config.external_id)
                .collect(),
            changes,
        };
        // 先记录操作意图，即使后续只完成部分写入，也能据此回滚。
        self.journal.save(Some(&record))?;
        let result: AppResult<()> = (|| {
            for change in &record.changes {
                if change.added {
                    self.network.add(&change.route)?;
                } else {
                    self.network.remove(&change.route)?;
                }
            }
            if self.status(
                Some(config),
                &self.network.adapters()?,
                &self.network.routes()?,
            )? != RoutingStatus::Active
            {
                return Err("写入后的实际路由校验失败".into());
            }
            Ok(())
        })();
        if let Err(error) = result {
            return match self.undo(true) {
                Ok(()) => Err(format!("启用失败，已撤销：{error}")),
                Err(undo) => Err(format!(
                    "启用失败：{error}；撤销未完成：{undo}。记录已保留。"
                )),
            };
        }
        Ok(())
    }
    /// 按本次记录撤销修改，先检查网卡环境及相关路由是否仍匹配。
    ///
    /// # 作者
    /// lhy
    pub fn restore(&mut self) -> AppResult<()> {
        self.undo(false)
    }
    /// 逆序撤销已记录变更，每步校验并保存进度；失败回滚允许尚未执行的意图。
    ///
    /// 逐步保存剩余记录，允许失败后继续处理未完成部分。
    ///
    /// # 参数
    /// - `rollback`：是否为启用失败后的回滚，允许记录中的部分写入尚未执行。
    ///
    /// # 错误
    /// 环境不匹配、相关路由被外部修改或逐步校验失败时停止并保留剩余记录。
    ///
    /// # 作者
    /// lhy
    fn undo(&mut self, rollback: bool) -> AppResult<()> {
        let Some(mut record) = self.current_record()? else {
            return Ok(());
        };
        if !same_environment(&record, &self.network.adapters()?) {
            return Err("网卡、IP 或网关已变化，已停止撤销；请检查当前网络".into());
        }
        let current = self.network.routes()?;
        for change in &record.changes {
            if !rollback && current.contains(&change.route) != change.added {
                return Err("相关路由已变化，已停止撤销；请检查当前路由".into());
            }
            if let Some(found) = current.iter().find(|r| r.same_key(&change.route)) {
                if found != &change.route {
                    return Err("相关路由已被其他软件修改，已停止撤销".into());
                }
            }
        }
        // 逆序处理，并在每步成功后缩短记录，避免失败重试时重复撤销。
        while let Some(change) = record.changes.last() {
            let current = self.network.routes()?;
            let found = current.iter().find(|r| r.same_key(&change.route));
            if let Some(found) = found {
                if found != &change.route {
                    return Err("路由在撤销期间发生变化，已停止操作".into());
                }
            }
            if change.added {
                if found.is_some() {
                    self.network.remove(&change.route)?;
                }
            } else if found.is_none() {
                self.network.add(&change.route)?;
            }
            let after = self.network.routes()?;
            let exists = after.contains(&change.route);
            if exists == change.added {
                return Err("撤销后的路由校验失败，记录已保留".into());
            }
            record.changes.pop();
            self.journal.save(Some(&record))?;
        }
        self.journal.save(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tests::fixture;
    /// 测试使用的内存撤销记录存储。
    ///
    /// # 作者
    /// lhy
    #[derive(Default)]
    struct Memory(Option<Recovery>);
    impl Journal for Memory {
        /// 读取测试内存中的撤销记录。
        ///
        /// # 作者
        /// lhy
        fn load(&self) -> AppResult<Option<Recovery>> {
            Ok(self.0.clone())
        }
        /// 替换测试内存中的撤销记录。
        ///
        /// # 作者
        /// lhy
        fn save(&mut self, r: Option<&Recovery>) -> AppResult<()> {
            self.0 = r.cloned();
            Ok(())
        }
    }
    /// 支持模拟写入失败和开机变化的网络测试替身。
    ///
    /// # 作者
    /// lhy
    struct Fake {
        /// 测试网络模拟的开机标识。
        boot: String,
        /// 启用时保存的两张接口及其 IPv4 环境。
        adapters: Vec<Adapter>,
        /// 测试网络中的路由条目列表。
        rows: Vec<Route>,
        /// 是否模拟下一次写入失败。
        fail_once: bool,
    }
    impl Network for Fake {
        /// 返回测试网络的模拟开机标识。
        ///
        /// # 作者
        /// lhy
        fn boot_id(&self) -> AppResult<String> {
            Ok(self.boot.clone())
        }
        /// 返回测试网络中的接口快照。
        ///
        /// # 作者
        /// lhy
        fn adapters(&self) -> AppResult<Vec<Adapter>> {
            Ok(self.adapters.clone())
        }
        /// 返回测试网络中的路由快照。
        ///
        /// # 作者
        /// lhy
        fn routes(&self) -> AppResult<Vec<Route>> {
            Ok(self.rows.clone())
        }
        /// 向测试路由表新增路由，并支持模拟一次失败。
        ///
        /// # 作者
        /// lhy
        fn add(&mut self, r: &Route) -> AppResult<()> {
            if self.fail_once {
                self.fail_once = false;
                return Err("injected".into());
            }
            self.rows.push(r.clone());
            Ok(())
        }
        /// 从测试路由表中移除匹配路由。
        ///
        /// # 作者
        /// lhy
        fn remove(&mut self, r: &Route) -> AppResult<()> {
            self.rows.retain(|v| v != r);
            Ok(())
        }
    }
    /// 构造应用流程测试所需的网络替身、存储和原始路由。
    ///
    /// # 作者
    /// lhy
    fn setup() -> (Config, Service<Fake, Memory>, Vec<Route>) {
        let (c, a) = fixture();
        let mut old = plan(&c, &a).unwrap();
        old.remove(0);
        old[0].metric = 35;
        old[0].protocol = 16;
        let mut internal = old[0].clone();
        internal.interface = c.internal_id.clone();
        internal.gateway = c.internal_gateway.parse().unwrap();
        old.push(internal);
        (
            c,
            Service {
                network: Fake {
                    boot: "boot-a".into(),
                    adapters: a,
                    rows: old.clone(),
                    fail_once: false,
                },
                journal: Memory::default(),
            },
            old,
        )
    }
    /// 验证重复启用与撤销后可回到原始路由状态。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn enable_restore_repeat() {
        let (c, mut s, old) = setup();
        s.enable(&c).unwrap();
        assert!(s.pending().unwrap());
        assert_eq!(
            s.status(Some(&c), &s.network.adapters, &s.network.rows)
                .unwrap(),
            RoutingStatus::Active
        );
        assert!(s.enable(&c).is_err());
        s.restore().unwrap();
        let mut actual = s.network.rows;
        actual.sort_by_key(|r| r.interface.clone());
        let mut old = old;
        old.sort_by_key(|r| r.interface.clone());
        assert_eq!(actual, old);
    }
    /// 验证相关路由缺失时停止手动撤销，但允许失败流程回滚。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn missing_route_stops_manual_undo_but_not_failure_rollback() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        s.network.rows.retain(|r| r.prefix != 8);
        let before = s.network.rows.clone();
        assert!(s.restore().is_err());
        assert_eq!(s.network.rows, before);
    }
    /// 验证工具自行移除默认路由不会被误判为网卡环境变化。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn own_default_removal_does_not_look_like_network_change() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        s.network.adapters[0].gateway.clear();
        assert_eq!(
            s.status(Some(&c), &s.network.adapters, &s.network.rows)
                .unwrap(),
            RoutingStatus::Active
        );
        s.restore().unwrap();
    }
    /// 验证路由写入失败后尝试回滚已执行变更。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn failure_rolls_back() {
        let (c, mut s, old) = setup();
        s.network.fail_once = true;
        assert!(s.enable(&c).is_err());
        assert_eq!(s.network.rows, old);
        assert!(!s.pending().unwrap());
    }
    /// 验证重启后不执行旧记录，也不阻止重新启用。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn reboot_does_not_replay_old_routes() {
        let (c, mut s, old) = setup();
        s.enable(&c).unwrap();
        s.network.boot = "boot-b".into();
        s.network.rows = old.clone();
        assert!(!s.pending().unwrap());
        s.restore().unwrap();
        assert_eq!(s.network.rows, old);
        assert_eq!(
            s.status(Some(&c), &s.network.adapters, &s.network.rows)
                .unwrap(),
            RoutingStatus::Inactive
        );
        s.enable(&c).unwrap();
    }
    /// 验证网卡环境变化后停止撤销。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn network_change_stops_undo() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        s.network.adapters[0].addresses = vec!["10.9.127.3/24".into()];
        let before = s.network.rows.clone();
        assert!(s.restore().is_err());
        assert_eq!(s.network.rows, before);
        assert!(s.pending().unwrap());
    }
    /// 验证外部修改相关路由时，在开始撤销前停止操作。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn external_route_change_stops_before_any_undo() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        s.network
            .rows
            .iter_mut()
            .find(|r| r.prefix == 8)
            .unwrap()
            .metric += 1;
        let before = s.network.rows.clone();
        assert!(s.restore().is_err());
        assert_eq!(s.network.rows, before);
        assert_eq!(
            s.status(Some(&c), &s.network.adapters, &s.network.rows)
                .unwrap(),
            RoutingStatus::Changed
        );
    }
    /// 验证第三接口默认路由冲突时不修改 VPN 等现有出口。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn vpn_is_untouched() {
        let (c, mut s, _) = setup();
        let mut vpn = s.network.rows[0].clone();
        vpn.interface = "3".into();
        s.network.rows.push(vpn);
        let before = s.network.rows.clone();
        assert!(s.enable(&c).is_err());
        assert_eq!(s.network.rows, before);
        assert!(!s.pending().unwrap());
    }
    /// 验证重开后的状态由实际路由决定。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn reopening_rechecks_real_routes() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        let s = Service {
            network: s.network,
            journal: s.journal,
        };
        assert_eq!(
            s.status(None, &s.network.adapters, &s.network.rows)
                .unwrap(),
            RoutingStatus::Active
        );
    }
}
