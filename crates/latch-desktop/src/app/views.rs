use super::*;

impl Render for Latch {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let palette = self
            .preview
            .as_ref()
            .filter(|_| self.page == Page::Preview)
            .or(self.appearance.as_ref())
            .and_then(|appearance| appearance.palette(dark).ok().map(|(palette, _)| palette))
            .unwrap_or_else(|| Palette::fallback(dark));
        cx.set_global(palette.clone());
        if self.page == Page::Browse && self.focus.is_focused(window) {
            window.focus(&self.query.read(cx).focus_handle(cx), cx);
        }
        let heading = match self.page {
            Page::Setup => "Create your vault",
            Page::Locked => "Unlock Latch",
            Page::Browse => "Latch",
            Page::Details => "Credential",
            Page::Edit => {
                if self.editing.is_some() {
                    "Edit credential"
                } else {
                    "New credential"
                }
            }
            Page::Delete => "Delete credential?",
            Page::Settings => "Settings",
            Page::Rotate => "Change master password",
            Page::Health => "Password health",
            Page::Actions => "Actions",
            Page::Appearance => "Appearance",
            Page::Preview => "Appearance preview",
            Page::Providers => "Email alias providers",
            Page::Provider => "Configure alias provider",
            Page::DeleteProvider => "Remove alias provider?",
            Page::Generator => "Generate password",
            Page::Discard => "Discard credential draft?",
        };
        let mut body = div()
            .id("body")
            .flex()
            .flex_col()
            .gap_3()
            .flex_1()
            .min_h_0()
            .track_scroll(&self.scroll)
            .overflow_y_scroll();
        if matches!(
            self.page,
            Page::Settings | Page::Providers | Page::Health | Page::Appearance | Page::Preview
        ) {
            body = body.bg(palette.color(Color::Surface)).p_3();
        } else if self.page == Page::Actions {
            body = body.bg(palette.color(Color::Popup)).p_3();
        }
        match self.page {
            Page::Setup | Page::Locked | Page::Rotate => {
                body=body.child(div().text_color(palette.color(Color::Muted)).child(if self.page==Page::Setup{"Private, encrypted, stored on this device. Choose a password you will remember."}else{"Your credentials stay encrypted until you unlock."}));
                if !self.blocked {
                    for field in &self.fields {
                        body = body.child(
                            div()
                                .rounded_md()
                                .border_1()
                                .border_color(palette.color(Color::Border))
                                .child(field.clone()),
                        );
                    }
                    body = body.child(self.button(
                        "unlock",
                        if self.page == Page::Setup {
                            "Create vault"
                        } else if self.page == Page::Rotate {
                            "Change password"
                        } else {
                            "Unlock"
                        },
                        |this, window, cx| this.submit(&Submit, window, cx),
                        cx,
                    ));
                }
            }
            Page::Browse => {
                body = body.child(
                    div()
                        .border_b_1()
                        .border_color(palette.color(Color::Border))
                        .child(self.query.clone()),
                );
                if self.query.read(cx).value().starts_with('>') {
                    body = body.child(self.button(
                        "commands",
                        "Open commands",
                        |this, window, cx| {
                            this.open_actions(window, cx);
                        },
                        cx,
                    ));
                } else if self.previews.is_empty() {
                    body = body.child(div().py_6().text_color(palette.color(Color::Muted)).child(
                        if self.query.read(cx).value().is_empty() {
                            "Your vault is empty. Add your first credential with Ctrl+N."
                        } else {
                            "No matching credentials"
                        },
                    ));
                } else {
                    for (index, entry) in self.previews.iter().enumerate() {
                        body = body.child(
                            div()
                                .id(("entry", index))
                                .role(Role::ListBoxOption)
                                .aria_label(format!("{}, {}", entry.title, entry.username))
                                .aria_selected(index == self.selected)
                                .flex()
                                .flex_col()
                                .gap_1()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .when(index == self.selected, |row| {
                                    row.bg(palette.color(Color::Selected))
                                        .text_color(palette.color(Color::SelectedText))
                                })
                                .cursor_pointer()
                                .hover({
                                    let selected = index == self.selected;
                                    let palette = palette.clone();
                                    move |style| {
                                        if selected {
                                            style
                                                .bg(palette.color(Color::Selected))
                                                .text_color(palette.color(Color::SelectedText))
                                        } else {
                                            style
                                                .bg(palette.color(Color::Hover))
                                                .text_color(palette.color(Color::HoverText))
                                        }
                                    }
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.selected = index;
                                    this.submit(&Submit, window, cx);
                                }))
                                .child(entry.title.clone())
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(palette.color(if index == self.selected {
                                            Color::SelectedText
                                        } else {
                                            Color::Muted
                                        }))
                                        .child(entry.username.clone()),
                                ),
                        );
                    }
                }
            }
            Page::Edit => {
                for field in &self.fields {
                    body = body.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(palette.color(Color::Muted))
                                    .child(field.read(cx).label()),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(palette.color(Color::Border))
                                    .child(field.clone()),
                            ),
                    );
                }
                body = body
                    .child(self.button(
                        "alias",
                        "Generate email alias",
                        |this, _, cx| this.generate_alias(cx),
                        cx,
                    ))
                    .child(self.button(
                        "generate",
                        "Generate password",
                        |this, window, cx| this.open_generator(window, cx),
                        cx,
                    ))
                    .child(self.button(
                        "save",
                        "Save credential",
                        |this, window, cx| this.submit(&Submit, window, cx),
                        cx,
                    ));
            }
            Page::Details => {
                if let Some(field) = self.fields.first() {
                    body = body.child(field.clone());
                }
                body = body.child(self.button(
                    "reveal",
                    "Reveal password for 10 seconds",
                    |this, _, cx| {
                        this.reveal_until = Some(SystemTime::now() + Duration::from_secs(10));
                        if let Some(field) = this.fields.first() {
                            field.update(cx, |field, cx| field.set_masked(false, cx));
                        }
                    },
                    cx,
                ));
                if let Some(entry) = &self.detail {
                    body = body
                        .child(div().text_xl().child(entry.title.clone()))
                        .child(entry.username.clone())
                        .child(if self.reveal_until.is_some() {
                            "Password revealed temporarily. Enter copies it."
                        } else {
                            "Password hidden. Enter copies it."
                        });
                    if let Some(url) = &entry.url {
                        let url = url.clone();
                        body = body.child(url.clone());
                        body = body.child(self.button(
                            "open-url",
                            "Open website",
                            move |this, _, cx| match url::Url::parse(&url) {
                                Ok(parsed)
                                    if matches!(parsed.scheme(), "http" | "https")
                                        && parsed.host_str().is_some() =>
                                {
                                    cx.open_url(parsed.as_str())
                                }
                                _ => {
                                    this.notice =
                                        "Only HTTP and HTTPS website links can be opened.".into();
                                    cx.notify();
                                }
                            },
                            cx,
                        ));
                    }
                    if let Some(secret) = &entry.totp_secret {
                        let time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        body = body.child(format!(
                            "TOTP {} / {} seconds",
                            vault::totp::generate_token(secret, time)
                                .unwrap_or_else(|_| "Unavailable".into()),
                            vault::totp::remaining_seconds(time)
                        ));
                    }
                }
                body = body
                    .child(self.button(
                        "copy",
                        "Copy password",
                        |this, _, cx| this.copy_field("password", cx),
                        cx,
                    ))
                    .child(self.button(
                        "username",
                        "Copy username",
                        |this, _, cx| this.copy_field("username", cx),
                        cx,
                    ))
                    .child(self.button(
                        "edit",
                        "Edit credential",
                        |this, window, cx| this.edit(window, cx),
                        cx,
                    ))
                    .child(self.button(
                        "delete",
                        "Delete credential",
                        |this, window, cx| {
                            this.page = Page::Delete;
                            window.focus(&this.focus, cx);
                            cx.notify();
                        },
                        cx,
                    ));
            }
            Page::Delete => {
                body = body
                    .child("This permanently removes the selected credential.")
                    .child(self.button(
                        "confirm-delete",
                        "Delete permanently",
                        |this, window, cx| this.submit(&Submit, window, cx),
                        cx,
                    ))
                    .child(self.button(
                        "cancel-delete",
                        "Cancel",
                        |this, window, cx| this.back(&Back, window, cx),
                        cx,
                    ));
            }
            Page::Actions => {
                body = body.child(self.action_query.clone());
                let filter = self.action_query.read(cx).value().trim().to_lowercase();
                let mut found = false;
                for (id, label) in COMMANDS {
                    if label.to_lowercase().contains(&filter) {
                        found = true;
                        body = body.child(self.button(
                            id,
                            label,
                            move |this, window, cx| this.run_command(id, window, cx),
                            cx,
                        ));
                    }
                }
                if !found {
                    body = body.child("No matching actions");
                }
            }
            Page::Appearance | Page::Preview => {
                body=body.child("Import explicit color overrides from existing editor settings. Unrelated settings are discarded.")
                    .child(self.button("import-vscode","Import VS Code settings.json",|this,_,cx|this.import_appearance(Source::VsCode,cx),cx))
                    .child(self.button("import-zed","Import Zed settings.json",|this,_,cx|this.import_appearance(Source::Zed,cx),cx));
                if self.page == Page::Preview {
                    if palette.low_contrast() {
                        body = body.child("Low contrast detected. Some text may be difficult to read. You can cancel or reset appearance.");
                    }
                    body = body
                        .child(
                            div()
                                .p_3()
                                .bg(palette.color(Color::Selected))
                                .text_color(palette.color(Color::SelectedText))
                                .child("Selected credential preview"),
                        )
                        .child(
                            div()
                                .text_color(palette.color(Color::Error))
                                .child("Error: example validation message"),
                        )
                        .child(self.button(
                            "apply-appearance",
                            "Apply appearance",
                            |this, _, cx| {
                                if let Some(preview) = this.preview.take() {
                                    match preview.save(&this.appearance_directory) {
                                        Ok(()) => {
                                            this.appearance = Some(preview);
                                            this.page = Page::Appearance;
                                            this.notice = "Appearance saved".into();
                                        }
                                        Err(error) => {
                                            this.preview = Some(preview);
                                            this.notice = error
                                        }
                                    }
                                }
                                cx.notify();
                            },
                            cx,
                        ));
                }
                body = body
                    .child(self.button(
                        "open-settings",
                        "Open appearance settings.json",
                        |this, _, cx| {
                            let path = this.appearance_directory.join("settings.json");
                            if path.exists() {
                                match url::Url::from_file_path(&path) {
                                    Ok(url) => cx.open_url(url.as_str()),
                                    Err(_) => {
                                        this.notice = "Cannot open settings path".into();
                                        cx.notify();
                                    }
                                }
                            } else {
                                this.notice =
                                    "Import editor settings before opening the managed file."
                                        .into();
                                cx.notify();
                            }
                        },
                        cx,
                    ))
                    .child(self.button(
                        "reload-appearance",
                        "Reload appearance",
                        |this, _, cx| this.reload_appearance(cx),
                        cx,
                    ))
                    .child(self.button(
                        "reset-appearance",
                        "Reset appearance",
                        |this, _, cx| this.reset_appearance(cx),
                        cx,
                    ));
            }
            Page::Settings => {
                body = body.child(self.button(
                    "check-updates",
                    "Check native updates",
                    |this, _, cx| this.check_updates(cx),
                    cx,
                ));
                if self.update.is_some() {
                    body = body.child(self.button(
                        "install-update",
                        "Lock vault, install verified update and exit",
                        |this, window, cx| this.install_update(window, cx),
                        cx,
                    ));
                }
                if let Some(integration) = cx.try_global::<crate::integration::Integration>() {
                    body = body.child(integration.status.clone());
                }
                body = body
                    .child(self.button(
                        "providers",
                        "Email alias providers",
                        |this, _, cx| this.open_providers(cx),
                        cx,
                    ))
                    .child(self.button(
                        "appearance",
                        "Appearance settings",
                        |this, window, cx| {
                            this.page = Page::Appearance;
                            window.focus(&this.focus, cx);
                            cx.notify();
                        },
                        cx,
                    ))
                    .child(format!("Latch {} / Native GPUI", env!("CARGO_PKG_VERSION")))
                    .child("Vault locks after 30 minutes of inactivity.")
                    .child(self.button(
                        "rotate",
                        "Change master password",
                        |this, window, cx| {
                            this.page = Page::Rotate;
                            this.fields.clear();
                            this.field("New master password", true, cx);
                            this.field("Confirm master password", true, cx);
                            this.focus_first(window, cx);
                            cx.notify();
                        },
                        cx,
                    ))
                    .child(self.button(
                        "health",
                        "Password health",
                        |this, _, cx| this.health(cx),
                        cx,
                    ))
                    .child(self.button(
                        "lock",
                        "Lock vault",
                        |this, window, cx| this.lock(window, cx),
                        cx,
                    ));
                if crate::biometric::supported() && !self.biometric {
                    body = body.child(self.button(
                        "device-key",
                        "Switch to device key (replaces master password)",
                        |this, _, cx| {
                            this.work(cx, |vault| {
                                let encoded = crate::biometric::create_or_retrieve()?;
                                let decoded = Zeroizing::new(
                                    hex::decode(encoded.as_str())
                                        .map_err(|_| "Invalid device key")?,
                                );
                                let key = Zeroizing::new(
                                    <[u8; 32]>::try_from(decoded.as_slice())
                                        .map_err(|_| "Invalid device key")?,
                                );
                                vault.with_vault(|storage, workspace| {
                                    vault::rotate::rotate(
                                        storage,
                                        workspace,
                                        &key,
                                        AuthMethod::Biometric,
                                        "",
                                    )?;
                                    Ok(Reply::Biometric)
                                })
                            });
                        },
                        cx,
                    ));
                }
            }
            Page::Health => {
                body=body.child("Local checks for weak and reused passwords. Breach checking sends only SHA-1 hash prefixes.")
                    .child(self.button("breach-check","Check breached passwords",|this,_,cx|this.check_breaches(cx),cx))
                    .child(self.button("refresh-health","Refresh local checks",|this,_,cx|this.health(cx),cx));
                if self.issues.is_empty() && self.breaches.is_empty() {
                    body = body.child("No weak or reused passwords found.");
                }
                for (index, (id, label)) in
                    self.issues.iter().chain(self.breaches.iter()).enumerate()
                {
                    let id = id.clone();
                    body = body.child(
                        gpui_kit::base::Button::new(("issue", index))
                            .accessibility_label(label.clone())
                            .disabled(self.busy)
                            .tab_stop(true)
                            .p_3()
                            .rounded_md()
                            .bg(palette.color(Color::Hover))
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let id = id.clone();
                                this.work(cx, move |vault| {
                                    vault.with_vault(|_, workspace| {
                                        vault::entries::get_full(workspace, &id).map(Reply::Detail)
                                    })
                                });
                            }))
                            .child(label.clone()),
                    );
                }
            }
            Page::Providers => {
                body = body
                    .child("API tokens stay inside your encrypted vault.")
                    .child(self.button(
                        "simplelogin",
                        "Configure SimpleLogin",
                        |this, window, cx| this.configure_provider("simplelogin", window, cx),
                        cx,
                    ))
                    .child(self.button(
                        "duckduckgo",
                        "Configure DuckDuckGo",
                        |this, window, cx| this.configure_provider("duckduckgo", window, cx),
                        cx,
                    ));
                for (index, provider) in self.providers.iter().enumerate() {
                    let id = provider.provider_id.clone();
                    let remove = id.clone();
                    let default = self.default_provider.as_deref() == Some(id.as_str());
                    body = body.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(format!("{}{}", id, if default { " (default)" } else { "" }))
                            .child(
                                gpui_kit::base::Button::new(("default-provider", index))
                                    .disabled(self.busy)
                                    .p_2()
                                    .bg(palette.color(Color::Button))
                                    .child("Use as default")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let id = id.clone();
                                        this.work(cx, move |vault| {
                                            vault.with_vault(|storage, workspace| {
                                                vault::alias::set_default_config(
                                                    workspace, storage, &id,
                                                )?;
                                                Ok(Reply::Providers(
                                                    vault::alias::list_configs(workspace)?,
                                                    workspace.default_provider_id.clone(),
                                                ))
                                            })
                                        });
                                    })),
                            )
                            .child(
                                gpui_kit::base::Button::new(("remove-provider", index))
                                    .disabled(self.busy)
                                    .p_2()
                                    .bg(palette.color(Color::Button))
                                    .child("Remove provider")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.provider = remove.clone();
                                        this.page = Page::DeleteProvider;
                                        window.focus(&this.focus, cx);
                                        cx.notify();
                                    })),
                            ),
                    );
                }
            }
            Page::Provider => {
                body = body.child(self.provider.clone());
                for field in &self.fields {
                    body = body.child(
                        div()
                            .border_1()
                            .border_color(palette.color(Color::Border))
                            .child(field.clone()),
                    );
                }
                body = body.child(self.button(
                    "save-provider",
                    "Save encrypted provider token",
                    |this, window, cx| this.submit(&Submit, window, cx),
                    cx,
                ));
            }
            Page::DeleteProvider => {
                body=body.child("Removing the provider deletes its stored API token. Existing aliases remain valid.")
                    .child(self.button("confirm-remove-provider","Remove provider",|this,window,cx|this.submit(&Submit,window,cx),cx));
            }
            Page::Generator => {
                let options = &self.generator_options;
                body = body
                    .child(format!("{} characters", options.length))
                    .child("Password hidden until explicitly copied or used.")
                    .child(format!(
                        "Strength: {}",
                        password_generator::analyze_password_strength(&self.generated).label
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(self.button(
                                "shorter",
                                "Shorter",
                                |this, _, cx| {
                                    this.generator_options.length =
                                        this.generator_options.length.saturating_sub(1).max(8);
                                    this.regenerate(cx);
                                },
                                cx,
                            ))
                            .child(self.button(
                                "longer",
                                "Longer",
                                |this, _, cx| {
                                    this.generator_options.length =
                                        (this.generator_options.length + 1).min(128);
                                    this.regenerate(cx);
                                },
                                cx,
                            )),
                    );
                for (index, (label, enabled)) in [
                    ("Uppercase", options.uppercase),
                    ("Lowercase", options.lowercase),
                    ("Numbers", options.numbers),
                    ("Symbols", options.symbols),
                    ("Exclude ambiguous", options.exclude_ambiguous),
                ]
                .into_iter()
                .enumerate()
                {
                    body = body.child(
                        gpui_kit::base::Button::new(("generator-option", index))
                            .p_2()
                            .bg(palette.color(Color::Button))
                            .child(format!("{}: {}", label, if enabled { "on" } else { "off" }))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let options = &mut this.generator_options;
                                match index {
                                    0 => options.uppercase = !options.uppercase,
                                    1 => options.lowercase = !options.lowercase,
                                    2 => options.numbers = !options.numbers,
                                    3 => options.symbols = !options.symbols,
                                    _ => options.exclude_ambiguous = !options.exclude_ambiguous,
                                };
                                this.regenerate(cx);
                            })),
                    );
                }
                body = body
                    .child(self.button(
                        "regenerate",
                        "Regenerate",
                        |this, _, cx| this.regenerate(cx),
                        cx,
                    ))
                    .child(self.button(
                        "copy-generated",
                        "Copy generated password",
                        |this, _, cx| this.copy(Zeroizing::new(this.generated.to_string()), cx),
                        cx,
                    ))
                    .child(self.button(
                        "use-generated",
                        "Use password in credential",
                        |this, window, cx| this.use_generated(window, cx),
                        cx,
                    ));
            }
            Page::Discard => {
                body = body
                    .child("Your unsaved credential changes will be lost.")
                    .child(self.button(
                        "discard",
                        "Discard changes",
                        |this, window, cx| this.submit(&Submit, window, cx),
                        cx,
                    ))
                    .child(self.button(
                        "keep-editing",
                        "Keep editing",
                        |this, window, cx| this.back(&Back, window, cx),
                        cx,
                    ));
            }
        }
        div()
            .id("latch")
            .role(Role::Application)
            .aria_label("Latch password manager")
            .key_context("Latch")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                this.clear_clipboard(cx);
                cx.quit();
            }))
            .on_action(cx.listener(Self::back))
            .on_action(cx.listener(|this, _: &ResetAppearance, _, cx| this.reset_appearance(cx)))
            .on_action(cx.listener(|this, action: &Submit, window, cx| {
                if this.focus.is_focused(window)
                    || this.query.read(cx).focus_handle(cx).is_focused(window)
                    || this
                        .action_query
                        .read(cx)
                        .focus_handle(cx)
                        .is_focused(window)
                    || this
                        .fields
                        .iter()
                        .any(|field| field.read(cx).focus_handle(cx).is_focused(window))
                {
                    this.submit(action, window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .on_action(cx.listener(|this, _: &Add, window, cx| this.add(window, cx)))
            .on_action(cx.listener(|this, _: &Edit, window, cx| this.edit(window, cx)))
            .on_action(cx.listener(|this, _: &Lock, window, cx| this.lock(window, cx)))
            .on_action(cx.listener(|this, _: &Settings, window, cx| this.settings(window, cx)))
            .on_action(cx.listener(|this, _: &Actions, window, cx| {
                if !matches!(this.page, Page::Locked | Page::Setup)
                    && !this.busy
                    && !this.guard_draft(window, cx)
                {
                    this.open_actions(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CopyUsername, _, cx| this.copy_field("username", cx)))
            .on_action(cx.listener(|this, _: &CopyTotp, _, cx| this.copy_field("totp", cx)))
            .on_action(cx.listener(|this, _: &Next, window, cx| {
                if this.page == Page::Actions {
                    window.focus_next(cx);
                    return;
                }
                if this.page != Page::Browse {
                    return;
                }
                this.selected = (this.selected + 1).min(this.previews.len().saturating_sub(1));
                this.scroll.scroll_to_item(this.selected + 1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Previous, window, cx| {
                if this.page == Page::Actions {
                    window.focus_prev(cx);
                    return;
                }
                if this.page != Page::Browse {
                    return;
                }
                this.selected = this.selected.saturating_sub(1);
                this.scroll.scroll_to_item(this.selected + 1);
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &Tab, window, cx| window.focus_next(cx)))
            .on_action(cx.listener(|_, _: &TabPrevious, window, cx| window.focus_prev(cx)))
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_3()
            .bg(palette.color(Color::Background))
            .text_color(palette.color(Color::Text))
            .text_size(px(15.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(heading),
                    )
                    .when(!matches!(self.page, Page::Locked | Page::Setup), |row| {
                        row.child(self.button(
                            "back",
                            "Back / Esc",
                            |this, window, cx| this.back(&Back, window, cx),
                            cx,
                        ))
                    }),
            )
            .child(body)
            .when(self.busy, |root| root.child("Working..."))
            .when(!self.notice.is_empty(), |root| {
                root.child(
                    div()
                        .id("notice")
                        .role(Role::Status)
                        .aria_label(self.notice.clone())
                        .text_color(palette.color(Color::Warning))
                        .child(self.notice.clone()),
                )
            })
            .child(
                div()
                    .border_t_1()
                    .border_color(palette.color(Color::Border))
                    .pt_2()
                    .text_sm()
                    .bg(palette.color(Color::Footer))
                    .text_color(palette.color(Color::FooterText))
                    .child(if self.page == Page::Browse {
                        "Up/Down Navigate   Enter Copy   Ctrl+K Actions   Ctrl+N Add"
                    } else {
                        "Tab Navigate   Enter Confirm   Esc Back"
                    }),
            )
    }
}
