use crate::domain::AppResult;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};
pub fn require_admin() -> AppResult<()> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|e| format!("无法读取进程权限：{e}"))?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        let _ = CloseHandle(token);
        result.map_err(|e| format!("无法验证进程权限：{e}"))?;
        if elevation.TokenIsElevated == 0 {
            return Err("ADMIN_REQUIRED".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn restart_as_admin(
    config: Option<crate::domain::Config>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        use windows::{
            core::PCWSTR,
            Win32::UI::{
                Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
                WindowsAndMessaging::SW_SHOWNORMAL,
            },
        };
        if require_admin().is_ok() {
            return Ok(());
        }
        if let Some(config) = config {
            use crate::application::Network;
            crate::domain::plan(
                &config,
                &crate::infrastructure::network::WindowsNetwork.adapters()?,
            )?;
            crate::infrastructure::storage::FileJournal::new()?.save_config(&config)?;
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(path.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        unsafe {
            if let Err(e) = ShellExecuteExW(&mut info) {
                if e.code().0 as u32 == 0x800704c7 {
                    return Err("ELEVATION_CANCELLED".into());
                }
                return Err(format!("无法启动管理员进程：{e}"));
            }
            if !info.hProcess.is_invalid() {
                #[cfg(debug_assertions)]
                {
                    use tauri::Manager;
                    use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
                    // Keep the CLI-owned Vite service alive until the elevated debug app exits.
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                    app.remove_tray_by_id("main");
                    WaitForSingleObject(info.hProcess, INFINITE);
                }
                let _ = CloseHandle(info.hProcess);
            }
        }
        app.exit(0);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
