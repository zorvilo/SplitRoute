use crate::domain::AppResult;
use std::{os::windows::process::CommandExt, process::Command};
fn powershell(script: &str) -> AppResult<String> {
    let system = std::env::var_os("SystemRoot").ok_or("无法定位 Windows 目录")?;
    let exe =
        std::path::PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output=Command::new(exe).args(["-NoProfile","-NonInteractive","-Command",&format!("$ErrorActionPreference='Stop';[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new();{script}")]).creation_flags(0x08000000).output().map_err(|e|e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "登录启动设置失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn task_script(enable: bool, path: &str) -> String {
    let head="$user=[System.Security.Principal.WindowsIdentity]::GetCurrent();$name='SplitRoute-'+$user.User.Value;";
    if !enable {
        return format!("{head} $task=Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue;if($task){{Unregister-ScheduledTask -InputObject $task -Confirm:$false}}");
    }
    format!("{head} $action=New-ScheduledTaskAction -Execute {};$trigger=New-ScheduledTaskTrigger -AtLogOn -User $user.Name;$principal=New-ScheduledTaskPrincipal -UserId $user.Name -LogonType Interactive -RunLevel Highest;$settings=New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero);Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Start SplitRoute for the current user; does not enable routing.' -Force|Out-Null",quoted(path))
}
pub fn enabled() -> AppResult<bool> {
    let path = std::env::current_exe().map_err(|e| e.to_string())?;
    let script=format!("$user=[System.Security.Principal.WindowsIdentity]::GetCurrent();$task=Get-ScheduledTask -TaskName ('SplitRoute-'+$user.User.Value) -ErrorAction SilentlyContinue;if($task -and $task.State -ne 'Disabled' -and $task.Actions.Execute -eq {}){{'true'}}else{{'false'}}",quoted(&path.to_string_lossy()));
    Ok(powershell(&script)? == "true")
}

pub fn set_enabled(enabled: bool) -> AppResult<()> {
    crate::infrastructure::privilege::require_admin()?;
    powershell(&task_script(
        enabled,
        &std::env::current_exe()
            .map_err(|e| e.to_string())?
            .to_string_lossy(),
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_startup_path_without_command_injection() {
        let s = task_script(true, "C:\\Program Files\\O'Brien\\SplitRoute.exe");
        assert!(s.contains("O''Brien"));
        assert!(s.contains("-LogonType Interactive"));
        assert!(!s.contains("enable_route"));
    }
    #[test]
    fn reads_startup_without_changing_it() {
        let _ = enabled().unwrap();
    }
}
