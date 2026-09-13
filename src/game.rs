//! Game addresses and calls needed by the cutscene skip.
//!
//! The source of truth is `src/game/cutscene_skip.rs` and `src/game/phase.rs` in
//! the drmod-rs project; this is a minimal port (keep it in sync with changes
//! made there). All addresses are RVAs from the game module base.

use std::ffi::CString;

/// `Trigger::staFlags`: word0 holds codes 0..31, the next dword holds codes 32..63.
pub(crate) const STA_FLAGS: usize = 0x17EA060;
/// `GameMenuStatus` (1 = InGame, 6 = CutscenePause, 12 = Pause1).
pub(crate) const MENU_STATUS: usize = 0x17E9F9C;
/// Cutscene-menu lifecycle step (0..6), alive while the status is 6.
pub(crate) const MENU_STEP: usize = 0x17EA118;
/// Pointer to the live `cEventPauseMenu` object.
pub(crate) const MENU_OBJ: usize = 0x17EA140;
/// Current subphase: its hash (the state object at `+0x38`).
pub(crate) const SUB_HASH: usize = 0x14B9178;
/// `cSlowRateManager::updateFrameTime` (thiscall) — the per-frame point, called
/// once per main-loop iteration `0xA52510`.
pub(crate) const FRAME_TIME_UPDATE: usize = 0xA03970;

/// `request_subphase(this, name, arg)` (thiscall, `ret 8`) — requests a subphase
/// change through the engine's regular path; ignored during an event
/// (`STA_EVENT`).
const REQUEST_SUBPHASE_RVA: usize = 0x94E1F0;
/// Game state object (`this` for the subphase request).
const STATE_OBJECT_RVA: usize = 0x14B9140;
/// `Trigger::staFlags`: the `STA_EVENT` flag (code 2) blocks the request.
const STA_EVENT_MASK: u32 = 0x4000_0000;

/// Requests a subphase change with the engine's own function. `clear_event`
/// clears `STA_EVENT` first (otherwise the request is ignored during an event).
///
/// Must be called from the game thread only: the engine is not thread-safe.
pub(crate) fn order_subphase(
    base_addr: usize,
    name: &str,
    arg: u32,
    clear_event: bool,
) -> Option<()> {
    let cname = CString::new(name).ok()?;
    unsafe {
        if clear_event {
            let flags = (base_addr + STA_FLAGS) as *mut u32;
            *flags &= !STA_EVENT_MASK;
        }
        let request: extern "thiscall" fn(*mut u8, *const i8, u32) =
            std::mem::transmute(base_addr + REQUEST_SUBPHASE_RVA);
        request(
            (base_addr + STATE_OBJECT_RVA) as *mut u8,
            cname.as_ptr(),
            arg,
        );
    }
    Some(())
}

pub(crate) fn read_u32(addr: usize) -> u32 {
    unsafe { *(addr as *const u32) }
}

pub(crate) fn read_i32(addr: usize) -> i32 {
    unsafe { *(addr as *const i32) }
}

pub(crate) fn read_usize(addr: usize) -> usize {
    unsafe { *(addr as *const usize) }
}

pub(crate) fn write_u32(addr: usize, value: u32) {
    unsafe { *(addr as *mut u32) = value };
}

/// Checks that the address refers to committed, readable memory — a guard against
/// a dangling pause-menu-object pointer (port of `game::is_readable_ptr`).
pub(crate) fn is_readable_ptr(addr: usize) -> bool {
    use windows::Win32::System::Memory::{
        MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_GUARD,
        PAGE_PROTECTION_FLAGS, PAGE_READONLY, PAGE_READWRITE, VirtualQuery,
    };

    const PAGE_READABLE: PAGE_PROTECTION_FLAGS = PAGE_PROTECTION_FLAGS(
        PAGE_READONLY.0 | PAGE_READWRITE.0 | PAGE_EXECUTE_READ.0 | PAGE_EXECUTE_READWRITE.0,
    );

    let mut mbi = MEMORY_BASIC_INFORMATION::default();
    let ok = unsafe {
        VirtualQuery(
            Some(addr as *const core::ffi::c_void),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    // PAGE_GUARD is a combined flag: reading a guard page raises
    // STATUS_GUARD_PAGE_VIOLATION even though the "readable" bit may be set.
    ok != 0 && (mbi.Protect & PAGE_READABLE).0 != 0 && (mbi.Protect & PAGE_GUARD).0 == 0
}
