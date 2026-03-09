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
- CSS theme system with hot-reload (edit theme CSS, see changes instantly)
- 3 bundled themes: Catppuccin Mocha, Nord, Dark Minimal

See [ROADMAP.md](ROADMAP.md) for planned features (drag & drop, previews, config hot-reload).

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
    ├── drives.rs                 # Mounted drive detection
    └── theme.rs                  # Theme loading, file watching, default theme installation
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

## Theming

hyprfiles uses CSS custom properties for all colors. Themes are plain CSS files that override `:root` variables — no special format, no build step.

### Setup

Add `theme` to `~/.config/hyprfiles/config.toml`:

```toml
[general]
theme = "nord"
```

### Bundled themes

Three starter themes are installed to `~/.config/hyprfiles/themes/` on first launch:

| Name | Description |
|------|-------------|
| `catppuccin-mocha` | Default colors (warm dark with blue accent) |
| `nord` | Cool blue-gray palette |
| `dark-minimal` | Pure grayscale with muted accent |

### Custom themes

Point to any CSS file:

```toml
[general]
theme = "~/my-themes/custom.css"
```

A theme file is just `:root {}` overrides. Copy a starter theme and edit:

```css
/* ~/.config/hyprfiles/themes/my-theme.css */
:root {
    --bg-primary: #1a1a2e;
    --bg-secondary: #16213e;
    --bg-surface: #0f3460;
    --bg-hover: #1a4080;
    --text-primary: #e0e0e0;
    --text-secondary: #a0a0a0;
    --text-muted: #666666;
    --accent: #e94560;
    --accent-hover: #ff6b81;
    --border: #0f3460;
    --danger: #c47070;
    --success: #7db87d;
    --warning: #c9b458;
}
```

### Available properties

Themes can override any of these:

| Property | Default | Description |
|----------|---------|-------------|
| `--bg-primary` | `#1e1e2e` | Main background |
| `--bg-secondary` | `#181825` | Sidebar, toolbar, dialog backgrounds |
| `--bg-surface` | `#313244` | Hover/selected row, input backgrounds |
| `--bg-hover` | `#45475a` | Hover state |
| `--text-primary` | `#cdd6f4` | Main text |
| `--text-secondary` | `#a6adc8` | Secondary text |
| `--text-muted` | `#6c7086` | Labels, hints |
| `--accent` | `#89b4fa` | Accent color (links, active tab, icons) |
| `--accent-hover` | `#74c7ec` | Accent hover state |
| `--border` | `#313244` | Borders and dividers |
| `--danger` | `#f38ba8` | Delete, error states |
| `--success` | `#a6e3a1` | Success states |
| `--warning` | `#f9e2af` | Warning states |
| `--radius` | `6px` | Border radius |
| `--overlay-bg` | `rgba(0,0,0,0.5)` | Dialog overlay backdrop |
| `--shadow-sm` | `0 4px 12px rgba(0,0,0,0.3)` | Small shadow (menus) |
| `--shadow-lg` | `0 8px 32px rgba(0,0,0,0.4)` | Large shadow (dialogs) |
| `--kbd-bg` | `rgba(255,255,255,0.1)` | Keyboard shortcut badge |
| `--scrollbar-width` | `8px` | Scrollbar width |
| `--transition-fast` | `0.1s` | Fast transitions |
| `--transition-normal` | `0.15s` | Normal transitions |
| `--font-mono` | JetBrains Mono Nerd Font, ... | Monospace / icon font |
| `--font-sans` | Inter, sans-serif | UI font |

### Hot-reload

Theme CSS files are watched at runtime. Edit a theme file while the app is running and changes apply instantly. Changing the `theme` value in `config.toml` requires a restart (config hot-reload is planned).
