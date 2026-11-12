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
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
    use ashpd::zbus::export::futures_core::Stream;
    use std::task::Poll;
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
    // The user can change or clear the key in the desktop's shortcut settings.
    let mut changed = std::pin::pin!(portal.receive_shortcuts_changed().await?);
    // Empty when no key is assigned.
    let trigger = |shortcuts: &[Shortcut]| {
        shortcuts
            .iter()
            .find(|shortcut| shortcut.id() == "toggle")
            .map(|shortcut| shortcut.trigger_description().to_owned())
            .unwrap_or_default()
    };
    let mut next = Some(PortalEvent::Bound(trigger(bound.shortcuts())));
    // Toggle is the only shortcut bound, so every activation is a toggle.
    while let Some(event) = next
        && events.send(event).is_ok()
    {
        next = std::future::poll_fn(|cx| {
            if let Poll::Ready(change) = changed.as_mut().poll_next(cx) {
                return Poll::Ready(change.map(|c| PortalEvent::Bound(trigger(c.shortcuts()))));
            }
            activated.as_mut().poll_next(cx).map(|press| {
                press.map(|press| {
                    PortalEvent::Activated(
                        press
                            .options()
                            .get("activation_token")
                            .and_then(|token| <&str>::try_from(&**token).ok())
                            .map(str::to_owned),
                    )
                })
            })
        })
        .await;
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
        Err(error) => (
            None,
            format!(
                "Global shortcut unavailable: {error}. Open Latch from your application launcher."
            ),
        ),
    }
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
                            // The raw portal error went to stderr above.
                            (
                                None,
                                "Global shortcut unavailable: your desktop did not grant one. Open Latch from your application launcher."
                                    .into(),
                            )
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
/// raises it with an activation `token` or KDE's PID-checked fallback.
pub fn present(window: &mut Window, token: Option<String>, cx: &mut App) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = hwnd(window) {
        show(hwnd);
    }
    match token {
        #[cfg(target_os = "linux")]
        Some(token) if wayland::activate(window, &token) => {}
        _ => {
            window.activate_window();
            #[cfg(target_os = "linux")]
            if std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| {
                desktop
                    .split(':')
                    .any(|name| name.eq_ignore_ascii_case("KDE"))
            }) {
                cx.background_executor()
                    .spawn(async {
                        if let Err(error) = kwin_activate().await {
                            eprintln!("KWin activation failed: {error}");
                        }
                    })
                    .detach();
            }
        }
    }
    cx.activate(true);
}

// KDE's shortcut portal can omit the token; only raise a window owned by us.
#[cfg(target_os = "linux")]
async fn kwin_activate() -> ashpd::zbus::Result<()> {
    use ashpd::zbus::{Connection, Proxy, zvariant::OwnedValue};
    use std::collections::HashMap;
    let connection = Connection::session().await?;
    let runner = Proxy::new(
        &connection,
        "org.kde.KWin",
        "/WindowsRunner",
        "org.kde.krunner1",
    )
    .await?;
    let kwin = Proxy::new(&connection, "org.kde.KWin", "/KWin", "org.kde.KWin").await?;
    type Match = (
        String,
        String,
        String,
        i32,
        f64,
        HashMap<String, OwnedValue>,
    );
    let matches: Vec<Match> = runner.call("Match", &("Latch",)).await?;
    for (id, ..) in matches {
        let Some((_, uuid)) = id.split_once('_') else {
            continue;
        };
        if uuid.contains('_') || !uuid.starts_with('{') || !uuid.ends_with('}') {
            continue;
        }
        let info: HashMap<String, OwnedValue> = match kwin.call("getWindowInfo", &(uuid,)).await {
            Ok(info) => info,
            Err(error) => {
                eprintln!("KWin window info failed: {error}");
                continue;
            }
        };
        if info
            .get("pid")
            .and_then(|pid| i32::try_from(pid).ok())
            .and_then(|pid| u32::try_from(pid).ok())
            == Some(std::process::id())
        {
            // WindowsRunnerAction::ActivateAction is 0; discard Match's action.
            let (): () = runner.call("Run", &(format!("0_{uuid}"), "")).await?;
            break;
        }
    }
    Ok(())
}

/// Raises the window with an xdg-activation token another process handed over
/// (the shortcut portal). GPUI only requests its own token, which compositors
/// refuse, so Latch makes the call itself on GPUI's libwayland connection.
#[cfg(target_os = "linux")]
mod wayland {
    use gpui_kit::Window;
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
    use std::cell::OnceCell;
    use wayland_backend::client::{Backend, ObjectId};
    use wayland_client::globals::{GlobalListContents, registry_queue_init};
    use wayland_client::protocol::{wl_registry::WlRegistry, wl_surface::WlSurface};
    use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
    use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

    struct Ignore;
    impl Dispatch<WlRegistry, GlobalListContents> for Ignore {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    wayland_client::delegate_noop!(Ignore: ignore XdgActivationV1);

    struct Activation {
        connection: Connection,
        global: XdgActivationV1,
        // Keeps the registry and global attached to a live queue. Never
        // dispatched: neither needs events.
        _queue: EventQueue<Ignore>,
    }

    thread_local! {
        // Bound once, on the UI thread that owns the window.
        static ACTIVATION: OnceCell<Option<Activation>> = const { OnceCell::new() };
    }

    fn bind(display: *mut std::ffi::c_void) -> Option<Activation> {
        // SAFETY: GPUI's libwayland display lives as long as the app, and a
        // foreign backend never closes it.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<Ignore>(&connection).ok()?;
        let global = globals.bind(&queue.handle(), 1..=1, ()).ok()?;
        Some(Activation {
            connection,
            global,
            _queue: queue,
        })
    }

    /// False off Wayland or when the compositor lacks xdg-activation.
    pub fn activate(window: &Window, token: &str) -> bool {
        // GPUI's own `window_handle` method shadows the raw-window-handle one.
        let surface = HasWindowHandle::window_handle(window);
        let (Ok(display), Ok(surface)) = (window.display_handle(), surface) else {
            return false;
        };
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) =
            (display.as_raw(), surface.as_raw())
        else {
            return false;
        };
        ACTIVATION.with(|cell| {
            let Some(activation) = cell.get_or_init(|| bind(display.display.as_ptr())) else {
                return false;
            };
            // SAFETY: the surface is GPUI's live window on that same display.
            let id = unsafe {
                ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast())
            };
            let Ok(surface) = id.and_then(|id| WlSurface::from_id(&activation.connection, id))
            else {
                return false;
            };
            activation.global.activate(token.to_owned(), &surface);
            activation.connection.flush().is_ok()
        })
    }
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
