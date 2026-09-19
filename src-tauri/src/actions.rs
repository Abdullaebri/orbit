//! What a slot does when you click it.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Start a program, or open a file or folder.
    Launch {
        path: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// Open a URL or shell protocol (https:, ms-settings:, mailto: ...).
    Url { url: String },
    /// Send a keyboard shortcut to whatever had focus.
    Keys { combo: String },
    /// Run a shell command line, with no console window.
    Command { command: String },
    /// Open Orbit's own settings.
    Settings,
    /// An unconfigured slot.
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub label: String,
    /// Key into the icon set in the frontend.
    pub icon: String,
    pub action: Action,
}

impl Slot {
    fn new(label: &str, icon: &str, action: Action) -> Self {
        Self {
            label: label.to_string(),
            icon: icon.to_string(),
            action,
        }
    }
}

fn launch(path: &str, args: &[&str]) -> Action {
    Action::Launch {
        path: path.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    }
}

fn keys(combo: &str) -> Action {
    Action::Keys {
        combo: combo.to_string(),
    }
}

fn url(u: &str) -> Action {
    Action::Url { url: u.to_string() }
}

/// Slot 0 sits at the top of the ring; the rest run clockwise.
pub fn default_slots() -> Vec<Slot> {
    vec![
        Slot::new("Play / Pause", "play_pause", keys("media_play_pause")),
        Slot::new("File Explorer", "folder", launch("explorer.exe", &[])),
        Slot::new("New Note", "note", launch("notepad.exe", &[])),
        Slot::new("Emoji Picker", "emoji", keys("win+.")),
        Slot::new("Screenshot", "screenshot", keys("win+shift+s")),
        Slot::new(
            "Lock Workstation",
            "lock",
            launch("rundll32.exe", &["user32.dll,LockWorkStation"]),
        ),
        Slot::new("Volume Mixer", "volume", launch("SndVol.exe", &[])),
        Slot::new("Task Manager", "taskmgr", launch("taskmgr.exe", &[])),
        Slot::new("Ask AI", "ask_ai", url("https://chatgpt.com/")),
    ]
}

fn quote(arg: &str) -> String {
    if arg.contains(' ') && !arg.starts_with('"') {
        format!("\"{arg}\"")
    } else {
        arg.to_string()
    }
}

/// Launch the way Explorer and the Run box do, rather than the way a plain
/// process spawn does.
///
/// `std::process::Command` only searches `%PATH%`, so names registered under
/// the App Paths registry key - `Winword.exe`, `OneNote.exe`, `chrome.exe` -
/// fail even though they work in the Run box. ShellExecute consults App
/// Paths, follows .lnk shortcuts (picking up their own arguments), and opens
/// documents and folders with their default handler.
#[cfg(windows)]
fn shell_open(path: &str, args: &[String]) -> Result<(), String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // Packaged (Store/MSIX) apps have no launchable .exe path at all; they
    // are addressed by AUMID through the shell's app folder.
    if path.starts_with("shell:") {
        let mut cmd = Command::new("explorer.exe");
        cmd.arg(path);
        cmd.creation_flags(CREATE_NO_WINDOW);
        return cmd
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("could not open {path}: {e}"));
    }

    let file = HSTRING::from(path);
    let operation = HSTRING::from("open");
    let joined: String = args
        .iter()
        .map(|a| quote(a))
        .collect::<Vec<_>>()
        .join(" ");
    let params = HSTRING::from(joined.as_str());

    let code = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(params.as_ptr())
            },
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // ShellExecute returns a value above 32 on success.
    if code.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!(
            "Windows could not open \"{path}\" (ShellExecute code {})",
            code.0 as isize
        ))
    }
}

#[cfg(not(windows))]
fn shell_open(path: &str, _args: &[String]) -> Result<(), String> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// Runs off the UI thread. A misconfigured slot must be a logged no-op, never
/// a crash.
pub fn execute(action: &Action) -> Result<(), String> {
    match action {
        Action::Launch { path, args } => shell_open(path, args),
        Action::Url { url } => tauri_plugin_opener::open_url(url, None::<&str>)
            .map_err(|e| format!("could not open {url}: {e}")),
        Action::Keys { combo } => crate::keys::send_combo(combo),
        Action::Command { command } => {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", command]);
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd.spawn()
                .map(|_| ())
                .map_err(|e| format!("could not run `{command}`: {e}"))
        }
        // Handled on the main thread by the caller.
        Action::Settings | Action::None => Ok(()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub name: String,
    /// Ready to drop straight into a Launch action's `path`.
    pub target: String,
}

#[derive(Deserialize)]
struct StartApp {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
}

/// Everything the Start menu can launch, including packaged Store apps that
/// have no .exe path. Blocking - call it off the UI thread.
#[cfg(windows)]
pub fn installed_apps() -> Vec<InstalledApp> {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-StartApps | Select-Object Name,AppID | ConvertTo-Json -Compress",
    ]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let Ok(output) = cmd.output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Ok(entries) = serde_json::from_str::<Vec<StartApp>>(&text) else {
        return Vec::new();
    };

    let mut apps: Vec<InstalledApp> = entries
        .into_iter()
        .filter(|e| !e.name.trim().is_empty() && !e.app_id.trim().is_empty())
        .map(|e| InstalledApp {
            name: e.name,
            // Every Start-menu entry, packaged or not, launches by AUMID.
            target: format!("shell:AppsFolder\\{}", e.app_id),
        })
        .collect();

    apps.sort_by_key(|a| a.name.to_lowercase());
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

#[cfg(not(windows))]
pub fn installed_apps() -> Vec<InstalledApp> {
    Vec::new()
}
