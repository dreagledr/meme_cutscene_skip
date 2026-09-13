//! Mod log in `%LOCALAPPDATA%\meme_cutscene_skip\meme_cutscene_skip.log` (the
//! launcher reads the same file with `--follow`).

use std::io::Write;
use std::sync::Mutex;

use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::SystemInformation::GetLocalTime;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

fn timestamp() -> String {
    let SYSTEMTIME {
        wHour,
        wMinute,
        wSecond,
        wMilliseconds,
        ..
    } = unsafe { GetLocalTime() };
    format!("{wHour:02}:{wMinute:02}:{wSecond:02}.{wMilliseconds:03}")
}

/// Appends a timestamped line. I/O errors are silently ignored: the log is not a
/// critical path, and panicking inside the hook detour is not acceptable.
pub(crate) fn log_line(line: &str) {
    let Ok(_guard) = LOG_MUTEX.lock() else {
        return;
    };
    let Some(path) = crate::log_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let _ = writeln!(f, "[{}] {}", timestamp(), line);
}
