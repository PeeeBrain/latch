# Product

## Register

product

## Users

Privacy-conscious individuals who need secure, local password management. They use a desktop command-palette interface for quick access. Context: unlocking the app to copy a password, add a new credential, or check vault health. Often in focused, time-sensitive moments.

## Product Purpose

A desktop password manager with a Raycast-style command palette UI. Credentials are stored in an encrypted Vault on disk, decrypted into an in-memory Workspace during an active Session. The design must communicate security, trustworthiness, and speed.

## Brand Personality

Secure, focused, and calm. The compact command window keeps credentials and actions close to the keyboard. Users control appearance through explicit colors in existing VS Code or Zed settings.json files.

## Anti-references

- Bundled theme galleries and decorative presets
- SaaS-dashboard clichés (navy + gold, gradient heroes, big metric cards)
- Cyberpunk neon aesthetics for a security tool
- Low-contrast "designery" text that sacrifices legibility

## Design Principles

1. **Security is visible** — Locked, working, invalid, and unavailable states are explicit. Secrets appear only through deliberate reveal, copy, or edit.
2. **Speed of access** — Users open this to get a password quickly. Every visual decision should reduce friction, not add it.
3. **Compact throughout** — Setup, search, editing, settings, and health stay in one keyboard-first command window.
4. **Readable appearance** — Light/dark fallbacks, focus indicators, severity labels, contrast feedback, and a keyboard reset remain available.
5. **Existing settings schemas** — Support selected VS Code/Zed color keys with documented component mappings; no public Latch theme schema or preset picker.

## Accessibility & Inclusion

- Target WCAG AA for fallback palettes; warn about low-contrast imported colors and provide a reset action
- Support for reduced motion
- Color-blind safe indicators (icons + color, never color alone)
- Keyboard-navigable command palette interface
