//! Parsing and synthesis of keyboard shortcuts, e.g. "win+shift+s".

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VIRTUAL_KEY,
};

/// Virtual-key code for one token of a shortcut string.
pub fn vk_for(token: &str) -> Option<u16> {
    let t = token.trim().to_ascii_lowercase();
    if t.is_empty() {
        return None;
    }

    // Single letters and digits.
    if t.len() == 1 {
        let c = t.as_bytes()[0];
        if c.is_ascii_lowercase() {
            return Some(0x41 + (c - b'a') as u16);
        }
        if c.is_ascii_digit() {
            return Some(0x30 + (c - b'0') as u16);
        }
    }

    // Function keys.
    if let Some(n) = t.strip_prefix('f') {
        if let Ok(n) = n.parse::<u16>() {
            if (1..=24).contains(&n) {
                return Some(0x6F + n);
            }
        }
    }

    let vk = match t.as_str() {
        "ctrl" | "control" => 0x11,
        "shift" => 0x10,
        "alt" | "menu" => 0x12,
        "win" | "meta" | "super" | "cmd" => 0x5B,
        "esc" | "escape" => 0x1B,
        "tab" => 0x09,
        "enter" | "return" => 0x0D,
        "space" => 0x20,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "insert" | "ins" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" | "pgup" => 0x21,
        "pagedown" | "pgdn" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "printscreen" | "prtsc" => 0x2C,
        "capslock" => 0x14,
        "." | "period" | "dot" => 0xBE,
        "," | "comma" => 0xBC,
        ";" | "semicolon" => 0xBA,
        "'" | "quote" => 0xDE,
        "/" | "slash" => 0xBF,
        "\\" | "backslash" => 0xDC,
        "[" => 0xDB,
        "]" => 0xDD,
        "-" | "minus" => 0xBD,
        "=" | "equal" | "plus" => 0xBB,
        "`" | "backtick" => 0xC0,
        "media_play_pause" | "playpause" => 0xB3,
        "media_next" | "next_track" => 0xB0,
        "media_prev" | "prev_track" => 0xB1,
        "media_stop" => 0xB2,
        "volume_up" => 0xAF,
        "volume_down" => 0xAE,
        "volume_mute" | "mute" => 0xAD,
        _ => return None,
    };
    Some(vk)
}

/// Keys that must carry KEYEVENTF_EXTENDEDKEY to be recognised correctly.
fn is_extended(vk: u16) -> bool {
    matches!(
        vk,
        0x5B | 0x5C // win
            | 0x21..=0x28 // page/end/home/arrows
            | 0x2D | 0x2E // insert/delete
            | 0x2C // print screen
            | 0x90 // num lock
    )
    // Media and volume keys (0xAD-0xB3) are deliberately NOT extended: the
    // shell ignores them when the extended bit is set.
}

#[cfg(windows)]
fn key_event(vk: u16, up: bool) -> INPUT {
    let mut flags: u32 = 0;
    if up {
        flags |= KEYEVENTF_KEYUP.0;
    }
    if is_extended(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY.0;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(flags),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Press every key in the combo in order, then release in reverse order.
#[cfg(windows)]
pub fn send_combo(combo: &str) -> Result<(), String> {
    let tokens: Vec<&str> = combo
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let codes: Vec<u16> = tokens.iter().filter_map(|t| vk_for(t)).collect();

    if codes.is_empty() {
        return Err(format!("no keys recognised in `{combo}`"));
    }
    if codes.len() != tokens.len() {
        return Err(format!("unknown key name in `{combo}`"));
    }

    let mut inputs: Vec<INPUT> = Vec::with_capacity(codes.len() * 2);
    for &vk in &codes {
        inputs.push(key_event(vk, false));
    }
    for &vk in codes.iter().rev() {
        inputs.push(key_event(vk, true));
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        // Usually UIPI: the foreground window belongs to an elevated process
        // and will not accept synthetic input from us.
        return Err(format!(
            "only {sent} of {} key events were accepted (is the focused app running as administrator?)",
            inputs.len()
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn send_combo(_combo: &str) -> Result<(), String> {
    Err("keyboard shortcuts are only implemented on Windows".into())
}
