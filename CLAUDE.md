# hyprfiles

Themeable file manager for Hyprland. Tauri v2 (Rust) + SvelteKit / Svelte 5 + bun.

## Checks

Run all four before considering any task complete:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
bunx svelte-check --tsconfig ./tsconfig.json
bunx biome check --write
```

Use the **Svelte MCP autofixer** if `svelte-check` reports component issues.

## Architecture

```
src/                              # Frontend (Svelte 5)
├── routes/+page.svelte           # Layout shell only (~300 lines) — no business logic
├── lib/
│   ├── stores/
│   │   ├── fileManager.svelte.ts # All app state (navigation, files, selection, clipboard)
│   │   └── dialogs.svelte.ts     # Dialog/busy/progress state + compress/extract orchestration
│   ├── fileOps.ts                # File operation handlers (accept store, call commands, refresh)
│   ├── contextMenu.ts            # Context menu item builders (discriminated union)
│   ├── commands.ts               # Tauri IPC wrappers (typed invoke calls)
│   ├── errors.ts                 # AppError type + isAppError, isCancelled, errorMessage
│   ├── constants.ts              # Shared constants (archiveExtensions, etc.)
│   ├── icons.ts                  # Nerd font icon lookup
│   ├── utils.ts                  # Path/format helpers
│   └── components/               # Presentational components — no business logic
│       ├── fileEditLogic.svelte.ts # Shared rename/create logic (used by FileList + FileGrid)
│       └── *.svelte              # FileList, FileGrid, Toolbar, StatusBar, Sidebar, etc.

src-tauri/src/                    # Backend (Rust)
├── lib.rs                        # Tauri builder + command registration (21 commands)
├── error.rs                      # AppError enum — all commands return Result<T, AppError>
├── fs_ops.rs                     # FileEntry/DriveEntry structs, read_directory, guess_mime, file ops
└── commands/                     # One module per domain
    ├── file_ops.rs               # CRUD, properties, dir stats
    ├── archive.rs                # Compress/extract with progress + cancellation
    ├── apps.rs                   # Open files, .desktop parsing, Open With
    ├── trash.rs                  # Freedesktop trash list/restore/empty
    └── drives.rs                 # Mounted drive detection
```

### Key patterns

- **Frontend state**: Svelte 5 `$state()` runes in factory functions (`createFileManager()`, `createDialogManager()`). No global stores — state is instantiated in `+page.svelte` and passed as props or function params.
- **File operations**: Functions in `fileOps.ts` accept a `FileManager` store as the first parameter, call Tauri commands, and call `fm.refresh()` on success.
- **Errors**: All Rust commands return `Result<T, AppError>`. `AppError` is `#[serde(tag = "kind")]` so the frontend receives `{ kind: "NotFound", path: "..." }` etc. Use `errorMessage(e)` to convert, `isCancelled(e)` to check. Cancelled errors are silently ignored.
- **Shared edit logic**: `fileEditLogic.svelte.ts` is a composable used by both FileList and FileGrid — rename/create input state, auto-focus with extension-aware selection.
- **Progress tracking**: `ProgressWriter`/`ProgressReader` wrappers in `archive.rs` emit Tauri events, checked by `AtomicBool` for cancellation.

### Where to put new code

| Adding...                  | Goes in                              |
|----------------------------|--------------------------------------|
| New Tauri command          | Relevant `commands/*.rs` module + register in `lib.rs` |
| New file operation         | `fileOps.ts` (handler) + `commands.ts` (IPC wrapper) |
| New UI state               | `fileManager.svelte.ts` or `dialogs.svelte.ts` |
| New presentational component | `src/lib/components/` |
| New context menu item      | `contextMenu.ts` |
| New error variant          | `error.rs` (Rust) + `errors.ts` (TypeScript) — keep in sync |

### Boundaries

- `+page.svelte` is layout and wiring only. Do not add business logic, handlers, or state.
- Components in `components/` are presentational. They receive data via props and emit events — no direct Tauri command calls.
- `fs_ops.rs` is the shared implementation layer. `commands/*.rs` are thin `#[tauri::command]` wrappers that delegate to it.

## Toolchain

| What         | Tool   | Notes                                    |
|--------------|--------|------------------------------------------|
| Package mgr  | bun    | Do not use npm/yarn/pnpm                 |
| Rust lint    | clippy |                                          |
| Rust test    | cargo test | 40 tests across archive, apps, trash, file ops |
| TS/Svelte types | svelte-check |                                   |
| Lint/format  | biome  | `bunx biome check --write`               |
| Dev server   | tauri  | `bun run tauri dev`                      |

## MCP Servers

- **Svelte** (`@sveltejs/mcp`) — Svelte/SvelteKit docs. Use for component patterns and API lookup.
- **Context7** — up-to-date docs for any library. Use instead of guessing APIs.

## Guidelines

- Do not add features, refactoring, or "improvements" beyond what was asked.
- Do not add docstrings, comments, or type annotations to code you didn't change.
- Do not add error handling for scenarios that can't happen. Only validate at system boundaries.
- Do not create abstractions for one-time operations. Three similar lines > premature helper.
- Do not add `Co-Authored-By` or other self-attribution to git commits.
- Prefer editing existing files over creating new ones.
- See `ROADMAP.md` for planned features and phasing.
- See `TODO.md` for refactoring history and architectural decisions.
