use base64::Engine;
use serde::{Deserialize, Serialize};

pub const HELPER_FLAG: &str = "--uacskip-task-helper";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperAction {
    /// "create" | "delete"
    pub op: String,
    pub task: String,
    #[serde(default)]
    pub exe: String,
    #[serde(default)]
    pub args: String,
    #[serde(default)]
    pub workdir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperPayload {
    pub actions: Vec<HelperAction>,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperActionResult {
    pub task: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperResult {
    pub ok: bool,
    pub results: Vec<HelperActionResult>,
}

impl HelperPayload {
    pub fn encode(&self) -> Result<String, String> {
        let json = serde_json::to_vec(self).map_err(|e| format!("序列化提权请求失败: {e}"))?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json))
    }
}

/// 若当前进程是提权助手，则执行任务并返回退出码
pub fn try_run_helper() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == HELPER_FLAG)?;
    let raw = args.get(pos + 1)?.trim().to_string();
    let payload = match base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<HelperPayload>(&bytes).ok())
    {
        Some(payload) => payload,
        None => return Some(2),
    };

    let mut results = Vec::with_capacity(payload.actions.len());
    for action in &payload.actions {
        let outcome = if action.op == "delete" {
            crate::task::delete_task(&action.task)
        } else {
            crate::task::create_task(
                &action.task,
                &action.exe,
                &action.args,
                action.workdir.as_deref(),
            )
        };
        results.push(HelperActionResult {
            task: action.task.clone(),
            ok: outcome.is_ok(),
            error: outcome.err(),
        });
    }

    let all_ok = results.iter().all(|item| item.ok);
    let payload_result = HelperResult {
        ok: all_ok,
        results,
    };
    if let Ok(text) = serde_json::to_string(&payload_result) {
        let _ = std::fs::write(&payload.result, text);
    }
    Some(if all_ok { 0 } else { 1 })
}

/// 通过提权助手批量执行任务操作（会触发一次 UAC 授权）
pub fn run_actions(actions: Vec<HelperAction>) -> Result<Vec<HelperActionResult>, String> {
    if actions.is_empty() {
        return Ok(Vec::new());
    }
    let result_path =
        std::env::temp_dir().join(format!("uacskip-{}.json", uuid::Uuid::new_v4().simple()));
    let payload = HelperPayload {
        actions,
        result: result_path.to_string_lossy().to_string(),
    };
    let encoded = payload.encode()?;
    let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {e}"))?;
    let params = format!("{HELPER_FLAG} {encoded}");
    let _ = std::fs::remove_file(&result_path);
    let code = crate::elevate::run_elevated(&exe, &params)?;

    let text = std::fs::read_to_string(&result_path)
        .map_err(|_| format!("提权助手没有返回结果（退出码 {code}）"))?;
    let _ = std::fs::remove_file(&result_path);
    let parsed: HelperResult =
        serde_json::from_str(&text).map_err(|e| format!("解析提权结果失败: {e}"))?;
    Ok(parsed.results)
}
