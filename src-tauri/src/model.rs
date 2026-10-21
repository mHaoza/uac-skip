use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 启动方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    /// 通过计划任务以管理员权限启动（不弹 UAC）
    #[default]
    Highest,
    /// 直接以当前权限启动
    Normal,
}

/// 一条被托管的启动项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub mode: LaunchMode,
    /// 已注册的计划任务名（无则未启用免 UAC）
    #[serde(default)]
    pub task_name: Option<String>,
    /// 启动命令被修改过，需要重新注册任务
    #[serde(default)]
    pub dirty: bool,
    /// 从 PE 清单探测到需要管理员权限
    #[serde(default)]
    pub needs_admin: bool,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub last_launched_at: Option<i64>,
    #[serde(default)]
    pub launch_count: u32,
}

impl AppEntry {
    /// 实际交给计划任务 / CreateProcess 的命令行
    pub fn command(&self) -> (String, String) {
        let lower = self.path.to_ascii_lowercase();
        if lower.ends_with(".bat") || lower.ends_with(".cmd") {
            let inner = if self.args.trim().is_empty() {
                format!("\"{}\"", self.path)
            } else {
                format!("\"{}\" {}", self.path, self.args)
            };
            let comspec = std::env::var("COMSPEC")
                .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());
            (comspec, format!("/c \"{}\"", inner))
        } else {
            (self.path.clone(), self.args.clone())
        }
    }

    /// 工作目录：未指定时使用程序所在目录
    pub fn effective_workdir(&self) -> Option<String> {
        match &self.working_dir {
            Some(w) if !w.trim().is_empty() => Some(w.clone()),
            _ => std::path::Path::new(&self.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string()),
        }
    }
}

/// 返回给前端的视图
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    #[serde(flatten)]
    pub entry: AppEntry,
    /// 目标文件是否仍然存在
    pub path_exists: bool,
    /// 计划任务是否存在（None = 无法确认）
    pub task_exists: Option<bool>,
    /// 是否已处于「免 UAC」可用状态
    pub enabled: bool,
}

impl AppView {
    pub fn new(entry: AppEntry, task_exists: Option<bool>) -> Self {
        let path_exists = std::path::Path::new(&entry.path).is_file();
        let enabled = entry.mode == LaunchMode::Highest
            && entry.task_name.is_some()
            && task_exists == Some(true)
            && !entry.dirty;
        Self {
            entry,
            path_exists,
            task_exists,
            enabled,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppPatch {
    pub name: Option<String>,
    pub path: Option<String>,
    pub args: Option<String>,
    pub working_dir: Option<String>,
    pub mode: Option<LaunchMode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOutcome {
    pub ok: bool,
    pub path: String,
    pub id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutcome {
    pub id: String,
    pub ok: bool,
    pub message: Option<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOutcome {
    pub ok: bool,
    pub message: Option<String>,
    pub elevated: bool,
    /// 建议改用免 UAC 启动（目标程序要求管理员权限）
    pub suggest_elevate: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveOutcome {
    pub ok: bool,
    pub message: Option<String>,
}
