#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod biometric;
mod clipboard;
mod input;
mod integration;
mod theme;
mod ui;
mod update;
use gpui_kit::{AppContext, TitlebarOptions, WindowOptions, component::TitleBar, px};
use latch_core::vault::storage::VaultStorage;
struct ProcessLock(std::fs::File);
impl gpui_kit::Global for ProcessLock {}
impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn main() {
    // Debug builds can point at a throwaway vault for UI work.
    let dev_vault = std::env::var_os("LATCH_DEV_VAULT").filter(|_| cfg!(debug_assertions));
    let storage = match dev_vault.map_or_else(VaultStorage::new, |path| {
        Ok(VaultStorage { path: path.into() })
    }) {
        Ok(storage) => storage,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    let process_lock = match storage.acquire_process_lock() {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    gpui_kit::application()
        .with_assets(ui::Icons)
        .run(move |cx| {
            cx.set_global(ProcessLock(process_lock));
            cx.set_app_identity("com.latch.passwordmanager", "Latch");
            gpui_kit::init(cx);
            input::init(cx);
            app::init(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let options = WindowOptions {
                app_id: Some("com.latch.passwordmanager".to_string()),
                // Native title text stays populated for the taskbar and alt-tab
                // even though the bar itself is drawn by the app.
                titlebar: Some(TitlebarOptions {
                    title: Some("Latch".into()),
                    ..TitleBar::title_bar_options()
                }),
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(
                    gpui_kit::Bounds::centered(None, gpui_kit::size(px(720.), px(480.)), cx),
                )),
                window_min_size: Some(gpui_kit::size(px(520.), px(360.))),
                ..TitleBar::window_options()
            };
            match gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| app::Latch::new(storage, window, cx))
            }) {
                Ok((window, view)) => integration::start(window, view, cx),
                Err(error) => {
                    eprintln!("Unable to open Latch: {error}");
                    cx.quit();
                }
            }
            cx.activate(true);
        });
}
