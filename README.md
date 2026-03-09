# hyprfiles

A themeable file manager for Hyprland, built with Tauri, Svelte, and Rust.

## Why

GTK file managers are held hostage by libadwaita's anti-theming philosophy. hyprfiles is a from-scratch file manager that you actually control the look of — just CSS.

## Stack

- **Backend:** Rust (Tauri v2)
- **Frontend:** Svelte 5 + TypeScript
- **Rendering:** WebKitGTK (webview, not GTK widgets — libadwaita can't touch it)
- **Target:** Hyprland / Wayland

## Features

- Browse, create, rename, copy, move, and delete files/directories
- Trash support (freedesktop trash spec)
- Compress and extract archives (zip, tar.gz, tar.xz, tar.zst, tar.bz2) with progress and cancellation
- Sortable list view with directory item counts
- Sidebar with XDG bookmarks and mounted drives
- Tabs with vim-style switching (`gt`/`gT`, `1`-`9`)
- Back/forward/up navigation with history
- Fuzzy filter within current directory
- Yazi-inspired keyboard navigation (vim keys + arrows, chords, visual mode)
- Configurable keybinds and settings via `~/.config/hyprfiles/config.toml`
- Context menu with "Open With" (reads `.desktop` files)
- File properties dialog with async directory stats
- Hidden files toggle
- Multi-select (Space toggle, visual mode, Ctrl+A, Shift+click)
- MIME type detection (extension + magic bytes)
- CSS custom property theming

See [ROADMAP.md](ROADMAP.md) for planned features (drag & drop, theming engine, previews).

## Architecture

```
src/                              # Frontend (Svelte 5)
├── routes/+page.svelte           # Layout shell + keybind wiring
├── lib/
│   ├── stores/
│   │   ├── fileManager.svelte.ts # Navigation, file, selection, clipboard state
│   │   ├── tabs.svelte.ts        # Tab management
│   │   └── dialogs.svelte.ts     # Dialog/busy/progress state + orchestration
│   ├── keybinds.ts               # Keybind/chord definitions + config overrides
│   ├── fileOps.ts                # File operation handlers
│   ├── contextMenu.ts            # Context menu item builders
│   ├── commands.ts               # Tauri IPC wrappers
│   ├── errors.ts                 # Structured error types + helpers
│   ├── constants.ts              # Shared constants
│   ├── icons.ts                  # Nerd font icon lookup
│   ├── utils.ts                  # Path/format helpers
│   └── components/               # Presentational components
│       ├── FileList.svelte       # List view
│       ├── fileEditLogic.svelte.ts # Shared rename/create logic
│       ├── Breadcrumb.svelte     # Clickable path breadcrumb
│       ├── Toolbar.svelte        # Nav controls + breadcrumb host
│       ├── TabBar.svelte         # Tab strip
│       ├── StatusBar.svelte      # Selection/clipboard info
│       ├── FilterBar.svelte      # Fuzzy filter input
│       ├── BusyOverlay.svelte    # Progress overlay
│       ├── Sidebar.svelte        # Places/drives sidebar
│       ├── ContextMenu.svelte    # Right-click menu
│       ├── FolderPicker.svelte   # Folder selection dialog
│       ├── CompressDialog.svelte # Archive format + name dialog
│       ├── ConfirmDialog.svelte  # Yes/no confirmation
│       └── PropertiesDialog.svelte

src-tauri/src/                    # Backend (Rust)
├── lib.rs                        # Tauri builder + command registration
├── config.rs                     # TOML config loading (~/.config/hyprfiles/config.toml)
├── error.rs                      # AppError enum (structured errors)
├── fs_ops.rs                     # FileEntry/DriveEntry models, read_directory, MIME, file ops
└── commands/
    ├── config.rs                 # get_config command
    ├── file_ops.rs               # Directory listing, create/rename/copy/move/delete, properties
    ├── archive.rs                # Compress/extract with progress + cancellation
    ├── apps.rs                   # Open files, .desktop file parsing, Open With
    ├── trash.rs                  # Freedesktop trash list/restore/empty
    └── drives.rs                 # Mounted drive detection
```

## Development

Requires:
- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)
- `webkit2gtk-4.1` (Arch: `sudo pacman -S webkit2gtk-4.1`)

```bash
bun install
bun run tauri dev
```

## Checks

```bash
# Rust
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend
bunx svelte-check --tsconfig ./tsconfig.json
bunx biome check --write
```
