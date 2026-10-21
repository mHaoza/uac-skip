//! 命令行静默模式：`uac-skip.exe --run <任务名>`
//!
//! 桌面快捷方式就是指向这个模式。它不创建窗口，只触发一次计划任务后立即退出，
//! 因此双击快捷方式等于直接启动目标程序，且不会弹出 UAC。

use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE};

pub const RUN_FLAG: &str = "--run";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn show_error(message: &str) {
    let text = wide(message);
    let caption = wide("UACSkip");
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MESSAGEBOX_STYLE(MB_OK.0 | MB_ICONERROR.0),
        );
    }
}

/// 若当前是 `--run` 模式则执行并返回退出码
pub fn try_run_task_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let position = args.iter().position(|arg| arg == RUN_FLAG)?;
    let task = match args.get(position + 1).map(|value| value.trim()) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return Some(2),
    };

    match crate::task::run_task(&task) {
        Ok(()) => Some(0),
        Err(message) => {
            show_error(&format!(
                "无法启动该程序：{message}\n\n请打开 UACSkip，为它重新启用「免 UAC 启动」。"
            ));
            Some(1)
        }
    }
}
