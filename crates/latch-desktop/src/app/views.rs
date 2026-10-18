use super::*;
use crate::ui::{self, Icon};
use gpui_kit::component::TitleBar;
use gpui_kit::component::kbd::Kbd;
use std::rc::Rc;

type Handler = dyn Fn(&mut Latch, &mut Window, &mut Context<Latch>);

/// Platform-formatted shortcut chip; `secondary` is Cmd on macOS, Ctrl elsewhere.
fn kbd(p: &Palette, keys: &str) -> Option<gpui_kit::Div> {
    let stroke = Keystroke::parse(keys).ok()?;
    let label = match keys {
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        _ => Kbd::format(&stroke),
    };
    Some(
        div()
            .flex_shrink_0()
            .px(px(5.))
            .py(px(1.))
            .rounded(px(4.))
            .bg(ui::wash(p, 0.07))
            .text_size(px(11.))
            .text_color(p.color(Color::Muted))
            .child(label),
    )
}

/// Stable tint for a credential's initial tile, derived from its title.
fn title_tint(title: &str) -> Hsla {
    let hash = title.bytes().fold(7u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    hsla((hash % 360) as f32 / 360., 0.55, 0.62, 1.)
}

fn initial(title: &str) -> String {
    title
        .chars()
        .next()
        .map(|ch| ch.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

fn muted(p: &Palette, text: impl Into<SharedString>) -> gpui_kit::Div {
    div()
        .text_size(px(ui::TEXT_SM))
        .text_color(p.color(Color::Muted))
        .child(text.into())
}

impl Latch {
    /// Registers an element as the page's next keyboard stop.
    pub(super) fn stop(&self, id: &ElementId, cx: &mut Context<Self>) -> FocusHandle {
        let handle = self.stop_handle(id.clone(), cx);
        self.page_stops.borrow_mut().push(handle.clone());
        handle
    }

    /// Invisible layer that scrolls its parent into view when the parent's
    /// stop gains focus. The parent must be `relative`.
    pub(super) fn reveal(&self, handle: FocusHandle) -> impl IntoElement {
        let scroll = if self.page == Page::Actions {
            self.palette_scroll.clone()
        } else {
            self.scroll.clone()
        };
        let revealed = self.revealed.clone();
        canvas(
            move |bounds, window, _| {
                if !handle.is_focused(window) || revealed.borrow().as_ref() == Some(&handle) {
                    return;
                }
                *revealed.borrow_mut() = Some(handle);
                let view = scroll.bounds();
                let margin = px(6.);
                let mut offset = scroll.offset();
                if bounds.top() < view.top() + margin {
                    offset.y += view.top() + margin - bounds.top();
                } else if bounds.bottom() > view.bottom() - margin {
                    offset.y -= bounds.bottom() - (view.bottom() - margin);
                } else {
                    return;
                }
                let floor = -scroll.max_offset().y;
                if offset.y < floor {
                    offset.y = floor;
                }
                if offset.y > px(0.) {
                    offset.y = px(0.);
                }
                scroll.set_offset(offset);
                // A refresh requested mid-draw is dropped; repaint next frame.
                window.on_next_frame(|window, _| window.refresh());
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }

    /// The shell's one list row: leading tile, title, inline subtitle, and
    /// accessories appended by the caller. Focus and selection share a fill.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &self,
        id: impl Into<ElementId>,
        leading: impl IntoElement,
        title: impl Into<SharedString>,
        subtitle: Option<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> gpui_kit::base::Button {
        let p = cx.global::<Palette>().clone();
        let id = id.into();
        let stop = self.stop(&id, cx);
        let title = title.into();
        let selected_bg = p.color(Color::Selected);
        let selected_text = p.color(Color::SelectedText);
        let hover = p.color(Color::Hover);
        let label = match &subtitle {
            Some(subtitle) => format!("{title}, {subtitle}"),
            None => title.to_string(),
        };
        let on_click: Rc<Handler> = Rc::new(on_click);
        gpui_kit::base::Button::new(id)
            .track_focus(&stop)
            .relative()
            .child(self.reveal(stop))
            .accessibility_label(label)
            .disabled(self.busy && self.page != Page::Browse)
            .w_full()
            .h(px(ui::ROW))
            .flex_shrink_0()
            .justify_start()
            .gap(px(10.))
            .px(px(8.))
            .rounded(px(ui::RADIUS))
            .cursor_pointer()
            .text_size(px(ui::TEXT))
            .line_height(px(20.))
            .text_color(p.color(Color::Text))
            .when(selected, |row| {
                row.bg(selected_bg).text_color(selected_text)
            })
            .hover(move |style| if selected { style } else { style.bg(hover) })
            .focus(move |style| style.bg(selected_bg).text_color(selected_text))
            .on_action(cx.listener({
                let on_click = on_click.clone();
                move |this, _: &Submit, window, cx| on_click(this, window, cx)
            }))
            .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
            .child(leading)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_shrink_0()
                            .max_w(relative(0.75))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(title),
                    )
                    .when_some(subtitle, |line, subtitle| {
                        line.child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_color(p.color(Color::Muted))
                                .child(subtitle),
                        )
                    }),
            )
    }

    /// Icon row with a neutral tile.
    fn icon_row(
        &self,
        id: impl Into<ElementId>,
        icon: Icon,
        title: impl Into<SharedString>,
        subtitle: Option<&str>,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> gpui_kit::base::Button {
        let p = cx.global::<Palette>().clone();
        self.row(
            id,
            ui::tile(&p, icon, None),
            title,
            subtitle.map(|text| SharedString::from(text.to_owned())),
            false,
            cx,
            on_click,
        )
    }

    /// A labeled form line: right-aligned label column, field, optional action.
    fn form_row(
        &self,
        p: &Palette,
        label: &'static str,
        field: impl IntoElement,
        stop: Option<FocusHandle>,
        action: Option<AnyElement>,
    ) -> gpui_kit::Div {
        div()
            .relative()
            .when_some(stop, |line, stop| line.child(self.reveal(stop)))
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(8.))
            .child(
                div()
                    .w(px(104.))
                    .flex_shrink_0()
                    .flex()
                    .justify_end()
                    .text_size(px(ui::TEXT_SM + 0.5))
                    .text_color(p.color(Color::Muted))
                    .child(label),
            )
            .child(div().flex_1().min_w_0().child(field))
            // A fixed slot keeps every field the same width.
            .when_some(action, |line, action| {
                line.child(div().w(px(100.)).flex_shrink_0().flex().child(action))
            })
    }

    fn back_button(&self, p: &Palette, cx: &mut Context<Self>) -> gpui_kit::base::Button {
        let hover = ui::wash(p, 0.1);
        let focus = p.color(Color::Focus);
        // Escape is the keyboard path back, so arrows start on the content.
        gpui_kit::base::Button::new("back")
            .accessibility_label("Back")
            .tab_stop(false)
            .disabled(self.busy)
            .size(px(26.))
            .flex_shrink_0()
            .rounded(px(6.))
            .bg(ui::wash(p, 0.06))
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .text_size(px(15.))
            .text_color(p.color(Color::Muted))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .focus(move |style| style.border_color(focus))
            .on_click(cx.listener(|this, _, window, cx| this.back(&Back, window, cx)))
            .child(Icon::ChevronLeft)
    }

    fn header(&self, p: &Palette, heading: &'static str, cx: &mut Context<Self>) -> gpui_kit::Div {
        let bar = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .h(px(ui::HEADER))
            .px(px(ui::INSET + 2.))
            .border_b_1()
            .border_color(p.color(Color::Border));
        if matches!(self.page, Page::Browse | Page::Actions) {
            let count = self.previews.len();
            bar.child(
                div()
                    .text_size(px(16.))
                    .text_color(p.color(Color::Muted))
                    .child(Icon::Search),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(ui::TEXT_LG))
                    .line_height(px(24.))
                    .child(self.query.clone()),
            )
            .when(count > 0, |bar| {
                bar.child(ui::tag(
                    p,
                    format!("{count} {}", if count == 1 { "item" } else { "items" }),
                ))
            })
        } else {
            bar.child(self.back_button(p, cx)).child(
                div()
                    .text_size(px(ui::TEXT_LG - 1.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(heading),
            )
        }
    }

    /// Label of the page action that Enter runs, shown in the footer.
    fn primary_label(&self, cx: &App) -> Option<&'static str> {
        Some(match self.page {
            Page::Setup => "Create vault",
            Page::Locked if self.blocked => return None,
            Page::Locked if self.biometric => "Unlock with device key",
            Page::Locked => "Unlock",
            Page::Rotate => "Change password",
            Page::Browse => {
                let shown = self.shown_previews(cx);
                if self.selected < shown {
                    "Copy password"
                } else if self.selected < self.rows(cx) {
                    "Run command"
                } else {
                    return None;
                }
            }
            Page::Details => "Copy password",
            Page::Edit => "Save credential",

            Page::Actions => "Run action",
            Page::Preview => "Apply appearance",
            Page::Provider => "Save provider",

            Page::Generator => "Use password",

            Page::Settings
            | Page::Health
            | Page::Appearance
            | Page::Providers
            | Page::Delete
            | Page::Discard
            | Page::DeleteProvider => return None,
        })
    }

    fn footer(&self, p: &Palette, window: &Window, cx: &mut Context<Self>) -> gpui_kit::Div {
        let auth = matches!(self.page, Page::Locked | Page::Setup);
        let status: Option<(Icon, String)> = if self.busy {
            Some((Icon::RefreshCw, "Working…".into()))
        } else if !self.notice.is_empty()
            && !matches!(self.page, Page::Locked | Page::Setup | Page::Rotate)
        {
            Some((Icon::Info, self.notice.clone()))
        } else {
            None
        };
        let left = div()
            .id("status")
            .role(Role::Status)
            .flex()
            .items_center()
            .gap(px(8.))
            .min_w_0()
            .flex_1()
            .overflow_hidden()
            .text_size(px(ui::TEXT_SM))
            .when_some(status.clone(), |left, (_, text)| left.aria_label(text))
            .map(|left| match status {
                Some((icon, text)) => left
                    .text_color(p.color(Color::Text))
                    .child(div().text_color(p.color(Color::Link)).child(icon))
                    .child(
                        div()
                            .min_w_0()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(text),
                    ),
                None => left
                    .text_color(p.color(Color::FooterText))
                    .child(Icon::LockKeyhole)
                    .child("Latch"),
            });
        let link = |id: &'static str| {
            let hover = ui::wash(p, 0.07);
            div()
                .id(id)
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(6.))
                .h(px(24.))
                .px(px(6.))
                .rounded(px(5.))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
        };
        let mut right = div().flex().flex_shrink_0().items_center().gap(px(4.));
        // A focused row or button owns Enter; only fields leave it to the page.
        let on_row = !matches!(self.page, Page::Browse | Page::Actions)
            && self
                .page_stops
                .borrow()
                .iter()
                .any(|stop| stop.is_focused(window))
            && !self
                .fields
                .iter()
                .any(|field| field.read(cx).focus_handle(cx).is_focused(window));
        let primary = self.primary_label(cx).filter(|_| !on_row);
        if primary.is_none() && !auth {
            right = right.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(6.))
                    .text_color(p.color(Color::Muted))
                    .child("Navigate")
                    .children(kbd(p, "up"))
                    .children(kbd(p, "down"))
                    .child(div().pl(px(6.)).child("Select"))
                    .children(kbd(p, "enter")),
            );
        }
        if let Some(label) = primary {
            right = right.child(
                link("primary")
                    .text_color(p.color(Color::Text))
                    .font_weight(FontWeight::MEDIUM)
                    .on_click(cx.listener(|this, _, window, cx| this.submit(&Submit, window, cx)))
                    .child(label)
                    .children(kbd(p, "enter")),
            );
        }
        if !auth && self.page != Page::Actions {
            right = right
                .child(
                    div()
                        .w(px(1.))
                        .h(px(14.))
                        .mx(px(4.))
                        .bg(p.color(Color::Border)),
                )
                .child(
                    link("actions")
                        .text_color(p.color(Color::Muted))
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !this.busy && !this.guard_draft(window, cx) {
                                this.open_actions(window, cx);
                            }
                        }))
                        .child("Actions")
                        .children(kbd(p, "secondary-k")),
                );
        }
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .h(px(ui::FOOTER))
            .px(px(ui::INSET))
            .border_t_1()
            .border_color(p.color(Color::Border))
            .bg(p.color(Color::Footer))
            .text_size(px(ui::TEXT_SM))
            .child(left)
            .child(right)
    }

    // Keep page builders in separate frames: debug GPUI builder temporaries
    // otherwise exhaust the Windows main thread's 1 MiB stack at render entry.
    fn render_auth(
        &mut self,
        body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let (title, caption, action) = match self.page {
            Page::Setup => (
                "Create your vault",
                "Your vault is encrypted and stored only on this device. There is no password recovery, so choose a password you will remember.",
                "Create vault",
            ),
            Page::Rotate => (
                "",
                "Use 12 or more characters. The vault is re-encrypted with the new password immediately.",
                "Change password",
            ),
            _ if self.biometric => (
                "Unlock Latch",
                "This vault is protected by a key stored on this device. You can switch to a master password in Settings.",
                "Unlock with device key",
            ),
            _ => (
                "Unlock Latch",
                "Enter your master password to decrypt your vault.",
                "Unlock",
            ),
        };
        let accent = p.color(Color::Link);
        let mut stack = div()
            .w_full()
            .max_w(px(340.))
            .flex()
            .flex_col()
            .gap(px(10.));
        if self.page != Page::Rotate {
            stack = stack.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.))
                    .pb(px(4.))
                    .child(
                        div()
                            .size(px(44.))
                            .rounded(px(12.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(accent.opacity(0.14))
                            .text_color(accent)
                            .text_size(px(20.))
                            .child(if self.biometric && self.page == Page::Locked {
                                Icon::FingerprintPattern
                            } else {
                                Icon::LockKeyhole
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    ),
            );
        }
        stack = stack.child(muted(p, caption).text_center().pb(px(6.)));
        if !self.blocked {
            for field in &self.fields {
                self.page_stops
                    .borrow_mut()
                    .push(field.read(cx).focus_handle(cx));
                stack = stack.child(field.clone());
            }
            stack = stack.child(
                self.button(
                    "unlock",
                    None,
                    action,
                    ButtonKind::Primary,
                    |this, window, cx| this.submit(&Submit, window, cx),
                    cx,
                )
                .w_full()
                .h(px(32.))
                .mt(px(4.)),
            );
        }
        if !self.notice.is_empty() {
            stack = stack.child(
                muted(p, self.notice.clone())
                    .text_center()
                    .text_color(p.color(Color::Warning)),
            );
        }
        body.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .items_center()
                .justify_center()
                .px(px(24.))
                .child(stack),
        )
    }

    fn render_browse(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let shown = self.shown_previews(cx);
        let commands = self.root_commands(cx);
        if self.empty_vault(cx) {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.))
                    .py(px(28.))
                    .child(
                        ui::tile(p, Icon::KeyRound, Some(p.color(Color::Link)))
                            .size(px(36.))
                            .rounded(px(10.))
                            .text_size(px(17.)),
                    )
                    .child(div().pt(px(4.)).child("No credentials yet"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(ui::TEXT_SM))
                            .text_color(p.color(Color::Muted))
                            .child("Add your first one with")
                            .children(kbd(p, "secondary-n")),
                    ),
            );
        } else if shown == 0 && commands.is_empty() {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .py(px(28.))
                    .child(muted(p, "No matching credentials or commands")),
            );
        }
        for (index, entry) in self.previews.iter().enumerate().take(shown) {
            let row = self
                .row(
                    ("entry", index),
                    ui::tile(p, initial(&entry.title), Some(title_tint(&entry.title)))
                        .font_weight(FontWeight::SEMIBOLD),
                    entry.title.clone(),
                    Some(entry.username.clone().into()),
                    index == self.selected,
                    cx,
                    move |this, window, cx| {
                        this.selected = index;
                        window.focus(&this.query.read(cx).focus_handle(cx), cx);
                        this.submit(&Submit, window, cx);
                    },
                )
                .tab_stop(false)
                .aria_selected(index == self.selected)
                .when(entry.has_totp, |row| row.child(ui::tag(p, "2FA")));
            body = body.child(
                div()
                    .flex_shrink_0()
                    .when(index == 0, |group| {
                        group.child(ui::section(p, "Credentials"))
                    })
                    .child(row),
            );
        }
        for (offset, command) in commands.into_iter().enumerate() {
            let index = shown + offset;
            let row = self
                .row(
                    ("command", offset),
                    ui::tile(p, command.icon, None),
                    command.label,
                    None,
                    index == self.selected,
                    cx,
                    move |this, window, cx| {
                        this.selected = index;
                        window.focus(&this.query.read(cx).focus_handle(cx), cx);
                        this.submit(&Submit, window, cx);
                    },
                )
                .tab_stop(false)
                .aria_selected(index == self.selected)
                .children(command.keys.and_then(|keys| kbd(p, keys)));
            body = body.child(
                div()
                    .flex_shrink_0()
                    .when(offset == 0, |group| group.child(ui::section(p, "Commands")))
                    .child(row),
            );
        }
        body
    }

    fn render_edit(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let labels = ["Title", "Username", "Password", "Website", "Authenticator"];
        let mut form = div().flex().flex_col().gap(px(8.)).py(px(10.));
        for (index, (field, label)) in self.fields.iter().zip(labels).enumerate() {
            let stop = field.read(cx).focus_handle(cx);
            self.page_stops.borrow_mut().push(stop.clone());
            let action = match index {
                1 => self
                    .button(
                        "alias",
                        Some(Icon::AtSign),
                        "Alias",
                        ButtonKind::Ghost,
                        |this, _, cx| this.generate_alias(cx),
                        cx,
                    )
                    .into_any_element(),
                2 => self
                    .button(
                        "generate",
                        Some(Icon::Dices),
                        "Generate",
                        ButtonKind::Ghost,
                        |this, window, cx| this.open_generator(window, cx),
                        cx,
                    )
                    .into_any_element(),
                _ => div().into_any_element(),
            };
            form = form.child(self.form_row(p, label, field.clone(), Some(stop), Some(action)));
        }
        form = form.child(
            div().pl(px(8. + 104. + 12.)).pt(px(2.)).child(muted(
                p,
                "Website and authenticator are optional. The authenticator accepts a secret or an otpauth:// URI.",
            )),
        );
        body = body.child(form);
        body
    }

    fn render_details(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(entry) = self.detail.clone() else {
            return body;
        };
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(8.))
                .pt(px(8.))
                .pb(px(6.))
                .child(
                    ui::tile(p, initial(&entry.title), Some(title_tint(&entry.title)))
                        .size(px(38.))
                        .rounded(px(10.))
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(ui::TEXT_LG))
                                .font_weight(FontWeight::SEMIBOLD)
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(entry.title.clone()),
                        )
                        .child(
                            muted(p, entry.username.clone())
                                .whitespace_nowrap()
                                .text_ellipsis(),
                        ),
                ),
        );
        let value = |text: String| {
            div()
                .flex_shrink_0()
                .max_w(px(260.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_color(p.color(Color::Muted))
                .child(text)
        };
        let copy_hint = || div().text_color(p.color(Color::Muted)).child(Icon::Copy);
        body = body.child(ui::section(p, "Fields"));
        body = body.child(
            self.icon_row(
                "detail-username",
                Icon::User,
                "Username",
                None,
                cx,
                |this, _, cx| this.copy_field("username", cx),
            )
            .child(value(entry.username.clone()))
            .child(copy_hint()),
        );
        if let Some(field) = self.fields.first() {
            let revealed = self.reveal_until.is_some();
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(ui::ROW))
                    .px(px(8.))
                    .child(ui::tile(p, Icon::KeyRound, None))
                    .child(div().w(px(80.)).flex_shrink_0().child("Password"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_family(ui::mono_font())
                            .text_size(px(ui::TEXT))
                            .line_height(px(20.))
                            .text_color(p.color(Color::Muted))
                            .child(field.clone()),
                    )
                    .child(self.button(
                        "reveal",
                        Some(if revealed { Icon::Clock } else { Icon::Eye }),
                        if revealed { "Visible" } else { "Reveal" },
                        ButtonKind::Ghost,
                        |this, _, cx| {
                            this.reveal_until = Some(SystemTime::now() + Duration::from_secs(10));
                            if let Some(field) = this.fields.first() {
                                field.update(cx, |field, cx| field.set_masked(false, cx));
                            }
                        },
                        cx,
                    ))
                    .child(self.button(
                        "copy",
                        Some(Icon::Copy),
                        "Copy",
                        ButtonKind::Ghost,
                        |this, _, cx| this.copy_field("password", cx),
                        cx,
                    )),
            );
        }
        if let Some(url) = entry.url.clone() {
            let host = url::Url::parse(&url)
                .ok()
                .and_then(|parsed| parsed.host_str().map(str::to_owned))
                .unwrap_or_else(|| url.clone());
            body = body.child(
                self.icon_row(
                    "detail-url",
                    Icon::Globe,
                    "Website",
                    None,
                    cx,
                    move |this, _, cx| match url::Url::parse(&url) {
                        Ok(parsed)
                            if matches!(parsed.scheme(), "http" | "https")
                                && parsed.host_str().is_some() =>
                        {
                            cx.open_url(parsed.as_str())
                        }
                        _ => {
                            this.notice = "Only HTTP and HTTPS website links can be opened.".into();
                            cx.notify();
                        }
                    },
                )
                .child(value(host))
                .child(
                    div()
                        .text_color(p.color(Color::Muted))
                        .child(Icon::ExternalLink),
                ),
            );
        }
        if let Some(secret) = &entry.totp_secret {
            let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let code = vault::totp::generate_token(secret, time)
                .map(|code| {
                    let (head, tail) = code.split_at(code.len() / 2);
                    format!("{head} {tail}")
                })
                .unwrap_or_else(|_| "Unavailable".into());
            body = body.child(
                self.icon_row(
                    "detail-totp",
                    Icon::Clock,
                    "One-time code",
                    None,
                    cx,
                    |this, _, cx| this.copy_field("totp", cx),
                )
                .child(
                    div()
                        .font_family(ui::mono_font())
                        .text_color(p.color(Color::Text))
                        .child(code),
                )
                .child(ui::tag(
                    p,
                    format!("{}s", vault::totp::remaining_seconds(time)),
                ))
                .children(kbd(p, "secondary-t")),
            );
        }
        body = body.child(ui::section(p, "Manage"));
        body = body.child(
            self.icon_row(
                "detail-edit",
                Icon::Pencil,
                "Edit credential",
                None,
                cx,
                |this, window, cx| this.edit(window, cx),
            )
            .children(kbd(p, "secondary-e")),
        );
        let error = p.color(Color::Error);
        let selected = p.color(Color::Selected);
        body = body.child(
            self.row(
                "detail-delete",
                ui::tile(p, Icon::Trash, Some(error)),
                "Delete credential",
                None,
                false,
                cx,
                |this, window, cx| {
                    this.page = Page::Delete;
                    window.focus(&this.focus, cx);
                    cx.notify();
                },
            )
            .text_color(error)
            .focus(move |style| style.bg(selected).text_color(error)),
        );
        body
    }

    /// Centered confirmation for irreversible or lossy steps.
    #[allow(clippy::too_many_arguments)]
    fn confirm(
        &self,
        body: Stateful<Div>,
        p: &Palette,
        title: String,
        caption: &'static str,
        confirm: &'static str,
        cancel: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let error = p.color(Color::Error);
        body.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .items_center()
                .justify_center()
                .px(px(24.))
                .child(
                    div()
                        .max_w(px(360.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            ui::tile(p, Icon::TriangleAlert, Some(error))
                                .size(px(40.))
                                .rounded(px(11.))
                                .text_size(px(18.)),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .text_size(px(ui::TEXT_LG))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_center()
                                .child(title),
                        )
                        .child(muted(p, caption).text_center())
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .pt(px(10.))
                                .child(self.button(
                                    "cancel",
                                    None,
                                    cancel,
                                    ButtonKind::Secondary,
                                    |this, window, cx| this.back(&Back, window, cx),
                                    cx,
                                ))
                                .child(self.button(
                                    "confirm",
                                    None,
                                    confirm,
                                    ButtonKind::Danger,
                                    |this, window, cx| this.submit(&Submit, window, cx),
                                    cx,
                                )),
                        ),
                ),
        )
    }

    fn render_delete(
        &mut self,
        body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let name = self
            .detail
            .as_ref()
            .map(|entry| entry.title.clone())
            .unwrap_or_else(|| "this credential".into());
        self.confirm(
            body,
            p,
            format!("Delete “{name}”?"),
            "This permanently removes the credential from your vault. It cannot be undone.",
            "Delete permanently",
            "Cancel",
            cx,
        )
    }

    /// Raycast-style action panel anchored above the footer's Actions hint.
    fn palette_overlay(
        &self,
        p: &Palette,
        filter_focused: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Div {
        let mut rows = div()
            .id("palette-rows")
            .flex()
            .flex_col()
            .p(px(6.))
            .flex_1()
            .min_h_0()
            .track_scroll(&self.palette_scroll)
            .overflow_y_scroll();
        let filter = self.action_query.read(cx).value().trim().to_lowercase();
        let mut first = true;
        for (index, command) in COMMANDS.iter().enumerate() {
            if !command.label.to_lowercase().contains(&filter) {
                continue;
            }
            let id = command.id;
            let row = self
                .row(
                    ("palette", index),
                    ui::tile(p, command.icon, None),
                    command.label,
                    None,
                    first && filter_focused,
                    cx,
                    move |this, window, cx| this.run_command(id, window, cx),
                )
                .h(px(34.))
                .children(command.keys.and_then(|keys| kbd(p, keys)));
            rows = rows.child(row);
            first = false;
        }
        if first {
            rows = rows.child(div().p(px(10.)).child(muted(p, "No matching actions")));
        }
        let panel = div()
            .id("palette")
            .absolute()
            .right(px(10.))
            .bottom(px(ui::FOOTER + 6.))
            .w(px(360.))
            .max_h(px(360.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(10.))
            .bg(p.color(Color::Popup))
            .border_1()
            .border_color(p.color(Color::Border))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., if p.dark { 0.5 } else { 0.18 }),
                offset: point(px(0.), px(10.)),
                blur_radius: px(30.),
                spread_radius: px(-6.),
                inset: false,
            }])
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(rows)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .h(px(40.))
                    .px(px(14.))
                    .border_t_1()
                    .border_color(p.color(Color::Border))
                    .text_size(px(ui::TEXT))
                    .line_height(px(20.))
                    .child(self.action_query.clone()),
            );
        div()
            .absolute()
            .inset_0()
            .bg(hsla(0., 0., 0., if p.dark { 0.25 } else { 0.06 }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.back(&Back, window, cx)),
            )
            .child(panel)
    }

    fn render_appearance(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        body = body.child(div().px(px(8.)).pt(px(8.)).child(muted(
            p,
            "Use the colors from a T3 Code, VS Code or Zed theme file, or from the color overrides in an editor settings.json. Only colors are kept.",
        )));
        if self.page == Page::Preview {
            body = body.child(ui::section(p, "Preview"));
            if p.low_contrast() {
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(ui::TEXT_SM))
                        .text_color(p.color(Color::Warning))
                        .child(Icon::TriangleAlert)
                        .child("Low contrast: some text may be hard to read. Cancel with Esc or reset afterwards."),
                );
            }
            let sample = |selected: bool, title: &'static str, subtitle: &'static str| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(ui::ROW))
                    .px(px(8.))
                    .rounded(px(ui::RADIUS))
                    .when(selected, |row| {
                        row.bg(p.color(Color::Selected))
                            .text_color(p.color(Color::SelectedText))
                    })
                    .child(ui::tile(p, initial(title), Some(title_tint(title))))
                    .child(title)
                    .child(div().text_color(p.color(Color::Muted)).child(subtitle))
            };
            body = body
                .child(sample(true, "Example credential", "person@example.com"))
                .child(sample(false, "Another credential", "someone@example.com"))
                .child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(ui::TEXT_SM))
                        .text_color(p.color(Color::Error))
                        .child("Error: example validation message"),
                )
                .child(
                    self.row(
                        "apply-appearance",
                        ui::tile(p, Icon::Check, Some(p.color(Color::Link))),
                        "Apply appearance",
                        None,
                        false,
                        cx,
                        |this, _, cx| this.apply_appearance(cx),
                    )
                    .children(kbd(p, "enter")),
                );
        }
        body = body.child(ui::section(p, "Import"));
        body = body.child(self.icon_row(
            "import",
            Icon::FileText,
            "Import theme or settings.json",
            Some("T3 Code, VS Code or Zed"),
            cx,
            |this, _, cx| this.import_appearance(cx),
        ));
        body = body.child(ui::section(p, "Manage"));
        body.child(self.icon_row(
            "open-settings",
            Icon::ExternalLink,
            "Open appearance settings.json",
            None,
            cx,
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
                    this.notice = "Import editor settings before opening the managed file.".into();
                    cx.notify();
                }
            },
        ))
        .child(self.icon_row(
            "reload-appearance",
            Icon::RefreshCw,
            "Reload from disk",
            None,
            cx,
            |this, _, cx| this.reload_appearance(cx),
        ))
        .child(
            self.icon_row(
                "reset-appearance",
                Icon::RotateCw,
                "Reset to default colors",
                None,
                cx,
                |this, _, cx| this.reset_appearance(cx),
            )
            .children(kbd(p, "secondary-alt-r")),
        )
    }

    fn render_settings(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let chevron = || {
            div()
                .text_color(p.color(Color::Muted))
                .child(Icon::ChevronRight)
        };
        body = body.child(ui::section(p, "Security"));
        body = body.child(
            self.icon_row(
                "rotate",
                Icon::KeyRound,
                "Change master password",
                self.biometric.then_some("Replaces the device key"),
                cx,
                |this, window, cx| {
                    this.page = Page::Rotate;
                    this.fields.clear();
                    this.field("New master password", true, cx);
                    this.field("Confirm master password", true, cx);
                    this.focus_first(window, cx);
                    cx.notify();
                },
            )
            .child(chevron()),
        );
        if crate::biometric::supported() && !self.biometric {
            body = body.child(self.icon_row(
                "device-key",
                Icon::FingerprintPattern,
                "Switch to device key",
                Some("Replaces the master password"),
                cx,
                |this, _, cx| {
                    this.work(cx, |vault| {
                        let encoded = crate::biometric::create_or_retrieve()?;
                        let decoded = Zeroizing::new(
                            hex::decode(encoded.as_str()).map_err(|_| "Invalid device key")?,
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
            ));
        }
        body = body
            .child(
                self.icon_row(
                    "health",
                    Icon::ShieldCheck,
                    "Password health",
                    None,
                    cx,
                    |this, _, cx| this.health(cx),
                )
                .child(chevron()),
            )
            .child(
                self.icon_row(
                    "lock",
                    Icon::Lock,
                    "Lock vault",
                    Some("Locks automatically after 30 minutes idle"),
                    cx,
                    |this, window, cx| this.lock(window, cx),
                )
                .children(kbd(p, "secondary-l")),
            );
        body = body.child(ui::section(p, "Preferences"));
        body = body
            .child(
                self.icon_row(
                    "providers",
                    Icon::Mail,
                    "Email alias providers",
                    None,
                    cx,
                    |this, _, cx| this.open_providers(cx),
                )
                .child(chevron()),
            )
            .child(
                self.icon_row(
                    "appearance",
                    Icon::Palette,
                    "Appearance",
                    None,
                    cx,
                    |this, window, cx| {
                        this.page = Page::Appearance;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    },
                )
                .child(chevron()),
            );
        body = body.child(ui::section(p, "About"));
        let version = format!("Latch {}", env!("CARGO_PKG_VERSION"));
        body = body.child(self.icon_row(
            "check-updates",
            Icon::RefreshCw,
            "Check for updates",
            Some(version.as_str()),
            cx,
            |this, _, cx| this.check_updates(cx),
        ));
        if let Some(update) = &self.update {
            let label = format!("Install {} and restart", update.version);
            body = body.child(self.row(
                "install-update",
                ui::tile(p, Icon::Download, Some(p.color(Color::Link))),
                label,
                Some("Locks the vault first".into()),
                false,
                cx,
                |this, window, cx| this.install_update(window, cx),
            ));
        }
        if let Some(integration) = cx.try_global::<crate::integration::Integration>() {
            body = body.child(
                div()
                    .px(px(8.))
                    .pt(px(8.))
                    .child(muted(p, integration.status.clone())),
            );
        }
        body
    }

    fn render_health(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        body = body.child(div().px(px(8.)).pt(px(8.)).child(muted(
            p,
            "Weak and reused passwords are found on this device. The breach check sends only SHA-1 hash prefixes to Have I Been Pwned.",
        )));
        body = body.child(ui::section(p, "Checks"));
        body = body
            .child(self.icon_row(
                "refresh-health",
                Icon::RefreshCw,
                "Run local checks again",
                None,
                cx,
                |this, _, cx| this.health(cx),
            ))
            .child(self.icon_row(
                "breach-check",
                Icon::ShieldAlert,
                "Check for breached passwords",
                Some("Uses the network"),
                cx,
                |this, _, cx| this.check_breaches(cx),
            ));
        let count = self.issues.len() + self.breaches.len();
        if count == 0 {
            body = body.child(ui::section(p, "Issues")).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(ui::ROW))
                    .px(px(8.))
                    .child(ui::tile(
                        p,
                        Icon::ShieldCheck,
                        Some(p.color(Color::Success)),
                    ))
                    .child("No weak or reused passwords found"),
            );
            return body;
        }
        body = body.child(ui::section(p, format!("Issues · {count}")));
        let warning = p.color(Color::Warning);
        let error = p.color(Color::Error);
        let issues: Vec<_> = self
            .issues
            .iter()
            .map(|issue| (issue, warning))
            .chain(self.breaches.iter().map(|issue| (issue, error)))
            .map(|((id, label), tint)| (id.clone(), label.clone(), tint))
            .collect();
        for (index, (id, label, tint)) in issues.into_iter().enumerate() {
            let (title, reason) = label
                .rsplit_once(": ")
                .map(|(title, reason)| (title.to_owned(), reason.to_owned()))
                .unwrap_or((label, String::new()));
            body = body.child(self.row(
                ("issue", index),
                ui::tile(p, Icon::TriangleAlert, Some(tint)),
                title,
                Some(reason.into()),
                false,
                cx,
                move |this, _, cx| {
                    let id = id.clone();
                    this.work(cx, move |vault| {
                        vault.with_vault(|_, workspace| {
                            vault::entries::get_full(workspace, &id).map(Reply::Detail)
                        })
                    });
                },
            ));
        }
        body
    }

    fn render_providers(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        body = body.child(div().px(px(8.)).pt(px(8.)).child(muted(
            p,
            "Generate a private email alias from the username field. API tokens stay inside your encrypted vault.",
        )));
        if !self.providers.is_empty() {
            body = body.child(ui::section(p, "Configured"));
        }
        for (index, provider) in self.providers.iter().enumerate() {
            let id = provider.provider_id.clone();
            let remove = id.clone();
            let default = self.default_provider.as_deref() == Some(id.as_str());
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(ui::ROW))
                    .px(px(8.))
                    .child(ui::tile(p, Icon::AtSign, Some(p.color(Color::Link))))
                    .child(div().flex_1().min_w_0().child(id.clone()))
                    .when(default, |row| row.child(ui::tag(p, "Default")))
                    .when(!default, |row| {
                        row.child(self.button(
                            ("default-provider", index),
                            None,
                            "Make default",
                            ButtonKind::Ghost,
                            move |this, _, cx| {
                                let id = id.clone();
                                this.work(cx, move |vault| {
                                    vault.with_vault(|storage, workspace| {
                                        vault::alias::set_default_config(workspace, storage, &id)?;
                                        Ok(Reply::Providers(
                                            vault::alias::list_configs(workspace)?,
                                            workspace.default_provider_id.clone(),
                                        ))
                                    })
                                });
                            },
                            cx,
                        ))
                    })
                    .child(self.button(
                        ("remove-provider", index),
                        Some(Icon::Trash),
                        "Remove",
                        ButtonKind::Ghost,
                        move |this, window, cx| {
                            this.provider = remove.clone();
                            this.page = Page::DeleteProvider;
                            window.focus(&this.focus, cx);
                            cx.notify();
                        },
                        cx,
                    )),
            );
        }
        let chevron = || {
            div()
                .text_color(p.color(Color::Muted))
                .child(Icon::ChevronRight)
        };
        body = body.child(ui::section(p, "Add a provider"));
        body.child(
            self.icon_row(
                "simplelogin",
                Icon::Mail,
                "SimpleLogin",
                Some("API token"),
                cx,
                |this, window, cx| this.configure_provider("simplelogin", window, cx),
            )
            .child(chevron()),
        )
        .child(
            self.icon_row(
                "duckduckgo",
                Icon::Mail,
                "DuckDuckGo Email Protection",
                Some("API token"),
                cx,
                |this, window, cx| this.configure_provider("duckduckgo", window, cx),
            )
            .child(chevron()),
        )
    }

    fn render_provider(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let mut form = div().flex().flex_col().gap(px(8.)).py(px(10.));
        form = form.child(self.form_row(
            p,
            "Provider",
            div().flex().child(ui::tag(p, self.provider.clone())),
            None,
            None,
        ));
        for (field, label) in self.fields.iter().zip(["API token", "Description"]) {
            let stop = field.read(cx).focus_handle(cx);
            self.page_stops.borrow_mut().push(stop.clone());
            form = form.child(self.form_row(p, label, field.clone(), Some(stop), None));
        }
        form = form.child(
            div()
                .pl(px(8. + 104. + 12.))
                .pt(px(2.))
                .child(muted(p, "The token is encrypted with your vault and never leaves this device except to call the provider.")),
        );
        body = body.child(form);
        body
    }

    fn render_delete_provider(
        &mut self,
        body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let title = format!("Remove {}?", self.provider);
        self.confirm(
            body,
            p,
            title,
            "This deletes the stored API token. Aliases you already created keep working.",
            "Remove provider",
            "Cancel",
            cx,
        )
    }

    fn render_generator(
        &mut self,
        mut body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let options = self.generator_options.clone();
        let strength = password_generator::analyze_password_strength(&self.generated).label;
        let strength_color = p.color(ui::strength_color(&strength));
        body = body.child(
            div()
                .mx(px(8.))
                .mt(px(10.))
                .px(px(14.))
                .py(px(14.))
                .rounded(px(ui::RADIUS))
                .bg(p.color(Color::Input))
                .border_1()
                .border_color(p.color(Color::Border))
                .font_family(ui::mono_font())
                .text_size(px(ui::TEXT_LG))
                .line_height(px(24.))
                .child(self.generated.to_string()),
        );
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .px(px(10.))
                .pt(px(8.))
                .text_size(px(ui::TEXT_SM))
                .child(div().size(px(7.)).rounded_full().bg(strength_color))
                .child(div().text_color(strength_color).child(strength))
                .child(
                    div()
                        .text_color(p.color(Color::Muted))
                        .child(format!("· {} characters", options.length)),
                ),
        );
        body = body.child(ui::section(p, "Options"));
        body = body.child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(ui::ROW))
                .px(px(8.))
                .child(ui::tile(p, Icon::Info, None))
                .child(div().flex_1().child("Length"))
                .children(kbd(p, "left"))
                .children(kbd(p, "right"))
                .child(self.button(
                    "shorter",
                    None,
                    "−",
                    ButtonKind::Secondary,
                    |this, _, cx| this.resize_generated(-1, cx),
                    cx,
                ))
                .child(
                    div()
                        .w(px(32.))
                        .text_center()
                        .font_family(ui::mono_font())
                        .child(options.length.to_string()),
                )
                .child(self.button(
                    "longer",
                    None,
                    "+",
                    ButtonKind::Secondary,
                    |this, _, cx| this.resize_generated(1, cx),
                    cx,
                )),
        );
        let accent = p.color(Color::Link);
        for (index, (label, enabled)) in [
            ("Uppercase letters", options.uppercase),
            ("Lowercase letters", options.lowercase),
            ("Numbers", options.numbers),
            ("Symbols", options.symbols),
            ("Exclude look-alike characters", options.exclude_ambiguous),
        ]
        .into_iter()
        .enumerate()
        {
            let check = div()
                .flex_shrink_0()
                .size(px(18.))
                .mx(px(3.))
                .rounded(px(5.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .map(|check| {
                    if enabled {
                        check
                            .bg(p.color(Color::Button))
                            .text_color(p.color(Color::ButtonText))
                            .child(Icon::Check)
                    } else {
                        check
                            .border_1()
                            .border_color(p.color(Color::Muted).opacity(0.6))
                    }
                });
            body = body.child(
                self.row(
                    ("generator-option", index),
                    check,
                    label,
                    None,
                    false,
                    cx,
                    move |this, _, cx| {
                        let options = &mut this.generator_options;
                        match index {
                            0 => options.uppercase = !options.uppercase,
                            1 => options.lowercase = !options.lowercase,
                            2 => options.numbers = !options.numbers,
                            3 => options.symbols = !options.symbols,
                            _ => options.exclude_ambiguous = !options.exclude_ambiguous,
                        };
                        this.regenerate(cx);
                    },
                )
                .aria_selected(enabled),
            );
        }
        body = body.child(ui::section(p, "Actions"));
        let use_label = if self.generator_return == Page::Edit {
            "Use password"
        } else {
            "New credential with this password"
        };
        body.child(
            self.row(
                "use-generated",
                ui::tile(p, Icon::Check, Some(accent)),
                use_label,
                None,
                false,
                cx,
                |this, window, cx| this.use_generated(window, cx),
            )
            .children(kbd(p, "enter")),
        )
        .child(self.icon_row(
            "regenerate",
            Icon::RefreshCw,
            "Regenerate",
            None,
            cx,
            |this, _, cx| this.regenerate(cx),
        ))
        .child(self.icon_row(
            "copy-generated",
            Icon::Copy,
            "Copy password",
            Some("Clears from the clipboard after 30 seconds"),
            cx,
            |this, _, cx| this.copy(Zeroizing::new(this.generated.to_string()), cx),
        ))
    }

    fn render_discard(
        &mut self,
        body: Stateful<Div>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.confirm(
            body,
            p,
            "Discard this draft?".into(),
            "Your unsaved credential changes will be lost.",
            "Discard changes",
            "Keep editing",
            cx,
        )
    }
}

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
        let p = &palette;
        ui::sync_component_theme(p, cx);
        self.page_stops.borrow_mut().clear();
        if self.page != self.shown {
            let returning = matches!(self.page, Page::Browse | Page::Actions)
                && !matches!(self.shown, Page::Browse | Page::Actions);
            if !matches!(self.page, Page::Browse | Page::Actions) || self.selected == 0 {
                self.scroll.set_offset(point(px(0.), px(0.)));
            } else if returning {
                // The root list shares the scroll handle; restore its selection.
                let offset = usize::from(self.empty_vault(cx));
                self.scroll.scroll_to_item(self.selected + offset);
            }
            *self.revealed.borrow_mut() = None;
            self.shown = self.page.clone();
        }
        if self.page == Page::Browse && self.focus.is_focused(window) {
            window.focus(&self.query.read(cx).focus_handle(cx), cx);
        }
        let heading = match self.page {
            Page::Setup => "Create your vault",
            Page::Locked => "Unlock Latch",
            Page::Browse | Page::Actions => "Latch",
            Page::Details => "Credential",
            Page::Edit => {
                if self.editing.is_some() {
                    "Edit credential"
                } else {
                    "New credential"
                }
            }
            Page::Delete => "Delete credential",
            Page::Settings => "Settings",
            Page::Rotate => "Change master password",
            Page::Health => "Password health",
            Page::Appearance => "Appearance",
            Page::Preview => "Appearance preview",
            Page::Providers => "Email alias providers",
            Page::Provider => "Configure provider",
            Page::DeleteProvider => "Remove provider",
            Page::Generator => "Generate password",
            Page::Discard => "Discard draft",
        };

        let mut body = div()
            .id("body")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .px(px(6.))
            .pb(px(8.))
            .track_scroll(&self.scroll)
            .overflow_y_scroll();

        body = match self.page {
            // Root-list rows stay direct children so `scroll_to_item` can reach
            // them; the action panel floats over the list.
            Page::Browse | Page::Actions => self.render_browse(body, p, cx),
            _ => {
                // Other pages scroll as one block, so their rows never shrink.
                let page = div()
                    .id("page")
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .min_h_full();
                body.child(match self.page {
                    Page::Setup | Page::Locked | Page::Rotate => self.render_auth(page, p, cx),
                    Page::Edit => self.render_edit(page, p, cx),
                    Page::Details => self.render_details(page, p, cx),
                    Page::Delete => self.render_delete(page, p, cx),
                    Page::Appearance | Page::Preview => self.render_appearance(page, p, cx),
                    Page::Settings => self.render_settings(page, p, cx),
                    Page::Health => self.render_health(page, p, cx),
                    Page::Providers => self.render_providers(page, p, cx),
                    Page::Provider => self.render_provider(page, p, cx),
                    Page::DeleteProvider => self.render_delete_provider(page, p, cx),
                    Page::Generator => self.render_generator(page, p, cx),
                    Page::Discard => self.render_discard(page, p, cx),
                    Page::Browse | Page::Actions => unreachable!(),
                })
            }
        };

        // The titlebar is a platform drag region, so it carries no controls
        // besides the window buttons.
        let titlebar = TitleBar::new().child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.color(Color::Muted))
                .child(Icon::LockKeyhole)
                .child("Latch"),
        );
        if self.focus_lands_on_rows()
            && self.focus.is_focused(window)
            && let Some(first) = self.page_stops.borrow().first()
        {
            window.focus(first, cx);
        }
        let auth = matches!(self.page, Page::Locked | Page::Setup);
        let filter_focused = self
            .action_query
            .read(cx)
            .focus_handle(cx)
            .is_focused(window);

        let mut root = div()
            .id("latch")
            .role(Role::Application)
            .aria_label("Latch password manager")
            .key_context("Latch")
            .track_focus(&self.focus)
            .relative()
            .capture_any_mouse_down(cx.listener(|this, _, window, cx| this.activity(window, cx)))
            .on_action(cx.listener(|this, _: &Quit, window, cx| {
                this.lock(window, cx);
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
                if this.page == Page::Actions {
                    this.back(&Back, window, cx);
                } else if !matches!(this.page, Page::Locked | Page::Setup)
                    && !this.busy
                    && !this.guard_draft(window, cx)
                {
                    this.open_actions(window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CopyUsername, _, cx| this.copy_field("username", cx)))
            .on_action(cx.listener(|this, _: &CopyTotp, _, cx| this.copy_field("totp", cx)))
            .on_action(cx.listener(|this, _: &Next, window, cx| this.move_selection(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &Previous, window, cx| this.move_selection(-1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Decrease, window, cx| match this.page {
                    Page::Generator => this.resize_generated(-1, cx),
                    // Confirmation buttons sit side by side.
                    Page::Delete | Page::Discard | Page::DeleteProvider => {
                        this.move_selection(-1, window, cx)
                    }
                    _ => cx.propagate(),
                }),
            )
            .on_action(
                cx.listener(|this, _: &Increase, window, cx| match this.page {
                    Page::Generator => this.resize_generated(1, cx),
                    // Confirmation buttons sit side by side.
                    Page::Delete | Page::Discard | Page::DeleteProvider => {
                        this.move_selection(1, window, cx)
                    }
                    _ => cx.propagate(),
                }),
            )
            .on_action(cx.listener(|this, _: &Tab, window, cx| {
                if this.page == Page::Actions {
                    this.cycle_palette(window, cx, 1);
                    return;
                }
                window.focus_next(cx);
            }))
            .on_action(cx.listener(|this, _: &TabPrevious, window, cx| {
                if this.page == Page::Actions {
                    this.cycle_palette(window, cx, -1);
                    return;
                }
                window.focus_prev(cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .bg(p.color(Color::Background))
            .text_color(p.color(Color::Text))
            .text_size(px(ui::TEXT))
            .child(titlebar)
            .when(!auth, |root| root.child(self.header(p, heading, cx)))
            .child(body)
            .child(self.footer(p, window, cx));
        if self.page == Page::Actions {
            root = root.child(self.palette_overlay(p, filter_focused, cx));
        }
        root
    }
}
