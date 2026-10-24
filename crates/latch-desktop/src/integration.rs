use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui_kit::{AnyWindowHandle, App, Entity, Global, Window};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, IsWindowVisible, SW_HIDE, SW_RESTORE, SW_SHOW, ShowWindow,
};

pub struct Integration {
    _hotkey: Option<GlobalHotKeyManager>,
    _tray: Option<TrayIcon>,
    tray_available: Arc<AtomicBool>,
    pub status: String,
}
impl Global for Integration {}

fn tray() -> Result<TrayIcon, String> {
    let menu = Menu::new();
    let show = MenuItem::with_id("show", "Open Latch", true, None);
    let lock = MenuItem::with_id("lock", "Lock vault", true, None);
    let quit = MenuItem::with_id("quit", "Quit Latch", true, None);
    menu.append_items(&[&show, &lock, &quit])
        .map_err(|e| e.to_string())?;
    let image = image::load_from_memory(include_bytes!("../icons/icon.png"))
        .map_err(|e| e.to_string())?
        .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    TrayIconBuilder::new()
        .with_tooltip("Latch")
        .with_menu(Box::new(menu))
        .with_icon(Icon::from_rgba(image.into_raw(), 32, 32).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())
}

pub fn start(window: AnyWindowHandle, view: Entity<crate::app::Latch>, cx: &mut App) {
    // Same quick-access chord as the previous app: Ctrl+Space on every platform.
    let hotkey = GlobalHotKeyManager::new().and_then(|manager| {
        manager.register(HotKey::new(Some(Modifiers::CONTROL), Code::Space))?;
        Ok(manager)
    });
    let status = match &hotkey {
        Ok(_) => "Quick access: Ctrl+Space shows or hides Latch.".into(),
        Err(error) => format!(
            "Global shortcut unavailable: {error}. Open Latch from your application launcher."
        ),
    };
    let available = Arc::new(AtomicBool::new(false));
    #[cfg(not(target_os = "linux"))]
    let icon = match tray() {
        Ok(icon) => {
            available.store(true, Ordering::SeqCst);
            Some(icon)
        }
        Err(error) => {
            eprintln!("Tray unavailable: {error}");
            None
        }
    };
    #[cfg(target_os = "linux")]
    let icon = {
        // GPUI owns its event loop. GTK's tray needs its own initialized thread.
        let available = available.clone();
        std::thread::spawn(move || {
            if let Err(error) = gtk::init() {
                eprintln!("Tray unavailable: {error}");
                return;
            }
            let _icon = match tray() {
                Ok(icon) => icon,
                Err(error) => {
                    eprintln!("Tray unavailable: {error}");
                    return;
                }
            };
            available.store(true, Ordering::SeqCst);
            gtk::main();
        });
        None
    };
    cx.set_global(Integration {
        _hotkey: hotkey.ok(),
        _tray: icon,
        tray_available: available,
        status,
    });
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            let mut commands = Vec::new();
            while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                if event.state == HotKeyState::Pressed {
                    commands.push("toggle".to_string());
                }
            }
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                commands.push(event.id.0);
            }
            for command in commands {
                if window
                    .update(cx, |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.integration_command(&command, window, cx)
                        });
                    })
                    .is_err()
                {
                    return;
                }
            }
        }
    })
    .detach();
}

pub fn has_tray(cx: &App) -> bool {
    // ponytail: Linux tray creation does not prove a visible desktop host. Quit
    // on close until StatusNotifier host presence and loss are tracked reliably.
    if cfg!(target_os = "linux") {
        return false;
    }
    cx.try_global::<Integration>()
        .is_some_and(|integration| integration.tray_available.load(Ordering::SeqCst))
}

/// Takes the window off screen. With a tray on Windows it is hidden outright so
/// the taskbar button and Alt+Tab entry go too; otherwise it is minimized.
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub fn dismiss(window: &mut Window, cx: &App) {
    #[cfg(target_os = "windows")]
    if has_tray(cx)
        && let Some(hwnd) = hwnd(window)
    {
        // SAFETY: `hwnd` is the live GPUI window, used on its own thread.
        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        return;
    }
    window.minimize_window();
}

/// Hides a window the platform minimized (title bar button, Win+Down) so it
/// leaves the taskbar the same way `dismiss` does.
#[cfg(target_os = "windows")]
pub fn hide_if_minimized(window: &Window, cx: &App) {
    if has_tray(cx)
        && let Some(hwnd) = hwnd(window)
    {
        // SAFETY: `hwnd` is the live GPUI window, used on its own thread.
        unsafe {
            if IsIconic(hwnd).as_bool() && IsWindowVisible(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }
}

/// Shows and focuses the window, undoing `dismiss` or a minimize.
pub fn present(window: &mut Window, cx: &mut App) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = hwnd(window) {
        show(hwnd);
    }
    window.activate_window();
    cx.activate(true);
}

/// Brings an already running Latch forward when a second launch loses the
/// process lock. A hidden Latch has no taskbar button, so launching it again
/// from Start is how users expect to get it back.
#[cfg(target_os = "windows")]
pub fn present_running_instance() {
    use windows::{
        Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow},
        core::w,
    };
    // GPUI's window class plus the title set in `main`.
    // SAFETY: Win32 lookups on a handle that is only passed back to Win32.
    if let Ok(hwnd) = unsafe { FindWindowW(w!("Zed::Window"), w!("Latch")) } {
        show(hwnd);
        // SAFETY: as above. A process the user just launched may take focus.
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
}

/// Restores a minimized window (hidden or not) and shows a hidden one.
#[cfg(target_os = "windows")]
fn show(hwnd: windows::Win32::Foundation::HWND) {
    // SAFETY: Win32 visibility calls; a stale handle only makes them fail.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else if !IsWindowVisible(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }
}

#[cfg(target_os = "windows")]
fn hwnd(window: &Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(windows::Win32::Foundation::HWND(
            handle.hwnd.get() as *mut std::ffi::c_void
        )),
        _ => None,
    }
}
