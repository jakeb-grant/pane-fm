# hyprfiles — Development Roadmap

## Dependencies

### Rust Crates (src-tauri/Cargo.toml)

| Crate | Purpose | Notes |
|-------|---------|-------|
| trash | Freedesktop trash support | Phase 1 |
| notify-debouncer-full | FS watching with debounce + rename tracking | Phase 4 |
| toml (0.9) | Config file parsing | Phase 3 (already transitive) |
| chrono | Date formatting | Phase 1 (already transitive) |
| mime_guess | MIME type detection (extension-based) | Phase 1 |
| infer | MIME type detection (magic bytes fallback) | Phase 1 |
| dirs | XDG directory paths (~/.config, etc.) | Phase 1 (sidebar, trash, home) |

**Already provided by Tauri (do not add separately):**
- `open` → use `tauri-plugin-opener`
- `tokio` → use `tauri::async_runtime`
- `walkdir` → already transitive
- `serde` / `serde_json` → already direct deps

**Deferred:**
- Hyprland IPC → raw unix socket + serde_json (Phase 4, no crate needed)

### Frontend (package.json)

No additional npm dependencies. Svelte 5 + `@tauri-apps/api` only.

- Icons: nerd font glyphs or inline SVGs
- Drag and drop: HTML5 drag events
- Fuzzy filtering: plain JS function
- Virtual scrolling: simple Svelte snippet if needed
- Toasts/notifications: small Svelte component
- CSS classes: Svelte `class:` directive

---

## Phase 1: Core Foundation

Get a working file browser with basic operations. The app should be usable (if ugly) by the end of this phase.

### Rust Backend
- [x] Read directory contents (name, size, modified date, type, permissions)
- [x] Open files with tauri-plugin-opener
- [x] Create files and directories
- [x] Rename files/directories
- [x] Delete files/directories (move to freedesktop trash via `trash` crate)
- [x] Copy and move files/directories
- [x] MIME type detection (mime_guess + infer)
- [x] Compress and extract archives (zip, tar.gz, tar.xz, tar.zst, tar.bz2) with progress + cancellation
- [x] Basic error handling and user feedback (structured AppError types)

### Svelte Frontend
- [x] File list view (sortable by name, size, date, type)
- [x] Clickable breadcrumb path bar
- [x] Navigate into directories, back/forward/up
- [x] Context menu (right-click) with basic operations
- [x] Hidden files toggle
- [x] Loading and error states
- [x] Open With menu (.desktop file integration)
- [x] Compress/extract dialogs with progress overlay
- [x] File properties dialog with async directory stats

## Phase 2: Navigation & Views

Make it fast and comfortable to navigate. Keyboard users should feel at home.

### Navigation
- [x] Editable path bar with directory autocomplete
- [x] Sidebar with bookmarks / pinned directories (XDG dirs via `dirs` crate)
- [x] Back/forward history stack
- [ ] Tabs (open multiple directories)
- [x] Fuzzy search / filter within current directory

### Views
- [x] ~~Grid/icon view~~ (removed — list view only)
- [x] Sortable column headers in list view
- [x] File size formatting (human-readable)
- [x] Directory item counts

### Keyboard
Yazi-inspired keybinds. Arrow keys always work alongside vim keys.

#### Done
- [x] `j`/`k`/`Up`/`Down` — move selection up/down
- [x] `h`/`l`/`Left`/`Right`/`Enter` — parent / open
- [x] `Home`/`End` — first / last entry
- [x] `.` — toggle hidden files
- [x] `/` — fuzzy filter
- [x] Auto-select first entry on navigate, re-select dir on go-up

#### Wire up (functionality exists, just needs keybind)
- [ ] `y` — yank (copy)
- [ ] `x` — cut
- [ ] `p` — paste
- [ ] `d` — trash
- [ ] `r` — rename
- [ ] `a` — create (file or directory)
- [ ] `Y`/`X` — cancel yank
- [ ] `gg` — go to top (chord)
- [ ] `G` — go to bottom
- [ ] `H`/`L` — history back/forward
- [ ] `Escape` — cascading clear (filter → selection)

#### New functionality needed
- [ ] `Space` — toggle selection on entry (multi-select)
- [ ] `v` — visual mode (range select)
- [ ] `Ctrl-a` — select all
- [ ] `Ctrl-u`/`Ctrl-d` — half page up/down
- [ ] `n`/`N` — next/prev filter match (cycle through matches)
- [ ] `,` prefix — sort chords (`,s` size, `,m` modified, `,n` name)
- [ ] `g` prefix — goto chords (`gh` home, `gd` downloads)
- [ ] `c` prefix — copy to system clipboard (`cc` path, `cf` filename)
- [ ] `D` — permanent delete (bypass trash)
- [ ] Configurable keybinds via config file

### Multi-select
Prerequisite for bulk operations and full keybind support. Do this before
wiring remaining keybinds so every keybind works for single and multi from
day one.

#### Design: Cursor vs Selection (yazi/ranger model)
- **Cursor** (`cursorPath`/`cursorEntry`): the focused entry that j/k moves,
  what `l`/Enter/open acts on, where the highlight bar sits, drives scroll-into-view
- **Selection** (`selectedPaths: Set<string>`): entries that bulk operations act on
- **`effectiveSelection`** (derived): if `selectedPaths` non-empty, use those entries;
  otherwise fall back to `[cursorEntry]`
- Keep `selectedPath`/`selectedEntry` as backward-compat aliases during migration

#### Step 1: Refactor `fileManager.svelte.ts`
- [x] Rename `selectedPath`→`cursorPath`, `selectedEntry`→`cursorEntry`
- [x] Add `selectedPaths: Set<string>` state
- [x] Add `effectiveSelection` derived getter
- [x] Add `toggleSelect(entry)`, `selectRange(from, to)`, `selectAll()`, `clearMultiSelection()`
- [x] `select()`/click clears multi-selection and moves cursor
- [x] `navigate()` clears both cursor and selectedPaths

#### Step 2: Update `fileOps.ts`
- [ ] `handleCopy`/`handleCut` → use `fm.effectiveSelection` (clipboard already holds `FileEntry[]`)
- [ ] `handleDelete` → loop over `fm.effectiveSelection`
- [ ] `handleMoveTo`/`handleCopyTo` → pass `effectiveSelection` to folder picker
- [ ] `handleRestore` → loop over effective selection
- [ ] `handleRename`, `handleProperties`, `handleOpen` → stay single-entry, use `cursorEntry`
- [ ] `handlePaste` → already loops `clipboard.entries`, no change needed

#### Step 3: Update `dialogs.svelte.ts`
- [ ] `handleCompress` → pass all effective entries' paths (Rust `compress` already accepts `paths[]`)
- [ ] `handleExtract`/`handleExtractTo` → single archive only, use `cursorEntry`
- [ ] `handleProperties` → single entry, use `cursorEntry`
- [ ] `folderPicker` type → accept `entries: FileEntry[]` instead of single `entry`

#### Step 4: Update `FileList.svelte`
- [ ] Rename prop `selectedPath`→`cursorPath`, add `selectedPaths: Set<string>` prop
- [ ] Row classes: `class:cursor` for highlight bar, `class:selected` for multi-select
- [ ] Click handlers: plain=move cursor, Ctrl+click=toggle select, Shift+click=range select

#### Step 5: Update `+page.svelte`
- [ ] Pass new props to FileList
- [ ] Wire `Space` (toggle), `v` (visual mode), `Ctrl-a` (select all)
- [ ] `Escape` cascades: close filter → clear multi-selection → clear cursor

#### Step 6: Update `contextMenu.ts`
- [ ] Multi-selection labels ("Move 3 items to Trash", etc.)
- [ ] Hide/disable single-only items (Rename, Properties, Open With) when multi-selected

#### Step 7: StatusBar
- [ ] Show "N items selected" when `selectedPaths.size > 0`

## Phase 3: Theming & Drag and Drop

The two features that justify this project's existence.

### Theming
- [ ] CSS-based theme system — app loads user CSS at runtime
- [ ] TOML config file for non-visual settings (~/.config/hyprfiles/config.toml)
- [ ] CSS custom properties for all colors, spacing, fonts, radii, etc.
- [ ] Hot-reload themes (watch config/theme files, apply without restart)
- [ ] Ship 3 starter themes: dark minimal, catppuccin, nord
- [ ] Icon display (nerd font glyphs by default, configurable)

### Drag and Drop
- [ ] Internal DnD — drag files to move/copy within the app
- [ ] Visual drag feedback (ghost element, drop indicators)
- [ ] External DnD out — drag files from hyprfiles to browsers/other apps
- [ ] External DnD in — drop files from browsers/other apps into hyprfiles
- [ ] Modifier keys (hold Ctrl to copy instead of move)

## Phase 4: Power Features

Features that make hyprfiles a daily driver.

### File Operations
- [ ] Bulk select with patterns (glob or regex)
- [ ] Bulk rename (pattern-based)
- [ ] Async operations with progress bar (large copies/moves)
- [ ] Undo/redo for file operations (operation history)
- [ ] File permissions viewing and editing (chmod dialog)
- [ ] Symlink creation

### Previews
- [ ] Image thumbnails in grid view
- [ ] Text file preview panel (syntax highlighted)
- [ ] Image preview panel
- [ ] Video/audio thumbnail generation (optional, via ffmpeg)
- [ ] PDF first-page preview

### Integration
- [ ] "Open terminal here" (configurable terminal emulator)
- [ ] Custom context menu actions (user-defined shell scripts)
- [ ] Hyprland IPC — update window title with current path (raw unix socket)
- [ ] Filesystem watching (live directory updates via notify-debouncer-full)

## Phase 5: Polish & Distribution

### UX
- [ ] Command palette (Ctrl+P style)
- [ ] Split pane view (dual-pane file manager mode)
- [ ] Smooth animations and transitions
- [ ] Search across subdirectories (recursive find)
- [ ] Remembering window size, position, last directory per tab

### Distribution
- [ ] AUR package
- [ ] Man page
- [ ] Default config generation on first run
- [ ] CLI flags (open specific directory, etc.)
