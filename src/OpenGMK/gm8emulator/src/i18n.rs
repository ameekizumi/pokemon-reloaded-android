//! Minimal ES/EN switch for the engine's own on-screen text (menus drawn by the emulator).
//! The launcher writes the language index to `data/text/config.ini` (" 0" Spanish, " 1" English).

use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn init(program_directory: &str) {
    let base = std::path::Path::new(program_directory);
    for rel in ["data/text/config.ini", "game/data/text/config.ini"] {
        if let Ok(text) = std::fs::read_to_string(base.join(rel)) {
            ENGLISH.store(text.trim().starts_with('1'), Ordering::Relaxed);
            return
        }
    }
}

pub fn tr(es: &'static str, en: &'static str) -> &'static str {
    if ENGLISH.load(Ordering::Relaxed) { en } else { es }
}
