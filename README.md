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
- Async file operations (copy, move, delete, empty trash) with progress bar and cancellation
- Sortable list view with directory item counts
- Sidebar with XDG bookmarks, mounted/unmounted drives (click-to-mount via udisks2)
- Tabs with vim-style switching (`gt`/`gT`, `1`-`9`) and session restore
- Back/forward/up navigation with history
- Fuzzy filter within current directory
- Recursive file search across subdirectories (`s`)
- Command palette with fuzzy search (`Ctrl+Shift+P`)
- Yazi-inspired keyboard navigation (vim keys + arrows, chords, visual mode)
- Configurable keybinds and settings via `~/.config/hyprfiles/config.toml`
- Default config with commented examples generated on first run
- Context menu with "Open With" (reads `.desktop` files) and custom user actions
- Preview panel with syntax highlighting (Web Worker), image thumbnailing, directory listing, and PDF preview
- Preview cache (LRU, 5MB) with adjacent entry prefetch for instant navigation
- Directory listing cache with aggressive prefetch (all subdirs + ancestor chain)
- Material Icon Theme SVGs (~1,100 colorful file/folder icons with light/dark mode)
- File properties dialog with async directory stats
- Hidden files toggle
- Multi-select (Space toggle, visual mode, Ctrl+A, Shift+click)
- MIME type detection (extension + magic bytes)
- CSS theme system with hot-reload (edit theme CSS, see changes instantly)
- 3 bundled themes: Catppuccin Mocha, Nord, Dark Minimal
- Drag and drop (internal move/copy, drag-out to other apps, drop-in from other apps)
- File permissions editing (chmod dialog)
- Symlink creation
- Config hot-reload (keybinds, theme, settings, custom actions update live)
- Filesystem watching (live directory updates)
- `$EDITOR` integration (text files open with your editor, fallback to xdg-open)
- Responsive layout (columns, preview panel, and sidebar adapt to window width)
- Smooth transitions on all dialogs, overlays, and panels

See [ROADMAP.md](ROADMAP.md) for planned features.

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
│   ├── commandRegistry.ts        # Command list for palette + help dialog
│   ├── fileOps.ts                # File operation handlers
│   ├── contextMenu.ts            # Context menu item builders
│   ├── commands.ts               # Tauri IPC wrappers
│   ├── errors.ts                 # Structured error types + helpers
│   ├── constants.ts              # Shared constants + text/image/PDF detection
│   ├── highlight.ts              # Syntax highlighting (highlight.js language maps)
│   ├── highlightWorker.ts        # Web Worker for non-blocking syntax highlighting
│   ├── previewCache.ts           # LRU preview cache (5MB, mtime-validated)
│   ├── transitions.ts            # Shared transition functions (fade, pop, fly)
│   ├── icons.ts                  # Material Icon Theme SVG lookup
│   ├── icons.gen.ts              # Generated icon maps (from sync-icons script)
│   ├── utils.ts                  # Path/format helpers
│   └── components/               # Presentational components
│       ├── FileList.svelte       # List view with virtual scrolling
│       ├── FileIcon.svelte       # SVG icon component
│       ├── fileEditLogic.svelte.ts # Shared rename/create logic
│       ├── Breadcrumb.svelte     # Clickable path breadcrumb
│       ├── Toolbar.svelte        # Navigation controls + breadcrumb
│       ├── TabBar.svelte         # Tab strip
│       ├── StatusBar.svelte      # Item count, view toggles, selection/clipboard overlay
│       ├── FilterBar.svelte      # Fuzzy filter input
│       ├── SearchOverlay.svelte  # Recursive file search
│       ├── CommandPalette.svelte # Ctrl+Shift+P command palette
│       ├── BusyOverlay.svelte    # Progress overlay
│       ├── Sidebar.svelte        # Places/drives sidebar
│       ├── ContextMenu.svelte    # Right-click menu
│       ├── FolderPicker.svelte   # Folder selection dialog
│       ├── PreviewPanel.svelte   # Text/image/PDF/directory preview with caching
│       ├── CompressDialog.svelte # Archive format + name dialog
│       ├── ConfirmDialog.svelte  # Yes/no confirmation
│       ├── HelpDialog.svelte     # Keybind reference (?)
│       └── PropertiesDialog.svelte

src-tauri/src/                    # Backend (Rust)
├── lib.rs                        # Tauri builder + command registration
├── config.rs                     # TOML config loading + default config generation
├── default-config.toml           # Default config template (embedded at compile time)
├── error.rs                      # AppError enum (structured errors)
├── progress.rs                   # Shared progress emission + cancellation
├── fs_ops.rs                     # FileEntry/DriveEntry models, read_directory, MIME, file ops
└── commands/
    ├── config.rs                 # get_config + watch_config commands
    ├── file_ops.rs               # Directory listing, create/rename/copy/move/delete, properties
    ├── archive.rs                # Compress/extract with progress + cancellation
    ├── apps.rs                   # Open files, .desktop file parsing, Open With
    ├── search.rs                 # Recursive file search with streaming results
    ├── trash.rs                  # Freedesktop trash list/restore/empty
    ├── drives.rs                 # Drive detection (lsblk) + mount (udisksctl)
    ├── watcher.rs                # Filesystem watching for live directory updates
    └── theme.rs                  # Theme loading, file watching, default theme installation
```

## Development

Requires:
- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)
- `webkit2gtk-4.1` (Arch: `sudo pacman -S webkit2gtk-4.1`)
- `udisks2` for drive mounting (Arch: `sudo pacman -S udisks2`)
- `poppler` for PDF preview (Arch: `sudo pacman -S poppler`)

```bash
bun install
bun run sync-icons   # generate Material Icon Theme SVGs (first time only)
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
| `--scrollbar-width` | `8px` | Scrollbar width |
| `--transition-fast` | `0.1s` | Fast transitions |
| `--transition-normal` | `0.15s` | Normal transitions |
| `--font-mono` | JetBrains Mono Nerd Font, ... | Monospace / icon font |
| `--font-sans` | Inter, sans-serif | UI font |

### Hot-reload

Theme CSS files are watched at runtime. Edit a theme file while the app is running and changes apply instantly. Config changes (including theme, keybinds, and custom actions) are also hot-reloaded.

## Custom Actions

Add your own shell commands to the right-click context menu via `[[actions]]` in `config.toml`.

### Example

```toml
[[actions]]
name = "Edit in Neovim"
command = "ghostty -e nvim %f"
context = "file"

[[actions]]
name = "Set as Wallpaper"
command = "hyprctl hyprpaper wallpaper eDP-1,%f"
context = "file"
mime = "image/*"

[[actions]]
name = "Make Executable"
command = "chmod +x %f"
context = "file"
refresh = true

[[actions]]
name = "Git Init"
command = "git init"
context = "background"
refresh = true
```

### Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | yes | — | Label shown in the context menu |
| `command` | yes | — | Shell command with placeholders |
| `context` | no | `"any"` | When to show: `"file"`, `"directory"`, `"any"`, `"background"` |
| `mime` | no | — | MIME filter (e.g. `"image/*"`, `"text/plain"`) |
| `refresh` | no | `false` | Refresh directory listing after command completes |

### Placeholders

| Placeholder | Description |
|-------------|-------------|
| `%f` | Full path of the focused file/directory |
| `%n` | Filename without extension |
| `%F` | All selected file paths (space-separated) |
| `%d` | Current directory path |
