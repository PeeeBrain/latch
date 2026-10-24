---
name: Latch
description: A local, keyboard-first password manager in one compact command window.
colors:
  cobalt-edge: "#3f5fe8"
  cobalt-edge-pressed: "#3a59e0"
  cobalt-edge-pressed-light: "#3555dc"
  cobalt-link-dark: "#8ea6ff"
  cobalt-link-light: "#3355d6"
  cobalt-focus-dark: "#6c8cff"
  on-cobalt: "#f5f7ff"
  graphite-canvas: "#17181c"
  graphite-surface: "#1d1e23"
  graphite-popup: "#222328"
  graphite-text: "#ebecf0"
  graphite-muted: "#8d909b"
  graphite-placeholder: "#868994"
  graphite-selected-text: "#f4f5f8"
  paper-canvas: "#f7f7f8"
  paper-surface: "#fcfcfd"
  paper-input: "#fdfdfe"
  paper-text: "#1b1c20"
  paper-muted: "#6b6e78"
  paper-placeholder: "#73767f"
  ink-selected-text-light: "#111216"
  error-dark: "#ff6b6b"
  warning-dark: "#f0b45a"
  success-dark: "#5fd394"
  error-light: "#c93838"
  warning-light: "#9a6400"
  success-light: "#1f7a4a"
typography:
  title:
    fontFamily: "Platform UI font (Segoe UI, SF Pro, system sans)"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: "24px"
  body:
    fontFamily: "Platform UI font (Segoe UI, SF Pro, system sans)"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: "20px"
  label:
    fontFamily: "Platform UI font (Segoe UI, SF Pro, system sans)"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: "18px"
  caption:
    fontFamily: "Platform UI font (Segoe UI, SF Pro, system sans)"
    fontSize: "11px"
    fontWeight: 400
  mono:
    fontFamily: "Cascadia Mono, Menlo, DejaVu Sans Mono"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: "20px"
rounded:
  chip: "4px"
  tag: "5px"
  tile: "6px"
  control: "6px"
  row: "8px"
  popover: "10px"
spacing:
  hair: "4px"
  tight: "6px"
  row-x: "8px"
  gap: "10px"
  inset: "14px"
  header: "52px"
  footer: "38px"
  row: "40px"
components:
  button-primary:
    backgroundColor: "{colors.cobalt-edge}"
    textColor: "{colors.on-cobalt}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "28px"
  button-primary-hover:
    backgroundColor: "{colors.cobalt-edge-pressed}"
    textColor: "{colors.on-cobalt}"
  input:
    backgroundColor: "{colors.graphite-canvas}"
    textColor: "{colors.graphite-text}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "6px 10px"
  list-row:
    textColor: "{colors.graphite-text}"
    typography: "{typography.body}"
    rounded: "{rounded.row}"
    padding: "0 8px"
    height: "40px"
  tag:
    textColor: "{colors.graphite-muted}"
    typography: "{typography.caption}"
    rounded: "{rounded.tag}"
    padding: "1px 6px"
  kbd:
    typography: "{typography.caption}"
    rounded: "{rounded.chip}"
    padding: "1px 5px"
  tile:
    rounded: "{rounded.tile}"
    size: "24px"
---

# Design System: Latch

## 1. Overview

**Creative North Star: "The Quiet Blade"**

One sharp edge, everything else silent. Latch is a single command window: header, list, footer. The only saturated color is Cobalt Edge, and it appears only where something acts: the primary button, the focus ring, a checked box. Everything else is tinted graphite (dark) or paper (light) neutrals, hairline separators, and type. If a screen has two things shouting, one of them is wrong.

The system is built to survive the user's own colors. Users import VS Code, Zed, or T3 Code themes, and those colors replace every role in the palette. Hierarchy therefore never depends on a specific hue. It comes from the fixed frame (52px header, 40px rows, 38px footer), spacing, weight, and neutral washes (white or black at low alpha) that read on any canvas. The fallback palette below is what users see before they import anything, and it has to stand on its own.

It is dense and compact, like Raycast, not roomy like a web app. It rejects 1Password/Bitwarden-style sidebars and vault browsers, SaaS dashboards (cards, stat tiles, tours), anything that reads as an Electron or web app, and security theater (decorative shields and padlocks, green "you are safe" banners, alarm colors).

**Key Characteristics:**
- One window, one frame: title bar, header (search or back + title), scrolling list, footer with the Enter action.
- Restrained color: tinted neutrals plus Cobalt Edge on well under 10% of any screen.
- Flat surfaces; tone shifts carry hover, press, selection, and focus.
- Platform UI font for everything except secrets and codes, which use the platform mono.
- Motion is effectively absent: state changes are instant tone swaps, and nothing repaints while idle.

## 2. Colors

Tinted neutrals with a faint cool cast, one cobalt accent, and three status hues kept for real status. Dark and light fallbacks are separate palettes; the light/dark choice follows the OS.

### Primary
- **Cobalt Edge** (`cobalt-edge`): the primary button fill, checked checkboxes, and nothing decorative. Its pressed/hover tone goes *darker*, not lighter (`cobalt-edge-pressed` in dark, `cobalt-edge-pressed-light` in light), so button text keeps AA contrast.
- **Cobalt Link** (`cobalt-link-dark` / `cobalt-link-light`): the lighter readable cobalt for the footer status icon and accent tiles on primary-action rows (Apply appearance, Use password, Install update).
- **Cobalt Focus** (`cobalt-focus-dark`): the 1px focus border on inputs and buttons, and the text caret.
- **On Cobalt** (`on-cobalt`): text on Cobalt Edge. Never pure white.

### Neutral
- **Graphite Canvas / Paper Canvas** (`graphite-canvas`, `paper-canvas`): the one flat background. Header, list, and footer all sit on it.
- **Graphite / Paper Surface and Popup** (`graphite-surface`, `graphite-popup`, `paper-surface`): the Actions popover and other raised surfaces, one step off the canvas.
- **Graphite / Paper Text** (`graphite-text`, `paper-text`): primary text, titles, row labels.
- **Graphite / Paper Muted** (`graphite-muted`, `paper-muted`): subtitles, section labels, captions, form labels, ghost buttons, footer hints. Holds 4.5:1 or better on the canvas.
- **Placeholder** (`graphite-placeholder`, `paper-placeholder`): input hints. Holds 4.5:1 on the input fill.
- **Washes**: hover, selection, borders, input fills, and chips are white (dark) or black (light) at low alpha (about 4–9%), so they read on any imported canvas.

### Tertiary: status
- **Error** (`error-dark` / `error-light`): destructive actions (Delete, Discard) and breached passwords.
- **Warning** (`warning-dark` / `warning-light`): weak or reused passwords, and auth notices such as a wrong password or a Lockout countdown.
- **Success** (`success-dark` / `success-light`): the password strength label only.

### Named Rules
**The One Edge Rule.** Cobalt marks what acts or what is focused. A cobalt tile, icon, or wash that is not a primary action, focus, or checked state is prohibited.

**The No Pure Extremes Rule.** `#ffffff` and `#000000` are forbidden as palette values. Every neutral carries the faint cool tint. Low-alpha washes are the only exception, because they are overlays, not colors.

**The Calm Status Rule.** Status colors describe real state. "Nothing wrong" is plain muted text, never a green badge. Red is for destruction and breaches only.

## 3. Typography

**UI Font:** the platform's system UI font (GPUI default: Segoe UI on Windows, SF Pro on macOS, system sans on Linux)
**Mono Font:** Cascadia Mono (Windows), Menlo (macOS), DejaVu Sans Mono (Linux)

**Character:** native and unbranded on purpose. Latch should look like it belongs to the OS, so it never ships a display face. Mono marks data the user will copy or compare: passwords, TOTP codes, generated passwords, the generator length.

### Hierarchy
- **Title** (600, 16px, 24px line): page headings in the header, the auth page title, the credential name in details, confirmation titles. The search query also uses 16px, at regular weight.
- **Body** (400, 13.5px, 20px line): row titles, input text, everything a user reads to decide.
- **Label** (500, 12px, 18px line): buttons, section labels (sentence case, never uppercase), footer text. Form labels and captions use the same 12px at regular weight, in muted color.
- **Caption** (400, 11px): tags (item count, "2FA", "Default") and keyboard hints.

The steps run 11, 12, 13.5, 16 (ratios of roughly 1.1 to 1.2), the tight scale a dense tool needs. Weight carries the rest of the hierarchy.

### Named Rules
**The Four Sizes Rule.** Text uses `TEXT_LG`, `TEXT`, `TEXT_SM`, or the 11px caption. Off-scale sizes (`TEXT_SM + 0.5`, `TEXT_LG - 1`, ad hoc 15/17/18px) are prohibited.

**The Mono Means Data Rule.** Mono is for secrets, codes, and values a user copies. Never for labels or decoration.

## 4. Elevation

Flat by default. The canvas, header, list, and footer are one plane, divided by 1px hairlines (a wash at about 8% alpha). Depth comes from tone: rows gain a wash on hover and a stronger one when selected or focused. The only shadow in the system belongs to the Actions popover, the one element that actually floats above the list, and it comes with a light scrim behind it.

### Shadow Vocabulary
- **Popover lift** (`0 10px 30px -6px` black at 50% in dark, 18% in light): the Actions panel only.
- **Popover scrim** (black at 25% in dark, 6% in light, full window): behind the Actions panel, click to dismiss.

### Named Rules
**The One Float Rule.** Only an element that overlaps content gets a shadow. Buttons, inputs, rows, and tiles stay flat at rest and in every state.

## 5. Components

Compact and tactile: small dense targets, with clear hover and press tone shifts and no ornament.

### Buttons
- **Shape:** gently rounded (6px), 28px tall, 12px side padding, 12px medium label, optional leading icon with 6px gap.
- **Primary:** Cobalt Edge fill, On Cobalt text. One per screen at most: Create vault, Unlock, Save credential.
- **Hover / Press / Focus:** hover darkens the fill; press drops opacity to 85%; focus shows a 1px Cobalt Focus border; disabled sits at 45% opacity.
- **Secondary:** a 5% wash with a hairline border and primary text (Cancel, generator −/+).
- **Ghost:** transparent with muted text and a 7% wash on hover. Used for inline field actions (Alias, Generate, Reveal, Copy).
- **Danger:** error text on a 12% error tint with a 30% error border. Only on confirmation pages.

### List Rows (signature component)
- **Shape:** full width, 40px tall (34px in the Actions popover), 8px radius, 8px side padding, 10px gap.
- **Anatomy:** leading 24px tile, title, optional inline muted subtitle, then accessories on the right (tag, keyboard hint, chevron, copy icon).
- **States:** hover adds the Hover wash; selected and keyboard-focused share the Selected wash with Selected Text; press on an unselected row flashes the Selected fill.
- **Destructive row:** Delete credential uses error text and an error-tinted tile; that is the only colored row.

### Tiles
- **Style:** 24px rounded square (6px), the glyph at full tint over the same tint at 14%. Neutral muted by default.
- **Credential tiles:** the site favicon when available, otherwise the title's initial on a stable hue derived from the title (55% saturation, lighter in dark mode). These are the only multi-hue element, and they identify credentials, not decorate them.

### Tags and Keyboard Hints
- **Tag:** 11px muted text on a 6% wash, 5px radius, 1px × 6px padding. For counts and flags ("3 items", "2FA", "Default", TOTP seconds).
- **Kbd:** 11px on a 7% wash, 4px radius, 1px × 5px padding, platform-formatted (Ctrl/Cmd). Every shortcut is shown next to the thing it triggers.

### Inputs / Fields
- **Style:** input wash fill, 1px hairline border, 6px radius, 6px × 10px padding, 13.5px body text.
- **Focus:** the border switches to Cobalt Focus. No glow, no ring offset.
- **Search field:** borderless and bare in the header at 16px, led by a muted search icon.
- **Secrets:** masked values render as dots and never enter shaped text. Revealed passwords use mono.

### Navigation
- **Header:** 52px, with a hairline beneath. On Browse it holds the search field and the item count; elsewhere a 26px back chip (6% wash, chevron) and the page title. Escape always goes back.
- **Footer:** 38px, with a hairline above. On the left, a status line (Cobalt Link icon plus text) that is empty when idle and on pages that show notices inline. On the right, the Enter action label with its key, a divider, and "Actions" with Ctrl/Cmd+K. On pages without a primary action it shows "Navigate ↑ ↓ Select Enter".
- **Section labels:** 12px medium muted text, sentence case, 10px above and 4px below.

### Actions Popover
- 360px wide, up to 360px tall, anchored 10px from the right just above the footer, 10px radius, Popup fill, hairline border, Popover lift shadow. Compact rows above a bottom-docked filter field.

## 6. Do's and Don'ts

### Do:
- **Do** keep every screen inside the header/list/footer frame, and make the footer tell the truth about what Enter does.
- **Do** use Cobalt Edge only for the primary button, focus, and checked state, on well under 10% of the screen.
- **Do** express hover, selection, and borders as low-alpha washes so imported themes keep the hierarchy.
- **Do** hold 4.5:1 contrast for text, placeholders, and button labels in both fallback palettes. For imported themes, `Palette::button_text` picks readable primary-button labels. `low_contrast` warns about four text/background pairs in Preview; it does not change colors or check placeholders.
- **Do** say "nothing wrong" in plain muted text, and keep status color for real state.
- **Do** show each message once. The footer status line is the default channel; auth pages and Edit show notices inline instead (Edit so long validation messages wrap rather than truncate beside Save), and leave the footer status empty.
- **Do** keep copy terse. Placeholders already say "(optional)", so the captions below don't repeat it.

### Don't:
- **Don't** build 1Password / Bitwarden layouts: no sidebars, no vault browser, no multi-pane chrome.
- **Don't** build a SaaS dashboard: no cards, stat tiles, onboarding tours, or upsell surfaces.
- **Don't** let it read as an Electron or web app: no web form styling, sluggish transitions, or generic component kits.
- **Don't** add security theater: no shields or padlocks as decoration, no green "you are safe" banners, no scare colors, no hacker-terminal look.
- **Don't** put a big icon tile above a heading (the "hero icon in a rounded square" template), on auth, empty, or confirmation screens.
- **Don't** use `#ffffff` or `#000000` as palette colors.
- **Don't** add shadows to anything that doesn't float over content.
- **Don't** use off-scale font sizes or a display face.
- **Don't** use a colored side stripe (`border-left` over 1px) on rows, callouts, or alerts.
- **Don't** animate anything that repaints while idle, or add motion that doesn't convey a state change.
