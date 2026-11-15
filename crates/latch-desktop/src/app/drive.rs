//! Script driver for the `verify-latch` skill. Renders the real Command Palette
//! with the platform's fonts and GPU renderer, plays a plain-text script against
//! a sandbox Vault, and saves screenshots. Run it through the skill's `verify.sh`.

use super::Latch;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Focusable as _, HeadlessAppContext, Window,
    WindowHandle, test::TestWindowExt as _,
};
use latch_core::vault::storage::VaultStorage;
use std::path::{Path, PathBuf};

/// Master password of every sandbox Vault the driver provisions.
const SANDBOX_PASSWORD: &str = "latch sandbox password";

struct Driver {
    cx: HeadlessAppContext,
    window: WindowHandle<Latch>,
    view: gpui_kit::Entity<Latch>,
    out: PathBuf,
    vault: PathBuf,
    shots: usize,
}

impl Driver {
    fn act(&mut self, f: impl FnOnce(&mut Window, &mut App)) {
        self.cx
            .update_window(self.window.into(), |_, window, cx| f(window, cx))
            .unwrap();
        self.settle();
    }

    /// Runs Vault work to completion and draws a fresh frame.
    fn settle(&mut self) {
        self.cx.run_until_parked();
        self.cx
            .update_window(self.window.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
    }

    fn read<R>(&mut self, f: impl FnOnce(&Latch, &Window, &App) -> R) -> R {
        let view = self.view.clone();
        self.cx
            .update_window(self.window.into(), |_, window, cx| {
                f(view.read(cx), window, cx)
            })
            .unwrap()
    }

    /// What the user sees, in one line. Field values never appear here.
    fn state(&mut self) -> String {
        self.read(|this, window, cx| {
            let rows: Vec<&str> = this.previews.iter().map(|row| row.title.as_str()).collect();
            let focus = if this.query.read(cx).focus_handle(cx).is_focused(window) {
                "search".to_string()
            } else if this
                .action_query
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
            {
                "actions".to_string()
            } else if let Some(index) = this
                .fields
                .iter()
                .position(|field| field.read(cx).focus_handle(cx).is_focused(window))
            {
                format!("field {index}")
            } else if let Some(index) = this
                .page_stops
                .borrow()
                .iter()
                .position(|stop| stop.is_focused(window))
            {
                format!("stop {index}")
            } else {
                "none".to_string()
            };
            format!(
                "page={:?} focus={focus} rows={rows:?} notice={:?}",
                this.page, this.notice
            )
        })
    }

    fn shot(&mut self, name: &str) -> Result<PathBuf, String> {
        self.shots += 1;
        let path = self.out.join(format!("{:02}-{name}.png", self.shots));
        let image = self
            .cx
            .capture_screenshot(AnyWindowHandle::from(self.window))
            .map_err(|error| format!("screenshot failed: {error}"))?;
        image.save(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn run(&mut self, command: &str, arg: &str) -> Result<(), String> {
        match command {
            "type" => self.act(|window, cx| window.input(arg, cx)),
            "press" => {
                for key in arg.split_whitespace() {
                    self.act(|window, cx| window.press(key, cx));
                }
            }
            "click" => {
                let id = gpui_kit::ElementId::Name(arg.to_string().into());
                let visible = self.read(|_, window, _| {
                    window.try_find(id.clone()).map(|element| element.visible())
                });
                match visible {
                    Some(true) => self.act(|window, cx| window.click(id, cx)),
                    Some(false) => return Err(format!("element {arg:?} is not visible")),
                    None => return Err(format!("no element {arg:?} on this page")),
                }
            }
            "unlock" => {
                let setup = self.read(|this, _, _| this.page == super::Page::Setup);
                self.run("type", SANDBOX_PASSWORD)?;
                if setup {
                    self.run("press", "tab")?;
                    self.run("type", SANDBOX_PASSWORD)?;
                }
                self.run("press", "enter")?;
            }
            "add" => {
                // Fills the real New credential form: Title, Username, (Alias),
                // Password, (Generate), Website, Authenticator.
                let parts: Vec<&str> = arg.split(" | ").collect();
                if !(3..=5).contains(&parts.len()) {
                    return Err("add takes Title | Username | Password [| Website [| TOTP]]".into());
                }
                self.run("press", "ctrl-n")?;
                for (index, (value, tabs)) in parts
                    .iter()
                    .zip(["tab", "tab tab", "tab tab", "tab", ""])
                    .enumerate()
                {
                    self.run("type", value)?;
                    if index + 1 < parts.len() {
                        self.run("press", tabs)?;
                    }
                }
                self.run("press", "ctrl-s")?;
                self.expect("page Browse")?;
            }
            "sleep" => {
                let ms = arg.parse().map_err(|_| "sleep takes milliseconds")?;
                std::thread::sleep(std::time::Duration::from_millis(ms));
                self.settle();
            }
            "shot" => {
                let path = self.shot(if arg.is_empty() { "shot" } else { arg })?;
                println!("  saved {}", path.display());
            }
            "expect" => self.expect(arg)?,
            _ => return Err(format!("unknown command {command:?}")),
        }
        Ok(())
    }

    fn expect(&mut self, arg: &str) -> Result<(), String> {
        let (what, want) = arg.split_once(' ').unwrap_or((arg, ""));
        let got = match what {
            "page" => self.read(|this, _, _| format!("{:?}", this.page)),
            "focus" => {
                let state = self.state();
                let focus = state.split(" focus=").nth(1).unwrap_or("");
                focus.split(" rows=").next().unwrap_or("").to_string()
            }
            "rows" => self.read(|this, _, _| {
                this.previews
                    .iter()
                    .map(|row| row.title.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
            "notice" => {
                let notice = self.read(|this, _, _| this.notice.clone());
                return if notice.contains(want) {
                    Ok(())
                } else {
                    Err(format!("notice {notice:?} lacks {want:?}"))
                };
            }
            "clipboard" => self
                .cx
                .update(|cx| cx.read_from_clipboard())
                .and_then(|item| item.text())
                .unwrap_or_default(),
            "visible" => {
                let id = gpui_kit::ElementId::Name(want.to_string().into());
                let visible = self.read(|_, window, _| {
                    window.try_find(id).is_some_and(|element| element.visible())
                });
                return visible
                    .then_some(())
                    .ok_or(format!("element {want:?} is not visible"));
            }
            "vault-lacks" => {
                let bytes = std::fs::read(&self.vault).map_err(|error| error.to_string())?;
                return (!bytes.windows(want.len()).any(|w| w == want.as_bytes()))
                    .then_some(())
                    .ok_or(format!("vault.enc contains {want:?} in plaintext"));
            }
            _ => return Err(format!("unknown expectation {what:?}")),
        };
        if got == want {
            Ok(())
        } else {
            Err(format!("expected {what} {want:?}, got {got:?}"))
        }
    }
}

fn open(vault: &Path, out: PathBuf, (width, height): (f32, f32)) -> Driver {
    use gpui_kit::{Bounds, WindowBounds, WindowOptions, point, px, size};
    let platform = gpui_kit::platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(
        platform.text_system(),
        std::sync::Arc::new(crate::ui::Icons),
        gpui_kit::platform::current_headless_renderer,
    );
    let storage = VaultStorage {
        path: vault.to_path_buf(),
    };
    let window = cx.update(|cx| {
        gpui_kit::init(cx);
        crate::input::init(cx);
        super::init(cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.), px(0.)),
                    size: size(px(width), px(height)),
                })),
                // Same frame choice as main.rs, so the drawn title bar matches.
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui_kit::WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Latch::new(storage, window, cx)),
        )
        .unwrap()
    });
    let view = cx.update(|cx| window.entity(cx).unwrap());
    let mut driver = Driver {
        cx,
        window,
        view,
        out,
        vault: vault.to_path_buf(),
        shots: 0,
    };
    driver.settle();
    driver
}

#[test]
#[ignore = "run through .agents/skills/verify-latch/verify.sh drive"]
fn drive() {
    let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is required"));
    let script = std::fs::read_to_string(var("LATCH_DRIVE_SCRIPT")).unwrap();
    let vault = PathBuf::from(var("LATCH_DRIVE_VAULT"));
    let out = PathBuf::from(var("LATCH_DRIVE_OUT"));
    let size = std::env::var("LATCH_DRIVE_SIZE").unwrap_or_else(|_| "720x480".into());
    let (width, height) = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .expect("LATCH_DRIVE_SIZE looks like 720x480");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(vault.parent().unwrap()).unwrap();

    let mut driver = open(&vault, out, (width, height));
    println!("  {}", driver.state());
    for (number, line) in script
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.trim()))
    {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (command, arg) = line.split_once(' ').unwrap_or((line, ""));
        println!("{number:>3}> {line}");
        if let Err(error) = driver.run(command, arg) {
            let state = driver.state();
            let shot = driver
                .shot("failure")
                .map(|path| path.display().to_string());
            panic!(
                "line {number} `{line}` failed: {error}\n  {state}\n  screenshot: {}",
                shot.unwrap_or_else(|error| error)
            );
        }
        if command != "expect" && command != "shot" {
            println!("     {}", driver.state());
        }
    }
}
