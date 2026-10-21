use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// 当前进程是否已提权
pub fn is_elevated() -> bool {
    unsafe {
        let mut token = Default::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

/// 以管理员身份启动指定程序并等待其退出，返回退出码
pub fn run_elevated(exe: &Path, params: &str) -> Result<i32, String> {
    unsafe {
        let file = wide(&exe.to_string_lossy());
        let args = wide(params);
        let verb = wide("runas");
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(args.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        if let Err(err) = ShellExecuteExW(&mut info) {
            let code = err.code().0 as u32;
            if code == 0x8007_04C7 {
                return Err("已取消管理员授权".to_string());
            }
            return Err(format!(
                "无法以管理员身份启动：{}",
                std::io::Error::from_raw_os_error(code as i32)
            ));
        }
        let handle = info.hProcess;
        if handle.is_invalid() {
            return Err("提权进程启动失败".to_string());
        }
        WaitForSingleObject(handle, INFINITE);
        let mut exit_code = 0u32;
        let _ = GetExitCodeProcess(handle, &mut exit_code);
        let _ = CloseHandle(handle);
        Ok(exit_code as i32)
    }
}

/// 以管理员身份重新启动自身（不等待）
pub fn spawn_self_elevated(exe: &Path, params: &str) -> Result<(), String> {
    unsafe {
        let file = wide(&exe.to_string_lossy());
        let args = wide(params);
        let verb = wide("runas");
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR(verb.as_ptr()),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(args.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        if let Err(err) = ShellExecuteExW(&mut info) {
            let code = err.code().0 as u32;
            if code == 0x8007_04C7 {
                return Err("已取消管理员授权".to_string());
            }
            return Err(format!(
                "无法以管理员身份重启：{}",
                std::io::Error::from_raw_os_error(code as i32)
            ));
        }
        if !info.hProcess.is_invalid() {
            let _ = CloseHandle(info.hProcess);
        }
        Ok(())
    }
}
