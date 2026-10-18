use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui_kit::{AnyWindowHandle, App, Entity, Global};
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
