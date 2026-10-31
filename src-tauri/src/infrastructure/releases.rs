//! 从固定 GitHub 仓库读取发布信息并比较语义化版本。
//!
//! # 作者
//! lhy

use crate::domain::AppResult;
use serde::{Deserialize, Serialize};
/// 当前应用与已发布版本的比较结果。
///
/// # 作者
/// lhy
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 是否存在高于当前版本的正式发布。
    pub available: bool,
    /// 发布版本号。
    pub latest: String,
    /// 仓库是否存在正式发布。
    pub published: bool,
}
/// 读取固定仓库的最新正式发布，区分无发布和请求失败。
///
/// # 作者
/// lhy
pub async fn check_update() -> AppResult<UpdateInfo> {
    let response = reqwest::Client::builder()
        .user_agent("SplitRoute/0.1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?
        .get("https://api.github.com/repos/zorvilo/SplitRoute/releases/latest")
        .send()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            available: false,
            latest: env!("CARGO_PKG_VERSION").into(),
            published: false,
        });
    }
    if !response.status().is_success() {
        return Err(format!("GitHub 返回 {}，请稍后重试", response.status()));
    }
    /// GitHub 发布接口返回的版本标签字段。
    ///
    /// # 作者
    /// lhy
    #[derive(Deserialize)]
    struct Release {
        /// GitHub 发布的版本标签。
        tag_name: String,
    }
    let release: Release = response.json().await.map_err(|e| e.to_string())?;
    release_info(&release.tag_name)
}
/// 解析发布标签并与编译时版本比较，判断是否存在新版。
///
/// # 作者
/// lhy
fn release_info(tag: &str) -> AppResult<UpdateInfo> {
    let latest =
        semver::Version::parse(tag.trim_start_matches('v')).map_err(|_| "发布版本号无效")?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    Ok(UpdateInfo {
        available: latest > current,
        latest: latest.to_string(),
        published: true,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    /// 验证未来版本、当前版本、旧版和非法标签的更新判断。
    ///
    /// # 作者
    /// lhy
    #[test]
    fn compares_actual_release_versions() {
        let current = env!("CARGO_PKG_VERSION");
        let mut next = semver::Version::parse(current).unwrap();
        next.major += 1;
        assert!(release_info(&format!("v{next}")).unwrap().available);
        assert!(!release_info(current).unwrap().available);
        assert!(!release_info("0.1.0").unwrap().available);
        assert!(!release_info("v0.1.0-beta.1").unwrap().available);
        assert!(release_info("invalid").is_err());
    }
}
