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
#[derive(Clone, Copy)]
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
        let values = if dark {
            [
                0x181c24, 0x232934, 0x282f3b, 0xe4e8ef, 0x9ba4b4, 0x202631, 0xe4e8ef, 0x9298a4,
                0x373d49, 0x303642, 0xe4e8ef, 0x303950, 0xffffff, 0x435680, 0xffffff, 0x52699b,
                0xabbcff, 0xa7b7ff, 0x435680, 0xff9595, 0xd9b879, 0x94d5a7, 0x181c24, 0x9ba4b4,
            ]
        } else {
            [
                0xf8f9fb, 0xffffff, 0xffffff, 0x232936, 0x5c6575, 0xffffff, 0x232936, 0x6b7380,
                0xc8ced8, 0xe9edf4, 0x232936, 0xdce5fb, 0x1b2947, 0x385ca8, 0xffffff, 0x294b94,
                0x294f9b, 0x385ca8, 0xb7cbf3, 0xa42b35, 0x855700, 0x246b3f, 0xf1f3f7, 0x5c6575,
            ]
        };
        Self {
            colors: values.map(|value| rgb(value).into()),
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
}
