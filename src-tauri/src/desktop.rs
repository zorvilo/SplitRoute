//! 托盘菜单、窗口关闭行为及固定项目链接。
//!
//! # 作者
//! lhy

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
/// 显示主窗口、恢复最小化并请求焦点。
///
/// # 作者
/// lhy
fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
/// 按界面语言创建托盘的打开与退出菜单。
///
/// # 作者
/// lhy
fn menu(app: &tauri::AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let (open, exit) = match language {
        "en" => ("Open SplitRoute", "Exit"),
        "ja" => ("SplitRoute を開く", "終了"),
        "zh-TW" => ("開啟 SplitRoute", "結束"),
        _ => ("打开 SplitRoute", "退出"),
    };
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", open, true, None::<&str>)?,
            &MenuItem::with_id(app, "exit", exit, true, None::<&str>)?,
        ],
    )
}
/// 在语言变化后更新现有托盘菜单。
///
/// # 作者
/// lhy
pub fn update_menu(app: &tauri::AppHandle, language: &str) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu(app, language)?))?;
    }
    Ok(())
}
/// 初始化托盘与主题，并根据偏好决定关闭窗口时隐藏还是退出。
///
/// # 作者
/// lhy
pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let state = app.state::<crate::settings::PreferencesState>();
    let preferences = state.0.lock().expect("preferences lock").clone();
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("SplitRoute")
        .menu(&menu(app.handle(), &preferences.language)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "exit" => app.exit(0),
            _ => (),
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    if let Some(w) = app.get_webview_window("main") {
        w.set_theme(match preferences.theme.as_str() {
            "dark" => Some(tauri::Theme::Dark),
            "light" => Some(tauri::Theme::Light),
            _ => None,
        })?;
        let window = w.clone();
        w.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window
                    .app_handle()
                    .state::<crate::settings::PreferencesState>();
                if state.0.lock().map(|p| p.close_to_tray).unwrap_or(false) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        });
    }
    Ok(())
}

/// 仅打开预定义的项目链接，拒绝任意目标。
///
/// # 作者
/// lhy
pub fn open_project_link(target: String) -> crate::domain::AppResult<()> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::HWND,
            UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        },
    };
    let url = match target.as_str() {
        "issues" => "https://github.com/zorvilo/SplitRoute/issues",
        "github" => "https://github.com/zorvilo/SplitRoute",
        "website" => "https://github.com/zorvilo/SplitRoute#readme",
        "changelog" => "https://github.com/zorvilo/SplitRoute/releases",
        "download" => "https://github.com/zorvilo/SplitRoute/releases/latest",
        _ => return Err("不支持的链接".into()),
    };
    let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    let link: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(operation.as_ptr()),
            PCWSTR(link.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err("无法打开系统浏览器".into());
    }
    Ok(())
}
