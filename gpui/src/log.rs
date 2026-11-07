//! Minimal file logger. GPUI's stderr is invisible when the app is launched
//! from a shortcut, so UI events and panics go to a file the user can send.

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

fn path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let directory = base.join("DragonFruit");
        let _ = std::fs::create_dir_all(&directory);
        directory.join("gpui.log")
    })
    .clone()
}

pub fn log(message: impl AsRef<str>) {
    let line = format!("{}\n", message.as_ref());
    eprintln!("{}", line.trim_end());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path())
    {
        let _ = file.write_all(line.as_bytes());
    }
}

/// Install a panic hook that appends the panic to the log before the default
/// hook prints it.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log(format!("PANIC: {info}"));
        default(info);
    }));
}
