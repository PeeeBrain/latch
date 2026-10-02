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
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let mark = (8..12).contains(&x) && (6..26).contains(&y)
                || (8..25).contains(&x) && (22..26).contains(&y);
            rgba.extend_from_slice(if mark {
                &[255, 255, 255, 255]
            } else {
                &[40, 80, 130, 255]
            });
        }
    }
    TrayIconBuilder::new()
        .with_tooltip("Latch")
        .with_menu(Box::new(menu))
        .with_icon(Icon::from_rgba(rgba, 32, 32).map_err(|e| e.to_string())?)
        .build()
        .map_err(|e| e.to_string())
}

pub fn start(window: AnyWindowHandle, view: Entity<crate::app::Latch>, cx: &mut App) {
    let modifiers = if cfg!(target_os = "macos") {
        Modifiers::SUPER
    } else {
        Modifiers::CONTROL
    } | Modifiers::SHIFT;
    let hotkey = GlobalHotKeyManager::new().and_then(|manager| {
        manager.register(HotKey::new(Some(modifiers), Code::Space))?;
        Ok(manager)
    });
    let status = match &hotkey {
        Ok(_) => "Quick access: Ctrl+Shift+Space (Cmd+Shift+Space on macOS).".into(),
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
                    commands.push("show".to_string());
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
    cx.try_global::<Integration>()
        .is_some_and(|integration| integration.tray_available.load(Ordering::SeqCst))
}
