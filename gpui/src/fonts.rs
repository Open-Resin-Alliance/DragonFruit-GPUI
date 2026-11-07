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
    let user_fonts = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Microsoft").join("Windows").join("Fonts"));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ttf") {
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
            if !destination.exists() {
                let _ = std::fs::copy(&path, &destination);
            }
            register_font(&destination);
        }
    }
}

#[cfg(windows)]
fn register_font(path: &Path) {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows NT\CurrentVersion\Fonts",
        winreg::enums::KEY_SET_VALUE,
    ) else {
        return;
    };
    // The value name is the display name DirectWrite reports; the file name is
    // what the font loader opens.
    let _ = key.set_value(&format!("Geist (TrueType)"), &name);
}

#[cfg(not(windows))]
pub fn ensure_geist() {}
