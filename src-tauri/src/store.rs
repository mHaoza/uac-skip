use crate::model::AppEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreData {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
}

fn default_version() -> u32 {
    1
}

impl Default for StoreData {
    fn default() -> Self {
        Self {
            version: 1,
            apps: Vec::new(),
        }
    }
}

pub struct StoreState {
    pub file: PathBuf,
    /// 配置是否放在程序所在目录（便携模式）
    pub portable: bool,
    pub data: Mutex<StoreData>,
}

/// 选择一个可写的目录来存放配置
///
/// 优先使用「exe 所在目录」，这样整个程序连同数据可以一起复制、迁移；
/// 只有当该目录不可写（例如装在只读位置）时才回退到用户数据目录。
pub fn resolve_dir(fallback: PathBuf) -> (PathBuf, bool) {
    if let Some(dir) = exe_dir() {
        if is_writable(&dir) {
            return (dir, true);
        }
    }
    (fallback, false)
}

fn exe_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(Path::to_path_buf)
}

fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".uacskip-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

impl StoreState {
    pub fn load(file: PathBuf, portable: bool) -> Self {
        let data = match std::fs::read_to_string(&file) {
            Ok(text) if !text.trim().is_empty() => match serde_json::from_str::<StoreData>(&text) {
                Ok(data) => data,
                Err(err) => {
                    // 配置损坏时备份后重建，避免用户数据被静默覆盖
                    let backup = file.with_extension("json.broken");
                    let _ = std::fs::copy(&file, &backup);
                    eprintln!("[uacskip] 配置解析失败，已备份到 {:?}: {err}", backup);
                    StoreData::default()
                }
            },
            _ => StoreData::default(),
        };
        Self {
            file,
            portable,
            data: Mutex::new(data),
        }
    }

    /// 获取数据锁；即使之前有线程 panic 也能继续使用
    pub fn lock(&self) -> std::sync::MutexGuard<'_, StoreData> {
        self.data.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub fn persist(&self, data: &StoreData) -> Result<(), String> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
        }
        let tmp = self.file.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(data).map_err(|e| format!("序列化配置失败: {e}"))?;
        std::fs::write(&tmp, text).map_err(|e| format!("写入配置失败: {e}"))?;
        std::fs::rename(&tmp, &self.file).map_err(|e| format!("保存配置失败: {e}"))?;
        Ok(())
    }

    pub fn store_dir(&self) -> PathBuf {
        self.file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
