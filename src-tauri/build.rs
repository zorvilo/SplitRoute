fn main() {
    let manifest = include_str!("app.manifest");
    // Development and read-only unit tests run without elevation; shipped releases request it.
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
