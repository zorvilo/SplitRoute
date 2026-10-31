//! 按构建类型生成 Windows 权限清单并执行 Tauri 构建配置。
//!
//! # 作者
//! lhy

/// 为正式构建请求管理员权限，为开发和只读测试使用普通权限清单。
///
/// # 作者
/// lhy
fn main() {
    let manifest = include_str!("app.manifest");
    // 开发版和只读测试使用普通权限，正式构建请求管理员权限。
    let manifest = if std::env::var("PROFILE").as_deref() == Ok("release") {
        manifest.to_owned()
    } else {
        manifest.replace("requireAdministrator", "asInvoker")
    };
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest)),
    )
    .expect("Tauri build failed");
}
