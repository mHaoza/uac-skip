#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 提权助手模式：由 ShellExecuteEx("runas") 拉起，仅创建/删除计划任务后立即退出
    if let Some(code) = uac_skip_lib::helper::try_run_helper() {
        std::process::exit(code);
    }
    // 静默触发模式：桌面快捷方式调用 `--run <任务名>`，不创建窗口
    if let Some(code) = uac_skip_lib::cli::try_run_task_cli() {
        std::process::exit(code);
    }
    uac_skip_lib::run();
}
