//! 运行时探针：验证 COM 任务计划、图标提取、清单探测是否真正可用。
//! 用法: cargo run --example task_probe

fn main() {
    println!("elevated = {}", uac_skip_lib::elevate::is_elevated());

    let name = "UACSkip-Probe-Test";
    println!("task exists (before) = {:?}", uac_skip_lib::task::task_exists(name));

    let notepad = "C:\\Windows\\System32\\notepad.exe";
    match uac_skip_lib::task::create_task(name, notepad, "", None) {
        Ok(()) => println!("create_task = OK"),
        Err(error) => println!("create_task = ERR: {error}"),
    }
    println!("task exists (after) = {:?}", uac_skip_lib::task::task_exists(name));

    match uac_skip_lib::task::delete_task(name) {
        Ok(()) => println!("delete_task = OK"),
        Err(error) => println!("delete_task = ERR: {error}"),
    }
    println!("task exists (final) = {:?}", uac_skip_lib::task::task_exists(name));

    for path in [
        "C:\\Windows\\System32\\notepad.exe",
        "C:\\Windows\\System32\\cmd.exe",
        "C:\\Windows\\regedit.exe",
        "C:\\Windows\\System32\\taskmgr.exe",
        "C:\\Windows\\System32\\mmc.exe",
    ] {
        match uac_skip_lib::icons::extract_png(path) {
            Some(bytes) => {
                let target = format!(
                    "{}\\icon-probe-{}.png",
                    std::env::temp_dir().display(),
                    std::path::Path::new(path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                );
                let _ = std::fs::write(&target, &bytes);
                println!("icon {path} = {} bytes", bytes.len());
            }
            None => println!("icon {path} = NONE"),
        }
        println!(
            "needs_admin {path} = {}",
            uac_skip_lib::icons::detect_needs_admin(path)
        );
        match uac_skip_lib::icons::read_manifest(path) {
            Some(text) => {
                let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
                let level = flat
                    .split("requestedExecutionLevel")
                    .nth(1)
                    .map(|rest| rest.chars().take(70).collect::<String>())
                    .unwrap_or_else(|| "<no requestedExecutionLevel>".to_string());
                println!("  manifest level: {level}");
            }
            None => println!("  manifest: <none>"),
        }
    }
}
