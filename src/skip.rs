//! Frame state machine for the "console-style" in-engine cutscene skip.
//!
//! Port of `src/game/cutscene_skip.rs` from the drmod-rs project (that is the
//! source of truth — keep in sync with changes made there; differences: no
//! `enabled` UI checkbox and no `/state`, a log file instead).
//!
//! While a scene from [`WATCH`] is running we hold two flags in
//! `Trigger::staFlags` — `STA_SOFT_EVENT` (code 4) and `STA_SOFT_EVENT_SKIP_OK`
//! (code 37): thanks to them the player's Esc opens the **console cutscene menu**
//! (`GameMenuStatus` = 6, `cEventPauseMenu`) with the CONTINUE and SKIP items.
//! The native Skip item is inert on PC, so we make the decision ourselves: we
//! close the menu through the engine's regular path, clear `STA_PAUSE` and (for
//! SKIP) order [`NEXT`].

use crate::game::{self, MENU_OBJ, MENU_STATUS, MENU_STEP, STA_FLAGS, SUB_HASH};
use crate::log::log_line;

/// Fields of the `cEventPauseMenu` object (`+0x00` — vtable, `+0x04` — state).
const OBJ_STATE: usize = 0x04;
/// Index of the confirmed item (`-1` — not confirmed yet).
const OBJ_CONFIRMED: usize = 0x3C;
/// Menu-machine state meaning the player confirmed an item.
const OBJ_STATE_DECIDED: i32 = -2;

/// The "SKIP" item in the menu (0 — CONTINUE).
const ITEM_SKIP: i32 = 1;

const SOFT_EVENT: u32 = 0x0800_0000; // code 4 → a pause brings up the cutscene menu
const SKIP_OK: u32 = 0x0400_0000; // code 37 (word1) → the menu gets a Skip item
const STA_PAUSE: u32 = 0x0000_1000; // code 19 → the game is paused (stalls loading)

const MENU_CUTSCENE: i32 = 6;
const MENU_PAUSE1: i32 = 12; // intermediate menu-closing status
const MENU_STEP_CLOSE: u32 = 6;

/// How many frames to wait for the engine to destroy the menu itself (step 6)
/// before clearing the pause forcibly (~5 s at 60 FPS), and the post-skip
/// quarantine so we do not catch the next scene's menu.
const CLOSE_TIMEOUT_FRAMES: u32 = 300;
const COOLDOWN_FRAMES: u32 = 120;

/// The scene where we enable the console menu, and where the skip leads.
const WATCH: [(&str, u32); 2] = [
    ("P370_RESTART", sub_hash("P370_RESTART")),
    ("P370_IN", sub_hash("P370_IN")),
];
const NEXT: &str = "P370_EVENT";

/// Hashes a subphase name the way the engine does (`RVA 0xA03EA0`): CRC32 of the
/// lowercased name with the top bit dropped.
pub(crate) const fn sub_hash(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut crc: u32 = 0xFFFF_FFFF;
    let mut i = 0;
    while i < bytes.len() {
        crc ^= bytes[i].to_ascii_lowercase() as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }
        i += 1;
    }
    !crc & 0x7FFF_FFFF
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Off,
    /// Flags are set, waiting for the player to confirm an item in the console menu.
    Armed,
    /// The engine is closing the menu — waiting for the object to be destroyed.
    Closing {
        item: i32,
        frames: u32,
    },
    /// The skip has been ordered — quarantine, so the next scene's menu is not caught.
    Cooldown {
        frames: u32,
    },
}

/// The cutscene-skip frame state machine (see the module docs).
pub(crate) struct CutsceneSkip {
    stage: Stage,
    /// Whether our flags are currently held in `staFlags`.
    flags_held: bool,
}

impl CutsceneSkip {
    pub(crate) const fn new() -> Self {
        Self {
            stage: Stage::Off,
            flags_held: false,
        }
    }

    /// Called once per frame from the `updateFrameTime` detour (game thread: the
    /// engine is not thread-safe). There is no `enabled` switch in this mod — the
    /// skip is always on.
    pub(crate) fn update(&mut self, base_addr: usize, menu_status: i32) {
        if base_addr == 0 {
            return;
        }
        match self.stage {
            Stage::Off | Stage::Armed => self.idle(base_addr, menu_status),
            Stage::Closing { item, frames } => self.closing(base_addr, item, frames),
            Stage::Cooldown { frames } => {
                if frames > 0 {
                    self.stage = Stage::Cooldown { frames: frames - 1 };
                } else {
                    self.stage = Stage::Off;
                }
            }
        }
    }

    /// Idle phase: hold the flags while the scene runs and catch the menu decision.
    fn idle(&mut self, base_addr: usize, menu_status: i32) {
        if !Self::in_scene(base_addr) {
            if self.flags_held {
                Self::set_flags(base_addr, false);
                self.flags_held = false;
                log_line(&format!(
                    "meme_cutscene_skip: scene ended (subphase 0x{:08X}) — flags cleared",
                    Self::sub_hash_raw(base_addr)
                ));
            }
            self.stage = Stage::Off;
            return;
        }
        // The game itself clears SKIP_OK when a pause is pressed without
        // SOFT_EVENT, so we do not set the flags once but keep them up.
        if !Self::flags_ready(base_addr) {
            Self::set_flags(base_addr, true);
            if !self.flags_held {
                log_line(&format!(
                    "meme_cutscene_skip: subphase {} — console menu enabled \
                     (SOFT_EVENT + SKIP_OK), Esc opens PAUSE/SKIP",
                    Self::sub_name(base_addr)
                ));
            }
            self.flags_held = true;
        }
        self.stage = Stage::Armed;
        if menu_status != MENU_CUTSCENE {
            return;
        }
        let Some(item) = Self::decision(base_addr) else {
            return;
        };
        log_line(&format!(
            "meme_cutscene_skip: player confirmed item {} — closing the menu through the engine",
            if item == ITEM_SKIP {
                "SKIP"
            } else {
                "CONTINUE"
            }
        ));
        // Status 6 + step 6: the engine destroys the menu object itself and walks
        // the status 6 → 12 → 1 (its step 5 is unpassable — see the module docs).
        game::write_u32(base_addr + MENU_STATUS, MENU_CUTSCENE as u32);
        game::write_u32(base_addr + MENU_STEP, MENU_STEP_CLOSE);
        self.stage = Stage::Closing { item, frames: 1 };
    }

    /// Waits for the engine to destroy the menu object, then clears the pause and
    /// (for SKIP) orders the next subphase.
    fn closing(&mut self, base_addr: usize, item: i32, frames: u32) {
        let closed = Self::menu_object(base_addr) == 0
            && !matches!(
                Self::menu_status_raw(base_addr),
                MENU_CUTSCENE | MENU_PAUSE1
            );
        if !closed && frames <= CLOSE_TIMEOUT_FRAMES {
            self.stage = Stage::Closing {
                item,
                frames: frames + 1,
            };
            return;
        }
        if closed {
            log_line("meme_cutscene_skip: menu closed by the engine");
        } else {
            log_line(&format!(
                "meme_cutscene_skip: menu did not close within {CLOSE_TIMEOUT_FRAMES} frames \
                 — clearing the pause as is"
            ));
        }
        self.flags_held = false;
        Self::set_flags(base_addr, false);
        Self::clear_pause(base_addr);
        if item == ITEM_SKIP {
            let ok = game::order_subphase(base_addr, NEXT, 1, true).is_some();
            log_line(&format!(
                "meme_cutscene_skip: ordered subphase {NEXT} → {}",
                if ok { "invoked" } else { "failed" }
            ));
        } else {
            log_line("meme_cutscene_skip: CONTINUE — the scene resumes, no order needed");
        }
        self.stage = Stage::Cooldown {
            frames: COOLDOWN_FRAMES,
        };
    }

    /// Index of the confirmed item, if the menu machine has already decided.
    /// The input handler (`RVA 0x5A5930`) writes the `-2` state and the index
    /// together, so `+0x3C >= 0` alone is not enough — we wait for the state too.
    fn decision(base_addr: usize) -> Option<i32> {
        let obj = Self::menu_object(base_addr);
        if obj == 0 {
            return None;
        }
        let state = game::read_i32(obj + OBJ_STATE);
        let confirmed = game::read_i32(obj + OBJ_CONFIRMED);
        (state == OBJ_STATE_DECIDED && confirmed >= 0).then_some(confirmed)
    }

    /// Pointer to the live menu object (0 if there is none or it is unreadable).
    fn menu_object(base_addr: usize) -> usize {
        let ptr = game::read_usize(base_addr + MENU_OBJ);
        if ptr != 0 && game::is_readable_ptr(ptr) {
            ptr
        } else {
            0
        }
    }

    fn menu_status_raw(base_addr: usize) -> i32 {
        game::read_i32(base_addr + MENU_STATUS)
    }

    fn sub_hash_raw(base_addr: usize) -> u32 {
        game::read_u32(base_addr + SUB_HASH)
    }

    fn in_scene(base_addr: usize) -> bool {
        let hash = Self::sub_hash_raw(base_addr);
        WATCH.iter().any(|(_, watched)| *watched == hash)
    }

    fn sub_name(base_addr: usize) -> &'static str {
        let hash = Self::sub_hash_raw(base_addr);
        WATCH
            .iter()
            .find(|(_, watched)| *watched == hash)
            .map(|(name, _)| *name)
            .unwrap_or("?")
    }

    fn flags_ready(base_addr: usize) -> bool {
        game::read_u32(base_addr + STA_FLAGS) & SOFT_EVENT != 0
            && game::read_u32(base_addr + STA_FLAGS + 4) & SKIP_OK != 0
    }

    /// Sets/clears our flags without touching the other bits of `staFlags`.
    fn set_flags(base_addr: usize, on: bool) {
        let (w0, w1) = (
            game::read_u32(base_addr + STA_FLAGS),
            game::read_u32(base_addr + STA_FLAGS + 4),
        );
        let (n0, n1) = if on {
            (w0 | SOFT_EVENT, w1 | SKIP_OK)
        } else {
            (w0 & !SOFT_EVENT, w1 & !SKIP_OK)
        };
        if n0 != w0 {
            game::write_u32(base_addr + STA_FLAGS, n0);
        }
        if n1 != w1 {
            game::write_u32(base_addr + STA_FLAGS + 4, n1);
        }
    }

    /// Clears `STA_PAUSE` — otherwise the scene and the loading machine stall.
    fn clear_pause(base_addr: usize) {
        let w0 = game::read_u32(base_addr + STA_FLAGS);
        if w0 & STA_PAUSE != 0 {
            game::write_u32(base_addr + STA_FLAGS, w0 & !STA_PAUSE);
            log_line("meme_cutscene_skip: cleared STA_PAUSE — the scene runs again");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CutsceneSkip, NEXT, WATCH, sub_hash};

    /// Subphase hashes were checked against live reads of the game state object.
    #[test]
    fn sub_hash_matches_game() {
        assert_eq!(sub_hash("P370_RESTART"), 0x3C9A_2F06);
        assert_eq!(sub_hash("P370_IN"), 0x6915_135D);
        assert_eq!(sub_hash("P370_EVENT"), 0x2540_A957);
        assert_eq!(sub_hash("P380_MONSOON"), 0x70AB_682C);
        // case does not matter: the engine hashes the lowercased name
        assert_eq!(sub_hash("p370_in"), sub_hash("P370_IN"));
    }

    #[test]
    fn watch_holds_precomputed_hashes() {
        assert!(WATCH.iter().any(|(name, _)| *name == "P370_IN"));
        assert!(WATCH.iter().any(|(name, _)| *name == "P370_RESTART"));
        assert_eq!(NEXT, "P370_EVENT");
    }

    /// With no base_addr (the mod has not found the game module yet) the machine
    /// does nothing.
    #[test]
    fn idle_on_zero_base() {
        let mut skip = CutsceneSkip::new();
        skip.update(0, 6);
    }
}
