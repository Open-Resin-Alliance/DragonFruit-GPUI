//! Geist installation.
//!
//! GPUI's Windows text stack enumerates the system font collection; a font
//! registered with `AddFontResourceExW(FR_PRIVATE)` is not guaranteed to appear
//! there. The TTFs converted from the app's `next/font` files are therefore
//! installed for the current user (copy into the per-user Fonts directory and
//! register under HKCU) and also added to the session, so DirectWrite resolves
//! `font_family("Geist")` instead of falling back to Segoe UI.

use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn ensure_geist() {
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn AddFontResourceExW(name: *const u16, fl: u32, res: *mut core::ffi::c_void) -> i32;
    }

    let source = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts"));
    let Ok(entries) = std::fs::read_dir(source) else {
        return;
    };
    // Remove the subset faces an earlier build installed.
    for stale in [
        "geist-latin",
        "geist-latin-ext",
        "geist-cyrillic",
        "geist-cyrillic-ext",
        "geist-vietnamese",
    ] {
        if let Some(directory) = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Microsoft").join("Windows").join("Fonts"))
        {
            let _ = std::fs::remove_file(directory.join(format!("{stale}.ttf")));
        }
        unregister_font(stale);
    }
    let user_fonts = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Microsoft").join("Windows").join("Fonts"));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ttf") {
            continue;
        }
        // Only the single full-coverage face may be installed. The `next/font`
        // woff2 files are unicode-range subsets that all declare the same
        // family/style; registering them together makes DirectWrite pick one
        // subset for all text, which renders latin as missing glyphs.
        let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or("");
        if !stem.starts_with("Geist") {
            continue;
        }
        // Session registration first: this is what makes the family visible to
        // the DirectWrite factory GPUI creates later in this process.
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            AddFontResourceExW(wide.as_ptr(), 0, std::ptr::null_mut());
        }
        // Persist for the user so the next launch starts with the family present.
        if let Some(directory) = &user_fonts {
            let _ = std::fs::create_dir_all(directory);
            let file = match path.file_name() {
                Some(name) => name,
                None => continue,
            };
            let destination = directory.join(file);
            // Always overwrite: a previous build may have installed a subset
            // face under the same name, and the family must be the full font.
            let _ = std::fs::copy(&path, &destination);
            register_font(&destination);
        }
    }
    unsafe extern "system" {
        fn SendMessageTimeoutW(
            hwnd: isize,
            msg: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }
    // `HWND_BROADCAST` + `WM_FONTCHANGE`: tell the font cache the set changed.
    unsafe {
        SendMessageTimeoutW(0xffff, 0x001d, 0, 0, 0x0002, 1000, std::ptr::null_mut());
    }
}

#[cfg(windows)]
fn unregister_font(stem: &str) {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows NT\CurrentVersion\Fonts",
        winreg::enums::KEY_SET_VALUE,
    ) else {
        return;
    };
    let _ = key.delete_value(format!("{stem} (TrueType)"));
}

#[cfg(windows)]
fn register_font(path: &Path) {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
        return;
    };
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows NT\CurrentVersion\Fonts",
        winreg::enums::KEY_SET_VALUE,
    ) else {
        return;
    };
    // Per-user fonts store the **full path** as the value data (system fonts
    // store the bare file name); the value name is only the Fonts UI label.
    let _ = key.set_value(
        &format!("{stem} (TrueType)"),
        &path.to_string_lossy().to_string(),
    );
}

#[cfg(not(windows))]
pub fn ensure_geist() {}
