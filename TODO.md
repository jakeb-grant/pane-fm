# Hyprfiles Refactoring TODO

## Priority 1: Extract State & Logic from +page.svelte

The main page component is 1,020 lines with ~40 state variables, 20+ handlers, and all app logic. It needs to be broken into a state management layer and focused handler modules.

### 1.1 Create a file manager store (`src/lib/stores/fileManager.svelte.ts`)
- [ ] Extract navigation state: `currentPath`, `history`, `historyIndex`, `loading`, `error`
- [ ] Extract file state: `entries`, `sortBy`, `sortAsc`, `showHidden`, `viewMode`
- [ ] Extract selection state: `selectedPath`, `selectedEntry`
- [ ] Extract clipboard state: `clipboard` (entries + mode)
- [ ] Extract derived state: `sortedEntries`, `isTrash`
- [ ] Expose action methods: `navigate()`, `goBack()`, `goForward()`, `goUp()`, `refresh()`
- [ ] Expose sort/filter methods: `handleSort()`, `toggleHidden()`
- [ ] Initialize drives list on creation
- [ ] Persist preferences (viewMode, showHidden, sortBy) to localStorage

### 1.2 Create file operations module (`src/lib/fileOps.ts`)
- [ ] Extract `handleOpen()` — open file/directory
- [ ] Extract `handleDelete()` — delete/trash entry
- [ ] Extract `handleCopy()`, `handleCut()`, `handlePaste()` — clipboard operations
- [ ] Extract `handleRename()`, `commitRename()` — rename flow
- [ ] Extract `handleNewFolder()`, `handleNewFile()`, `commitCreate()` — create flow
- [ ] Extract `handleMoveTo()`, `handleCopyTo()` — move/copy to destination
- [ ] Extract `handleRestore()`, `handleEmptyTrash()` — trash operations
- [ ] Each operation should accept the store as a parameter and call `refresh()` on success
- [ ] Return typed errors instead of setting a string directly

### 1.3 Create dialog/modal manager (`src/lib/stores/dialogs.svelte.ts`)
- [ ] Unified state for all dialogs: properties, folderPicker, compress, contextMenu, busy
- [ ] Methods: `open(type, data)`, `close(type)`, `closeAll()`
- [ ] Guard against opening multiple dialogs simultaneously
- [ ] Extract busy overlay state: `busyMessage`, `busyProgress`
- [ ] Extract compress/extract orchestration: `handleCompressConfirm()`, `handleExtract()`, `handleExtractTo()`
- [ ] Manage progress event listener lifecycle (subscribe/unsubscribe)

### 1.4 Extract context menu builder (`src/lib/contextMenu.ts`)
- [ ] Move `getContextMenuItems()` out of +page.svelte
- [ ] Accept current state (selectedEntry, clipboard, isTrash, openWithApps) as parameters
- [ ] Move `archiveExtensions` regex to a shared constants file
- [ ] Move background context menu items (new folder, new file, paste, properties) to separate builder

### 1.5 Extract CSS into component styles
- [ ] Move toolbar styles into a `Toolbar.svelte` component
- [ ] Move status bar styles into a `StatusBar.svelte` component
- [ ] Move busy overlay styles into a `BusyOverlay.svelte` component
- [ ] Keep only layout/grid styles in +page.svelte

### 1.6 Target outcome
- [ ] `+page.svelte` should be <300 lines: imports, layout markup, component wiring
- [ ] All business logic testable without rendering components
- [ ] Adding a new feature (tabs, multi-select) means extending the store, not the page

---

## Priority 2: Split commands.rs into Modules

`commands.rs` is 789 lines with 20 command handlers and 18 helpers mixing dispatch, business logic, I/O, and desktop integration.

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

### 2.4 Extract app/desktop integration (`src/commands/apps.rs`)
- [ ] Move: `open_default`, `list_apps_for_mime`, `open_with_app`
- [ ] Move: `get_xdg_data_dirs`, `parse_desktop_file`
- [ ] Move desktop-related code from `fs_ops.rs` into this module
- [ ] Deduplicate Exec= line parsing (used in both `list_apps_for_mime` and `open_with_app`)

### 2.5 Extract trash operations (`src/commands/trash.rs`)
- [ ] Move: `list_trash`, `restore_trash`, `empty_trash`
- [ ] Move: `urlencoding` helper
- [ ] Consolidate trash logic currently split between `commands.rs` and `fs_ops.rs`

### 2.6 Extract drive operations (`src/commands/drives.rs`)
- [ ] Move: `list_drives`
- [ ] Move related drive-detection logic from `fs_ops.rs`

### 2.7 Clean up fs_ops.rs
- [ ] After extractions, `fs_ops.rs` should only contain: `FileEntry` struct, `read_directory()`, `guess_mime_pub()`
- [ ] Consider renaming to `models.rs` or splitting into `models.rs` + `fs.rs`

### 2.8 Target outcome
- [ ] No single file exceeds 200 lines
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

### 4.1 Create error module (`src/error.rs` or `src/commands/error.rs`)
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

Zero tests exist in the backend. No safe refactoring without them.

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
- **Theming**: Will need config file handling in Rust + theme store in frontend. Mostly independent.
