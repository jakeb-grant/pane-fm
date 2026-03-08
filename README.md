# hyprfiles

A themeable file manager for Hyprland, built with Tauri, Svelte, and Rust.

## Why

GTK file managers are held hostage by libadwaita's anti-theming philosophy. hyprfiles is a from-scratch file manager that you actually control the look of — just CSS.

## Stack

- **Backend:** Rust (Tauri v2)
- **Frontend:** Svelte 5 + TypeScript
- **Rendering:** WebKitGTK (webview, not GTK widgets — libadwaita can't touch it)
- **Target:** Hyprland / Wayland

## Features (planned)

- Drag and drop (including to/from web browsers)
- Fully themeable via CSS
- Keyboard-driven workflow
- File previews
- Trash support
- Filesystem watching

## Development

Requires:
- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)
- `webkit2gtk-4.1` (Arch: `sudo pacman -S webkit2gtk-4.1`)

```bash
bun install
bun run tauri dev
```

## Linting

```bash
# Frontend (TypeScript/Svelte)
bunx biome check .
bunx biome check --write .

# Backend (Rust)
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml
```
