//! Starting with Windows. The HKCU Run entry (tauri-plugin-autostart) is what Task Manager shows
//! and toggles, but Explorer does not always run it: from 2026-11-01 the user's PC ran every
//! other Run entry at sign-in and silently skipped FlowVoice's (enabled, right path; no Shell-Core
//! 9707 event for it). A per-user logon task in Task Scheduler starts the app as well; whichever
//! comes second meets the single-instance lock with `--hidden` and does nothing.

/// Argument the logon task adds, so a launch can tell where it came from.
pub const TASK_ARG: &str = "--logon-task";

#[cfg(windows)]
const TASK_NAME: &str = "FlowVoice";

/// Startup turned off in Task Manager (or Settings → Apps → Startup): the first byte of the
/// StartupApproved value is odd (02 = enabled, 03 = disabled by the user).
#[cfg(windows)]
pub fn disabled_in_task_manager(name: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_BINARY};
    let mut buf = [0u8; 32];
    let mut len = buf.len() as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"),
            &HSTRING::from(name),
            RRF_RT_REG_BINARY,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    };
    ok.is_ok() && len > 0 && buf[0] & 1 == 1
}

#[cfg(not(windows))]
pub fn disabled_in_task_manager(_name: &str) -> bool {
    false
}

#[cfg(windows)]
fn schtasks(args: &[&str]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(windows)]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Creates (or repoints) the logon task, or removes it. Runs schtasks.exe, so call it off the
/// main thread.
#[cfg(windows)]
pub fn set_logon_task(exe: &std::path::Path, on: bool) -> Result<(), String> {
    if !on {
        // Nothing to delete is fine: only a task that exists and can't be removed is an error.
        if schtasks(&["/Query", "/TN", TASK_NAME]).is_err() {
            return Ok(());
        }
        return schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]);
    }
    let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
        (Ok(d), Ok(u)) => format!("{d}\\{u}"),
        (_, Ok(u)) => u,
        _ => return Err("no USERNAME".into()),
    };
    let user = xml_escape(&user);
    let exe = xml_escape(&exe.to_string_lossy());
    // Delay: when the Run entry works, the app is already up (the task's launch is then a no-op);
    // battery and time limits off, normal priority (a task's default 7 is below normal).
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Inicia o FlowVoice com o Windows.</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId><Delay>PT20S</Delay></LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author"><Exec><Command>"{exe}"</Command><Arguments>--hidden {TASK_ARG}</Arguments></Exec></Actions>
</Task>
"#
    );
    // schtasks reads the file as UTF-16 (BOM + LE), which also keeps non-ASCII user names intact.
    let path = std::env::temp_dir().join(format!("flowvoice-task-{}.xml", std::process::id()));
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let res = schtasks(&["/Create", "/TN", TASK_NAME, "/XML", &path.to_string_lossy(), "/F"]);
    let _ = std::fs::remove_file(&path);
    res
}

#[cfg(not(windows))]
pub fn set_logon_task(_exe: &std::path::Path, _on: bool) -> Result<(), String> {
    Ok(())
}
