# Product

## Register

product

## Users

Keyboard power users: developers and Raycast/Alfred people who live in hotkeys. They summon Latch with Ctrl/Cmd+Space while working in another app, need a credential in a second or two, and leave. They don't want explanations, onboarding, or confirmation chatter. They do notice a missed hotkey, a clipboard that doesn't clear, a lock that fires mid-edit, or a window that repaints while idle.

## Product Purpose

Latch is a local, keyboard-first password manager. One command window handles search, copy, edit, generate, health checks, and settings. The vault is a single encrypted file on disk: no cloud, no account. It succeeds when finding and copying a credential takes less time than thinking about it, and when users trust it completely without ever having to think about the security.

## Brand Personality

Sharp, minimal, confident. An opinionated tool with a clear point of view. Terse copy, decisive defaults, no hedging. It feels like a native system utility, not an app competing for attention. Confidence comes from speed and precision, never from reassurance.

## Anti-references

- **1Password / Bitwarden**: sidebar-plus-vault-browser layouts, heavy chrome, many panes.
- **SaaS dashboards**: cards, stat tiles, onboarding tours, upsell surfaces.
- **Electron / web-app feel**: web form styling, sluggish transitions, generic component kits. Anything that reads as a WebView.
- **Security theater**: shields and padlocks as decoration, green "you are safe" banners, scare colors, hacker-terminal aesthetics.

## Design Principles

1. **One window, one obvious next step.** Everything lives in the Command Palette. Enter does the obvious thing, Escape goes back, and the footer tells the truth about what Enter will do.
2. **Speed is the brand.** Opening, filtering, and copying should feel instant. Motion and visual effects earn their place only if they cost nothing perceptible and never repaint while idle.
3. **Assume expertise.** Power users don't need tours or explanations. Shortcuts are shown, not taught. Copy is short and plain.
4. **Calm security.** Trust comes from behavior (clearing, locking, staying local), not from visuals. Status is stated plainly; alarm is reserved for things that are actually wrong.
5. **Imported colors, intact hierarchy.** Users bring their editor themes. Structure comes from spacing, type, and the header/list/footer frame, so any palette restyles Latch without breaking it.

## Accessibility & Inclusion

- Every flow works fully from the keyboard; focus is always visible.
- WCAG AA contrast for text and essential UI, including under imported themes (Ctrl/Cmd+Alt+R resets appearance when a theme makes it unreadable).
- Meaningful labels for screen readers, as far as GPUI's accessibility support allows.
- Secrets never reach shaped text, logs, or assistive output unless the user explicitly reveals them.
