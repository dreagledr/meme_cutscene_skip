//! Cutscene skip as a standalone mod: a DLL with a MinHook hook on the game's
//! frame-time updater (`cSlowRateManager::updateFrameTime`).
//! The launcher is in `src/main.rs`, the skip logic in `src/skip.rs`.
//!
//! The `updateFrameTime` hook gives exactly one call per game main-loop
//! iteration — enough for the frame state machine (and it is the game thread,
//! where the engine may be called; see `game::order_subphase`).

mod game;
mod log;
mod minhook;
mod skip;

use core::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::PCSTR;

/// Game window title (the launcher looks the window up by it).
pub const DEFAULT_TITLE: &str = "METAL GEAR RISING: REVENGEANCE";
/// Game exe name (both the file name and the process name).
pub const PROCESS_NAME: &str = "METAL GEAR RISING REVENGEANCE.exe";
/// Our DLL name (the launcher embeds the DLL).
pub const LIB_NAME: &str = "meme_cutscene_skip_lib.dll";

const DATA_DIR_NAME: &str = "meme_cutscene_skip";
/// Mod log file name.
const LOG_FILE: &str = "meme_cutscene_skip.log";

/// Mod directory: `%LOCALAPPDATA%\meme_cutscene_skip` (log and extracted DLL).
pub fn data_dir() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|local| PathBuf::from(local).join(DATA_DIR_NAME))
}

/// Path to the mod log file (the launcher reads it with `--follow`).
pub fn log_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join(LOG_FILE))
}

/// Game module base (`GetModuleHandleA(null)`).
static BASE: AtomicUsize = AtomicUsize::new(0);
/// The skip state machine — it lives on the game thread (the detour), hence the
/// mutex.
static STATE: Mutex<skip::CutsceneSkip> = Mutex::new(skip::CutsceneSkip::new());
/// Trampoline of the original `updateFrameTime`.
static ORIG_FRAME_TIME: OnceLock<unsafe extern "thiscall" fn(*mut u8, i32, f32)> = OnceLock::new();

/// Detour of `cSlowRateManager::updateFrameTime` (thiscall, 0xA03970). Calls the
/// original and runs the skip state machine; it logs only on stage transitions,
/// which keeps the per-frame path cheap.
unsafe extern "thiscall" fn frame_time_detour(this: *mut u8, flag: i32, rate: f32) {
    if let Some(&orig) = ORIG_FRAME_TIME.get() {
        unsafe { orig(this, flag, rate) };
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let menu_status = game::read_i32(base + game::MENU_STATUS);
    if let Ok(mut state) = STATE.lock() {
        state.update(base, menu_status);
    }
}

/// Installs the hook. Called from a separate thread: `DllMain` runs under the
/// loader lock, where loading/patching code is not allowed.
fn install() {
    let base = unsafe { GetModuleHandleA(PCSTR::null()) }
        .map(|h| h.0 as usize)
        .unwrap_or(0);
    if base == 0 {
        log::log_line("meme_cutscene_skip: game module base not found");
        return;
    }
    BASE.store(base, Ordering::Relaxed);

    if let Err(e) = unsafe { minhook::initialize() } {
        log::log_line(&format!("meme_cutscene_skip: MH_Initialize: {e:?}"));
        return;
    }

    let target = (base + game::FRAME_TIME_UPDATE) as *mut c_void;
    let hook = match unsafe { minhook::MhHook::new(target, frame_time_detour as *mut c_void) } {
        Ok(hook) => hook,
        Err(e) => {
            log::log_line(&format!(
                "meme_cutscene_skip: MH_CreateHook 0x{:08X}: {e:?}",
                target as usize
            ));
            return;
        }
    };
    let trampoline: unsafe extern "thiscall" fn(*mut u8, i32, f32) =
        unsafe { std::mem::transmute(hook.trampoline()) };
    if ORIG_FRAME_TIME.set(trampoline).is_err() {
        log::log_line("meme_cutscene_skip: trampoline already stored — not reinstalling the hook");
        return;
    }
    if let Err(e) = unsafe { hook.queue_enable() } {
        log::log_line(&format!("meme_cutscene_skip: queue_enable: {e:?}"));
        return;
    }
    if let Err(e) = unsafe { minhook::apply_queued() } {
        log::log_line(&format!("meme_cutscene_skip: MH_ApplyQueued: {e:?}"));
        return;
    }
    // MhHook has no Drop — MinHook keeps the hook itself, so there is no need to
    // store the wrapper.
    log::log_line(&format!(
        "meme_cutscene_skip: hook installed (base=0x{base:08X}, target=0x{:08X})",
        target as usize
    ));
}

/// DLL entry point. The heavy work (MinHook, base lookup) runs on a separate
/// thread.
///
/// # Safety
///
/// Called by the Windows loader with `reason == 1` (`DLL_PROCESS_ATTACH`); it
/// runs under the loader lock, so no code may be loaded here.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hmodule: *mut c_void,
    reason: u32,
    _reserved: *mut c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        std::thread::spawn(install);
    }
    1
}
