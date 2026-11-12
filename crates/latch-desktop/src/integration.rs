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

/// `Toggle` and `Show` carry the activation token a Wayland desktop hands over
/// with a shortcut press; without one the compositor will not raise Latch.
pub enum IntegrationCommand {
    Toggle(Option<String>),
    Show(Option<String>),
    Lock,
    Quit,
}

/// What the GlobalShortcuts portal task reports back to the command loop.
#[cfg(target_os = "linux")]
enum PortalEvent {
    Bound(String),
    Failed(String),
    Activated(Option<String>),
}

/// Binds Ctrl+Space through the XDG GlobalShortcuts portal, the only way to
/// get a global shortcut on Wayland, and forwards presses until the session ends.
#[cfg(target_os = "linux")]
async fn portal_shortcut(events: std::sync::mpsc::Sender<PortalEvent>) -> ashpd::Result<()> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use ashpd::zbus::export::futures_core::Stream;
    // Registering the host app id must be the first portal call on a
    // connection, and GPUI has already used its own, so this one is fresh.
    let connection = ashpd::zbus::Connection::session().await?;
    ashpd::register_host_app_with_connection(
        connection.clone(),
        "com.latch.passwordmanager".try_into()?,
    )
    .await?;
    let portal = GlobalShortcuts::with_connection(connection).await?;
    // The portal ends the session when this connection closes, so the shortcut
    // stays bound while `portal` lives, which is as long as the loop below.
    let session = portal.create_session(Default::default()).await?;
    let shortcut = NewShortcut::new("toggle", "Show or hide Latch").preferred_trigger("CTRL+space");
    let bound = portal
        .bind_shortcuts(&session, &[shortcut], None, Default::default())
        .await?
        .response()?;
    let mut activated = std::pin::pin!(portal.receive_activated().await?);
    let trigger = bound
        .shortcuts()
        .iter()
        .find(|shortcut| shortcut.id() == "toggle")
        .map(|shortcut| shortcut.trigger_description().to_owned())
        .unwrap_or_default();
    let _ = events.send(PortalEvent::Bound(trigger));
    while let Some(press) = std::future::poll_fn(|cx| activated.as_mut().poll_next(cx)).await {
        if press.shortcut_id() != "toggle" {
            continue;
        }
        let token = press
            .options()
            .get("activation_token")
            .and_then(|token| <&str>::try_from(&**token).ok())
            .map(str::to_owned);
        if events.send(PortalEvent::Activated(token)).is_err() {
            break;
        }
    }
    Ok(())
}

/// The X11-era path: works on Windows, macOS, and X11, and nowhere else.
fn register_hotkey() -> (Option<GlobalHotKeyManager>, String) {
    // Same quick-access chord as the previous app: Ctrl+Space on every platform.
    let hotkey = GlobalHotKeyManager::new().and_then(|manager| {
        manager.register(HotKey::new(Some(Modifiers::CONTROL), Code::Space))?;
        Ok(manager)
    });
    match hotkey {
        Ok(manager) => (
            Some(manager),
            "Quick access: Ctrl+Space shows or hides Latch.".into(),
        ),
        Err(error) => (None, shortcut_unavailable(error)),
    }
}

fn shortcut_unavailable(error: impl std::fmt::Display) -> String {
    format!("Global shortcut unavailable: {error}. Open Latch from your application launcher.")
}

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

fn menu_command(id: &str) -> Option<IntegrationCommand> {
    match id {
        "show" => Some(IntegrationCommand::Show(None)),
        "lock" => Some(IntegrationCommand::Lock),
        "quit" => Some(IntegrationCommand::Quit),
        _ => None,
    }
}

pub fn start(window: AnyWindowHandle, view: Entity<crate::app::Latch>, cx: &mut App) {
    #[cfg(not(target_os = "linux"))]
    let (hotkey, status) = register_hotkey();
    // Linux tries the portal first (Wayland and modern X11 desktops). If it
    // fails, an X11 session falls back to the X11 grab.
    #[cfg(target_os = "linux")]
    let (hotkey, status, portal) = {
        let (events, portal) = std::sync::mpsc::channel();
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = portal_shortcut(events.clone()).await {
                    let _ = events.send(PortalEvent::Failed(error.to_string()));
                }
            })
            .detach();
        (
            None,
            "Quick access: waiting for your desktop to confirm Ctrl+Space.".to_string(),
            portal,
        )
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
        _hotkey: hotkey,
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
                    commands.push(IntegrationCommand::Toggle(None));
                }
            }
            #[cfg(target_os = "linux")]
            while let Ok(event) = portal.try_recv() {
                match event {
                    PortalEvent::Activated(token) => {
                        commands.push(IntegrationCommand::Toggle(token));
                    }
                    PortalEvent::Bound(trigger) => {
                        let status = if trigger.is_empty() {
                            "Quick access: assign a key to Latch in your desktop's shortcut settings."
                                .into()
                        } else {
                            format!("Quick access: {trigger} shows or hides Latch.")
                        };
                        cx.update_global(|integration: &mut Integration, _| {
                            integration.status = status;
                        });
                    }
                    PortalEvent::Failed(error) => {
                        eprintln!("Global shortcuts portal unavailable: {error}");
                        // GPUI picks Wayland the same way. An X11 grab there only
                        // reaches XWayland windows, so it would claim a shortcut
                        // that never fires.
                        let wayland = std::env::var_os("WAYLAND_DISPLAY")
                            .is_some_and(|display| !display.is_empty());
                        let (hotkey, status) = if wayland {
                            (None, shortcut_unavailable(error))
                        } else {
                            register_hotkey()
                        };
                        cx.update_global(|integration: &mut Integration, _| {
                            integration._hotkey = hotkey;
                            integration.status = status;
                        });
                    }
                }
            }
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some(command) = menu_command(&event.id.0) {
                    commands.push(command);
                }
            }
            for command in commands {
                if window
                    .update(cx, |_, window, cx| {
                        view.update(cx, |this, cx| this.integration_command(command, window, cx));
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

/// Shows and focuses the window, undoing `dismiss` or a minimize. Wayland only
/// raises it with an activation `token` from the shortcut press.
pub fn present(window: &mut Window, token: Option<String>, cx: &mut App) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = hwnd(window) {
        show(hwnd);
    }
    match token {
        Some(token) => window.activate_window_with_token(token),
        None => window.activate_window(),
    }
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
