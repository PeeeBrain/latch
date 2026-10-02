mod views;

use crate::{
    input::TextInput,
    theme::{self, Appearance, Color, Palette, Source},
};
use gpui_kit::{prelude::*, *};
use latch_core::{
    auth::{authenticator::AuthCredential, method::AuthMethod, password},
    password_generator::{self, PasswordOptions},
    vault::{
        self, Entry, EntryPreview, coordinator::VaultCoordinator, storage::VaultStorage,
        workspace::Workspace,
    },
};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;
gpui_kit::actions!(
    latch,
    [
        Quit,
        Back,
        Submit,
        Next,
        Previous,
        Add,
        Edit,
        Lock,
        Settings,
        Actions,
        CopyUsername,
        CopyTotp,
        Tab,
        TabPrevious,
        ResetAppearance
    ]
);
#[derive(Clone, PartialEq, Debug)]
enum Page {
    Setup,
    Locked,
    Browse,
    Details,
    Edit,
    Delete,
    Settings,
    Rotate,
    Health,
    Actions,
    Appearance,
    Preview,
    Providers,
    Provider,
    DeleteProvider,
    Generator,
    Discard,
}
enum Reply {
    Browse(Vec<EntryPreview>),
    Detail(Entry),
    Saved,
    Copied(Zeroizing<String>),
    Health(Vec<(String, String)>),
    Rotated,
    Biometric,
    Providers(Vec<vault::alias::AliasProviderInfo>, Option<String>),
}
const COMMANDS: [(&str, &str); 9] = [
    ("generator", "Generate password"),
    ("details", "View selected credential"),
    ("new", "New credential"),
    ("edit", "Edit selected credential"),
    ("username", "Copy username"),
    ("totp", "Copy TOTP"),
    ("health", "Password health"),
    ("settings", "Settings"),
    ("lock", "Lock vault"),
];
pub struct Latch {
    window: AnyWindowHandle,
    vault: Arc<Mutex<VaultCoordinator>>,
    epoch: Arc<AtomicU64>,
    page: Page,
    fields: Vec<Entity<TextInput>>,
    query: Entity<TextInput>,
    action_query: Entity<TextInput>,
    focus: FocusHandle,
    previews: Vec<EntryPreview>,
    selected: usize,
    scroll: ScrollHandle,
    editing: Option<String>,
    detail: Option<Entry>,
    reveal_until: Option<SystemTime>,
    notice: String,
    busy: bool,
    blocked: bool,
    biometric: bool,
    clipboard: Option<(Zeroizing<String>, String, SystemTime)>,
    issues: Vec<(String, String)>,
    breaches: Vec<(String, String)>,
    last_query: String,
    appearance: Option<Appearance>,
    preview: Option<Appearance>,
    appearance_directory: PathBuf,
    appearance_modified: Option<SystemTime>,
    providers: Vec<vault::alias::AliasProviderInfo>,
    default_provider: Option<String>,
    provider: String,
    draft_alias_provider: Option<String>,
    generated: Zeroizing<String>,
    generator_options: PasswordOptions,
    generator_return: Page,
    update: Option<cargo_packager_updater::Update>,
    installing: bool,
}
impl Latch {
    pub fn new(storage: VaultStorage, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (page, notice, blocked) = match storage.inspect() {
            Ok(None) => (Page::Setup, String::new(), false),
            Ok(Some(header)) if matches!(header.kdf.as_str(), "password-pbkdf2" | "argon2id") => (Page::Locked, String::new(), false),
            Ok(Some(header)) if header.kdf.starts_with("oauth-") => (Page::Locked, "This Google vault needs migration in the previous Latch app. Your vault has not been changed.".into(), true),
            Ok(Some(header)) if header.kdf == "biometric-keychain" => (Page::Locked, "Unlock with your device key. You can switch to a master password after unlocking.".into(), !crate::biometric::supported()),
            Ok(Some(_)) => (Page::Locked, "This vault needs a compatible Latch release. Your vault has not been changed.".into(), true),
            Err(error) => (Page::Locked, error, true),
        };
        let biometric = storage
            .inspect()
            .ok()
            .flatten()
            .is_some_and(|header| header.kdf == "biometric-keychain");
        let appearance_directory = storage
            .path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        let appearance = Appearance::load(&appearance_directory).unwrap_or_else(|_| {
            let backup = appearance_directory.join("settings.last-valid.json");
            theme::read(&backup).ok().and_then(|text| {
                Appearance::parse(&text, Source::VsCode)
                    .ok()
                    .filter(|appearance| appearance.palette(true).is_ok_and(|(_, count)| count > 0))
                    .or_else(|| Appearance::parse(&text, Source::Zed).ok())
            })
        });
        let appearance_modified = std::fs::metadata(appearance_directory.join("settings.json"))
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        let query =
            cx.new(|cx| TextInput::new("Search credentials or type > for commands", false, cx));
        let action_query = cx.new(|cx| TextInput::new("Search actions", false, cx));
        let mut this = Self {
            window: window.window_handle(),
            vault: Arc::new(Mutex::new(VaultCoordinator::new(
                storage,
                Workspace::new(),
                Box::new(|_| {}),
            ))),
            epoch: Arc::new(AtomicU64::new(0)),
            page,
            fields: vec![],
            query,
            action_query,
            focus: cx.focus_handle(),
            previews: vec![],
            selected: 0,
            scroll: ScrollHandle::new(),
            editing: None,
            detail: None,
            reveal_until: None,
            notice,
            busy: false,
            blocked,
            biometric,
            clipboard: None,
            issues: vec![],
            breaches: vec![],
            last_query: String::new(),
            appearance,
            preview: None,
            appearance_directory,
            appearance_modified,
            providers: Vec::new(),
            default_provider: None,
            provider: String::new(),
            draft_alias_provider: None,
            generated: Zeroizing::new(String::new()),
            generator_options: PasswordOptions::default(),
            generator_return: Page::Browse,
            update: None,
            installing: false,
        };
        cx.observe(&this.query, |this, query, cx| {
            if this.page == Page::Browse && !this.busy && this.last_query != query.read(cx).value()
            {
                this.selected = 0;
                this.search(query.read(cx).value().to_owned(), cx);
            }
        })
        .detach();
        cx.observe(&this.action_query, |_, _, cx| cx.notify())
            .detach();
        this.auth_fields(window, cx);
        let activity_view = cx.entity().downgrade();
        cx.intercept_keystrokes(move |_, window, cx| {
            let _ = activity_view.update(cx, |this, cx| {
                if this.window == window.window_handle() {
                    this.activity(window, cx);
                }
            });
        })
        .detach();
        let view = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            let _ = view.update(cx, |this, cx| this.lock(window, cx));
            if crate::integration::has_tray(cx) {
                window.minimize_window();
                false
            } else {
                true
            }
        });
        cx.on_release(|this, cx| {
            this.epoch.fetch_add(1, Ordering::SeqCst);
            this.clear_clipboard(cx);
            if let Ok(mut vault) = this.vault.try_lock() {
                let _ = vault.with_vault(|_, workspace| {
                    workspace.lock();
                    Ok(())
                });
            }
        })
        .detach();
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if view
                    .update_in(cx, |this, window, cx| this.tick(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        this
    }
    fn field(&mut self, label: &str, secret: bool, cx: &mut Context<Self>) {
        self.fields
            .push(cx.new(|cx| TextInput::new(label, secret, cx)));
    }
    pub fn integration_command(
        &mut self,
        command: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            "show" => {
                window.activate_window();
                cx.activate(true);
                self.focus_first(window, cx);
            }
            "lock" => self.lock(window, cx),
            "quit" => {
                self.lock(window, cx);
                cx.quit();
            }
            _ => {}
        }
    }
    fn auth_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.fields.clear();
        if !self.biometric {
            self.field("Master password", true, cx);
        }
        if self.page == Page::Setup {
            self.field("Confirm master password", true, cx);
        }
        self.focus_first(window, cx);
    }
    fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(field) = self.fields.first() {
            window.focus(&field.read(cx).focus_handle(cx), cx);
        } else {
            window.focus(&self.focus, cx);
        }
    }
    fn value(&self, index: usize, cx: &App) -> Zeroizing<String> {
        Zeroizing::new(
            self.fields
                .get(index)
                .map(|field| field.read(cx).value().to_owned())
                .unwrap_or_default(),
        )
    }
    fn work(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&mut VaultCoordinator) -> Result<Reply, String> + Send + 'static,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.notice.clear();
        let vault = self.vault.clone();
        let epoch = self.epoch.clone();
        let generation = epoch.load(Ordering::SeqCst);
        let task = cx.background_executor().spawn(async move {
            let mut vault = vault
                .lock()
                .map_err(|_| "Vault service unavailable".to_string())?;
            if generation != epoch.load(Ordering::SeqCst) {
                return Err("Operation canceled".into());
            }
            let result = operation(&mut vault);
            if generation != epoch.load(Ordering::SeqCst) {
                vault.with_vault(|_, workspace| {
                    workspace.lock();
                    Ok(())
                })?;
                return Err("Operation canceled".into());
            }
            result
        });
        cx.spawn(async move |view, cx| {
            let result = task.await;
            view.update(cx, |this, cx| {
                if this.epoch.load(Ordering::SeqCst) != generation {
                    return;
                }
                this.busy = false;
                let previous_page = this.page.clone();
                match result {
                    Ok(Reply::Browse(entries)) => {
                        this.previews = entries;
                        this.selected = this.selected.min(this.previews.len().saturating_sub(1));
                        if matches!(this.page, Page::Locked | Page::Setup) {
                            this.page = Page::Browse;
                            this.fields.clear();
                        }
                        if this.last_query != this.query.read(cx).value() {
                            this.search(this.query.read(cx).value().to_owned(), cx);
                        }
                    }
                    Ok(Reply::Detail(entry)) => {
                        this.fields.clear();
                        this.field("Password", true, cx);
                        this.fields[0].update(cx, |field, cx| { field.set_value(entry.password.clone(), cx); field.read_only(); });
                        this.reveal_until = None;
                        this.detail = Some(entry);
                        this.page = Page::Details;
                    }
                    Ok(Reply::Saved) => {
                        this.fields.clear();
                        this.detail = None;
                        this.editing = None;
                        this.page = Page::Browse;
                        this.search(this.query.read(cx).value().to_owned(), cx);
                        this.notice = "Saved to encrypted vault".into();
                    }
                    Ok(Reply::Copied(text)) => this.copy(text, cx),
                    Ok(Reply::Health(issues)) => {
                        this.issues = issues;
                        this.breaches.clear();
                        this.page = Page::Health;
                    }
                    Ok(Reply::Rotated) => {
                        this.biometric = false;
                        this.fields.clear();
                        this.page = Page::Settings;
                        this.notice = "Master password changed".into();
                    }
                    Ok(Reply::Biometric) => {
                        this.biometric = true;
                        this.fields.clear();
                        this.page = Page::Settings;
                        this.notice = "Vault now uses your device key. Keep this device key available to unlock.".into();
                    }
                    Ok(Reply::Providers(providers, default)) => {
                        this.providers = providers;
                        this.default_provider = default;
                        this.fields.clear();
                        this.page = Page::Providers;
                    }
                    Err(error) => this.notice = error,
                }
                if previous_page != this.page {
                    let focus = if this.page == Page::Browse {
                        this.query.read(cx).focus_handle(cx)
                    } else {
                        this.focus.clone()
                    };
                    let window = this.window;
                    cx.defer(move |cx| {
                        let _ = cx.update_window(window, |_, window, cx| window.focus(&focus, cx));
                    });
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn search(&mut self, query: String, cx: &mut Context<Self>) {
        self.last_query = query.clone();
        if query.starts_with('>') {
            cx.notify();
            return;
        }
        self.work(cx, move |vault| {
            vault.with_vault(|_, workspace| {
                vault::search::search(workspace, &query).map(Reply::Browse)
            })
        });
    }
    fn import_appearance(&mut self, source: Source, cx: &mut Context<Self>) {
        let epoch = self.epoch.load(Ordering::SeqCst);
        let page = self.page.clone();
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose editor settings.json".into()),
        });
        cx.spawn(async move |view, cx| {
            let path = match picker.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let Some(path) = path else { return; };
            let result = cx.background_executor().spawn(async move {
                let text = theme::read(&path)?;
                Appearance::parse(&text, source)
            }).await;
            view.update(cx, |this, cx| {
                if this.epoch.load(Ordering::SeqCst) != epoch || this.page != page { return; }
                match result {
                    Ok(appearance) if [true, false].into_iter().any(|dark|
                        appearance.palette(dark).is_ok_and(|(_, count)| count > 0)) => {
                        this.preview = Some(appearance);
                        this.page = Page::Preview;
                        this.notice = "Preview only. Apply saves explicit colors; a theme name cannot supply its palette.".into();
                    }
                    Ok(_) => this.notice = "No applicable color overrides. Existing appearance retained.".into(),
                    Err(error) => this.notice = error,
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn reset_appearance(&mut self, cx: &mut Context<Self>) {
        let result = (|| {
            for name in [
                "settings.json",
                "settings.last-valid.json",
                "appearance-source",
            ] {
                match std::fs::remove_file(self.appearance_directory.join(name)) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return Err("Cannot reset appearance".to_string()),
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.appearance = None;
                self.preview = None;
                self.appearance_modified = None;
                self.notice = "Appearance reset to system fallback".into();
            }
            Err(error) => self.notice = error,
        }
        cx.notify();
    }
    fn reload_appearance(&mut self, cx: &mut Context<Self>) {
        match Appearance::load(&self.appearance_directory) {
            Ok(Some(appearance)) => {
                if [true, false]
                    .into_iter()
                    .any(|dark| appearance.palette(dark).is_ok_and(|(_, count)| count > 0))
                {
                    let _ = theme::atomic_write(
                        &self.appearance_directory.join("settings.last-valid.json"),
                        serde_json::to_string_pretty(&appearance.document)
                            .unwrap_or_default()
                            .as_bytes(),
                    );
                    self.appearance = Some(appearance);
                    self.notice = "Appearance reloaded".into();
                } else {
                    self.notice = "No applicable colors. Last valid appearance retained.".into();
                }
            }
            Ok(None) => self.appearance = None,
            Err(error) => self.notice = format!("{error}. Last valid appearance retained."),
        }
        cx.notify();
    }
    fn activity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.page, Page::Locked | Page::Setup) {
            return;
        }
        let expired = if let Ok(mut vault) = self.vault.try_lock() {
            vault
                .with_vault(|_, workspace| {
                    workspace.check_session()?;
                    workspace.refresh();
                    Ok(())
                })
                .is_err()
        } else {
            false
        };
        if expired {
            self.lock(window, cx);
            self.notice = "Session expired. Unlock to continue.".into();
            cx.stop_propagation();
        }
    }
    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .reveal_until
            .is_some_and(|deadline| SystemTime::now() >= deadline)
        {
            self.reveal_until = None;
            if let Some(field) = self.fields.first() {
                field.update(cx, |field, cx| field.set_masked(true, cx));
            }
        }
        // ponytail: poll one small managed file each second; use an OS watcher if
        // appearance editing ever needs sub-second updates. Parent replacement is detected.
        let modified = std::fs::metadata(self.appearance_directory.join("settings.json"))
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        if modified != self.appearance_modified {
            self.appearance_modified = modified;
            self.reload_appearance(cx);
        }
        if let Some((_, _, start)) = &self.clipboard
            && start
                .elapsed()
                .map_or(true, |elapsed| elapsed >= Duration::from_secs(30))
        {
            self.clear_clipboard(cx);
        }
        let expired = self
            .vault
            .try_lock()
            .ok()
            .and_then(|mut vault| {
                vault
                    .with_vault(|_, workspace| {
                        let had = workspace.is_unlocked();
                        if let Some(id) = workspace.session_id() {
                            workspace.expire_session(id, SystemTime::now());
                        }
                        Ok(had && !workspace.is_unlocked())
                    })
                    .ok()
            })
            .unwrap_or(false);
        if expired {
            self.lock(window, cx);
            self.notice = "Locked after 30 minutes of inactivity".into();
        }
        if self.page == Page::Details {
            cx.notify();
        }
    }
    fn clear_clipboard(&mut self, cx: &mut App) {
        if let Some((text, marker, _)) = self.clipboard.take()
            && cx.read_from_clipboard().is_some_and(|item| {
                item.metadata().map(String::as_str) == Some(marker.as_str())
                    && item.text().as_deref() == Some(text.as_str())
            })
        {
            cx.write_to_clipboard(ClipboardItem::new_string(String::new()));
        }
    }
    fn copy(&mut self, text: Zeroizing<String>, cx: &mut Context<Self>) {
        let marker = uuid::Uuid::new_v4().to_string();
        match crate::clipboard::write(&text, &marker, cx) {
            Ok(()) => {
                self.clipboard = Some((text, marker, SystemTime::now()));
                self.notice = "Copied. Clipboard clears in 30 seconds.".into();
            }
            Err(error) => self.notice = error,
        }
    }
    fn lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_until = None;
        let was_setup = self.page == Page::Setup;
        self.epoch.fetch_add(1, Ordering::SeqCst);
        self.busy = self.installing;
        self.fields.clear();
        self.previews.clear();
        self.detail = None;
        self.editing = None;
        self.issues.clear();
        self.breaches.clear();
        self.providers.clear();
        self.default_provider = None;
        self.draft_alias_provider = None;
        self.generated = Zeroizing::new(String::new());
        self.clear_clipboard(cx);
        let vault = self.vault.clone();
        let generation = self.epoch.load(Ordering::SeqCst);
        let task = cx.background_executor().spawn(async move {
            vault
                .lock()
                .map_err(|_| "Vault service unavailable".to_owned())?
                .with_vault(|storage, workspace| {
                    workspace.lock();
                    if was_setup {
                        storage.inspect().map(|header| header.is_some())
                    } else {
                        Ok(false)
                    }
                })
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = task.await;
            let _ = view.update_in(cx, |this, window, cx| {
                if this.epoch.load(Ordering::SeqCst) != generation || this.page != Page::Setup {
                    return;
                }
                match result {
                    Ok(false) => return,
                    Ok(true) => this.page = Page::Locked,
                    Err(error) => {
                        this.page = Page::Locked;
                        this.blocked = true;
                        this.notice = error;
                    }
                }
                this.auth_fields(window, cx);
                cx.notify();
            });
        })
        .detach();
        self.page = if was_setup { Page::Setup } else { Page::Locked };
        self.notice.clear();
        self.query
            .update(cx, |query, cx| query.set_value(String::new(), cx));
        self.auth_fields(window, cx);
        cx.notify();
    }
    fn selected_id(&self) -> Option<String> {
        self.detail
            .as_ref()
            .map(|entry| entry.id.clone())
            .or_else(|| {
                self.previews
                    .get(self.selected)
                    .map(|entry| entry.id.clone())
            })
    }
    fn open_actions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let filter = self
            .query
            .read(cx)
            .value()
            .strip_prefix('>')
            .unwrap_or("")
            .trim()
            .to_owned();
        self.action_query
            .update(cx, |query, cx| query.set_value(filter, cx));
        self.page = Page::Actions;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn run_command(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        match id {
            "generator" => self.open_generator(window, cx),
            "details" => {
                if let Some(id) = self.selected_id() {
                    self.work(cx, move |vault| {
                        vault.with_vault(|_, workspace| {
                            vault::entries::get_full(workspace, &id).map(Reply::Detail)
                        })
                    });
                }
            }
            "new" => self.add(window, cx),
            "edit" => self.edit(window, cx),
            "username" => self.copy_field("username", cx),
            "totp" => self.copy_field("totp", cx),
            "health" => self.health(cx),
            "settings" => self.settings(window, cx),
            "lock" => self.lock(window, cx),
            _ => {}
        }
    }
    fn copy_field(&mut self, field: &'static str, cx: &mut Context<Self>) {
        let Some(id) = self.selected_id() else {
            return;
        };
        self.work(cx, move |vault| {
            vault.with_vault(|_, workspace| {
                let entry = vault::entries::get_full(workspace, &id)?;
                let text = match field {
                    "username" => entry.username.clone(),
                    "totp" => vault::totp::generate_token(
                        entry.totp_secret.as_deref().ok_or("No TOTP configured")?,
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_err(|_| "Invalid system clock")?
                            .as_secs(),
                    )?,
                    _ => entry.password.clone(),
                };
                Ok(Reply::Copied(Zeroizing::new(text)))
            })
        });
    }
    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.page, Page::Locked | Page::Setup) || self.busy {
            return;
        }
        if self.guard_draft(window, cx) {
            return;
        }
        self.detail = None;
        self.editing = None;
        self.form(None, window, cx);
    }
    fn form(&mut self, entry: Option<Entry>, window: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Edit;
        self.fields.clear();
        self.notice.clear();
        self.draft_alias_provider = entry
            .as_ref()
            .and_then(|entry| entry.alias_provider_id.clone());
        for (label, secret) in [
            ("Title", false),
            ("Username", false),
            ("Password", true),
            ("Website URL (optional)", false),
            ("TOTP secret or otpauth URI (optional)", true),
        ] {
            self.field(label, secret, cx);
        }
        if let Some(entry) = entry {
            self.editing = Some(entry.id.clone());
            for (field, value) in self.fields.iter().zip([
                entry.title.clone(),
                entry.username.clone(),
                entry.password.clone(),
                entry.url.clone().unwrap_or_default(),
                entry.totp_secret.clone().unwrap_or_default(),
            ]) {
                field.update(cx, |field, cx| field.set_value(value, cx));
            }
        }
        self.focus_first(window, cx);
        cx.notify();
    }
    fn edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy
            || matches!(self.page, Page::Locked | Page::Setup)
            || self.guard_draft(window, cx)
        {
            return;
        }
        if let Some(entry) = self.detail.take() {
            self.form(Some(entry), window, cx);
            return;
        }
        if let Some(id) = self.selected_id() {
            let vault = self.vault.clone();
            if let Ok(mut vault) = vault.try_lock() {
                match vault.with_vault(|_, workspace| vault::entries::get_full(workspace, &id)) {
                    Ok(entry) => self.form(Some(entry), window, cx),
                    Err(error) => self.notice = error,
                }
            }
        }
    }
    fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy
            || self.query.read(cx).composing()
            || self.action_query.read(cx).composing()
            || self.fields.iter().any(|field| field.read(cx).composing())
        {
            return;
        }
        match self.page {
            Page::Setup | Page::Locked | Page::Rotate => {
                if self.blocked {
                    return;
                }
                if self.page == Page::Locked && self.biometric {
                    self.work(cx, |vault| {
                        let key = crate::biometric::retrieve()?;
                        vault.access(AuthCredential::RawKeyHex(key.to_string()))?;
                        vault.with_vault(|_, workspace| {
                            vault::search::search(workspace, "").map(Reply::Browse)
                        })
                    });
                    return;
                }
                let value = self.value(0, cx);
                let setup = self.page == Page::Setup;
                let rotate = self.page == Page::Rotate;
                if (setup || rotate)
                    && (!(12..=1024).contains(&value.len())
                        || value.as_str() != self.value(1, cx).as_str())
                {
                    self.notice="Use 12 to 1024 bytes and matching passwords. There is no password recovery.".into();
                    cx.notify();
                    return;
                }
                if value.is_empty() || value.len() > 1024 {
                    self.notice = "Enter your master password".into();
                    cx.notify();
                    return;
                }
                for field in &self.fields {
                    field.update(cx, |field, cx| field.set_value(String::new(), cx));
                }
                self.work(cx, move |vault| {
                    if setup || rotate {
                        let salt = password::generate_salt();
                        let key = Zeroizing::new(password::derive_key(&value, &salt));
                        vault.with_vault(|storage, workspace| {
                            if rotate {
                                vault::rotate::rotate(
                                    storage,
                                    workspace,
                                    &key,
                                    AuthMethod::Password,
                                    &hex::encode(salt),
                                )
                            } else {
                                vault::provision::provision(
                                    storage,
                                    workspace,
                                    &key,
                                    AuthMethod::Password,
                                    &hex::encode(salt),
                                )
                            }
                        })?;
                    } else {
                        vault.access(AuthCredential::Password(value.to_string()))?;
                    }
                    if rotate {
                        Ok(Reply::Rotated)
                    } else {
                        vault.with_vault(|_, workspace| {
                            vault::search::search(workspace, "").map(Reply::Browse)
                        })
                    }
                });
            }
            Page::Edit => {
                let editing = self.editing.is_some();
                let entry = Entry {
                    id: self
                        .editing
                        .clone()
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    title: self.value(0, cx).to_string(),
                    username: self.value(1, cx).to_string(),
                    password: self.value(2, cx).to_string(),
                    url: Some(self.value(3, cx).to_string()).filter(|value| !value.is_empty()),
                    icon_url: None,
                    totp_secret: Some(self.value(4, cx).to_string()),
                    alias_provider_id: self.draft_alias_provider.clone(),
                };
                self.work(cx, move |vault| {
                    vault.with_vault(|storage, workspace| {
                        if editing {
                            vault::entries::update(workspace, storage, entry)?;
                        } else {
                            let mut entry = entry;
                            if entry.totp_secret.as_deref() == Some("") {
                                entry.totp_secret = None;
                            }
                            vault::entries::add(workspace, storage, entry)?;
                        }
                        Ok(Reply::Saved)
                    })
                });
            }
            Page::Browse => {
                if self.query.read(cx).value().starts_with('>') {
                    self.open_actions(window, cx);
                } else {
                    self.copy_field("password", cx);
                }
            }
            Page::Delete => {
                if let Some(id) = self.selected_id() {
                    self.work(cx, move |vault| {
                        vault.with_vault(|storage, workspace| {
                            vault::entries::delete(workspace, storage, &id)?;
                            Ok(Reply::Saved)
                        })
                    });
                }
            }
            Page::Details => self.copy_field("password", cx),
            Page::Actions => {
                let filter = self.action_query.read(cx).value().trim().to_lowercase();
                if let Some((id, _)) = COMMANDS
                    .iter()
                    .find(|(_, label)| label.to_lowercase().contains(&filter))
                {
                    self.run_command(id, window, cx);
                }
            }
            Page::Generator => self.use_generated(window, cx),
            Page::Discard => {
                self.page = Page::Browse;
                self.fields.clear();
                self.editing = None;
                self.draft_alias_provider = None;
                window.focus(&self.query.read(cx).focus_handle(cx), cx);
                self.search(self.query.read(cx).value().to_owned(), cx);
            }
            Page::Provider => {
                let token = self.value(0, cx);
                let description = self.value(1, cx);
                let provider = self.provider.clone();
                self.work(cx, move |vault| {
                    vault.with_vault(|storage, workspace| {
                        vault::alias::save_config(
                            workspace,
                            storage,
                            &provider,
                            &token,
                            Some(&description),
                        )?;
                        Ok(Reply::Providers(
                            vault::alias::list_configs(workspace)?,
                            workspace.default_provider_id.clone(),
                        ))
                    })
                });
            }
            Page::DeleteProvider => {
                let provider = self.provider.clone();
                self.work(cx, move |vault| {
                    vault.with_vault(|storage, workspace| {
                        vault::alias::delete_config(workspace, storage, &provider)?;
                        Ok(Reply::Providers(
                            vault::alias::list_configs(workspace)?,
                            workspace.default_provider_id.clone(),
                        ))
                    })
                });
            }
            _ => {}
        }
        cx.notify();
    }
    fn back(&mut self, _: &Back, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if matches!(self.page, Page::Locked | Page::Setup) {
            cx.quit();
            return;
        }
        if self.page == Page::Generator {
            self.page = self.generator_return.clone();
            self.generated = Zeroizing::new(String::new());
            self.focus_first(window, cx);
            cx.notify();
            return;
        }
        if self.page == Page::Edit
            && self
                .fields
                .iter()
                .any(|field| !field.read(cx).value().is_empty())
        {
            self.page = Page::Discard;
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        if self.page == Page::Discard {
            self.page = Page::Edit;
            self.focus_first(window, cx);
            cx.notify();
            return;
        }
        if matches!(self.page, Page::Preview | Page::Appearance) {
            self.preview = None;
            self.page = Page::Settings;
            cx.notify();
            return;
        }
        self.fields.clear();
        self.detail = None;
        self.editing = None;
        self.notice.clear();
        self.page = Page::Browse;
        window.focus(&self.query.read(cx).focus_handle(cx), cx);
        self.search(self.query.read(cx).value().to_owned(), cx);
        cx.notify();
    }
    fn settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.guard_draft(window, cx) {
            return;
        }
        if !matches!(self.page, Page::Locked | Page::Setup) {
            self.page = Page::Settings;
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }
    fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.notice = "Checking native updates...".into();
        let epoch = self.epoch.load(Ordering::SeqCst);
        let task = cx
            .background_executor()
            .spawn(async { crate::update::check() });
        cx.spawn(async move |view, cx| {
            let result = task.await;
            let _ = view.update(cx, |this, cx| {
                if this.epoch.load(Ordering::SeqCst) != epoch { return; }
                this.busy = false;
                match result {
                    Ok(Some(update)) => { this.notice = format!("Native update {} available. Installation locks the vault and exits Latch.", update.version); this.update = Some(update); },
                    Ok(None) => { this.update = None; this.notice = "You are using the latest published native release.".into(); },
                    Err(error) => this.notice = error,
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn install_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(update) = self.update.take() else {
            return;
        };
        self.installing = true;
        self.lock(window, cx);
        self.busy = true;
        self.notice = "Downloading and verifying the signed update...".into();
        let vault = self.vault.clone();
        let task = cx.background_executor().spawn(async move {
            // The installer's exit path must never bypass secret cleanup.
            vault
                .lock()
                .map_err(|_| "Vault unavailable".to_string())?
                .with_vault(|_, workspace| {
                    workspace.lock();
                    Ok(())
                })?;
            update
                .download_and_install()
                .map_err(|error| format!("Update rejected or installation failed: {error}"))
        });
        cx.spawn(async move |view, cx| {
            let result = task.await;
            let _ = view.update(cx, |this, cx| {
                this.installing = false;
                this.busy = false;
                match result {
                    Ok(()) => cx.quit(),
                    Err(error) => this.notice = error,
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn guard_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.page == Page::Discard {
            return true;
        }
        if self.page == Page::Edit
            && self
                .fields
                .iter()
                .any(|field| !field.read(cx).value().is_empty())
        {
            self.page = Page::Discard;
            window.focus(&self.focus, cx);
            cx.notify();
            return true;
        }
        if matches!(self.page, Page::Provider | Page::Rotate | Page::Generator) {
            self.notice = "Finish this form or press Escape before switching pages.".into();
            cx.notify();
            return true;
        }
        false
    }
    fn health(&mut self, cx: &mut Context<Self>) {
        self.work(cx, |vault| {
            vault.with_vault(|_, workspace| {
                workspace.check_session()?;
                workspace.refresh();
                let mut issues: Vec<_> =
                    latch_core::vault_health::audit::check_weak_passwords(&workspace.credentials)
                        .into_iter()
                        .map(|entry| (entry.entry_id, format!("{}: {}", entry.title, entry.label)))
                        .collect();
                for group in
                    latch_core::vault_health::audit::check_reused_passwords(&workspace.credentials)
                {
                    for entry in group.entries {
                        issues.push((entry.entry_id, format!("{}: Reused password", entry.title)));
                    }
                }
                Ok(Reply::Health(issues))
            })
        });
    }
    fn check_breaches(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.notice = "Checking password hash prefixes with Have I Been Pwned...".into();
        let vault = self.vault.clone();
        let epoch = self.epoch.load(Ordering::SeqCst);
        let task = cx.background_executor().spawn(async move {
            let (entries,mut cancel) = vault.lock().map_err(|_|"Vault unavailable")?.with_vault(|_,workspace| {
                workspace.check_session()?;
                workspace.refresh();
                Ok((workspace.credentials.clone(),workspace.alias_cancel_receiver()))
            })?;
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"Network runtime unavailable")?;
            runtime.block_on(async move {
                let checker = latch_core::vault_health::breach_checker::PwnedPasswordsApi;
                tokio::select! {
                    report = latch_core::vault_health::audit::check_breach_status(&entries,&checker) => Ok(report),
                    _ = cancel.changed() => Err("Vault locked; breach check canceled".to_string()),
                }
            })
        });
        cx.spawn(async move |view,cx| {
            let result = task.await;
            view.update(cx,|this,cx| {
                if this.epoch.load(Ordering::SeqCst) != epoch { return; }
                this.busy = false;
                match result {
                    Ok((breached,unavailable)) => {
                        this.breaches = breached.into_iter().map(|entry| (entry.entry_id,format!("{}: found in {} breaches",entry.title,entry.breach_count))).collect();
                        for id in &unavailable { this.breaches.push((id.clone(), "Breach status unavailable; result unknown".into())); }
                        this.notice = if unavailable.is_empty() { "Breach check completed".into() } else { format!("Breach check unavailable for {} credentials. Their status is unknown.",unavailable.len()) };
                    },
                    Err(error) => this.notice = error,
                }
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
    }
    fn open_providers(&mut self, cx: &mut Context<Self>) {
        self.work(cx, |vault| {
            vault.with_vault(|_, workspace| {
                Ok(Reply::Providers(
                    vault::alias::list_configs(workspace)?,
                    workspace.default_provider_id.clone(),
                ))
            })
        });
    }
    fn configure_provider(&mut self, provider: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.provider = provider.into();
        self.page = Page::Provider;
        self.fields.clear();
        self.field("API token", true, cx);
        self.field("Description (optional)", false, cx);
        self.focus_first(window, cx);
        cx.notify();
    }
    fn generate_alias(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.page != Page::Edit {
            return;
        }
        self.busy = true;
        let vault = self.vault.clone();
        let epoch = self.epoch.load(Ordering::SeqCst);
        let task = cx.background_executor().spawn(async move {
            let (config,mut cancel) = vault.lock().map_err(|_|"Vault unavailable")?.with_vault(|_,workspace|{
                workspace.check_session()?;workspace.refresh();
                let provider=workspace.default_provider_id.as_deref().or_else(||workspace.alias_configs.first().map(|config|config.provider_id.as_str())).ok_or("Configure an alias provider in Settings first")?;
                let config=workspace.alias_configs.iter().find(|config|config.provider_id==provider).cloned().ok_or("Default alias provider is unavailable")?;
                Ok((config,workspace.alias_cancel_receiver()))
            })?;
            let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"Network runtime unavailable")?;
            runtime.block_on(async move {
                let client=vault::alias::AliasClient::new()?;
                let email=tokio::select! { result=client.generate(&config)=>result?, _=cancel.changed()=>return Err("Vault locked; alias request canceled".to_string()) };
                Ok((email,config.provider_id.clone()))
            })
        });
        cx.spawn(async move |view, cx| {
            let result = task.await;
            view.update(cx, |this, cx| {
                if this.epoch.load(Ordering::SeqCst) != epoch {
                    return;
                }
                this.busy = false;
                match result {
                    Ok((email, provider)) if this.page == Page::Edit => {
                        this.fields[1].update(cx, |field, cx| field.set_value(email, cx));
                        this.draft_alias_provider = Some(provider);
                        this.notice = "Alias added to your draft".into();
                    }
                    Ok(_) => {}
                    Err(error) => this.notice = error,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
    fn regenerate(&mut self, cx: &mut Context<Self>) {
        match password_generator::generate_password(&self.generator_options) {
            Ok(value) => self.generated = Zeroizing::new(value),
            Err(error) => {
                self.generated = Zeroizing::new(String::new());
                self.notice = error;
            }
        }
        cx.notify();
    }
    fn open_generator(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.generator_return = self.page.clone();
        self.page = Page::Generator;
        self.regenerate(cx);
        window.focus(&self.focus, cx);
    }
    fn use_generated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.generated.is_empty() {
            return;
        }
        let password = self.generated.to_string();
        if self.generator_return != Page::Edit {
            self.editing = None;
            self.form(None, window, cx);
        } else {
            self.page = Page::Edit;
            self.focus_first(window, cx);
        }
        self.fields[2].update(cx, |field, cx| field.set_value(password, cx));
        self.generated = Zeroizing::new(String::new());
        cx.notify();
    }
    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = cx.global::<Palette>();
        let border = palette.color(Color::Border);
        let hover = palette.color(Color::ButtonHover);
        let focus = palette.color(Color::Focus);
        gpui_kit::base::Button::new(id)
            .accessibility_label(label)
            .disabled(self.busy)
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(palette.color(Color::Button))
            .text_color(palette.color(Color::ButtonText))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focus(move |style| style.border_color(focus))
            .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
            .child(label)
    }
}
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Back, Some("Latch")),
        KeyBinding::new("ctrl-alt-r", ResetAppearance, Some("Latch")),
        KeyBinding::new("cmd-alt-r", ResetAppearance, Some("Latch")),
        KeyBinding::new("enter", Submit, Some("Latch")),
        KeyBinding::new("down", Next, Some("Latch")),
        KeyBinding::new("up", Previous, Some("Latch")),
        KeyBinding::new("tab", Tab, Some("Latch")),
        KeyBinding::new("shift-tab", TabPrevious, Some("Latch")),
        KeyBinding::new("ctrl-n", Add, Some("Latch")),
        KeyBinding::new("cmd-n", Add, Some("Latch")),
        KeyBinding::new("ctrl-e", Edit, Some("Latch")),
        KeyBinding::new("cmd-e", Edit, Some("Latch")),
        KeyBinding::new("ctrl-l", Lock, Some("Latch")),
        KeyBinding::new("cmd-l", Lock, Some("Latch")),
        KeyBinding::new("ctrl-k", Actions, Some("Latch")),
        KeyBinding::new("cmd-k", Actions, Some("Latch")),
        KeyBinding::new("ctrl-,", Settings, Some("Latch")),
        KeyBinding::new("cmd-,", Settings, Some("Latch")),
        KeyBinding::new("ctrl-shift-c", CopyUsername, Some("Latch")),
        KeyBinding::new("cmd-shift-c", CopyUsername, Some("Latch")),
        KeyBinding::new("ctrl-t", CopyTotp, Some("Latch")),
        KeyBinding::new("cmd-t", CopyTotp, Some("Latch")),
        KeyBinding::new("ctrl-q", Quit, Some("Latch")),
        KeyBinding::new("cmd-q", Quit, Some("Latch")),
    ]);
}

#[cfg(test)]
mod tests {
    use super::{Latch, Page, Submit};
    use gpui_kit::{Focusable, TestAppContext};
    use latch_core::vault::storage::VaultStorage;

    #[gpui_kit::test]
    fn lock_after_setup_commits_can_unlock_the_created_vault(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| {
            Latch::new(
                VaultStorage {
                    path: directory.path().join("vault.enc"),
                },
                window,
                cx,
            )
        });
        window
            .update(cx, |this, window, cx| {
                // The storage commit can finish before the UI receives its setup reply.
                let salt = latch_core::auth::password::generate_salt();
                let key =
                    latch_core::auth::password::derive_key("a long test master password", &salt);
                this.vault
                    .lock()
                    .unwrap()
                    .with_vault(|storage, workspace| {
                        latch_core::vault::provision::provision(
                            storage,
                            workspace,
                            &key,
                            latch_core::auth::method::AuthMethod::Password,
                            &hex::encode(salt),
                        )
                    })
                    .unwrap();
                this.lock(window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, window, cx| {
                assert!(this.page == Page::Locked, "{:?}", this.page);
                assert_eq!(this.fields.len(), 1);
                this.fields[0].update(cx, |field, cx| {
                    field.set_value("a long test master password".into(), cx)
                });
                this.submit(&Submit, window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, _, _| {
                assert!(this.page == Page::Browse, "{}", this.notice)
            })
            .unwrap();
    }

    #[gpui_kit::test]
    fn keyboard_can_submit_auth_and_activate_an_action_button(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            super::init(cx);
            crate::input::init(cx);
        });
        let (view, cx) = cx.add_window_view(|window, cx| {
            Latch::new(
                VaultStorage {
                    path: directory.path().join("vault.enc"),
                },
                window,
                cx,
            )
        });
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                for field in &this.fields {
                    field.update(cx, |field, cx| {
                        field.set_value("a long test master password".into(), cx)
                    });
                }
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                view.read(cx).page == Page::Browse,
                "{}",
                view.read(cx).notice
            );
            view.read(cx)
                .vault
                .lock()
                .unwrap()
                .with_vault(|_, workspace| {
                    workspace.session_start =
                        Some(std::time::SystemTime::now() - std::time::Duration::from_secs(60));
                    Ok(())
                })
                .unwrap();
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.update(|_, cx| {
            view.read(cx)
                .vault
                .lock()
                .unwrap()
                .with_vault(|_, workspace| {
                    assert!(
                        workspace.session_start.unwrap().elapsed().unwrap()
                            < std::time::Duration::from_secs(5)
                    );
                    Ok(())
                })
                .unwrap();
        });
        cx.simulate_keystrokes("ctrl-k");
        cx.update(|window, cx| {
            assert!(view.read(cx).page == Page::Actions);
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(
                view.read(cx).page == Page::Generator,
                "{:?}",
                view.read(cx).page
            );
        });
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.back(&super::Back, window, cx);
                this.back(&super::Back, window, cx);
            });
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.query
                    .update(cx, |query, cx| query.set_value(">health".into(), cx));
                this.submit(&Submit, window, cx);
                window.focus(&this.action_query.read(cx).focus_handle(cx), cx);
            });
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!(
                view.read(cx).page == Page::Health,
                "{:?}",
                view.read(cx).page
            );
        });
    }

    #[gpui_kit::test]
    fn native_vault_flow_persists_credentials_and_lock_clears_drafts(cx: &mut TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vault.enc");
        cx.update(|cx| {
            gpui_kit::init(cx);
            super::init(cx);
            crate::input::init(cx);
        });
        let window =
            cx.add_window(|window, cx| Latch::new(VaultStorage { path: path.clone() }, window, cx));
        window
            .update(cx, |this, window, cx| {
                for field in &this.fields {
                    field.update(cx, |field, cx| {
                        field.set_value("a long test master password".into(), cx)
                    });
                }
                this.submit(&Submit, window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, window, cx| {
                assert!(this.page == Page::Browse, "{}", this.notice);
                this.add(window, cx);
                for (field, value) in this.fields.iter().zip([
                    "Example",
                    "person@example.com",
                    "a different credential secret",
                    "https://example.com",
                    "",
                ]) {
                    field.update(cx, |field, cx| field.set_value(value.into(), cx));
                }
                this.submit(&Submit, window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, window, cx| {
                assert!(this.page == Page::Browse, "{}", this.notice);
                assert_eq!(this.previews.len(), 1);
                this.copy_field("password", cx);
                window.focus(&this.focus, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, window, cx| {
                assert_eq!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .as_deref(),
                    Some("a different credential secret")
                );
                this.edit(window, cx);
                assert_eq!(this.value(2, cx).as_str(), "a different credential secret");
                this.settings(window, cx);
                assert!(this.page == Page::Discard);
                assert_eq!(this.value(2, cx).as_str(), "a different credential secret");
                this.back(&super::Back, window, cx);
                assert!(this.page == Page::Edit);
                this.lock(window, cx);
                assert!(this.detail.is_none());
                assert!(this.previews.is_empty());
                assert!(
                    this.fields
                        .iter()
                        .all(|field| field.read(cx).value().is_empty())
                );
                assert!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .is_none_or(|value| value.is_empty())
                );
                this.copy(zeroize::Zeroizing::new("user copied text".into()), cx);
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                    "user copied text".into(),
                ));
                this.lock(window, cx);
                assert_eq!(
                    cx.read_from_clipboard()
                        .and_then(|item| item.text())
                        .as_deref(),
                    Some("user copied text")
                );
                this.fields[0].update(cx, |field, cx| {
                    field.set_value("a long test master password".into(), cx)
                });
                this.submit(&Submit, window, cx);
                this.lock(window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        window
            .update(cx, |this, _, _| {
                assert!(this.page == Page::Locked);
                assert!(this.previews.is_empty());
                assert!(
                    !this
                        .vault
                        .lock()
                        .unwrap()
                        .with_vault(|_, workspace| Ok(workspace.is_unlocked()))
                        .unwrap()
                );
            })
            .unwrap();
        assert!(
            !std::fs::read_to_string(path)
                .unwrap()
                .contains("credential secret")
        );
    }
}
