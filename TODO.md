# Hyprfiles Refactoring TODO

## Project Context

**Hyprfiles** is a themeable file manager for Hyprland built with **Tauri v2** (Rust backend) + **SvelteKit / Svelte 5** (frontend) + **bun** (runtime/package manager). See `CLAUDE.md` for toolchain and conventions, `ROADMAP.md` for the full feature roadmap.

### Architecture Overview

```
src/                          # Frontend (Svelte 5)
├── routes/+page.svelte       # Main app — ALL state, handlers, and UI (1,020 lines, needs splitting)
├── lib/
│   ├── commands.ts           # Tauri IPC wrappers (typed invoke calls)
│   ├── icons.ts              # Nerd font icon lookup tables
│   ├── utils.ts              # formatSize, pathSegments helpers
│   └── components/           # Presentational components
│       ├── FileList.svelte   # List view (has inline edit logic — duplicated with FileGrid)
│       ├── FileGrid.svelte   # Grid view (has inline edit logic — duplicated with FileList)
│       ├── Sidebar.svelte    # Places/drives sidebar
│       ├── Breadcrumb.svelte # Path breadcrumb nav
│       ├── ContextMenu.svelte # Right-click menu (presentational only)
│       ├── FolderPicker.svelte    # Folder selection dialog (move/copy/extract)
│       ├── CompressDialog.svelte  # Archive format + name dialog
│       └── PropertiesDialog.svelte # File/dir properties (async dir stats)

src-tauri/src/                # Backend (Rust)
├── main.rs                   # Entry point
├── lib.rs                    # Tauri builder + command registration
├── commands.rs               # ALL 21 Tauri commands + 14 helpers (789 lines, needs splitting)
└── fs_ops.rs                 # FileEntry/DriveEntry models, read_directory, MIME, basic file ops
```

### Key Patterns
- **Frontend state**: All in `+page.svelte` as Svelte 5 `$state()` runes — no store layer yet
- **Backend commands**: All `#[tauri::command]` functions in one file, return `Result<T, String>`
- **Progress tracking**: `ProgressWriter`/`ProgressReader` wrappers emit Tauri events, checked by `AtomicBool` for cancellation
- **Inline editing**: Rename and create use phantom entries with auto-focused inputs (logic duplicated in FileList + FileGrid)
- **CSS theming**: Custom properties (`--bg-primary`, `--accent`, etc.) defined in `app.css`

### What's Working
Phase 1 is complete. Partial Phase 2 (sidebar, grid view, dir counts, back/forward, sorting). Compression/extraction with progress + cancel. Trash management. Properties dialog with async dir stats. Open With via `.desktop` file parsing.

### What This TODO Covers
Refactoring to support Phase 2+ features (tabs, multi-select, keyboard nav, drag & drop, theming). The current monolithic structure won't scale — both `+page.svelte` and `commands.rs` need to be broken up before adding more features.

### Workflow for Agents

Work through sections **one at a time** (e.g., 1.1, then 1.2, then 1.3). For each section:

1. **Plan** — Read the section's checklist and all affected files. Use plan mode or an agent to design the approach before writing code. Identify which files will be created, modified, or deleted.
2. **Implement** — Make the changes for that section only. Don't touch unrelated sections.
3. **Verify** — Run all checks and fix any issues before moving on:
   - `cargo check` (Rust compilation)
   - `cargo clippy` (Rust linting)
   - `bunx svelte-check --tsconfig ./tsconfig.json` (Svelte/TS type checking)
   - `bunx biome check --write` (frontend linting/formatting)
   - Use the **Svelte MCP autofixer** if svelte-check reports component issues
4. **Check off** — Mark completed items as `[x]` in this file.
5. **Commit** — Commit the section's changes with a descriptive message.
6. **Repeat** — Move to the next section.

**Do not** skip verification steps. **Do not** combine multiple sections into one pass — each section should be a self-contained, working change. If a section is too large, break it into sub-commits but still verify after each.

---

## Priority 1: Extract State & Logic from +page.svelte

The main page component is 1,020 lines with 23 `$state` variables, 2 `$derived` values, ~22 handlers, and all app logic. It needs to be broken into a state management layer and focused handler modules.

### 1.1 Create a file manager store (`src/lib/stores/fileManager.svelte.ts`)
- [x] Extract navigation state: `currentPath`, `history`, `historyIndex`, `loading`, `error`
- [x] Extract file state: `entries`, `drives`, `sortBy`, `sortAsc`, `showHidden`, `viewMode`
- [x] Extract selection state: `selectedPath`, `selectedEntry`
- [x] Extract edit state: `renamingPath`, `creatingEntry`
- [x] Extract clipboard state: `clipboard` (entries + mode)
- [x] Extract app state: `openWithApps`
- [x] Extract derived state: `sortedEntries`, `isTrash`
- [x] Expose action methods: `navigate()`, `goBack()`, `goForward()`, `goUp()`, `refresh()`
- [x] Expose sort/filter methods: `handleSort()`, `toggleHidden()`
- [x] Initialize drives list on creation
- [x] Provide home dir to Sidebar (eliminate duplicate `getHomeDir()` IPC call)
- [x] Persist preferences (viewMode, showHidden, sortBy) to localStorage

### 1.2 Create file operations module (`src/lib/fileOps.ts`)
- [x] Extract `handleOpen()` — open file/directory
- [x] Extract `handleOpenWith()` — open with specific app
- [x] Extract `handleDelete()` — delete/trash entry
- [x] Extract `handleCopy()`, `handleCut()`, `handlePaste()` — clipboard operations
- [x] Extract `handleRename()`, `commitRename()` — rename flow
- [x] Extract `handleNewFolder()`, `handleNewFile()`, `commitCreate()` — create flow
- [x] Extract `handleMoveTo()`, `handleCopyTo()` — move/copy to destination
- [x] Extract `handleFolderPickerSelect()` — move/copy/extract after folder pick
- [x] Extract `handleRestore()`, `handleEmptyTrash()` — trash operations
- [x] Extract `handleSelect()`, `handleBgContextMenu()` — selection/context handlers
- [x] Each operation should accept the store as a parameter and call `refresh()` on success
- ~~Return typed errors instead of setting a string directly~~ *(deferred to Priority 4 — requires structured backend errors first)*

### 1.3 Create dialog/modal manager (`src/lib/stores/dialogs.svelte.ts`)
- [x] Unified state for all dialogs: properties, folderPicker, compress, contextMenu, busy
- [x] Methods: `open(type, data)`, `close(type)`, `closeAll()`
- [x] Guard against opening multiple dialogs simultaneously
- [x] Extract busy overlay state: `busyMessage`, `busyProgress`
- [x] Extract compress/extract orchestration: `handleCompressConfirm()`, `handleExtract()`, `handleExtractTo()`, `handleCancelOperation()`
- [x] Manage progress event listener lifecycle (subscribe/unsubscribe)

**Note:** This module and 1.2 share a tight dependency on busy/progress state. Design the progress management interface before implementing either.

### 1.4 Extract context menu builder (`src/lib/contextMenu.ts`)
- [x] Move `getContextMenuItems()` out of +page.svelte
- [x] Accept current state (selectedEntry, clipboard, isTrash, openWithApps) as parameters
- [x] Move `archiveExtensions` regex to a shared constants file
- [x] Move background context menu items (new folder, new file, paste, properties) to separate builder
- [x] Model entry vs background context menu as a discriminated union (see B.4)

### 1.5 Extract CSS into component styles
- [x] Move toolbar styles into a `Toolbar.svelte` component
- [x] Move status bar styles into a `StatusBar.svelte` component
- [x] Move busy overlay styles into a `BusyOverlay.svelte` component
- [x] Keep only layout/grid styles in +page.svelte

### 1.6 Target outcome
- [x] `+page.svelte` should be <300 lines: imports, layout markup, component wiring
- [x] All business logic testable without rendering components
- [x] Adding a new feature (tabs, multi-select) means extending the store, not the page

---

## Priority 2: Split commands.rs into Modules

`commands.rs` is 789 lines with 21 `#[tauri::command]` handlers and 14 private helpers mixing dispatch, business logic, I/O, and desktop integration.

### 2.1 Create module structure
- [ ] Create `src/commands/` directory with `mod.rs`
- [ ] Re-export all `#[tauri::command]` functions from `mod.rs`
- [ ] Update `lib.rs` imports (should still use `commands::function_name`)

### 2.2 Extract file operations (`src/commands/file_ops.rs`)
- [ ] Move: `list_directory`, `get_home_dir`, `create_directory`, `create_file`
- [ ] Move: `rename_entry`, `delete_entry`, `copy_entry`, `move_entry`
- [ ] Move: `path_exists`
- [ ] Move: `get_properties`, `get_dir_stats`, `get_properties_sync`
- [ ] Move: `dir_size_and_count`

### 2.3 Extract archive operations (`src/commands/archive.rs`)
- [ ] Move: `compress`, `extract`, `cancel_operation`
- [ ] Move: `ProgressWriter`, `ProgressReader`, `ProgressPayload`, `CANCEL_OPERATION`
- [ ] Move: `compress_zip`, `add_dir_to_zip`, `compress_tar`, `write_tar`, `add_dir_to_tar`
- [ ] Move: `extract_zip`, `unpack_tar`
- [ ] Deduplicate `add_dir_to_zip` / `add_dir_to_tar` directory traversal into a shared iterator

**Note:** This module will realistically be ~250 lines even after deduplication. That's acceptable given the complexity.

### 2.4 Extract app/desktop integration (`src/commands/apps.rs`)
- [ ] Move: `open_default`, `list_apps_for_mime`, `open_with_app`
- [ ] Move: `get_xdg_data_dirs`, `parse_desktop_file`
- [ ] Deduplicate Exec= line parsing (used in both `list_apps_for_mime` and `open_with_app`)

### 2.5 Extract trash operations (`src/commands/trash.rs`)
- [ ] Move: `list_trash` (from `fs_ops.rs`), `restore_trash`, `empty_trash` (from `commands.rs`)
- [ ] Move: `urlencoding` helper (rename per B.3)
- [ ] Consolidate all trash logic into this single module

### 2.6 Extract drive operations (`src/commands/drives.rs`)
- [ ] Move: `list_drives` (bulk of logic is in `fs_ops.rs`)
- [ ] Move: `is_removable` helper from `fs_ops.rs`

### 2.7 Clean up fs_ops.rs
- [ ] After all extractions (2.2–2.6), `fs_ops.rs` should only contain: `FileEntry` struct, `DriveEntry` struct, `read_directory()`, `guess_mime()` (make public, drop the `guess_mime_pub` wrapper)
- [ ] Remove the redundant Rust-side sort in `read_directory()` — the frontend re-sorts via `sortedEntries` anyway
- [ ] Consider renaming to `models.rs` or splitting into `models.rs` + `fs.rs`

### 2.8 Target outcome
- [ ] No single file exceeds ~250 lines
- [ ] Each module has a single responsibility
- [ ] Adding a new archive format = editing only `archive.rs`
- [ ] Adding a new command = adding to the relevant module + registering in `mod.rs`

---

## Priority 3: Extract Shared FileList/FileGrid Logic

FileList (301 lines) and FileGrid (234 lines) duplicate identical edit input logic.

### 3.1 Create shared edit logic (`src/lib/components/fileEditLogic.ts`)
- [ ] Extract `focusAndSelect()` helper (smart extension-aware selection)
- [ ] Extract `editValue` / `editInput` state management
- [ ] Extract `commitRenameForEntry()` and `commitCreateEntry()` wrappers
- [ ] Extract the `$effect` blocks for auto-focusing rename/create inputs

**Note:** Svelte 5 `$effect` blocks run in component context. Extracting them requires a factory/composable pattern — the shared module should return a setup function that each component calls.

### 3.2 Update FileList and FileGrid
- [ ] Import shared logic instead of duplicating it
- [ ] Each component only handles layout-specific rendering (list rows vs grid tiles)
- [ ] Ensure both components stay in sync for any future edit features

### 3.3 Target outcome
- [ ] Single place to fix edit/rename/create bugs
- [ ] Adding keyboard navigation for editing = one file change
- [ ] FileList and FileGrid focus purely on presentation

---

## Priority 4: Define Rust Error Types

All commands return `Result<T, String>` with no structured error information.

### 4.1 Create error module (`src/commands/error.rs`)
- [ ] Define `AppError` enum with variants: `Io`, `NotFound`, `PermissionDenied`, `Cancelled`, `Archive`, `Desktop`, `Trash`
- [ ] Implement `From<std::io::Error>` for automatic conversion
- [ ] Implement `Into<String>` or `Serialize` for Tauri command compatibility
- [ ] Include source error context (file path, operation attempted)

### 4.2 Migrate commands to use `AppError`
- [ ] Replace all `format!("Failed to...")` error strings
- [ ] Replace all `.map_err(|e| format!(...))` chains with `?` operator
- [ ] Handle `Cancelled` variant explicitly (not as a string match on frontend)

### 4.3 Update frontend error handling
- [ ] Parse structured errors from backend instead of displaying raw strings
- [ ] Show appropriate messages: "Permission denied" vs "File not found" vs "Operation cancelled"
- [ ] Silently handle `Cancelled` errors without showing error bar

### 4.4 Target outcome
- [ ] Errors carry context (which file, which operation)
- [ ] Frontend can make decisions based on error type, not string matching
- [ ] `Cancelled` is a first-class variant, not a string comparison

---

## Priority 5: Add Unit Tests

Zero tests exist in the backend. No safe refactoring without them. Consider writing tests for archive/trash/desktop parsing *before* the Priority 2 module split to catch regressions during extraction.

### 5.1 Archive tests (`src/commands/archive.rs` or `tests/`)
- [ ] Test zip compression roundtrip (compress → extract → compare)
- [ ] Test tar.gz, tar.xz, tar.zst, tar.bz2 roundtrips
- [ ] Test compression of empty directory
- [ ] Test compression of directory with symlinks (should skip)
- [ ] Test cancellation mid-operation
- [ ] Test extraction to existing directory

### 5.2 Desktop parsing tests
- [ ] Test `parse_desktop_file()` with valid .desktop file
- [ ] Test with missing Name= or Exec= fields
- [ ] Test with localized entries (Name[en]=...)
- [ ] Test `get_xdg_data_dirs()` returns valid paths

### 5.3 Trash tests
- [ ] Test `urlencoding()` with special characters, unicode, spaces
- [ ] Test trash list with empty trash
- [ ] Test restore with missing info file

### 5.4 File operations tests
- [ ] Test `dir_size_and_count()` accuracy
- [ ] Test `read_directory()` with hidden files, symlinks
- [ ] Test path edge cases: root `/`, home `~`, paths with spaces

### 5.5 Frontend tests (optional, lower priority)
- [ ] Test store actions (navigate, sort, filter) in isolation
- [ ] Test context menu builder returns correct items for different states
- [ ] Test file operation module error handling

### 5.6 Target outcome
- [ ] Safe to refactor any module with confidence
- [ ] CI can catch regressions
- [ ] Edge cases documented through test cases

---

## Future Considerations (Not Actionable Yet)

These are architectural notes for when roadmap features are implemented:

- **Tabs**: Will need per-tab store instances (store factory pattern). Address after Priority 1.
- **Multi-select**: Will need `selectedPaths: Set<string>` replacing scalar `selectedPath`. Requires Priority 1 store first.
- **Keyboard navigation**: Will need a keybind dispatcher. Can layer on top of Priority 1 store.
- **Drag & drop**: Will need drag state management. Benefits from Priority 1 store separation.
- **File watching**: Will need Rust event infrastructure (channels, notify crate). Benefits from Priority 2 module split.
- **Undo/redo**: Will need operation history in the store. Requires Priority 1 + Priority 4.
- **Theming**: Will need config file handling in Rust + theme store in frontend. CSS custom properties are already in place (`app.css`). Mostly independent.
