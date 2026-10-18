use gpui_kit::{Global, Hsla, rgb, rgba};
use serde_json::{Map, Value};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;
#[derive(Clone, Copy, PartialEq)]
pub enum Source {
    VsCode,
    Zed,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    Background,
    Surface,
    Popup,
    Text,
    Muted,
    Input,
    InputText,
    Placeholder,
    Border,
    Hover,
    HoverText,
    Selected,
    SelectedText,
    Button,
    ButtonText,
    ButtonHover,
    Link,
    Focus,
    Selection,
    Error,
    Warning,
    Success,
    Footer,
    FooterText,
}
const MAPPING: &[(&[&str], &[&str])] = &[
    (&["editor.background"], &["background", "editor.background"]),
    (
        &["sideBar.background", "panel.background"],
        &["surface.background", "panel.background"],
    ),
    (
        &["editorWidget.background", "quickInput.background"],
        &["elevated_surface.background"],
    ),
    (
        &["foreground", "editor.foreground"],
        &["text", "editor.foreground"],
    ),
    (&["descriptionForeground"], &["text.muted"]),
    (&["input.background"], &["element.background"]),
    (&["input.foreground"], &["text"]),
    (&["input.placeholderForeground"], &["text.placeholder"]),
    (
        &[
            "panel.border",
            "sideBar.border",
            "widget.border",
            "input.border",
        ],
        &["border", "border.variant"],
    ),
    (&["list.hoverBackground"], &["element.hover"]),
    (&["list.hoverForeground", "foreground"], &["text"]),
    (&["list.activeSelectionBackground"], &["element.selected"]),
    (&["list.activeSelectionForeground", "foreground"], &["text"]),
    (&["button.background"], &["element.selected"]),
    (&["button.foreground"], &["text"]),
    (&["button.hoverBackground"], &["element.hover"]),
    (&["textLink.foreground", "focusBorder"], &["text.accent"]),
    (&["focusBorder"], &["border.focused"]),
    (
        &["selection.background", "editor.selectionBackground"],
        &["element.selection_background"],
    ),
    (&["errorForeground", "editorError.foreground"], &["error"]),
    (
        &[
            "editorWarning.foreground",
            "notificationsWarningIcon.foreground",
        ],
        &["warning"],
    ),
    (&["testing.iconPassed"], &["success"]),
    (&["statusBar.background"], &["status_bar.background"]),
    (&["statusBar.foreground"], &["text.muted"]),
];
#[derive(Clone)]
pub struct Palette {
    colors: [Hsla; 24],
    /// Whether this palette targets the dark appearance; neutral washes in
    /// the shell pick light or dark overlays based on it.
    pub dark: bool,
}
impl Global for Palette {}
impl Palette {
    pub fn low_contrast(&self) -> bool {
        use gpui_kit::Rgba;
        let base = Rgba::from(self.color(Color::Background));
        [
            (Color::Text, Color::Background),
            (Color::InputText, Color::Input),
            (Color::SelectedText, Color::Selected),
            (Color::ButtonText, Color::Button),
        ]
        .into_iter()
        .any(|(text, background)| {
            let background = base.blend(self.color(background).into());
            let text = background.blend(self.color(text).into());
            let luminance = |color: Rgba| {
                let linear = |channel: f32| {
                    if channel <= 0.04045 {
                        channel / 12.92
                    } else {
                        ((channel + 0.055) / 1.055).powf(2.4)
                    }
                };
                linear(color.r) * 0.2126 + linear(color.g) * 0.7152 + linear(color.b) * 0.0722
            };
            let a = luminance(text);
            let b = luminance(background);
            (a.max(b) + 0.05) / (a.min(b) + 0.05) < 4.5
        })
    }
    pub fn fallback(dark: bool) -> Self {
        // RGBA: translucent roles (borders, hovers, selection washes) carry a
        // low alpha so they read on any canvas.
        let values = if dark {
            [
                0x17181cff, 0x1d1e23ff, 0x222328ff, 0xebecf0ff, 0x8d909bff, 0xffffff0a, 0xebecf0ff,
                0x6b6e79ff, 0xffffff14, 0xffffff0b, 0xebecf0ff, 0xffffff17, 0xffffffff, 0x3f5fe8ff,
                0xffffffff, 0x4a6af0ff, 0x8ea6ffff, 0x6c8cffff, 0x6c8cff4d, 0xff6b6bff, 0xf0b45aff,
                0x5fd394ff, 0x17181cff, 0x8d909bff,
            ]
        } else {
            [
                0xf7f7f8ff, 0xffffffff, 0xffffffff, 0x1b1c20ff, 0x6b6e78ff, 0xffffffff, 0x1b1c20ff,
                0x9a9ca5ff, 0x0000001a, 0x0000000a, 0x1b1c20ff, 0x00000012, 0x111216ff, 0x3f5fe8ff,
                0xffffffff, 0x3555dcff, 0x3355d6ff, 0x3f5fe8ff, 0x3f5fe833, 0xc93838ff, 0x9a6400ff,
                0x1f7a4aff, 0xf7f7f8ff, 0x6b6e78ff,
            ]
        };
        Self {
            colors: values.map(|value| rgba(value).into()),
            dark,
        }
    }
    pub fn color(&self, color: Color) -> Hsla {
        self.colors[color as usize]
    }
}
pub struct Appearance {
    pub document: Value,
    pub source: Source,
}
impl Appearance {
    pub fn parse(text: &str, source: Source) -> Result<Self, String> {
        if text.len() > 256 * 1024 {
            return Err("Settings file exceeds 256 KiB".into());
        }
        bound_depth(text)?;
        let value: Value = jsonc_parser::parse_to_serde_value(
            text,
            &jsonc_parser::ParseOptions {
                allow_comments: true,
                allow_trailing_commas: true,
                allow_loose_object_property_names: false,
                allow_missing_commas: false,
                allow_single_quoted_strings: false,
                allow_hexadecimal_numbers: false,
                allow_unary_plus_numbers: false,
            },
        )
        .map_err(|_| "Invalid JSONC settings".to_string())?;
        let object = value.as_object().ok_or("Settings must be a JSON object")?;
        let converted = theme_file_as_settings(object, source);
        let object = converted.as_ref().unwrap_or(object);
        let mut clean = Map::new();
        let keys: &[&str] = match source {
            Source::VsCode => &[
                "workbench.colorTheme",
                "workbench.preferredDarkColorTheme",
                "workbench.preferredLightColorTheme",
                "window.autoDetectColorScheme",
            ],
            Source::Zed => &["theme"],
        };
        for key in keys {
            if let Some(value) = object.get(*key) {
                if value.is_string() || value.is_boolean() {
                    clean.insert((*key).into(), value.clone());
                } else if *key == "theme" {
                    let mut selection = Map::new();
                    for field in ["mode", "light", "dark"] {
                        if let Some(value) = value.get(field).filter(|value| value.is_string()) {
                            selection.insert(field.into(), value.clone());
                        }
                    }
                    clean.insert((*key).into(), Value::Object(selection));
                }
            }
        }
        match source {
            Source::VsCode => {
                if let Some(colors) = object.get("workbench.colorCustomizations") {
                    clean.insert(
                        "workbench.colorCustomizations".into(),
                        filter_colors(colors, source, true)?,
                    );
                }
            }
            Source::Zed => {
                if let Some(overrides) = object.get("theme_overrides") {
                    let mut blocks = Map::new();
                    for (name, colors) in overrides
                        .as_object()
                        .ok_or("theme_overrides must be an object")?
                    {
                        blocks.insert(name.clone(), filter_colors(colors, source, false)?);
                    }
                    clean.insert("theme_overrides".into(), Value::Object(blocks));
                }
                if let Some(colors) = object.get("experimental.theme_overrides") {
                    clean.insert(
                        "experimental.theme_overrides".into(),
                        filter_colors(colors, source, false)?,
                    );
                }
            }
        }
        Ok(Self {
            document: Value::Object(clean),
            source,
        })
    }
    pub fn palette(&self, dark: bool) -> Result<(Palette, usize), String> {
        let mut colors = Map::new();
        match self.source {
            Source::VsCode => {
                let key = if dark {
                    "workbench.preferredDarkColorTheme"
                } else {
                    "workbench.preferredLightColorTheme"
                };
                let name = if self.document["window.autoDetectColorScheme"] == true {
                    self.document[key]
                        .as_str()
                        .or(self.document["workbench.colorTheme"].as_str())
                } else {
                    self.document["workbench.colorTheme"].as_str()
                }
                .unwrap_or("");
                if let Some(overrides) = self.document["workbench.colorCustomizations"].as_object()
                {
                    for (key, value) in overrides {
                        if !key.starts_with('[') {
                            colors.insert(key.clone(), value.clone());
                        }
                    }
                    for (selector, block) in overrides {
                        if selector.starts_with('[')
                            && matches_selector(selector, name)
                            && let Some(block) = block.as_object()
                        {
                            colors.extend(block.clone());
                        }
                    }
                }
            }
            Source::Zed => {
                if let Some(global) = self.document["experimental.theme_overrides"].as_object() {
                    colors.extend(global.clone());
                }
                let theme = &self.document["theme"];
                let selected = theme
                    .as_str()
                    .or_else(|| {
                        theme[match theme["mode"].as_str() {
                            Some("light") => "light",
                            Some("dark") => "dark",
                            _ => {
                                if dark {
                                    "dark"
                                } else {
                                    "light"
                                }
                            }
                        }]
                        .as_str()
                    })
                    .unwrap_or("");
                if let Some(block) = self.document["theme_overrides"][selected].as_object() {
                    colors.extend(block.clone());
                }
            }
        }
        let mut palette = Palette::fallback(dark);
        let mut applied = 0;
        for (role, (vs, zed)) in [
            Color::Background,
            Color::Surface,
            Color::Popup,
            Color::Text,
            Color::Muted,
            Color::Input,
            Color::InputText,
            Color::Placeholder,
            Color::Border,
            Color::Hover,
            Color::HoverText,
            Color::Selected,
            Color::SelectedText,
            Color::Button,
            Color::ButtonText,
            Color::ButtonHover,
            Color::Link,
            Color::Focus,
            Color::Selection,
            Color::Error,
            Color::Warning,
            Color::Success,
            Color::Footer,
            Color::FooterText,
        ]
        .into_iter()
        .zip(MAPPING)
        {
            let index = role as usize;
            for token in if self.source == Source::VsCode {
                *vs
            } else {
                *zed
            } {
                if let Some(value) = colors.get(*token) {
                    applied += 1;
                    if value.as_str() != Some("default") {
                        palette.colors[index] =
                            parse_color(value.as_str().ok_or("Color must be a hex string")?)?;
                    }
                    break;
                }
            }
        }
        // Neutral washes follow the imported canvas, not the OS appearance.
        palette.dark = palette.color(Color::Background).l < 0.5;
        Ok((palette, applied))
    }
    pub fn save(&self, directory: &Path) -> Result<(), String> {
        fs::create_dir_all(directory).map_err(|_| "Cannot create appearance directory")?;
        let bytes =
            serde_json::to_vec_pretty(&self.document).map_err(|_| "Cannot encode appearance")?;
        atomic_write(&directory.join("settings.json"), &bytes)?;
        // The managed file contains only one editor's existing schema, so recovery can
        // infer its format if writing this non-secret metadata is interrupted.
        atomic_write(
            &directory.join("appearance-source"),
            if self.source == Source::VsCode {
                b"vscode"
            } else {
                b"zed"
            },
        )?;
        atomic_write(&directory.join("settings.last-valid.json"), &bytes)
    }
    pub fn load(directory: &Path) -> Result<Option<Self>, String> {
        let path = directory.join("settings.json");
        if !path.exists() {
            return Ok(None);
        }
        let text = read(&path)?;
        let source = infer(&text)?;
        Self::parse(&text, source).map(Some)
    }
}
fn infer(text: &str) -> Result<Source, String> {
    // Infer only the appearance-only managed file, never a user's imported document.
    bound_depth(text)?;
    let value: Value = jsonc_parser::parse_to_serde_value(
        text,
        &jsonc_parser::ParseOptions {
            allow_comments: true,
            allow_trailing_commas: true,
            allow_loose_object_property_names: false,
            allow_missing_commas: false,
            allow_single_quoted_strings: false,
            allow_hexadecimal_numbers: false,
            allow_unary_plus_numbers: false,
        },
    )
    .map_err(|_| "Invalid JSONC settings".to_string())?;
    if value.get("workbench.colorCustomizations").is_some() {
        Ok(Source::VsCode)
    } else if value.get("theme_overrides").is_some()
        || value.get("experimental.theme_overrides").is_some()
    {
        Ok(Source::Zed)
    } else {
        Err("Appearance file has no supported color overrides".into())
    }
}
/// Theme files users download share the color keys of settings overrides, so
/// they are rewritten as the equivalent editor settings. Unparseable colors in
/// a theme file are skipped rather than rejecting the whole theme.
fn theme_file_as_settings(
    object: &Map<String, Value>,
    source: Source,
) -> Option<Map<String, Value>> {
    let usable = |colors: &Value| -> Value {
        Value::Object(
            colors
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, value)| {
                    value
                        .as_str()
                        .is_some_and(|color| parse_color(color).is_ok())
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    };
    let mut settings = Map::new();
    match source {
        // Zed theme family: {"themes": [{"name", "appearance", "style"}]}.
        Source::Zed => {
            let themes = object.get("themes")?.as_array()?;
            let pick = |appearance: &str| {
                themes
                    .iter()
                    .find(|theme| theme["appearance"] == appearance)
                    .and_then(|theme| {
                        Some((theme["name"].as_str()?.to_owned(), usable(&theme["style"])))
                    })
            };
            let (dark, light) = (pick("dark"), pick("light"));
            let mut overrides = Map::new();
            let theme = match (&dark, &light) {
                (Some((dark, _)), Some((light, _))) => {
                    serde_json::json!({"mode": "system", "dark": dark, "light": light})
                }
                (Some((name, _)), None) | (None, Some((name, _))) => Value::String(name.clone()),
                (None, None) => return None,
            };
            for (name, style) in [dark, light].into_iter().flatten() {
                overrides.insert(name, style);
            }
            settings.insert("theme".into(), theme);
            settings.insert("theme_overrides".into(), Value::Object(overrides));
        }
        Source::VsCode => {
            let colors = object.get("colors")?;
            if object.contains_key("workbench.colorCustomizations") {
                return None;
            }
            // Dotted color ids identify VS Code; T3 uses a flat palette with canvas.
            if colors.as_object()?.keys().any(|key| key.contains('.')) {
                settings.insert("workbench.colorCustomizations".into(), usable(colors));
            } else if colors.get("canvas").is_some() {
                let appearance = object.get("appearance")?.as_str()?;
                let pick = |mode: &str| {
                    if appearance == mode {
                        Some(t3_colors(colors))
                    } else {
                        object.get("variants")?.get(mode).map(t3_colors)
                    }
                };
                match (pick("dark"), pick("light")) {
                    (Some(dark), Some(light)) => {
                        // Fixed names avoid treating a downloaded name as a selector glob.
                        settings.extend(serde_json::json!({
                            "window.autoDetectColorScheme": true,
                            "workbench.colorTheme": if appearance == "dark" { "T3 Dark" } else { "T3 Light" },
                            "workbench.preferredDarkColorTheme": "T3 Dark",
                            "workbench.preferredLightColorTheme": "T3 Light",
                            "workbench.colorCustomizations": {"[T3 Dark]": dark, "[T3 Light]": light}
                        }).as_object()?.clone());
                    }
                    (Some(colors), None) | (None, Some(colors)) => {
                        settings.insert("workbench.colorCustomizations".into(), colors);
                    }
                    (None, None) => return None,
                }
            } else {
                return None;
            }
        }
    }
    Some(settings)
}

fn t3_colors(colors: &Value) -> Value {
    let mapping: &[(&str, &[&str])] = &[
        ("editor.background", &["canvas"]),
        ("sideBar.background", &["surface"]),
        (
            "editorWidget.background",
            &["surfaceOverlay", "surfaceRaised"],
        ),
        ("foreground", &["text"]),
        ("descriptionForeground", &["textMuted"]),
        ("input.background", &["input"]),
        ("input.foreground", &["text"]),
        ("input.placeholderForeground", &["placeholder", "textMuted"]),
        ("panel.border", &["border"]),
        (
            "list.hoverBackground",
            &["sidebarRowHover", "toolbarControlHover", "surfaceRaised"],
        ),
        ("list.hoverForeground", &["text"]),
        (
            "list.activeSelectionBackground",
            &["sidebarRowSelected", "sidebarRowActive", "accentSurface"],
        ),
        ("list.activeSelectionForeground", &["sidebarForeground"]),
        ("button.background", &["accent"]),
        ("button.foreground", &["accentForeground"]),
        ("button.hoverBackground", &["messageActionHover", "accent"]),
        ("textLink.foreground", &["accent"]),
        ("focusBorder", &["focus"]),
        ("selection.background", &["terminalSelection"]),
        ("errorForeground", &["error"]),
        ("editorWarning.foreground", &["warning"]),
        ("statusBar.background", &["chrome"]),
        ("statusBar.foreground", &["textMuted"]),
    ];
    Value::Object(
        mapping
            .iter()
            .filter_map(|(target, keys)| {
                let color = keys.iter().find_map(|key| colors.get(*key))?.as_str()?;
                let hex = if color.starts_with("oklch(") {
                    oklch_as_hex(color)?
                } else {
                    parse_color(color).ok()?;
                    color.to_owned()
                };
                Some(((*target).into(), Value::String(hex)))
            })
            .collect(),
    )
}

fn oklch_as_hex(value: &str) -> Option<String> {
    let mut components = value
        .strip_prefix("oklch(")?
        .strip_suffix(')')?
        .split_whitespace();
    let mut next = || {
        components
            .next()?
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    };
    let (lightness, chroma, hue) = (next()?, next()?, next()?.to_radians());
    if components.next().is_some() || chroma < 0.0 {
        return None;
    }
    let (a, b) = (chroma * hue.cos(), chroma * hue.sin());
    // OKLab inverse matrices: https://bottosson.github.io/posts/oklab/
    let l = (lightness + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (lightness - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (lightness - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    let linear = [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ];
    if !linear.iter().all(|channel| channel.is_finite()) {
        return None;
    }
    let [r, g, b] = linear.map(|channel| {
        let channel = channel.clamp(0.0, 1.0);
        let srgb = if channel <= 0.0031308 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u8
    });
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn filter_colors(value: &Value, source: Source, selectors: bool) -> Result<Value, String> {
    let mut clean = Map::new();
    for (key, value) in value
        .as_object()
        .ok_or("Color overrides must be an object")?
    {
        if selectors && key.starts_with('[') {
            clean.insert(key.clone(), filter_colors(value, source, false)?);
            continue;
        }
        if MAPPING.iter().any(|(vs, zed)| {
            if source == Source::VsCode {
                vs.contains(&key.as_str())
            } else {
                zed.contains(&key.as_str())
            }
        }) {
            let color = value
                .as_str()
                .ok_or("Color override must be a hex string")?;
            if !(source == Source::VsCode && color == "default") {
                parse_color(color)?;
            }
            clean.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(clean))
}
fn matches_selector(selector: &str, name: &str) -> bool {
    selector
        .split('[')
        .filter_map(|part| part.strip_suffix(']'))
        .any(|pattern| {
            let start = pattern.starts_with('*');
            let end = pattern.ends_with('*');
            let pattern = pattern.trim_matches('*');
            match (start, end) {
                (true, true) => name.contains(pattern),
                (true, false) => name.ends_with(pattern),
                (false, true) => name.starts_with(pattern),
                _ => name == pattern,
            }
        })
}
fn parse_color(value: &str) -> Result<Hsla, String> {
    let hex = value
        .strip_prefix('#')
        .ok_or("Colors must use #RGB, #RGBA, #RRGGBB or #RRGGBBAA")?;
    if !matches!(hex.len(), 3 | 4 | 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid hexadecimal color".into());
    }
    let expanded = if hex.len() < 5 {
        hex.chars().flat_map(|ch| [ch, ch]).collect::<String>()
    } else {
        hex.into()
    };
    let number = u32::from_str_radix(&expanded, 16).map_err(|_| "Invalid color")?;
    Ok(if expanded.len() == 6 {
        rgb(number).into()
    } else {
        rgba(number).into()
    })
}
fn bound_depth(text: &str) -> Result<(), String> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        if ch == '"' {
            quoted = true;
        } else if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == '\n' {
                    break;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(ch) = chars.next() {
                if ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else if ch == '{' || ch == '[' {
            depth += 1;
            if depth > 64 {
                return Err("Settings nesting exceeds 64 levels".into());
            }
        } else if ch == '}' || ch == ']' {
            depth = depth.saturating_sub(1);
        }
    }
    Ok(())
}
pub fn read(path: &Path) -> Result<Zeroizing<String>, String> {
    let file = fs::File::open(path).map_err(|_| "Cannot open settings file")?;
    let mut text = Zeroizing::new(String::new());
    file.take(256 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(|_| "Settings file must contain UTF-8 text")?;
    if text.len() > 256 * 1024 {
        return Err("Settings file exceeds 256 KiB".into());
    }
    Ok(text)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path.parent().ok_or("Invalid file location")?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(directory).map_err(|_| "Cannot create temporary file")?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| "Cannot save file")?;
    temporary.persist(path).map_err(|_| "Cannot replace file")?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_settings_select_sanitize_validate_and_roundtrip() {
        let vs = r##"{ // user's full settings
          "workbench.colorTheme":"My Dark", "unrelated.secret":"do-not-retain",
          "workbench.colorCustomizations":{"editor.background":"#123", "[My*][Other]":{"foreground":"#abcd", "input.background":"default"},},
        }"##;
        let appearance = Appearance::parse(vs, Source::VsCode).unwrap();
        let (palette, count) = appearance.palette(true).unwrap();
        assert!(count >= 3);
        assert_eq!(
            palette.color(Color::Background),
            parse_color("#112233").unwrap()
        );
        assert_eq!(
            palette.color(Color::Text),
            parse_color("#aabbccdd").unwrap()
        );
        assert!(!appearance.document.to_string().contains("do-not-retain"));
        let zed = r##"{"theme":{"mode":"system","light":"Light","dark":"Dark"},"experimental.theme_overrides":{"text":"#fff"},"theme_overrides":{"Dark":{"text":"#12345678"},"Light":{"background":"#eee"}}}"##;
        let zed = Appearance::parse(zed, Source::Zed).unwrap();
        assert_eq!(
            zed.palette(true).unwrap().0.color(Color::Text),
            parse_color("#12345678").unwrap()
        );
        assert_eq!(
            zed.palette(false).unwrap().0.color(Color::Background),
            parse_color("#eee").unwrap()
        );
        for invalid in [
            "{'theme':'x'}",
            "{unquoted:42}",
            "{\"a\":1 \"b\":2}",
            r##"{"theme_overrides":{"Dark":{"text":"red"}}}"##,
        ] {
            assert!(Appearance::parse(invalid, Source::Zed).is_err());
        }
        assert!(Appearance::parse(&"[".repeat(65), Source::Zed).is_err());
        let directory = tempfile::tempdir().unwrap();
        zed.save(directory.path()).unwrap();
        let loaded = Appearance::load(directory.path()).unwrap().unwrap();
        assert_eq!(loaded.document, zed.document);
        assert!(
            Appearance::parse("{\"theme\":\"Dark\"}", Source::Zed)
                .unwrap()
                .palette(true)
                .unwrap()
                .1
                == 0
        );
    }

    #[test]
    fn t3_theme_imports_variants_and_normalizes_colors() {
        let theme = serde_json::json!({
            "version": 1, "name": "[*]", "appearance": "dark",
            "colors": {
                "canvas": "#101820", "accent": "oklch(0.627955 0.257683 29.2339)",
                "accentSurface": "#432123", "error": "unsupported",
                "warning": "oklch(NaN 0 0)"
            },
            "variants": {"light": {
                "canvas": "#fafafa", "accent": "#33669980",
                "surface": "oklch(0.5 0 0)"
            }}
        });
        let appearance = Appearance::parse(&theme.to_string(), Source::VsCode).unwrap();
        for (dark, background, button) in [
            (true, "#101820", "#ff0000"),
            (false, "#fafafa", "#33669980"),
        ] {
            let (palette, count) = appearance.palette(dark).unwrap();
            assert!(count > 0);
            assert_eq!(
                palette.color(Color::Background),
                parse_color(background).unwrap()
            );
            assert_eq!(palette.color(Color::Button), parse_color(button).unwrap());
            assert_eq!(
                palette.color(Color::Error),
                Palette::fallback(dark).color(Color::Error)
            );
            assert_eq!(
                palette.color(Color::Warning),
                Palette::fallback(dark).color(Color::Warning)
            );
        }
        assert_eq!(
            appearance.palette(true).unwrap().0.color(Color::Selected),
            parse_color("#432123").unwrap()
        );
        assert_eq!(
            appearance.palette(false).unwrap().0.color(Color::Surface),
            parse_color("#636363").unwrap()
        );
        assert!(!appearance.document.to_string().contains("oklch"));
        let directory = tempfile::tempdir().unwrap();
        appearance.save(directory.path()).unwrap();
        let loaded = Appearance::load(directory.path()).unwrap().unwrap();
        assert_eq!(loaded.document, appearance.document);

        let mut single = theme;
        single.as_object_mut().unwrap().remove("variants");
        let single = Appearance::parse(&single.to_string(), Source::VsCode).unwrap();
        for dark in [true, false] {
            assert_eq!(
                single.palette(dark).unwrap().0.color(Color::Button),
                parse_color("#ff0000").unwrap()
            );
        }
    }

    #[test]
    fn downloaded_theme_files_import_as_editor_settings() {
        let zed = r##"{"name":"Pack","themes":[
          {"name":"Pack Light","appearance":"light","style":{"background":"#eeeeee"}},
          {"name":"Pack Dark","appearance":"dark","style":{"background":"#101010","text":null,"border":"oklch(1 0 0)"}}
        ]}"##;
        let zed = Appearance::parse(zed, Source::Zed).unwrap();
        let (dark, count) = zed.palette(true).unwrap();
        assert_eq!(count, 1, "unparseable theme colors are skipped");
        assert_eq!(
            dark.color(Color::Background),
            parse_color("#101010").unwrap()
        );
        assert_eq!(
            zed.palette(false).unwrap().0.color(Color::Background),
            parse_color("#eeeeee").unwrap()
        );
        let vs = r##"{"name":"Mine","type":"dark","colors":{"editor.background":"#202020"},"tokenColors":[]}"##;
        let (palette, count) = Appearance::parse(vs, Source::VsCode)
            .unwrap()
            .palette(true)
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            palette.color(Color::Background),
            parse_color("#202020").unwrap()
        );
    }
}
