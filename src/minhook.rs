//! Thin FFI over MinHook (C sources in `vendor/minhook`, BSD-2-Clause). Taken
//! from `vendor/hudhook/src/mh.rs` in drmod-rs — only what a single hook needs,
//! without tracing.
#![allow(dead_code)]

use core::ffi::c_void;
use std::ptr::null_mut;

#[allow(non_camel_case_types)]
#[must_use]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MH_STATUS {
    /// Should never be returned.
    MH_UNKNOWN = -1,
    MH_OK = 0,
    MH_ERROR_ALREADY_INITIALIZED,
    MH_ERROR_NOT_INITIALIZED,
    MH_ERROR_ALREADY_CREATED,
    MH_ERROR_NOT_CREATED,
    MH_ERROR_ENABLED,
    MH_ERROR_DISABLED,
    MH_ERROR_NOT_EXECUTABLE,
    MH_ERROR_UNSUPPORTED_FUNCTION,
    MH_ERROR_MEMORY_ALLOC,
    MH_ERROR_MEMORY_PROTECT,
    MH_ERROR_MODULE_NOT_FOUND,
    MH_ERROR_FUNCTION_NOT_FOUND,
}

unsafe extern "system" {
    fn MH_Initialize() -> MH_STATUS;
    fn MH_CreateHook(
        pTarget: *mut c_void,
        pDetour: *mut c_void,
        ppOriginal: *mut *mut c_void,
    ) -> MH_STATUS;
    fn MH_QueueEnableHook(pTarget: *mut c_void) -> MH_STATUS;
    fn MH_ApplyQueued() -> MH_STATUS;
}

/// A single hook: the target, our detour and the original's trampoline.
pub(crate) struct MhHook {
    addr: *mut c_void,
    trampoline: *mut c_void,
}

impl MhHook {
    /// Creates the hook (queue it for enabling with [`MhHook::queue_enable`]).
    ///
    /// # Safety
    ///
    /// `addr` must be executable game memory and `hook_impl` a function whose
    /// signature matches the target's.
    pub(crate) unsafe fn new(addr: *mut c_void, hook_impl: *mut c_void) -> Result<Self, MH_STATUS> {
        let mut trampoline = null_mut();
        let status = unsafe { MH_CreateHook(addr, hook_impl, &mut trampoline) };
        if status != MH_STATUS::MH_OK {
            return Err(status);
        }
        Ok(Self { addr, trampoline })
    }

    /// Address of the original's trampoline (valid only after a successful
    /// `new`).
    pub(crate) fn trampoline(&self) -> *mut c_void {
        self.trampoline
    }

    /// # Safety
    ///
    /// Queues the hook for enabling; it takes effect in [`MH_ApplyQueued`].
    pub(crate) unsafe fn queue_enable(&self) -> Result<(), MH_STATUS> {
        let status = unsafe { MH_QueueEnableHook(self.addr) };
        if status == MH_STATUS::MH_OK {
            Ok(())
        } else {
            Err(status)
        }
    }
}

/// Initializes MinHook (exactly once per process).
///
/// # Safety
///
/// Call once before creating any hooks.
pub(crate) unsafe fn initialize() -> Result<(), MH_STATUS> {
    match unsafe { MH_Initialize() } {
        MH_STATUS::MH_OK | MH_STATUS::MH_ERROR_ALREADY_INITIALIZED => Ok(()),
        status => Err(status),
    }
}

/// Applies the queued hook enable/disable operations.
///
/// # Safety
///
/// Call after [`MhHook::queue_enable`].
pub(crate) unsafe fn apply_queued() -> Result<(), MH_STATUS> {
    let status = unsafe { MH_ApplyQueued() };
    if status == MH_STATUS::MH_OK {
        Ok(())
    } else {
        Err(status)
    }
}
