#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod biometric;
mod clipboard;
mod input;
mod integration;
mod theme;
mod update;
use gpui_kit::{AppContext, TitlebarOptions, WindowOptions, px};
use latch_core::vault::storage::VaultStorage;
struct ProcessLock(std::fs::File);
impl gpui_kit::Global for ProcessLock {}
impl Drop for ProcessLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn main() {
    let storage = match VaultStorage::new() {
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
    gpui_kit::application().run(move |cx| {
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
            titlebar: Some(TitlebarOptions {
                title: Some("Latch".into()),
                ..Default::default()
            }),
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(
                gpui_kit::Bounds::centered(None, gpui_kit::size(px(640.), px(520.)), cx),
            )),
            window_min_size: Some(gpui_kit::size(px(480.), px(320.))),
            ..Default::default()
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
