//! On-disk settings: the nine slots, the trigger, and autostart.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::actions::{default_slots, Slot};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    /// "left", "right", "middle", "x1", "x2"...
    Mouse { button: String },
    /// An rdev key name, e.g. "F13" or "ControlRight".
    Key { key: String },
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger::Mouse {
            button: "middle".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_slots")]
    pub slots: Vec<Slot>,
    #[serde(default)]
    pub trigger: Trigger,
    #[serde(default)]
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            slots: default_slots(),
            trigger: Trigger::default(),
            autostart: false,
        }
    }
}

/// The ring stays readable between these counts: fewer than two is not a
/// ring, and past twelve the buttons are too small to hit reliably.
pub const MIN_SLOTS: usize = 2;
pub const MAX_SLOTS: usize = 12;

impl Config {
    pub fn normalised(mut self) -> Self {
        if self.slots.len() < MIN_SLOTS {
            let fallback = default_slots();
            while self.slots.len() < MIN_SLOTS {
                let i = self.slots.len();
                self.slots.push(fallback[i].clone());
            }
        }
        self.slots.truncate(MAX_SLOTS);
        self
    }
}

pub fn config_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("[radialdock] could not create config dir: {e}");
        return None;
    }
    Some(dir.join("config.json"))
}

/// A missing or corrupt file falls back to defaults rather than failing to
/// start; the user can always fix it from the settings window.
pub fn load(app: &AppHandle) -> Config {
    let Some(path) = config_path(app) else {
        return Config::default();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match serde_json::from_str::<Config>(&text) {
        Ok(cfg) => cfg.normalised(),
        Err(e) => {
            eprintln!("[radialdock] config.json is not usable ({e}); using defaults");
            Config::default()
        }
    }
}

pub fn save(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app).ok_or("no writable config directory")?;
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Keys offerable as a trigger. The hook compares `rdev::Key` values
/// directly; names round-trip through JSON via each variant's Debug form.
pub const TRIGGER_KEYS: &[rdev::Key] = {
    use rdev::Key::*;
    &[
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, Insert, Delete, Home, End, PageUp,
        PageDown, CapsLock, ScrollLock, Pause, NumLock, PrintScreen, ControlLeft, ControlRight,
        ShiftLeft, ShiftRight, Alt, AltGr, MetaLeft, MetaRight, Tab, Space, BackQuote, Escape,
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Num0, Num1, Num2, Num3,
        Num4, Num5, Num6, Num7, Num8, Num9,
    ]
};

pub fn key_name(key: rdev::Key) -> String {
    format!("{key:?}")
}

pub fn key_from_name(name: &str) -> Option<rdev::Key> {
    TRIGGER_KEYS
        .iter()
        .copied()
        .find(|k| key_name(*k).eq_ignore_ascii_case(name))
}

pub fn button_name(button: rdev::Button) -> String {
    match button {
        rdev::Button::Left => "left".into(),
        rdev::Button::Right => "right".into(),
        rdev::Button::Middle => "middle".into(),
        rdev::Button::Unknown(n) => format!("x{n}"),
    }
}

pub fn button_from_name(name: &str) -> Option<rdev::Button> {
    match name.trim().to_ascii_lowercase().as_str() {
        "left" => Some(rdev::Button::Left),
        "right" => Some(rdev::Button::Right),
        "middle" => Some(rdev::Button::Middle),
        other => other
            .strip_prefix('x')
            .and_then(|n| n.parse::<u8>().ok())
            .map(rdev::Button::Unknown),
    }
}

/// The trigger as the hook wants it: a value it can compare with `==`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolved {
    Mouse(rdev::Button),
    Key(rdev::Key),
}

pub fn resolve(trigger: &Trigger) -> Resolved {
    match trigger {
        Trigger::Mouse { button } => {
            Resolved::Mouse(button_from_name(button).unwrap_or(rdev::Button::Middle))
        }
        Trigger::Key { key } => match key_from_name(key) {
            Some(k) => Resolved::Key(k),
            None => Resolved::Mouse(rdev::Button::Middle),
        },
    }
}
