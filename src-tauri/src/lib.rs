// RadialDock - global trigger, radial overlay, action execution.

mod actions;
mod config;
mod keys;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{LazyLock, Mutex, OnceLock, RwLock};
use std::thread;
use std::time::Duration;

use actions::Action;
use config::{Config, Resolved};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};

/// Overlay edge length in logical pixels.
const OVERLAY_LOGICAL_SIZE: f64 = 360.0;

static CURSOR_X: AtomicI32 = AtomicI32::new(0);
static CURSOR_Y: AtomicI32 = AtomicI32::new(0);
static OVERLAY_OPEN: AtomicBool = AtomicBool::new(false);
static OV_X: AtomicI32 = AtomicI32::new(0);
static OV_Y: AtomicI32 = AtomicI32::new(0);
static OV_W: AtomicI32 = AtomicI32::new(0);
static OV_H: AtomicI32 = AtomicI32::new(0);
/// Window that had focus when the ring opened, so shortcuts land there.
static PREV_HWND: AtomicIsize = AtomicIsize::new(0);
/// True while the settings window is waiting to learn a new trigger.
static CAPTURING: AtomicBool = AtomicBool::new(false);

static TRIGGER: LazyLock<RwLock<Resolved>> =
    LazyLock::new(|| RwLock::new(Resolved::Mouse(rdev::Button::Middle)));
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

enum HookEvent {
    Toggle,
    Dismiss,
    CapturedMouse(String),
    CapturedKey(String),
}

/// Appends to orbit.log next to config.json. Best effort: logging must
/// never be the reason an action fails.
fn log(line: &str) {
    eprintln!("[orbit] {line}");
    if let Some(path) = LOG_PATH.get() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn set_trigger(resolved: Resolved) {
    if let Ok(mut guard) = TRIGGER.write() {
        *guard = resolved;
    }
}

fn trigger_matches_button(button: rdev::Button) -> bool {
    TRIGGER
        .read()
        .map(|t| *t == Resolved::Mouse(button))
        .unwrap_or(button == rdev::Button::Middle)
}

fn trigger_matches_key(key: rdev::Key) -> bool {
    TRIGGER
        .read()
        .map(|t| *t == Resolved::Key(key))
        .unwrap_or(false)
}

#[cfg(windows)]
mod win {
    use super::{log, PREV_HWND};
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, SetForegroundWindow,
        SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    fn hwnd(raw: isize) -> HWND {
        HWND(raw as *mut core::ffi::c_void)
    }

    /// WS_EX_NOACTIVATE is what keeps the ring from stealing focus when it is
    /// clicked, so a shortcut still reaches the app the user was working in.
    pub fn make_no_activate(raw: isize) {
        if raw == 0 {
            log("overlay has no window handle; focus protection not applied");
            return;
        }
        unsafe {
            let h = hwnd(raw);
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
            let next = ex | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0;
            SetWindowLongPtrW(h, GWL_EXSTYLE, next as isize);
        }
    }

    pub fn remember_foreground() {
        unsafe {
            PREV_HWND.store(GetForegroundWindow().0 as isize, Ordering::Relaxed);
        }
    }

    /// Put focus back on the window that had it when the ring opened.
    ///
    /// SetForegroundWindow refuses calls from a process that is not already
    /// foreground, so we borrow the current foreground thread's input queue
    /// first - the standard AttachThreadInput dance.
    pub fn restore_foreground() {
        let raw = PREV_HWND.load(Ordering::Relaxed);
        if raw == 0 {
            return;
        }
        unsafe {
            let target = hwnd(raw);
            let current = GetForegroundWindow();
            if current.0 == target.0 {
                return;
            }

            let us = GetCurrentThreadId();
            let theirs = GetWindowThreadProcessId(current, None);

            let attached = theirs != 0 && theirs != us && AttachThreadInput(us, theirs, true).as_bool();
            let _ = SetForegroundWindow(target);
            let _ = SetActiveWindow(target);
            if attached {
                let _ = AttachThreadInput(us, theirs, false);
            }
        }
    }
}

#[cfg(not(windows))]
mod win {
    pub fn make_no_activate(_raw: isize) {}
    pub fn remember_foreground() {}
    pub fn restore_foreground() {}
}

fn point_in_overlay(x: i32, y: i32) -> bool {
    let ox = OV_X.load(Ordering::Relaxed);
    let oy = OV_Y.load(Ordering::Relaxed);
    let ow = OV_W.load(Ordering::Relaxed);
    let oh = OV_H.load(Ordering::Relaxed);
    ow > 0 && x >= ox && x < ox + ow && y >= oy && y < oy + oh
}

/// Low-level global input hook. Runs on its own thread; `rdev::grab` never
/// returns. The callback is on the Windows hook timeout budget, so it only
/// touches atomics, a read lock and a channel - never any UI work.
fn start_input_hook(tx: Sender<HookEvent>) {
    thread::spawn(move || {
        let callback = move |event: rdev::Event| -> Option<rdev::Event> {
            use rdev::{EventType, Key};
            match event.event_type {
                EventType::MouseMove { x, y } => {
                    CURSOR_X.store(x as i32, Ordering::Relaxed);
                    CURSOR_Y.store(y as i32, Ordering::Relaxed);
                    Some(event)
                }

                EventType::ButtonPress(button) => {
                    if CAPTURING.swap(false, Ordering::Relaxed) {
                        let _ = tx.send(HookEvent::CapturedMouse(config::button_name(button)));
                        return None;
                    }
                    if trigger_matches_button(button) {
                        let _ = tx.send(HookEvent::Toggle);
                        return None;
                    }
                    // A click elsewhere dismisses the ring, and still lands on
                    // whatever the user actually clicked.
                    if OVERLAY_OPEN.load(Ordering::Relaxed)
                        && !point_in_overlay(
                            CURSOR_X.load(Ordering::Relaxed),
                            CURSOR_Y.load(Ordering::Relaxed),
                        )
                    {
                        let _ = tx.send(HookEvent::Dismiss);
                    }
                    Some(event)
                }

                EventType::ButtonRelease(button) => {
                    if trigger_matches_button(button) {
                        None
                    } else {
                        Some(event)
                    }
                }

                EventType::KeyPress(key) => {
                    if CAPTURING.swap(false, Ordering::Relaxed) {
                        let name = if key == Key::Escape {
                            String::new() // cancelled
                        } else {
                            config::key_name(key)
                        };
                        let _ = tx.send(HookEvent::CapturedKey(name));
                        return None;
                    }
                    if trigger_matches_key(key) {
                        let _ = tx.send(HookEvent::Toggle);
                        return None;
                    }
                    // Esc closes the ring without it ever taking focus.
                    if key == Key::Escape && OVERLAY_OPEN.load(Ordering::Relaxed) {
                        let _ = tx.send(HookEvent::Dismiss);
                        return None;
                    }
                    Some(event)
                }

                EventType::KeyRelease(key) => {
                    if trigger_matches_key(key) {
                        None
                    } else {
                        Some(event)
                    }
                }

                _ => Some(event),
            }
        };

        if let Err(error) = rdev::grab(callback) {
            log(&format!("input hook failed: {error:?}"));
        }
    });
}

/// Physical bounds + scale factor of the monitor the cursor is on.
fn monitor_at(app: &AppHandle, x: i32, y: i32) -> (PhysicalPosition<i32>, PhysicalSize<u32>, f64) {
    if let Ok(monitors) = app.available_monitors() {
        for m in &monitors {
            let p = m.position();
            let s = m.size();
            if x >= p.x && x < p.x + s.width as i32 && y >= p.y && y < p.y + s.height as i32 {
                return (*p, *s, m.scale_factor());
            }
        }
        if let Some(m) = monitors.first() {
            return (*m.position(), *m.size(), m.scale_factor());
        }
    }
    (
        PhysicalPosition::new(0, 0),
        PhysicalSize::new(1920, 1080),
        1.0,
    )
}

fn show(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else {
        return;
    };

    win::remember_foreground();

    let cx = CURSOR_X.load(Ordering::Relaxed);
    let cy = CURSOR_Y.load(Ordering::Relaxed);
    let (mon_pos, mon_size, scale) = monitor_at(app, cx, cy);
    let side = (OVERLAY_LOGICAL_SIZE * scale).round() as i32;

    // Centre on the cursor, then keep the whole ring on-screen.
    let max_x = mon_pos.x + mon_size.width as i32 - side;
    let max_y = mon_pos.y + mon_size.height as i32 - side;
    let x = (cx - side / 2).clamp(mon_pos.x.min(max_x), max_x.max(mon_pos.x));
    let y = (cy - side / 2).clamp(mon_pos.y.min(max_y), max_y.max(mon_pos.y));

    OV_X.store(x, Ordering::Relaxed);
    OV_Y.store(y, Ordering::Relaxed);
    OV_W.store(side, Ordering::Relaxed);
    OV_H.store(side, Ordering::Relaxed);

    let _ = win.set_size(PhysicalSize::new(side as u32, side as u32));
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let _ = win.set_always_on_top(true);
    let _ = win.show();
    OVERLAY_OPEN.store(true, Ordering::Relaxed);
    let _ = win.emit("overlay-open", ());
}

fn hide(app: &AppHandle) {
    OVERLAY_OPEN.store(false, Ordering::Relaxed);
    OV_W.store(0, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.emit("overlay-close", ());
        let _ = win.hide();
    }
}

fn toggle(app: &AppHandle) {
    if OVERLAY_OPEN.load(Ordering::Relaxed) {
        hide(app);
    } else {
        show(app);
    }
}

fn open_settings(app: &AppHandle) {
    hide(app);
    match app.get_webview_window("settings") {
        Some(win) => {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
        None => log("settings window is missing from the app config"),
    }
}

/// A process-wide config rather than Tauri managed state: the windows are
/// created from the config file before `setup` runs, so their first
/// `get_config` would otherwise race the `manage` call and come back empty.
static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::default()));

fn current_config() -> Config {
    CONFIG
        .lock()
        .map(|c| c.clone())
        .unwrap_or_else(|_| Config::default())
}

/// Returns what the registry actually says afterwards, not what we asked for.
fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let outcome = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    if let Err(e) = outcome {
        let message = e.to_string();
        // Removing an entry that was never registered is not a failure.
        let harmless = !enabled && message.contains("cannot find the file");
        if !harmless {
            log(&format!("could not change 'start with Windows': {message}"));
            return Err(message);
        }
    }

    manager.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config() -> Config {
    current_config()
}

/// Applies immediately and reports the verified state back to the UI.
#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let actual = apply_autostart(&app, enabled)?;
    if let Ok(mut guard) = CONFIG.lock() {
        guard.autostart = actual;
        let _ = config::save(&app, &guard);
    }
    log(&format!("'start with Windows' is now {actual}"));
    Ok(actual)
}

#[tauri::command]
fn autostart_enabled(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn commit(app: &AppHandle, cfg: Config) -> Result<Config, String> {
    let cfg = cfg.normalised();
    config::save(app, &cfg)?;
    set_trigger(config::resolve(&cfg.trigger));

    if let Ok(mut guard) = CONFIG.lock() {
        *guard = cfg.clone();
    }
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.emit("config-changed", ());
    }
    Ok(cfg)
}

#[tauri::command]
fn save_config(app: AppHandle, config: Config) -> Result<Config, String> {
    let saved = commit(&app, config)?;
    log(&format!("settings saved ({} slots)", saved.slots.len()));
    Ok(saved)
}

#[tauri::command]
fn reset_config(app: AppHandle) -> Result<Config, String> {
    let cfg = commit(&app, Config::default())?;
    log("settings reset to defaults");
    Ok(cfg)
}

#[tauri::command]
fn hide_overlay(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
fn close_settings(app: AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.hide();
    }
}

/// The next key or mouse press anywhere becomes the new trigger. Esc cancels.
#[tauri::command]
fn start_trigger_capture() {
    CAPTURING.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn cancel_trigger_capture() {
    CAPTURING.store(false, Ordering::Relaxed);
}

/// Async so the PowerShell call behind it never blocks the UI thread.
#[tauri::command]
async fn list_apps() -> Vec<actions::InstalledApp> {
    actions::installed_apps()
}

/// Downloads and installs a newer release if one is published, then restarts.
/// Returns the version it installed, or None when already up to date.
async fn try_update(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };

    let version = update.version.clone();
    log(&format!("update {version} available; downloading"));
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    log(&format!("update {version} installed"));
    Ok(Some(version))
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    let installed = try_update(app.clone()).await?;
    if installed.is_some() {
        app.restart();
    }
    Ok(installed)
}

#[tauri::command]
fn config_location(app: AppHandle) -> String {
    config::config_path(&app)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unavailable".into())
}

/// Close the ring first, then act - so a synthesised shortcut lands in the
/// app the user was actually working in.
#[tauri::command]
fn run_slot(app: AppHandle, index: usize) {
    let slot = current_config().slots.get(index).cloned();
    let Some(slot) = slot else {
        log(&format!("slot {index} does not exist"));
        return;
    };

    hide(&app);

    match slot.action {
        Action::Settings => open_settings(&app),
        Action::None => {}
        action => {
            let label = slot.label;
            thread::spawn(move || {
                // Let the overlay finish hiding, hand focus back, then act.
                thread::sleep(Duration::from_millis(80));
                win::restore_foreground();
                thread::sleep(Duration::from_millis(40));
                if let Err(e) = actions::execute(&action) {
                    log(&format!("slot \"{label}\" failed: {e}"));
                }
            });
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. A second launch (desktop shortcut, or
        // autostart racing a manual start) would mean two input hooks
        // fighting over the trigger, so it surfaces settings and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log("already running; focusing settings instead of starting again");
            open_settings(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            reset_config,
            set_autostart,
            autostart_enabled,
            run_slot,
            hide_overlay,
            close_settings,
            start_trigger_capture,
            cancel_trigger_capture,
            list_apps,
            check_for_updates,
            config_location
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            if let Some(dir) = config::config_path(&handle).and_then(|p| p.parent().map(PathBuf::from))
            {
                let _ = LOG_PATH.set(dir.join("orbit.log"));
            }

            let cfg = config::load(&handle);
            set_trigger(config::resolve(&cfg.trigger));
            if let Ok(mut guard) = CONFIG.lock() {
                *guard = cfg;
            }

            if let Some(overlay) = handle.get_webview_window("overlay") {
                #[cfg(windows)]
                {
                    let raw = overlay.hwnd().map(|h| h.0 as isize).unwrap_or(0);
                    win::make_no_activate(raw);
                }
                let _ = overlay.set_always_on_top(true);
                // The webview may already have asked for the config before it
                // was loaded above, so tell it to read again.
                let _ = overlay.emit("config-changed", ());
            }

            // Closing the settings window should park it, not destroy it.
            if let Some(settings) = handle.get_webview_window("settings") {
                let parked = settings.clone();
                settings.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = parked.hide();
                    }
                });
            }

            let settings_i =
                MenuItem::with_id(app, "settings", "Settings\u{2026}", true, None::<&str>)?;
            let update_i =
                MenuItem::with_id(app, "update", "Check for updates", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Orbit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &update_i, &quit_i])?;

            let mut tray = TrayIconBuilder::new().menu(&menu).tooltip("Orbit");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "settings" => open_settings(app),
                "update" => {
                    let a = app.clone();
                    tauri::async_runtime::spawn(async move {
                        match try_update(a.clone()).await {
                            Ok(Some(v)) => {
                                log(&format!("restarting into {v}"));
                                a.restart();
                            }
                            Ok(None) => log("already on the latest version"),
                            Err(e) => log(&format!("update check failed: {e}")),
                        }
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;

            let (tx, rx) = channel::<HookEvent>();
            start_input_hook(tx);

            thread::spawn(move || {
                while let Ok(ev) = rx.recv() {
                    match ev {
                        HookEvent::CapturedMouse(name) => {
                            let _ = handle.emit_to("settings", "trigger-captured", ("mouse", name));
                        }
                        HookEvent::CapturedKey(name) => {
                            let _ = handle.emit_to("settings", "trigger-captured", ("key", name));
                        }
                        other => {
                            let h = handle.clone();
                            let _ = handle.run_on_main_thread(move || match other {
                                HookEvent::Toggle => toggle(&h),
                                _ => hide(&h),
                            });
                        }
                    }
                }
            });

            // Quiet check a few seconds after launch, so everyone running
            // Orbit picks up new releases without being told to.
            let updater_handle = app.handle().clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(6));
                tauri::async_runtime::block_on(async move {
                match try_update(updater_handle.clone()).await {
                    Ok(Some(v)) => {
                        log(&format!("updated to {v} on launch; restarting"));
                        updater_handle.restart();
                    }
                    Ok(None) => {}
                    Err(e) => log(&format!("update check failed: {e}")),
                }
                });
            });

            log("started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
