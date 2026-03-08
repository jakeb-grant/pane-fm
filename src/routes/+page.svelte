<script lang="ts">
import { onMount, onDestroy } from "svelte";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
	cancelOperation,
	compress,
	copyEntry,
	createDirectory,
	createFile,
	deleteEntry,
	emptyTrash,
	extract,
	type DriveEntry,
	type FileEntry,
	type FileProperties,
	getHomeDir,
	getProperties,
	listAppsForMime,
	listDirectory,
	listDrives,
	listTrash,
	moveEntry,
	openDefault,
	openWithApp,
	renameEntry,
	restoreTrash,
} from "$lib/commands";
import Breadcrumb from "$lib/components/Breadcrumb.svelte";
import ContextMenu from "$lib/components/ContextMenu.svelte";
import type { MenuEntry } from "$lib/components/ContextMenu.svelte";
import CompressDialog from "$lib/components/CompressDialog.svelte";
import FileGrid from "$lib/components/FileGrid.svelte";
import FileList from "$lib/components/FileList.svelte";
import FolderPicker from "$lib/components/FolderPicker.svelte";
import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
import Sidebar from "$lib/components/Sidebar.svelte";
import { formatSize, parentPath } from "$lib/utils";

let currentPath = $state("/");
let entries = $state<FileEntry[]>([]);
let drives = $state<{ name: string; path: string; icon: string }[]>([]);
let showHidden = $state(false);
let loading = $state(true);
let error = $state<string | null>(null);
let selectedPath = $state<string | null>(null);
let selectedEntry = $state<FileEntry | null>(null);

// Sort state
let sortBy = $state("name");
let sortAsc = $state(true);
let viewMode = $state<"list" | "grid">("list");

// Rename / inline create state
let renamingPath = $state<string | null>(null);
let creatingEntry = $state<"file" | "directory" | null>(null);

// Clipboard state
let clipboard = $state<{ entries: FileEntry[]; mode: "copy" | "cut" } | null>(null);

// Open With state
let openWithApps = $state<Array<{ name: string; desktop_id: string; icon: string }>>([]);

// Properties dialog state
let propertiesData = $state<FileProperties | null>(null);

// Folder picker state
let folderPicker = $state<{ mode: "move" | "copy" | "extract"; entry: FileEntry } | null>(null);

// Compress dialog state
let compressEntry = $state<FileEntry | null>(null);

// Background operation indicator
let busyMessage = $state<string | null>(null);
let busyProgress = $state<{ processed: number; total: number } | null>(null);
let progressUnlisten: UnlistenFn | null = null;

const archiveExtensions = /\.(zip|tar|tar\.gz|tgz|tar\.xz|tar\.bz2|tar\.zst)$/i;
function formatBusyProgress(processed: number, total: number): string {
	return formatSize(processed) + " / " + formatSize(total);
}

function isArchive(entry: FileEntry): boolean {
	return !entry.is_dir && archiveExtensions.test(entry.name);
}

// Context menu state
let contextMenu = $state<{ x: number; y: number; entry: FileEntry } | null>(
	null,
);

// Navigation history
let history = $state<string[]>([]);
let historyIndex = $state(-1);

let sortedEntries = $derived.by(() => {
	const nameCmp = (a: FileEntry, b: FileEntry) =>
		a.name.toLowerCase().localeCompare(b.name.toLowerCase());

	const sorted = [...entries];
	sorted.sort((a, b) => {
		// Directories always first, files always after
		if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;

		// For size column, sort directories by children count instead of inode size
		if (a.is_dir && b.is_dir && sortBy === "size") {
			const cmp = (a.children_count ?? 0) - (b.children_count ?? 0);
			const directed = sortAsc ? cmp : -cmp;
			return directed !== 0 ? directed : nameCmp(a, b);
		}

		let cmp = 0;
		if (sortBy === "name") {
			cmp = nameCmp(a, b);
		} else if (sortBy === "size") {
			cmp = a.size - b.size;
		} else if (sortBy === "modified") {
			cmp = a.modified.localeCompare(b.modified);
		}

		// Apply sort direction, then stable tiebreaker by name
		const directed = sortAsc ? cmp : -cmp;
		return directed !== 0 ? directed : nameCmp(a, b);
	});
	return sorted;
});

let isTrash = $derived(currentPath === "trash://");

async function navigate(path: string, addToHistory = true) {
	loading = true;
	error = null;
	selectedPath = null;
	selectedEntry = null;
	contextMenu = null;

	try {
		if (path === "trash://") {
			entries = await listTrash();
		} else {
			entries = await listDirectory(path, showHidden);
		}
		currentPath = path;

		if (addToHistory) {
			history = [...history.slice(0, historyIndex + 1), path];
			historyIndex = history.length - 1;
		}
	} catch (e) {
		error = String(e);
	} finally {
		loading = false;
	}
}

function goBack() {
	if (historyIndex > 0) {
		historyIndex--;
		navigate(history[historyIndex], false);
	}
}

function goForward() {
	if (historyIndex < history.length - 1) {
		historyIndex++;
		navigate(history[historyIndex], false);
	}
}

function goUp() {
	const parent = parentPath(currentPath);
	if (parent !== currentPath) {
		navigate(parent);
	}
}

async function handleOpen(entry: FileEntry) {
	if (entry.is_dir) {
		navigate(entry.path);
		return;
	}
	try {
		await openDefault(entry.path);
	} catch (e) {
		error = "Failed to open: " + String(e);
	}
}

async function handleOpenWith(entry: FileEntry) {
	const pos = { x: contextMenu?.x ?? 0, y: contextMenu?.y ?? 0 };
	try {
		const apps = await listAppsForMime(entry.mime_type);
		if (apps.length === 0) {
			error = "No applications found for this file type";
			return;
		}
		openWithApps = apps;
		contextMenu = { x: pos.x, y: pos.y, entry };
	} catch (e) {
		error = "Failed to list applications: " + String(e);
	}
}

async function handleExtract() {
	if (!selectedEntry || busyMessage) return;
	const entry = selectedEntry;
	const dest = parentPath(entry.path);
	busyMessage = "Extracting\u2026";
	busyProgress = null;
	try {
		await extract(entry.path, dest);
	} catch (e) {
		const msg = String(e);
		if (msg !== "Cancelled") {
			error = "Failed to extract: " + msg;
		}
	}
	busyMessage = null;
	busyProgress = null;
	await navigate(currentPath, false);
}

function handleMoveTo() {
	if (!selectedEntry) return;
	folderPicker = { mode: "move", entry: selectedEntry };
}

function handleCopyTo() {
	if (!selectedEntry) return;
	folderPicker = { mode: "copy", entry: selectedEntry };
}

async function handleFolderPickerSelect(destDir: string) {
	if (!folderPicker) return;
	const src = folderPicker.entry;
	const mode = folderPicker.mode;
	folderPicker = null;

	if (mode === "extract") {
		busyMessage = "Extracting\u2026";
		busyProgress = null;
		try {
			await extract(src.path, destDir);
		} catch (e) {
			const msg = String(e);
			if (msg !== "Cancelled") {
				error = "Failed to extract: " + msg;
			}
		}
		busyMessage = null;
		busyProgress = null;
		await navigate(currentPath, false);
		return;
	}

	const dest = destDir === "/" ? "/" + src.name : destDir + "/" + src.name;
	try {
		if (mode === "move") {
			await moveEntry(src.path, dest);
		} else {
			await copyEntry(src.path, dest);
		}
		await navigate(currentPath, false);
	} catch (e) {
		error = "Failed to " + mode + ": " + String(e);
	}
}

function handleCompress() {
	if (!selectedEntry) return;
	compressEntry = selectedEntry;
}

async function handleCompressConfirm(archiveName: string) {
	if (!compressEntry || busyMessage) return;
	const entry = compressEntry;
	compressEntry = null;
	const dest = currentPath === "/" ? "/" + archiveName : currentPath + "/" + archiveName;
	busyMessage = "Compressing\u2026";
	busyProgress = null;
	try {
		await compress([entry.path], dest);
	} catch (e) {
		const msg = String(e);
		if (msg !== "Cancelled") {
			error = "Failed to compress: " + msg;
		}
	}
	busyMessage = null;
	busyProgress = null;
	await navigate(currentPath, false);
}

async function handleCancelOperation() {
	await cancelOperation();
}

async function handleProperties() {
	if (!selectedEntry) return;
	try {
		propertiesData = await getProperties(selectedEntry.path);
	} catch (e) {
		error = "Failed to get properties: " + String(e);
	}
}

function handleExtractTo() {
	if (!selectedEntry) return;
	folderPicker = { mode: "extract", entry: selectedEntry };
}

function handleSelect(entry: FileEntry) {
	selectedPath = entry.path;
	selectedEntry = entry;
}

function handleSort(column: string) {
	if (sortBy === column) {
		sortAsc = !sortAsc;
	} else {
		sortBy = column;
		sortAsc = true;
	}
}

function handleContextMenu(e: MouseEvent, entry: FileEntry) {
	e.stopPropagation();
	selectedPath = entry.path;
	selectedEntry = entry;
	openWithApps = [];
	contextMenu = { x: e.clientX, y: e.clientY, entry };
}

function handleBgContextMenu(e: MouseEvent) {
	e.preventDefault();
	selectedPath = null;
	selectedEntry = null;
	contextMenu = {
		x: e.clientX,
		y: e.clientY,
		entry: null as unknown as FileEntry,
	};
}

function handleCopy() {
	if (!selectedEntry) return;
	clipboard = { entries: [selectedEntry], mode: "copy" };
}

function handleCut() {
	if (!selectedEntry) return;
	clipboard = { entries: [selectedEntry], mode: "cut" };
}

async function handlePaste() {
	if (!clipboard) return;
	const isCut = clipboard.mode === "cut";

	try {
		for (const src of clipboard.entries) {
			const destPath = currentPath === "/" ? `/${src.name}` : `${currentPath}/${src.name}`;
			if (isCut) {
				await moveEntry(src.path, destPath);
			} else {
				await copyEntry(src.path, destPath);
			}
		}
		if (isCut) clipboard = null;
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to ${isCut ? "move" : "paste"}: ${e}`;
	}
}

async function handleDelete() {
	if (!selectedEntry) return;
	const name = selectedEntry.name;
	try {
		await deleteEntry(selectedEntry.path);
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to delete ${name}: ${e}`;
	}
}

function handleRename() {
	if (!selectedEntry) return;
	renamingPath = selectedEntry.path;
}

async function commitRename(entry: FileEntry, newName: string) {
	renamingPath = null;
	if (!newName || newName === entry.name) return;

	const parent = parentPath(entry.path);
	const newPath = parent === "/" ? `/${newName}` : `${parent}/${newName}`;

	try {
		await renameEntry(entry.path, newPath);
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to rename: ${e}`;
	}
}

function handleNewFolder() {
	creatingEntry = "directory";
}

function handleNewFile() {
	creatingEntry = "file";
}

async function commitCreate(name: string) {
	const type = creatingEntry;
	creatingEntry = null;
	if (!name || !type) return;

	const path = currentPath === "/" ? `/${name}` : `${currentPath}/${name}`;
	try {
		if (type === "directory") {
			await createDirectory(path);
		} else {
			await createFile(path);
		}
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to create ${type === "directory" ? "folder" : "file"}: ${e}`;
	}
}

function toggleHidden() {
	showHidden = !showHidden;
	navigate(currentPath, false);
}

async function handleRestore() {
	if (!selectedEntry) return;
	try {
		await restoreTrash(selectedEntry.name);
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to restore: ${e}`;
	}
}

async function handleEmptyTrash() {
	try {
		await emptyTrash();
		await navigate(currentPath, false);
	} catch (e) {
		error = `Failed to empty trash: ${e}`;
	}
}

function getContextMenuItems(): MenuEntry[] {
	if (!contextMenu) return [];

	// "Open with" submenu items
	if (openWithApps.length > 0) {
		const items: MenuEntry[] = openWithApps.map((app) => ({
			label: app.name,
			action: () => {
				openWithApp(contextMenu!.entry.path, app.desktop_id);
				openWithApps = [];
			},
		}));
		return items;
	}

	if (isTrash) {
		if (contextMenu.entry) {
			return [
				{ label: "Restore", action: handleRestore },
				{ label: "Delete Permanently", action: handleDelete, danger: true },
			];
		}
		return [
			{ label: "Empty Trash", action: handleEmptyTrash, danger: true },
		];
	}

	if (contextMenu.entry) {
		const entry = contextMenu.entry;
		const items: MenuEntry[] = [
			{ label: "Open", action: () => handleOpen(entry) },
			{ label: "Open With\u2026", action: () => handleOpenWith(entry) },
			{ separator: true },
			{ label: "Cut", action: handleCut },
			{ label: "Copy", action: handleCopy },
			{ label: "Move to\u2026", action: handleMoveTo },
			{ label: "Copy to\u2026", action: handleCopyTo },
			{ label: "Rename", action: handleRename },
			{ separator: true },
		];

		if (isArchive(entry)) {
			items.push(
				{ label: "Extract Here", action: handleExtract },
				{ label: "Extract to Folder\u2026", action: handleExtractTo },
			);
		}

		items.push(
			{ label: "Compress\u2026", action: handleCompress },
			{ separator: true },
			{ label: "Move to Trash", action: handleDelete, danger: true },
			{ separator: true },
			{ label: "Properties", action: handleProperties },
		);

		return items;
	}

	// Background context menu
	const items: MenuEntry[] = [];
	if (clipboard) {
		const pasteLabel = clipboard.entries.length === 1
			? "Paste \u201C" + clipboard.entries[0].name + "\u201D"
			: "Paste " + clipboard.entries.length + " items";
		items.push(
			{ label: pasteLabel, action: handlePaste },
			{ separator: true },
		);
	}
	items.push(
		{ label: "New Folder", action: handleNewFolder },
		{ label: "New File", action: handleNewFile },
		{ separator: true },
		{
			label: showHidden ? "Hide Hidden Files" : "Show Hidden Files",
			action: toggleHidden,
		},
	);
	return items;
}

onMount(async () => {
	try {
		const home = await getHomeDir();
		await navigate(home);
	} catch {
		await navigate("/");
	}

	try {
		const d = await listDrives();
		drives = d.map((drive) => ({
			name: drive.name,
			path: drive.path,
			icon: drive.removable ? "\uF0A0" : "\uF0A0",
		}));
	} catch {
		drives = [];
	}

	progressUnlisten = await listen<{ processed: number; total: number }>("compress-progress", (event) => {
		busyProgress = event.payload;
	});
});

onDestroy(() => {
	progressUnlisten?.();
});
</script>

<div class="app">
	<div class="toolbar">
		<button class="nav-btn icon" onclick={goBack} disabled={historyIndex <= 0} title="Back">
			{"\uf060"}
		</button>
		<button
			class="nav-btn icon"
			onclick={goForward}
			disabled={historyIndex >= history.length - 1}
			title="Forward"
		>
			{"\uf061"}
		</button>
		<button class="nav-btn icon" onclick={goUp} title="Up">{"\uf062"}</button>
		<div class="breadcrumb-wrapper">
			<Breadcrumb path={currentPath} onnavigate={navigate} />
		</div>
		<div class="toolbar-group">
			<div class="sort-control">
				{#each [["name", "Name"], ["size", "Size"], ["modified", "Date"]] as [key, label] (key)}
					<button class="sort-btn" class:active={sortBy === key} onclick={() => handleSort(key)}>
						{label}<span class="sort-arrow" class:visible={sortBy === key}>{sortAsc ? "▲" : "▼"}</span>
					</button>
				{/each}
			</div>
			<button class="nav-btn icon" class:active={viewMode === "grid"} onclick={() => viewMode = viewMode === "list" ? "grid" : "list"} title="Toggle view mode">
				{viewMode === "list" ? "\uf00a" : "\uf00b"}
			</button>
			<button class="nav-btn icon" class:active={showHidden} onclick={toggleHidden} title="Toggle hidden files">
				{showHidden ? "\uf06e" : "\uf070"}
			</button>
		</div>
	</div>

	{#if error}
		<div class="error-bar">
			<span>{error}</span>
			<button onclick={() => (error = null)}>✕</button>
		</div>
	{/if}

	<div class="main">
		<Sidebar {currentPath} onnavigate={navigate} {drives} />

		{#if loading}
			<div class="loading">Loading...</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="content-wrapper">
				{#if isTrash}
					<div class="context-bar">
						<span class="context-bar-text">
							{entries.length === 0 ? "Trash is empty" : `${entries.length} ${entries.length === 1 ? "item" : "items"} in trash`}
						</span>
						{#if entries.length > 0}
							<button class="context-bar-action" onclick={handleEmptyTrash}>Empty Trash</button>
						{/if}
					</div>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="content" oncontextmenu={handleBgContextMenu}>
				{#if viewMode === "list"}
					<FileList
						entries={sortedEntries}
						{selectedPath}
						{renamingPath}
						{creatingEntry}
						clipboardPaths={clipboard ? new Set(clipboard.entries.map(e => e.path)) : null}
						clipboardMode={clipboard?.mode ?? null}
						{sortBy}
						{sortAsc}
						onopen={handleOpen}
						onselect={handleSelect}
						oncontextmenu={handleContextMenu}
						onsort={handleSort}
						onrename={commitRename}
						oncreate={commitCreate}
					/>
				{:else}
					<FileGrid
						entries={sortedEntries}
						{selectedPath}
						{renamingPath}
						{creatingEntry}
						clipboardPaths={clipboard ? new Set(clipboard.entries.map(e => e.path)) : null}
						clipboardMode={clipboard?.mode ?? null}
						onopen={handleOpen}
						onselect={handleSelect}
						oncontextmenu={handleContextMenu}
						onrename={commitRename}
						oncreate={commitCreate}
					/>
				{/if}
				</div>
			</div>
		{/if}
	</div>

	{#if clipboard}
		<div class="status-bar">
			<span class="status-text">
				{clipboard.mode === "cut" ? "Moving" : "Copied"}: {clipboard.entries.length === 1 ? clipboard.entries[0].name : clipboard.entries.length + " items"}
			</span>
			<button class="status-clear" onclick={() => clipboard = null}>Clear</button>
		</div>
	{/if}

	{#if contextMenu}
		<ContextMenu
			x={contextMenu.x}
			y={contextMenu.y}
			items={getContextMenuItems()}
			onclose={() => { contextMenu = null; openWithApps = []; }}
		/>
	{/if}

	{#if propertiesData}
		<PropertiesDialog
			properties={propertiesData}
			onclose={() => (propertiesData = null)}
		/>
	{/if}

	{#if folderPicker}
		<FolderPicker
			title={folderPicker.mode === "move" ? "Move to\u2026" : folderPicker.mode === "extract" ? "Extract to\u2026" : "Copy to\u2026"}
			onselect={handleFolderPickerSelect}
			onclose={() => (folderPicker = null)}
		/>
	{/if}

	{#if compressEntry}
		<CompressDialog
			defaultName={compressEntry.name}
			onconfirm={(name) => handleCompressConfirm(name)}
			onclose={() => (compressEntry = null)}
		/>
	{/if}

	{#if busyMessage}
		<div class="busy-overlay">
			<div class="busy-card">
				<div class="busy-header">
					<div class="busy-spinner"></div>
					<span class="busy-text">{busyMessage}</span>
				</div>
				{#if busyProgress && busyProgress.total > 0}
					<div class="busy-progress-track">
						<div class="busy-progress-bar" style="width: {Math.min(100, (busyProgress.processed / busyProgress.total) * 100)}%"></div>
					</div>
					<span class="busy-detail">{formatBusyProgress(busyProgress.processed, busyProgress.total)}</span>
				{/if}
				<button class="busy-cancel" onclick={handleCancelOperation}>Cancel</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	.toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 6px 8px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
	}

	.nav-btn {
		background: none;
		border: 1px solid transparent;
		color: var(--text-secondary);
		font-size: 16px;
		width: 32px;
		height: 32px;
		border-radius: var(--radius);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.nav-btn.icon {
		font-family: var(--font-icon);
	}

	.nav-btn:hover:not(:disabled) {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.nav-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.nav-btn.active {
		color: var(--accent);
	}

	.toolbar-group {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.sort-control {
		display: flex;
		align-items: center;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.sort-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		padding: 5px 10px;
		cursor: pointer;
		transition: color 0.15s, background 0.15s;
		letter-spacing: 0.03em;
		white-space: nowrap;
	}

	.sort-btn:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.sort-btn.active {
		color: var(--accent);
		background: var(--bg-surface);
	}

	.context-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 12px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.context-bar-text {
		font-size: 12px;
		color: var(--text-muted);
	}

	.context-bar-action {
		background: none;
		border: none;
		color: var(--danger);
		font-size: 12px;
		font-family: var(--font-sans);
		cursor: pointer;
		padding: 2px 8px;
		border-radius: var(--radius);
		transition: background 0.15s;
	}

	.context-bar-action:hover {
		background: color-mix(in srgb, var(--danger) 15%, transparent);
	}

	.sort-arrow {
		font-size: 8px;
		margin-left: 3px;
		opacity: 0;
		display: inline-block;
		width: 8px;
	}

	.sort-arrow.visible {
		opacity: 0.7;
	}

	.breadcrumb-wrapper {
		flex: 1;
		min-width: 0;
	}

	.error-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 12px;
		background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary));
		color: var(--danger);
		font-size: 13px;
	}

	.error-bar button {
		background: none;
		border: none;
		color: var(--danger);
		cursor: pointer;
		font-size: 14px;
	}

	.loading {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
	}

	.main {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.status-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 12px;
		background: var(--bg-secondary);
		border-top: 1px solid var(--border);
		flex-shrink: 0;
	}

	.status-text {
		font-size: 12px;
		color: var(--text-secondary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-clear {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		cursor: pointer;
		padding: 2px 6px;
		border-radius: var(--radius);
	}

	.status-clear:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.content {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	.busy-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 300;
	}

	.busy-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		padding: 20px 24px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		min-width: 280px;
	}

	.busy-header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.busy-spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.busy-text {
		font-size: 13px;
		color: var(--text-primary);
	}

	.busy-progress-track {
		height: 4px;
		background: var(--bg-surface);
		border-radius: 2px;
		overflow: hidden;
	}

	.busy-progress-bar {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.2s ease;
	}

	.busy-detail {
		font-size: 11px;
		color: var(--text-muted);
		text-align: right;
	}

	.busy-cancel {
		align-self: flex-end;
		background: none;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: 12px;
		font-family: var(--font-sans);
		padding: 4px 14px;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.busy-cancel:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}
</style>
