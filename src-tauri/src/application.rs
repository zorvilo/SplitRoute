use crate::domain::{plan, Adapter, AppResult, Config, Route};
use serde::{Deserialize, Serialize};

pub trait Network {
    fn boot_id(&self) -> AppResult<String>;
    fn adapters(&self) -> AppResult<Vec<Adapter>>;
    fn routes(&self) -> AppResult<Vec<Route>>;
    fn add(&mut self, route: &Route) -> AppResult<()>;
    fn remove(&mut self, route: &Route) -> AppResult<()>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub route: Route,
    pub added: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    pub boot_id: String,
    pub config: Config,
    pub adapters: Vec<Adapter>,
    pub changes: Vec<Change>,
}
pub trait Journal {
    fn load(&self) -> AppResult<Option<Recovery>>;
    fn save(&mut self, record: Option<&Recovery>) -> AppResult<()>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RoutingStatus {
    Inactive,
    Active,
    Changed,
}
pub struct Service<N, J> {
    pub network: N,
    pub journal: J,
}
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
    pub fn current_record(&self) -> AppResult<Option<Recovery>> {
        let boot = self.network.boot_id()?;
        Ok(self
            .journal
            .load()?
            .filter(|r| r.boot_id == boot && !r.changes.is_empty()))
    }
    pub fn pending(&self) -> AppResult<bool> {
        Ok(self.current_record()?.is_some())
    }
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
    pub fn restore(&mut self) -> AppResult<()> {
        self.undo(false)
    }
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
    #[derive(Default)]
    struct Memory(Option<Recovery>);
    impl Journal for Memory {
        fn load(&self) -> AppResult<Option<Recovery>> {
            Ok(self.0.clone())
        }
        fn save(&mut self, r: Option<&Recovery>) -> AppResult<()> {
            self.0 = r.cloned();
            Ok(())
        }
    }
    struct Fake {
        boot: String,
        adapters: Vec<Adapter>,
        rows: Vec<Route>,
        fail_once: bool,
    }
    impl Network for Fake {
        fn boot_id(&self) -> AppResult<String> {
            Ok(self.boot.clone())
        }
        fn adapters(&self) -> AppResult<Vec<Adapter>> {
            Ok(self.adapters.clone())
        }
        fn routes(&self) -> AppResult<Vec<Route>> {
            Ok(self.rows.clone())
        }
        fn add(&mut self, r: &Route) -> AppResult<()> {
            if self.fail_once {
                self.fail_once = false;
                return Err("injected".into());
            }
            self.rows.push(r.clone());
            Ok(())
        }
        fn remove(&mut self, r: &Route) -> AppResult<()> {
            self.rows.retain(|v| v != r);
            Ok(())
        }
    }
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
    #[test]
    fn missing_route_stops_manual_undo_but_not_failure_rollback() {
        let (c, mut s, _) = setup();
        s.enable(&c).unwrap();
        s.network.rows.retain(|r| r.prefix != 8);
        let before = s.network.rows.clone();
        assert!(s.restore().is_err());
        assert_eq!(s.network.rows, before);
    }
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
    #[test]
    fn failure_rolls_back() {
        let (c, mut s, old) = setup();
        s.network.fail_once = true;
        assert!(s.enable(&c).is_err());
        assert_eq!(s.network.rows, old);
        assert!(!s.pending().unwrap());
    }
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
