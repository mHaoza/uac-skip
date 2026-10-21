//! 从可执行文件中提取图标（PNG），并探测程序是否需要管理员权限。

use std::ffi::c_void;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{FreeLibrary, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::System::LibraryLoader::{
    FindResourceW, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
    LOAD_LIBRARY_AS_DATAFILE,
};
use windows::Win32::UI::Shell::{
    SHCreateItemFromParsingName, SHGetFileInfoW, IShellItemImageFactory, SHFILEINFOW, SHGFI_ICON,
    SHGFI_LARGEICON, SIIGBF, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

const RT_MANIFEST_ID: u32 = 24;
const ICON_TARGET: u32 = 128;

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn alpha_is_empty(buf: &[u8]) -> bool {
    buf.chunks_exact(4).all(|px| px[3] == 0)
}

/// 把 32 位 DIB 转成 RGBA
unsafe fn dib_to_rgba(hbm: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    if hbm.is_invalid() {
        return None;
    }
    let mut bm = BITMAP::default();
    let copied = GetObjectW(
        hbm.into(),
        size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut c_void),
    );
    if copied == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        return None;
    }
    let width = bm.bmWidth as u32;
    let height = bm.bmHeight as u32;

    let mut info = BITMAPINFO::default();
    info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width as i32;
    info.bmiHeader.biHeight = -(height as i32); // 自顶向下
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB.0;

    let mut buffer = vec![0u8; (width as usize) * (height as usize) * 4];
    let hdc = CreateCompatibleDC(None);
    if hdc.is_invalid() {
        return None;
    }
    let lines = GetDIBits(
        hdc,
        hbm,
        0,
        height,
        Some(buffer.as_mut_ptr() as *mut c_void),
        &mut info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(hdc);
    if lines == 0 {
        return None;
    }

    // BGRA -> RGBA
    for px in buffer.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    Some((width, height, buffer))
}

unsafe fn icon_to_rgba(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = ICONINFO::default();
    GetIconInfo(icon, &mut info).ok()?;

    let result = (|| -> Option<(u32, u32, Vec<u8>)> {
        let (width, height, mut color) = dib_to_rgba(info.hbmColor)?;
        if alpha_is_empty(&color) {
            // 旧式图标没有 alpha 通道，用掩码位图还原透明区域
            match dib_to_rgba(info.hbmMask) {
                Some((mw, mh, mask)) if mw == width && mh == height => {
                    for (px, m) in color.chunks_exact_mut(4).zip(mask.chunks_exact(4)) {
                        let luminance = (m[0] as u32 + m[1] as u32 + m[2] as u32) / 3;
                        px[3] = if luminance > 127 { 0 } else { 255 };
                    }
                }
                _ => {
                    for px in color.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                }
            }
        }
        Some((width, height, color))
    })();

    let _ = DeleteObject(info.hbmColor.into());
    let _ = DeleteObject(info.hbmMask.into());
    let _ = DestroyIcon(icon);
    result
}

/// 通过 Shell 取 256px 图标（质量最好）
unsafe fn rgba_from_shell(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let path_w = wide(path);
    let factory: IShellItemImageFactory =
        SHCreateItemFromParsingName(PCWSTR(path_w.as_ptr()), None).ok()?;
    let bitmap = factory
        .GetImage(
            SIZE { cx: 256, cy: 256 },
            SIIGBF(SIIGBF_ICONONLY.0 | SIIGBF_BIGGERSIZEOK.0),
        )
        .ok()?;
    let rgba = dib_to_rgba(bitmap);
    let _ = DeleteObject(bitmap.into());
    let (width, height, mut buffer) = rgba?;
    if alpha_is_empty(&buffer) {
        for px in buffer.chunks_exact_mut(4) {
            px[3] = 255;
        }
    }
    Some((width, height, buffer))
}

/// 回退方案：SHGetFileInfoW 取 32px 图标
unsafe fn rgba_from_shell_info(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let path_w = wide(path);
    let mut info = SHFILEINFOW::default();
    let ok = SHGetFileInfoW(
        PCWSTR(path_w.as_ptr()),
        FILE_ATTRIBUTE_NORMAL,
        Some(&mut info),
        size_of::<SHFILEINFOW>() as u32,
        windows::Win32::UI::Shell::SHGFI_FLAGS(SHGFI_ICON.0 | SHGFI_LARGEICON.0),
    );
    if ok == 0 || info.hIcon.is_invalid() {
        return None;
    }
    icon_to_rgba(info.hIcon)
}

/// 提取图标并编码为 PNG
pub fn extract_png(path: &str) -> Option<Vec<u8>> {
    if path.is_empty() {
        return None;
    }
    let rgba = unsafe { rgba_from_shell(path) }
        .or_else(|| unsafe { rgba_from_shell_info(path) })?;
    let (width, height, buffer) = rgba;
    let image = image::RgbaImage::from_raw(width, height, buffer)?;
    let scaled = if width > ICON_TARGET || height > ICON_TARGET {
        image::imageops::resize(
            &image,
            ICON_TARGET,
            ICON_TARGET,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image
    };
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(scaled)
        .write_to(&mut out, image::ImageFormat::Png)
        .ok()?;
    Some(out.into_inner())
}

fn decode_manifest(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return String::from_utf8_lossy(&bytes[3..]).to_string();
    }
    let head = &bytes[..bytes.len().min(96)];
    let zeros = head.iter().filter(|b| **b == 0).count();
    if zeros > head.len() / 4 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).to_string()
}

/// 读取 PE 内嵌的应用程序清单文本
pub fn read_manifest(path: &str) -> Option<String> {
    unsafe {
        let path_w = wide(path);
        let module = LoadLibraryExW(PCWSTR(path_w.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE).ok()?;

        let text = (|| -> Option<String> {
            let resource = FindResourceW(
                Some(module),
                PCWSTR(1 as *const u16),
                PCWSTR(RT_MANIFEST_ID as *const u16),
            );
            if resource.is_invalid() {
                return None;
            }
            let size = SizeofResource(Some(module), resource);
            if size == 0 {
                return None;
            }
            let handle = LoadResource(Some(module), resource).ok()?;
            let pointer = LockResource(handle);
            if pointer.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(pointer as *const u8, size as usize);
            Some(decode_manifest(bytes))
        })();

        let _ = FreeLibrary(module);
        text
    }
}

/// 判断程序清单是否声明需要管理员权限。
/// `requireAdministrator` 一定弹 UAC；`highestAvailable` 对标准用户同样会弹窗，
/// 因此两者都视为「需要提权」。
pub fn detect_needs_admin(path: &str) -> bool {
    match read_manifest(path) {
        Some(text) => {
            let lower = text.to_ascii_lowercase();
            lower.contains("requireadministrator") || lower.contains("highestavailable")
        }
        None => false,
    }
}
