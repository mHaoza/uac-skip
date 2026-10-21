use crate::elevate;
use crate::helper::{self, HelperAction, HelperActionResult};
use crate::icons;
use crate::model::*;
use crate::store::StoreState;
use crate::task;
use base64::Engine;
use std::path::Path;
use tauri::{Manager, State};

/* ---------------- 工具函数 ---------------- */

fn normalize_path(raw: &str) -> Result<String, String> {
    let path = Path::new(raw.trim());
    let absolute = std::fs::canonicalize(path).map_err(|_| format!("无法访问该文件：{raw}"))?;
    let text = absolute.to_string_lossy().to_string();
    let cleaned = if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else {
        text
    };
    Ok(cleaned)
}

fn display_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn new_task_name(id: &str) -> String {
    let short: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(10)
        .collect();
    format!("UACSkip-{short}")
}

/// 拆分启动参数（支持双引号）
fn split_args(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for ch in raw.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started || !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started || !current.is_empty() {
        result.push(current);
    }
    result
}

fn bump_launch(state: &StoreState, id: &str) {
    let mut data = state.lock();
    if let Some(entry) = data.apps.iter_mut().find(|a| a.id == id) {
        entry.launch_count = entry.launch_count.saturating_add(1);
        entry.last_launched_at = Some(now_ms());
        let _ = state.persist(&data);
    }
}

fn view_of(state: &StoreState, id: &str) -> Option<AppView> {
    let entry = state.lock().apps.iter().find(|a| a.id == id).cloned()?;
    let exists = match entry.task_name.as_deref() {
        Some(name) if !name.is_empty() => task::task_exists(name),
        _ => Some(false),
    };
    Some(AppView::new(entry, exists))
}

/* ---------------- 查询类命令 ---------------- */

#[tauri::command]
pub fn is_elevated() -> bool {
    elevate::is_elevated()
}

/// 配置文件位置信息（便携模式下位于 exe 同目录）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreInfo {
    pub path: String,
    pub portable: bool,
    pub dir: String,
}

#[tauri::command]
pub fn store_info(state: State<'_, StoreState>) -> StoreInfo {
    StoreInfo {
        path: state.file.to_string_lossy().to_string(),
        portable: state.portable,
        dir: state.store_dir().to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub fn list_apps(state: State<'_, StoreState>) -> Vec<AppView> {
    let entries: Vec<AppEntry> = state.lock().apps.clone();
    entries
        .into_iter()
        .map(|entry| {
            let exists = match entry.task_name.as_deref() {
                Some(name) if !name.is_empty() => task::task_exists(name),
                _ => Some(false),
            };
            AppView::new(entry, exists)
        })
        .collect()
}

#[tauri::command]
pub fn app_icon(path: String) -> Option<String> {
    let png = icons::extract_png(&path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    Some(format!("data:image/png;base64,{encoded}"))
}

#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("文件不存在，无法定位".to_string());
    }
    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{path}"))
        .spawn()
        .map_err(|e| format!("打开资源管理器失败: {e}"))?;
    Ok(())
}

/* ---------------- 增删改 ---------------- */

#[tauri::command]
pub fn add_apps(state: State<'_, StoreState>, paths: Vec<String>) -> Vec<AddOutcome> {
    paths.iter().map(|raw| add_one(&state, raw)).collect()
}

fn add_one(state: &StoreState, raw: &str) -> AddOutcome {
    let normalized = match normalize_path(raw) {
        Ok(path) => path,
        Err(message) => {
            return AddOutcome {
                ok: false,
                path: raw.to_string(),
                id: None,
                message: Some(message),
            }
        }
    };

    let lower = normalized.to_ascii_lowercase();
    if ![".exe", ".bat", ".cmd"].iter().any(|ext| lower.ends_with(ext)) {
        return AddOutcome {
            ok: false,
            path: normalized,
            id: None,
            message: Some("仅支持 .exe / .bat / .cmd 程序".to_string()),
        };
    }

    if state
        .lock()
        .apps
        .iter()
        .any(|app| app.path.eq_ignore_ascii_case(&normalized))
    {
        return AddOutcome {
            ok: false,
            path: normalized,
            id: None,
            message: Some("该程序已在列表中".to_string()),
        };
    }

    let needs_admin = icons::detect_needs_admin(&normalized);
    let entry = AppEntry {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: display_name(&normalized),
        path: normalized.clone(),
        args: String::new(),
        working_dir: None,
        mode: if needs_admin {
            LaunchMode::Highest
        } else {
            LaunchMode::Normal
        },
        task_name: None,
        dirty: false,
        needs_admin,
        created_at: now_ms(),
        last_launched_at: None,
        launch_count: 0,
    };
    let id = entry.id.clone();

    let mut data = state.lock();
    data.apps.push(entry);
    if let Err(message) = state.persist(&data) {
        data.apps.retain(|app| app.id != id);
        return AddOutcome {
            ok: false,
            path: normalized,
            id: None,
            message: Some(message),
        };
    }
    AddOutcome {
        ok: true,
        path: normalized,
        id: Some(id),
        message: None,
    }
}

#[tauri::command]
pub fn update_app(
    state: State<'_, StoreState>,
    id: String,
    patch: AppPatch,
) -> Result<AppView, String> {
    {
        let mut data = state.lock();
        let entry = data
            .apps
            .iter_mut()
            .find(|app| app.id == id)
            .ok_or_else(|| "找不到该启动项".to_string())?;

        let mut command_changed = false;

        if let Some(name) = patch.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                entry.name = trimmed.to_string();
            }
        }
        if let Some(path) = patch.path {
            let normalized = normalize_path(&path)?;
            if !entry.path.eq_ignore_ascii_case(&normalized) {
                entry.path = normalized;
                command_changed = true;
            }
        }
        if let Some(args) = patch.args {
            if entry.args != args {
                entry.args = args;
                command_changed = true;
            }
        }
        if let Some(dir) = patch.working_dir {
            let value = if dir.trim().is_empty() {
                None
            } else {
                Some(dir.trim().to_string())
            };
            if entry.working_dir != value {
                entry.working_dir = value;
                command_changed = true;
            }
        }
        if let Some(mode) = patch.mode {
            if entry.mode != mode {
                entry.mode = mode;
                command_changed = true;
            }
        }
        if command_changed && entry.task_name.is_some() {
            entry.dirty = true;
        }
        state.persist(&data)?;
    }

    view_of(&state, &id).ok_or_else(|| "找不到该启动项".to_string())
}

#[tauri::command]
pub fn remove_app(
    state: State<'_, StoreState>,
    id: String,
    delete_task: Option<bool>,
) -> Result<RemoveOutcome, String> {
    let task_name = {
        let data = state.lock();
        let entry = data
            .apps
            .iter()
            .find(|app| app.id == id)
            .ok_or_else(|| "找不到该启动项".to_string())?;
        entry.task_name.clone()
    };

    if delete_task.unwrap_or(true) {
        if let Some(name) = task_name.filter(|name| !name.is_empty()) {
            if task::task_exists(&name) != Some(false) {
                if elevate::is_elevated() {
                    task::delete_task(&name)?;
                } else {
                    let results = helper::run_actions(vec![HelperAction {
                        op: "delete".to_string(),
                        task: name,
                        exe: String::new(),
                        args: String::new(),
                        workdir: None,
                    }])?;
                    if let Some(failed) = results.iter().find(|item| !item.ok) {
                        return Err(failed
                            .error
                            .clone()
                            .unwrap_or_else(|| "删除计划任务失败".to_string()));
                    }
                }
            }
        }
    }

    {
        let mut data = state.lock();
        data.apps.retain(|app| app.id != id);
        state.persist(&data)?;
    }
    Ok(RemoveOutcome {
        ok: true,
        message: None,
    })
}

/* ---------------- 免 UAC 启动项注册 ---------------- */

#[tauri::command]
pub fn ensure_tasks(
    state: State<'_, StoreState>,
    ids: Option<Vec<String>>,
) -> Result<Vec<TaskOutcome>, String> {
    ensure_internal(&state, ids)
}

fn ensure_internal(
    state: &StoreState,
    ids: Option<Vec<String>>,
) -> Result<Vec<TaskOutcome>, String> {
    let (creates, deletes, mut outcomes) = {
        let mut data = state.lock();
        let mut creates: Vec<(String, HelperAction)> = Vec::new();
        let mut deletes: Vec<(String, HelperAction)> = Vec::new();
        let mut outcomes: Vec<TaskOutcome> = Vec::new();
        let mut changed = false;

        let indexes: Vec<usize> = data
            .apps
            .iter()
            .enumerate()
            .filter(|(_, app)| match &ids {
                Some(list) => list.iter().any(|id| id == &app.id),
                None => true,
            })
            .map(|(index, _)| index)
            .collect();

        for index in indexes {
            let entry = &mut data.apps[index];

            if !Path::new(&entry.path).is_file() {
                outcomes.push(TaskOutcome {
                    id: entry.id.clone(),
                    ok: false,
                    message: Some(format!("目标文件不存在：{}", entry.path)),
                    cancelled: false,
                });
                continue;
            }

            if entry.mode == LaunchMode::Highest {
                if entry
                    .task_name
                    .as_deref()
                    .map(str::is_empty)
                    .unwrap_or(true)
                {
                    entry.task_name = Some(new_task_name(&entry.id));
                    changed = true;
                }
                let name = entry.task_name.clone().unwrap_or_default();
                let ready = !entry.dirty && task::task_exists(&name) == Some(true);
                if !ready {
                    let (exe, args) = entry.command();
                    creates.push((
                        entry.id.clone(),
                        HelperAction {
                            op: "create".to_string(),
                            task: name,
                            exe,
                            args,
                            workdir: entry.effective_workdir(),
                        },
                    ));
                }
            } else if entry.task_name.is_some() {
                let name = entry.task_name.clone().unwrap_or_default();
                deletes.push((
                    entry.id.clone(),
                    HelperAction {
                        op: "delete".to_string(),
                        task: name,
                        exe: String::new(),
                        args: String::new(),
                        workdir: None,
                    },
                ));
            }
        }

        if changed {
            state.persist(&data)?;
        }
        (creates, deletes, outcomes)
    };

    if creates.is_empty() && deletes.is_empty() {
        return Ok(outcomes);
    }

    let actions: Vec<HelperAction> = creates
        .iter()
        .map(|(_, action)| action.clone())
        .chain(deletes.iter().map(|(_, action)| action.clone()))
        .collect();

    let results: Vec<HelperActionResult> = if elevate::is_elevated() {
        actions.iter().map(execute_action).collect()
    } else {
        match helper::run_actions(actions) {
            Ok(results) => results,
            Err(message) => {
                let cancelled = message.contains("取消");
                for (id, _) in creates.iter().chain(deletes.iter()) {
                    outcomes.push(TaskOutcome {
                        id: id.clone(),
                        ok: false,
                        message: Some(message.clone()),
                        cancelled,
                    });
                }
                return Ok(outcomes);
            }
        }
    };

    let mut data = state.lock();
    for result in &results {
        if let Some((id, _)) = creates.iter().find(|(_, a)| a.task == result.task) {
            if let Some(entry) = data.apps.iter_mut().find(|app| &app.id == id) {
                if result.ok {
                    entry.dirty = false;
                }
            }
            outcomes.push(TaskOutcome {
                id: id.clone(),
                ok: result.ok,
                message: result.error.clone(),
                cancelled: false,
            });
        } else if let Some((id, _)) = deletes.iter().find(|(_, a)| a.task == result.task) {
            if let Some(entry) = data.apps.iter_mut().find(|app| &app.id == id) {
                if result.ok {
                    entry.task_name = None;
                    entry.dirty = false;
                }
            }
            outcomes.push(TaskOutcome {
                id: id.clone(),
                ok: result.ok,
                message: result.error.clone(),
                cancelled: false,
            });
        }
    }
    state.persist(&data)?;

    Ok(outcomes)
}

fn execute_action(action: &HelperAction) -> HelperActionResult {
    let outcome = if action.op == "delete" {
        task::delete_task(&action.task)
    } else {
        task::create_task(
            &action.task,
            &action.exe,
            &action.args,
            action.workdir.as_deref(),
        )
    };
    HelperActionResult {
        task: action.task.clone(),
        ok: outcome.is_ok(),
        error: outcome.err(),
    }
}

/* ---------------- 启动 ---------------- */

#[tauri::command]
pub fn launch_app(state: State<'_, StoreState>, id: String) -> LaunchOutcome {
    launch_internal(&state, &id)
}

fn launch_internal(state: &StoreState, id: &str) -> LaunchOutcome {
    let entry = match state.lock().apps.iter().find(|app| app.id == id).cloned() {
        Some(entry) => entry,
        None => {
            return LaunchOutcome {
                ok: false,
                message: Some("找不到该启动项".to_string()),
                elevated: false,
                suggest_elevate: false,
            }
        }
    };

    if !Path::new(&entry.path).is_file() {
        return LaunchOutcome {
            ok: false,
            message: Some(format!("目标文件不存在：{}", entry.path)),
            elevated: false,
            suggest_elevate: false,
        };
    }

    if entry.mode == LaunchMode::Highest {
        let ready = !entry.dirty
            && entry
                .task_name
                .as_deref()
                .map(|name| task::task_exists(name) == Some(true))
                .unwrap_or(false);

        if !ready {
            match ensure_internal(state, Some(vec![entry.id.clone()])) {
                Ok(list) => {
                    if let Some(failed) = list.iter().find(|item| !item.ok) {
                        return LaunchOutcome {
                            ok: false,
                            message: Some(
                                failed
                                    .message
                                    .clone()
                                    .unwrap_or_else(|| "创建免 UAC 启动项失败".to_string()),
                            ),
                            elevated: false,
                            suggest_elevate: false,
                        };
                    }
                }
                Err(message) => {
                    return LaunchOutcome {
                        ok: false,
                        message: Some(message),
                        elevated: false,
                        suggest_elevate: false,
                    }
                }
            }
        }

        let task_name = state
            .lock()
            .apps
            .iter()
            .find(|app| app.id == id)
            .and_then(|app| app.task_name.clone());

        let Some(task_name) = task_name else {
            return LaunchOutcome {
                ok: false,
                message: Some("尚未创建免 UAC 启动项".to_string()),
                elevated: false,
                suggest_elevate: false,
            };
        };

        return match task::run_task(&task_name) {
            Ok(()) => {
                bump_launch(state, id);
                LaunchOutcome {
                    ok: true,
                    message: None,
                    elevated: true,
                    suggest_elevate: false,
                }
            }
            Err(message) => LaunchOutcome {
                ok: false,
                message: Some(message),
                elevated: false,
                suggest_elevate: false,
            },
        };
    }

    let (exe, args) = entry.command();
    let mut command = std::process::Command::new(&exe);
    command.args(split_args(&args));
    if let Some(dir) = entry.effective_workdir() {
        if Path::new(&dir).is_dir() {
            command.current_dir(dir);
        }
    }

    match command.spawn() {
        Ok(_) => {
            bump_launch(state, id);
            LaunchOutcome {
                ok: true,
                message: None,
                elevated: false,
                suggest_elevate: false,
            }
        }
        Err(error) => {
            let code = error.raw_os_error().unwrap_or(0);
            let (message, suggest) = if code == 740 {
                (
                    "该程序要求管理员权限，请改为「免 UAC 启动」".to_string(),
                    true,
                )
            } else {
                (format!("启动失败：{error}"), false)
            };
            LaunchOutcome {
                ok: false,
                message: Some(message),
                elevated: false,
                suggest_elevate: suggest,
            }
        }
    }
}

/* ---------------- 桌面快捷方式 ---------------- */

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() {
        "UACSkip 启动项".to_string()
    } else {
        trimmed
    }
}

/// 在桌面创建一个「双击即启动、不弹 UAC」的快捷方式
#[tauri::command]
pub fn create_desktop_shortcut(
    app: tauri::AppHandle,
    state: State<'_, StoreState>,
    id: String,
) -> Result<String, String> {
    let entry = state
        .lock()
        .apps
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or_else(|| "找不到该启动项".to_string())?;

    if entry.mode != LaunchMode::Highest {
        return Err("请先把启动方式改为「免 UAC 提权启动」".to_string());
    }

    let task_name = entry
        .task_name
        .clone()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "该程序还没有创建免 UAC 启动项，请先启用".to_string())?;

    if task::task_exists(&task_name) == Some(false) {
        return Err("免 UAC 启动项不存在，请先点击「重新授权」".to_string());
    }

    let self_exe =
        std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;
    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|e| format!("无法定位桌面目录: {e}"))?;

    let link_path = desktop.join(format!("{}.lnk", sanitize_file_name(&entry.name)));
    let (target_exe, _) = entry.command();
    let arguments = format!("{} {}", crate::cli::RUN_FLAG, task_name);

    crate::shortcut::create_link(
        &link_path,
        &self_exe,
        &arguments,
        self_exe.parent(),
        Some((Path::new(&target_exe), 0)),
        &format!("{} · 免 UAC 启动", entry.name),
    )?;

    Ok(link_path.to_string_lossy().to_string())
}

/* ---------------- 提权重启 ---------------- */
#[tauri::command]
pub fn relaunch_elevated(app: tauri::AppHandle) -> Result<(), String> {
    if elevate::is_elevated() {
        return Err("当前已经以管理员身份运行".to_string());
    }
    let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;
    elevate::spawn_self_elevated(&exe, "")?;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(700));
        app.exit(0);
    });
    Ok(())
}
