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
            return Err(
                "此操作需要管理员权限，请以管理员身份运行 SplitRoute。开发版默认以普通权限运行。"
                    .into(),
            );
        }
    }
    Ok(())
}
