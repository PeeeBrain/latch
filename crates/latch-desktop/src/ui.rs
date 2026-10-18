// Shell design tokens: one flat canvas, hairline separators, one accent.
// Structure comes from spacing, type and the header/list/footer frame, so
// imported editor colors restyle the app without breaking its hierarchy.

use crate::theme::{Color, Palette};
use gpui_kit::{Div, Hsla, SharedString, div, hsla, prelude::*, px};

// Only the icons the shell draws are embedded, including the TitleBar's
// window controls.
gpui_kit::assets::icon_assets!(pub Icons, [
    AtSign, Check, ChevronLeft, ChevronRight, CircleAlert, Clock, Copy, Dices, Download,
    ExternalLink, Eye, Globe, Info, KeyRound, Lock, LockKeyhole, Mail, Palette, Pencil, Plus,
    RefreshCw, Search, Settings, ShieldAlert, ShieldCheck, TriangleAlert, User, WandSparkles,
    WindowClose, WindowMaximize, WindowMinimize, WindowRestore, Trash, FingerprintPattern, FileText,
    RotateCw,
]);

pub use gpui_kit::assets::IconName as Icon;

/// Header (search or title) height.
pub const HEADER: f32 = 52.;
/// Footer bar height.
pub const FOOTER: f32 = 38.;
/// List row height.
pub const ROW: f32 = 40.;
/// Horizontal inset shared by header, list and footer.
pub const INSET: f32 = 14.;
/// Row and field corner radius.
pub const RADIUS: f32 = 8.;
/// Body text size.
pub const TEXT: f32 = 13.5;
/// Caption, accessory and section text size.
pub const TEXT_SM: f32 = 12.;
/// Header input and page title size.
pub const TEXT_LG: f32 = 16.;

/// Maps a password strength label to the status color role used across the UI.
/// Unknown labels default to the success tone so new strength tiers never
/// masquerade as warnings.
pub fn strength_color(label: &str) -> Color {
    let label = label.to_lowercase();
    if label.contains("weak") {
        Color::Error
    } else if label.contains("fair") {
        Color::Warning
    } else {
        Color::Success
    }
}

/// The platform mono face used for secret and data text.
pub fn mono_font() -> SharedString {
    if cfg!(target_os = "windows") {
        "Cascadia Mono".into()
    } else if cfg!(target_os = "macos") {
        "Menlo".into()
    } else {
        "DejaVu Sans Mono".into()
    }
}

/// Neutral wash that reads on any imported background.
pub fn wash(p: &Palette, alpha: f32) -> Hsla {
    if p.dark {
        hsla(0., 0., 1., alpha)
    } else {
        hsla(0., 0., 0., alpha)
    }
}

/// Uppercase-free section label above a group of rows.
pub fn section(p: &Palette, title: impl Into<SharedString>) -> Div {
    div()
        .px(px(8.))
        .pt(px(10.))
        .pb(px(4.))
        .text_size(px(TEXT_SM))
        .font_weight(gpui_kit::FontWeight::MEDIUM)
        .text_color(p.color(Color::Muted))
        .child(title.into())
}

/// Square icon tile used as the leading visual of a row.
pub fn tile(p: &Palette, content: impl IntoElement, tint: Option<Hsla>) -> Div {
    let tint = tint.unwrap_or_else(|| p.color(Color::Muted));
    div()
        .flex_shrink_0()
        .size(px(24.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .bg(tint.opacity(0.14))
        .text_color(tint)
        .text_size(px(13.))
        .child(content)
}

/// Small muted pill for row accessories and data such as codes.
pub fn tag(p: &Palette, value: impl Into<SharedString>) -> Div {
    div()
        .flex_shrink_0()
        .px(px(6.))
        .py(px(1.))
        .rounded(px(5.))
        .bg(wash(p, 0.06))
        .text_size(px(11.))
        .text_color(p.color(Color::Muted))
        .child(value.into())
}

/// Syncs the palette into the shared component theme so the `TitleBar`
/// window controls and `Kbd` hints paint with the shell's colors.
///
/// Runs inside render, so it must use `global_mut`: `Theme::update` refreshes
/// every window on write, which would loop render → refresh → render forever.
pub fn sync_component_theme(p: &Palette, cx: &mut gpui_kit::App) {
    use gpui_kit::component::theme::Theme;
    let theme = Theme::global_mut(cx);
    theme.colors.background = p.color(Color::Background);
    theme.colors.foreground = p.color(Color::Text);
    theme.colors.border = p.color(Color::Border);
    theme.colors.title_bar = p.color(Color::Background);
    theme.colors.title_bar_border = gpui_kit::transparent_black();
    theme.colors.secondary_foreground = p.color(Color::Muted);
    theme.colors.secondary_hover = wash(p, 0.08);
    theme.colors.secondary_active = wash(p, 0.14);
    theme.colors.danger_foreground = gpui_kit::white();
    theme.colors.danger = hsla(0., 0.72, 0.5, 1.);
    theme.colors.danger_active = hsla(0., 0.72, 0.42, 1.);
    theme.mono_font_family = mono_font();
    theme.radius = px(RADIUS);
    theme.radius_lg = px(RADIUS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_labels_map_to_status_colors() {
        assert_eq!(strength_color("Very Weak"), Color::Error);
        assert_eq!(strength_color("Weak"), Color::Error);
        assert_eq!(strength_color("Fair"), Color::Warning);
        assert_eq!(strength_color("Strong"), Color::Success);
        assert_eq!(strength_color("Very Strong"), Color::Success);
    }

    #[test]
    fn fallback_palettes_pass_their_own_contrast_check() {
        assert!(!Palette::fallback(true).low_contrast());
        assert!(!Palette::fallback(false).low_contrast());
    }
}
