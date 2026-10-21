//! 通过 Shell 的 IShellLink 创建 Windows 快捷方式（.lnk）。

use std::path::Path;
use windows::core::{BSTR, HSTRING, Interface};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

/// 创建 .lnk 快捷方式
///
/// - `target`：快捷方式指向的可执行文件
/// - `arguments`：传给它的命令行参数
/// - `icon`：图标来源文件与索引
pub fn create_link(
    link_path: &Path,
    target: &Path,
    arguments: &str,
    working_dir: Option<&Path>,
    icon: Option<(&Path, i32)>,
    description: &str,
) -> Result<(), String> {
    unsafe {
        // 已初始化（RPC_E_CHANGED_MODE）时忽略
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("创建快捷方式对象失败: {e}"))?;

        link.SetPath(&BSTR::from(target.to_string_lossy().to_string()))
            .map_err(|e| format!("设置快捷方式目标失败: {e}"))?;

        if !arguments.trim().is_empty() {
            link.SetArguments(&BSTR::from(arguments))
                .map_err(|e| format!("设置快捷方式参数失败: {e}"))?;
        }

        if let Some(dir) = working_dir {
            let _ = link.SetWorkingDirectory(&BSTR::from(dir.to_string_lossy().to_string()));
        }

        if let Some((icon_path, index)) = icon {
            let _ = link.SetIconLocation(&BSTR::from(icon_path.to_string_lossy().to_string()), index);
        }

        if !description.is_empty() {
            let _ = link.SetDescription(&BSTR::from(description));
        }

        let persist: IPersistFile = link
            .cast()
            .map_err(|e| format!("获取快捷方式保存接口失败: {e}"))?;
        persist
            .Save(&HSTRING::from(link_path.to_string_lossy().to_string()), true)
            .map_err(|e| format!("写入快捷方式文件失败: {e}"))?;

        Ok(())
    }
}
