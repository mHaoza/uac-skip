//! Windows 任务计划程序（Task Scheduler）封装。
//!
//! 免 UAC 的原理：注册一个「最高权限 + 交互式登录令牌」的计划任务，
//! 之后用 `IRegisteredTask::Run` 触发即可在用户会话中以管理员权限启动目标程序，
//! 且不会弹出 UAC 授权窗口。注册任务本身需要一次管理员权限。

use windows::core::{BSTR, Interface};
use windows::Win32::Foundation::{VARIANT_FALSE, VARIANT_TRUE};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IActionCollection, IExecAction, IPrincipal, ITaskDefinition, ITaskService, ITaskSettings,
    TaskScheduler, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE, TASK_INSTANCES_PARALLEL,
    TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_RUNLEVEL_LUA, TASK_RUNLEVEL_TYPE,
};
use windows::Win32::System::Variant::VARIANT;

const TASK_FOLDER: &str = "\\";
/// 任务不存在
const HR_FILE_NOT_FOUND: u32 = 0x8007_0002;
const HR_PATH_NOT_FOUND: u32 = 0x8007_0003;
/// 拒绝访问
const HR_ACCESS_DENIED: u32 = 0x8007_0005;

fn hr(e: &windows::core::Error) -> u32 {
    e.code().0 as u32
}

/// 当前登录用户的 `域\用户名`
fn current_user() -> Option<String> {
    let user = std::env::var("USERNAME").ok()?;
    if user.is_empty() {
        return None;
    }
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => Some(format!("{domain}\\{user}")),
        _ => Some(user),
    }
}

fn connect() -> Result<ITaskService, String> {
    unsafe {
        // 已初始化（RPC_E_CHANGED_MODE）时忽略即可
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("初始化任务计划服务失败: {e}"))?;
        let empty = VARIANT::default();
        service
            .Connect(&empty, &empty, &empty, &empty)
            .map_err(|e| format!("连接任务计划服务失败: {e}"))?;
        Ok(service)
    }
}

fn root_folder(service: &ITaskService) -> Result<windows::Win32::System::TaskScheduler::ITaskFolder, String> {
    unsafe {
        service
            .GetFolder(&BSTR::from(TASK_FOLDER))
            .map_err(|e| format!("打开任务计划根目录失败: {e}"))
    }
}

/// 注册（或覆盖）一个免 UAC 的启动任务。需要管理员权限。
pub fn create_task(
    task_name: &str,
    exe: &str,
    args: &str,
    workdir: Option<&str>,
) -> Result<(), String> {
    register(task_name, exe, args, workdir, TASK_RUNLEVEL_HIGHEST)
}

fn register(
    task_name: &str,
    exe: &str,
    args: &str,
    workdir: Option<&str>,
    run_level: TASK_RUNLEVEL_TYPE,
) -> Result<(), String> {
    if task_name.is_empty() || exe.is_empty() {
        return Err("任务名或程序路径为空".to_string());
    }
    unsafe {
        let service = connect()?;
        let root = root_folder(&service)?;
        let definition: ITaskDefinition = service
            .NewTask(0)
            .map_err(|e| format!("创建任务定义失败: {e}"))?;

        if let Ok(info) = definition.RegistrationInfo() {
            let _ = info.SetAuthor(&BSTR::from("UACSkip"));
            let _ = info.SetDescription(&BSTR::from(
                "由 UACSkip 创建：以管理员权限启动目标程序，不弹出 UAC 提示。",
            ));
        }

        // 以当前登录用户的交互式令牌运行，并使用最高权限
        let principal: IPrincipal = definition
            .Principal()
            .map_err(|e| format!("读取任务主体失败: {e}"))?;
        principal
            .SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)
            .map_err(|e| format!("设置登录类型失败: {e}"))?;
        principal
            .SetRunLevel(run_level)
            .map_err(|e| format!("设置运行级别失败: {e}"))?;
        if let Some(user) = current_user() {
            let _ = principal.SetUserId(&BSTR::from(user));
        }

        let settings: ITaskSettings = definition
            .Settings()
            .map_err(|e| format!("读取任务设置失败: {e}"))?;
        // 允许手动触发、电池供电时也运行、不限制执行时长、允许多开
        let _ = settings.SetAllowDemandStart(VARIANT_TRUE);
        let _ = settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE);
        let _ = settings.SetStopIfGoingOnBatteries(VARIANT_FALSE);
        let _ = settings.SetExecutionTimeLimit(&BSTR::from("PT0S"));
        let _ = settings.SetMultipleInstances(TASK_INSTANCES_PARALLEL);
        let _ = settings.SetEnabled(VARIANT_TRUE);
        let _ = settings.SetHidden(VARIANT_FALSE);

        let actions: IActionCollection = definition
            .Actions()
            .map_err(|e| format!("读取任务动作失败: {e}"))?;
        let action = actions
            .Create(TASK_ACTION_EXEC)
            .map_err(|e| format!("创建启动动作失败: {e}"))?;
        let exec: IExecAction = action
            .cast()
            .map_err(|e| format!("转换启动动作失败: {e}"))?;
        exec.SetPath(&BSTR::from(exe))
            .map_err(|e| format!("设置程序路径失败: {e}"))?;
        if !args.trim().is_empty() {
            exec.SetArguments(&BSTR::from(args))
                .map_err(|e| format!("设置启动参数失败: {e}"))?;
        }
        if let Some(dir) = workdir.filter(|d| !d.trim().is_empty()) {
            exec.SetWorkingDirectory(&BSTR::from(dir))
                .map_err(|e| format!("设置工作目录失败: {e}"))?;
        }

        let empty = VARIANT::default();
        let registered = root
            .RegisterTaskDefinition(
                &BSTR::from(task_name),
                &definition,
                TASK_CREATE_OR_UPDATE.0,
                &empty,
                &empty,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty,
            )
            .map_err(|e| {
                if hr(&e) == HR_ACCESS_DENIED {
                    "注册任务被拒绝：需要管理员权限".to_string()
                } else {
                    format!("注册计划任务失败: {e}")
                }
            })?;

        // 读回校验：确认「最高权限」真的写进去了，避免静默降级成普通权限
        if run_level == TASK_RUNLEVEL_HIGHEST {
            let stored = registered
                .Definition()
                .and_then(|definition| definition.Principal())
                .and_then(|principal| {
                    let mut level = TASK_RUNLEVEL_LUA;
                    principal.RunLevel(&mut level).map(|_| level)
                });
            match stored {
                Ok(level) if level != TASK_RUNLEVEL_HIGHEST => {
                    return Err("计划任务已创建，但最高权限级别未生效".to_string())
                }
                Err(e) => return Err(format!("无法校验计划任务权限级别: {e}")),
                _ => {}
            }
        }

        Ok(())
    }
}

/// 删除任务
pub fn delete_task(task_name: &str) -> Result<(), String> {
    if task_name.is_empty() {
        return Ok(());
    }
    unsafe {
        let service = connect()?;
        let root = root_folder(&service)?;
        match root.DeleteTask(&BSTR::from(task_name), 0) {
            Ok(()) => Ok(()),
            Err(e) => match hr(&e) {
                HR_FILE_NOT_FOUND | HR_PATH_NOT_FOUND => Ok(()),
                HR_ACCESS_DENIED => Err("删除计划任务被拒绝：需要管理员权限".to_string()),
                _ => Err(format!("删除计划任务失败: {e}")),
            },
        }
    }
}

/// 任务是否存在。None 表示无法确认（例如权限不足）。
pub fn task_exists(task_name: &str) -> Option<bool> {
    if task_name.is_empty() {
        return Some(false);
    }
    let service = connect().ok()?;
    let root = root_folder(&service).ok()?;
    unsafe {
        match root.GetTask(&BSTR::from(task_name)) {
            Ok(_) => Some(true),
            Err(e) => match hr(&e) {
                HR_ACCESS_DENIED => None,
                _ => Some(false),
            },
        }
    }
}

/// 触发任务（免 UAC 启动）
pub fn run_task(task_name: &str) -> Result<(), String> {
    if task_name.is_empty() {
        return Err("尚未创建免 UAC 启动项".to_string());
    }
    let service = connect()?;
    let root = root_folder(&service)?;
    unsafe {
        let task = root
            .GetTask(&BSTR::from(task_name))
            .map_err(|e| format!("找不到计划任务 {}: {e}", task_name))?;
        let empty = VARIANT::default();
        task.Run(&empty)
            .map(|_| ())
            .map_err(|e| format!("启动计划任务失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::TaskScheduler::TASK_RUNLEVEL_LUA;

    /// 以「普通权限级别」注册任务，不需要管理员授权。
    /// 用于验证任务定义（主体、设置、动作、注册）的完整构造是否正确。
    #[test]
    fn registers_and_removes_limited_task() {
        let name = "UACSkip-SelfTest";
        let exe = "C:\\Windows\\System32\\notepad.exe";

        let created = register(name, exe, "", None, TASK_RUNLEVEL_LUA);
        println!("register -> {created:?}");
        assert!(created.is_ok(), "任务注册失败: {created:?}");
        assert_eq!(task_exists(name), Some(true), "注册后任务应存在");

        // 导出 XML，确认各项设置真的落到了系统里
        if let Ok(output) = std::process::Command::new("schtasks")
            .args(["/query", "/tn", name, "/xml", "ONE"])
            .output()
        {
            let xml = String::from_utf8_lossy(&output.stdout).to_string();
            for probe in [
                "<UserId>",
                "<MultipleInstancesPolicy>Parallel</MultipleInstancesPolicy>",
                "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
                "<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>",
                "<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>",
                "<Command>C:\\Windows\\System32\\notepad.exe</Command>",
            ] {
                assert!(xml.contains(probe), "任务 XML 缺少 {probe}\n{xml}");
            }
            // 无触发器：任务只能被手动触发
            assert!(
                xml.contains("<Triggers />") || xml.contains("<Triggers/>"),
                "任务不应包含触发器\n{xml}"
            );
            println!("xml settings verified");
        }

        assert!(delete_task(name).is_ok(), "删除任务失败");
        assert_eq!(task_exists(name), Some(false), "删除后任务应不存在");
    }

    #[test]
    fn empty_name_is_rejected() {
        assert!(create_task("", "C:\\Windows\\System32\\notepad.exe", "", None).is_err());
    }
}
