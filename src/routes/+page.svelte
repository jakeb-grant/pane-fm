<script lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onDestroy, onMount } from "svelte";
import {
	cancelOperation,
	compress,
	copyEntry,
	createDirectory,
	createFile,
	deleteEntry,
	emptyTrash,
	extract,
	type FileEntry,
	type FileProperties,
	getProperties,
	listAppsForMime,
	moveEntry,
	openDefault,
	openWithApp,
	renameEntry,
	restoreTrash,
} from "$lib/commands";
import Breadcrumb from "$lib/components/Breadcrumb.svelte";
import CompressDialog from "$lib/components/CompressDialog.svelte";
import type { MenuEntry } from "$lib/components/ContextMenu.svelte";
import ContextMenu from "$lib/components/ContextMenu.svelte";
import FileGrid from "$lib/components/FileGrid.svelte";
import FileList from "$lib/components/FileList.svelte";
import FolderPicker from "$lib/components/FolderPicker.svelte";
import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
import Sidebar from "$lib/components/Sidebar.svelte";
import { createFileManager } from "$lib/stores/fileManager.svelte";
import { formatSize, parentPath } from "$lib/utils";

const fm = createFileManager();

// Properties dialog state
let propertiesData = $state<FileProperties | null>(null);

// Folder picker state
let folderPicker = $state<{
	mode: "move" | "copy" | "extract";
	entry: FileEntry;
} | null>(null);

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
let contextMenu = $state<{
	x: number;
	y: number;
	entry: FileEntry | null;
} | null>(null);

async function handleOpen(entry: FileEntry) {
	if (entry.is_dir) {
		fm.navigate(entry.path);
		return;
	}
	try {
		await openDefault(entry.path);
	} catch (e) {
		fm.setError("Failed to open: " + String(e));
	}
}

async function handleOpenWith(entry: FileEntry) {
	const pos = { x: contextMenu?.x ?? 0, y: contextMenu?.y ?? 0 };
	try {
		const apps = await listAppsForMime(entry.mime_type);
		if (apps.length === 0) {
			fm.setError("No applications found for this file type");
			return;
		}
		fm.openWithApps = apps;
		contextMenu = { x: pos.x, y: pos.y, entry };
	} catch (e) {
		fm.setError("Failed to list applications: " + String(e));
	}
}

async function handleExtract() {
	if (!fm.selectedEntry || busyMessage) return;
	const entry = fm.selectedEntry;
	const dest = parentPath(entry.path);
	busyMessage = "Extracting\u2026";
	busyProgress = null;
	try {
		await extract(entry.path, dest);
	} catch (e) {
		const msg = String(e);
		if (msg !== "Cancelled") {
			fm.setError("Failed to extract: " + msg);
		}
	}
	busyMessage = null;
	busyProgress = null;
	await fm.refresh();
}

function handleMoveTo() {
	if (!fm.selectedEntry) return;
	folderPicker = { mode: "move", entry: fm.selectedEntry };
}

function handleCopyTo() {
	if (!fm.selectedEntry) return;
	folderPicker = { mode: "copy", entry: fm.selectedEntry };
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
				fm.setError("Failed to extract: " + msg);
			}
		}
		busyMessage = null;
		busyProgress = null;
		await fm.refresh();
		return;
	}

	const dest = destDir === "/" ? "/" + src.name : destDir + "/" + src.name;
	try {
		if (mode === "move") {
			await moveEntry(src.path, dest);
		} else {
			await copyEntry(src.path, dest);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError("Failed to " + mode + ": " + String(e));
	}
}

function handleCompress() {
	if (!fm.selectedEntry) return;
	compressEntry = fm.selectedEntry;
}

async function handleCompressConfirm(archiveName: string) {
	if (!compressEntry || busyMessage) return;
	const entry = compressEntry;
	compressEntry = null;
	const dest =
		fm.currentPath === "/"
			? "/" + archiveName
			: fm.currentPath + "/" + archiveName;
	busyMessage = "Compressing\u2026";
	busyProgress = null;
	try {
		await compress([entry.path], dest);
	} catch (e) {
		const msg = String(e);
		if (msg !== "Cancelled") {
			fm.setError("Failed to compress: " + msg);
		}
	}
	busyMessage = null;
	busyProgress = null;
	await fm.refresh();
}

async function handleCancelOperation() {
	await cancelOperation();
}

async function handleProperties() {
	if (!fm.selectedEntry) return;
	try {
		propertiesData = await getProperties(fm.selectedEntry.path);
	} catch (e) {
		fm.setError("Failed to get properties: " + String(e));
	}
}

function handleExtractTo() {
	if (!fm.selectedEntry) return;
	folderPicker = { mode: "extract", entry: fm.selectedEntry };
}

function handleContextMenu(e: MouseEvent, entry: FileEntry) {
	e.stopPropagation();
	fm.select(entry);
	fm.openWithApps = [];
	contextMenu = { x: e.clientX, y: e.clientY, entry };
}

function handleBgContextMenu(e: MouseEvent) {
	e.preventDefault();
	fm.clearSelection();
	contextMenu = {
		x: e.clientX,
		y: e.clientY,
		entry: null,
	};
}

function handleCopy() {
	if (!fm.selectedEntry) return;
	fm.clipboard = { entries: [fm.selectedEntry], mode: "copy" };
}

function handleCut() {
	if (!fm.selectedEntry) return;
	fm.clipboard = { entries: [fm.selectedEntry], mode: "cut" };
}

async function handlePaste() {
	if (!fm.clipboard) return;
	const isCut = fm.clipboard.mode === "cut";

	try {
		for (const src of fm.clipboard.entries) {
			const destPath =
				fm.currentPath === "/"
					? `/${src.name}`
					: `${fm.currentPath}/${src.name}`;
			if (isCut) {
				await moveEntry(src.path, destPath);
			} else {
				await copyEntry(src.path, destPath);
			}
		}
		if (isCut) fm.clipboard = null;
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to ${isCut ? "move" : "paste"}: ${e}`);
	}
}

async function handleDelete() {
	if (!fm.selectedEntry) return;
	const name = fm.selectedEntry.name;
	try {
		await deleteEntry(fm.selectedEntry.path);
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to delete ${name}: ${e}`);
	}
}

function handleRename() {
	if (!fm.selectedEntry) return;
	fm.renamingPath = fm.selectedEntry.path;
}

async function commitRename(entry: FileEntry, newName: string) {
	fm.renamingPath = null;
	if (!newName || newName === entry.name) return;

	const parent = parentPath(entry.path);
	const newPath = parent === "/" ? `/${newName}` : `${parent}/${newName}`;

	try {
		await renameEntry(entry.path, newPath);
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to rename: ${e}`);
	}
}

function handleNewFolder() {
	fm.creatingEntry = "directory";
}

function handleNewFile() {
	fm.creatingEntry = "file";
}

async function commitCreate(name: string) {
	const type = fm.creatingEntry;
	fm.creatingEntry = null;
	if (!name || !type) return;

	const path =
		fm.currentPath === "/" ? `/${name}` : `${fm.currentPath}/${name}`;
	try {
		if (type === "directory") {
			await createDirectory(path);
		} else {
			await createFile(path);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(
			`Failed to create ${type === "directory" ? "folder" : "file"}: ${e}`,
		);
	}
}

async function handleRestore() {
	if (!fm.selectedEntry) return;
	try {
		await restoreTrash(fm.selectedEntry.name);
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to restore: ${e}`);
	}
}

async function handleEmptyTrash() {
	try {
		await emptyTrash();
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to empty trash: ${e}`);
	}
}

function getContextMenuItems(): MenuEntry[] {
	if (!contextMenu) return [];

	// "Open with" submenu items
	if (fm.openWithApps.length > 0) {
		const items: MenuEntry[] = fm.openWithApps.map((app) => ({
			label: app.name,
			action: () => {
				openWithApp(contextMenu?.entry?.path ?? "", app.desktop_id);
				fm.openWithApps = [];
			},
		}));
		return items;
	}

	if (fm.isTrash) {
		if (contextMenu.entry) {
			return [
				{ label: "Restore", action: handleRestore },
				{ label: "Delete Permanently", action: handleDelete, danger: true },
			];
		}
		return [{ label: "Empty Trash", action: handleEmptyTrash, danger: true }];
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
	if (fm.clipboard) {
		const pasteLabel =
			fm.clipboard.entries.length === 1
				? "Paste \u201C" + fm.clipboard.entries[0].name + "\u201D"
				: "Paste " + fm.clipboard.entries.length + " items";
		items.push({ label: pasteLabel, action: handlePaste }, { separator: true });
	}
	items.push(
		{ label: "New Folder", action: handleNewFolder },
		{ label: "New File", action: handleNewFile },
		{ separator: true },
		{
			label: fm.showHidden ? "Hide Hidden Files" : "Show Hidden Files",
			action: fm.toggleHidden,
		},
	);
	return items;
}

onMount(async () => {
	await fm.init();

	progressUnlisten = await listen<{ processed: number; total: number }>(
		"compress-progress",
		(event) => {
			busyProgress = event.payload;
		},
	);
});

onDestroy(() => {
	progressUnlisten?.();
});
</script>

<div class="app">
	<div class="toolbar">
		<button class="nav-btn icon" onclick={fm.goBack} disabled={fm.historyIndex <= 0} title="Back">
			{"\uf060"}
		</button>
		<button
			class="nav-btn icon"
			onclick={fm.goForward}
			disabled={fm.historyIndex >= fm.history.length - 1}
			title="Forward"
		>
			{"\uf061"}
		</button>
		<button class="nav-btn icon" onclick={fm.goUp} title="Up">{"\uf062"}</button>
		<div class="breadcrumb-wrapper">
			<Breadcrumb path={fm.currentPath} onnavigate={fm.navigate} />
		</div>
		<div class="toolbar-group">
			<div class="sort-control">
				{#each [["name", "Name"], ["size", "Size"], ["modified", "Date"]] as [key, label] (key)}
					<button class="sort-btn" class:active={fm.sortBy === key} onclick={() => fm.handleSort(key)}>
						{label}<span class="sort-arrow" class:visible={fm.sortBy === key}>{fm.sortAsc ? "▲" : "▼"}</span>
					</button>
				{/each}
			</div>
			<button class="nav-btn icon" class:active={fm.viewMode === "grid"} onclick={() => fm.setViewMode(fm.viewMode === "list" ? "grid" : "list")} title="Toggle view mode">
				{fm.viewMode === "list" ? "\uf00a" : "\uf00b"}
			</button>
			<button class="nav-btn icon" class:active={fm.showHidden} onclick={fm.toggleHidden} title="Toggle hidden files">
				{fm.showHidden ? "\uf06e" : "\uf070"}
			</button>
		</div>
	</div>

	{#if fm.error}
		<div class="error-bar">
			<span>{fm.error}</span>
			<button onclick={() => fm.setError(null)}>✕</button>
		</div>
	{/if}

	<div class="main">
		<Sidebar currentPath={fm.currentPath} onnavigate={fm.navigate} drives={fm.drives} homeDir={fm.homeDir} />

		{#if fm.loading}
			<div class="loading">Loading...</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="content-wrapper">
				{#if fm.isTrash}
					<div class="context-bar">
						<span class="context-bar-text">
							{fm.entries.length === 0 ? "Trash is empty" : `${fm.entries.length} ${fm.entries.length === 1 ? "item" : "items"} in trash`}
						</span>
						{#if fm.entries.length > 0}
							<button class="context-bar-action" onclick={handleEmptyTrash}>Empty Trash</button>
						{/if}
					</div>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="content" oncontextmenu={handleBgContextMenu}>
				{#if fm.viewMode === "list"}
					<FileList
						entries={fm.sortedEntries}
						selectedPath={fm.selectedPath}
						renamingPath={fm.renamingPath}
						creatingEntry={fm.creatingEntry}
						clipboardPaths={fm.clipboard ? new Set(fm.clipboard.entries.map(e => e.path)) : null}
						clipboardMode={fm.clipboard?.mode ?? null}
						sortBy={fm.sortBy}
						sortAsc={fm.sortAsc}
						onopen={handleOpen}
						onselect={fm.select}
						oncontextmenu={handleContextMenu}
						onsort={fm.handleSort}
						onrename={commitRename}
						oncreate={commitCreate}
					/>
				{:else}
					<FileGrid
						entries={fm.sortedEntries}
						selectedPath={fm.selectedPath}
						renamingPath={fm.renamingPath}
						creatingEntry={fm.creatingEntry}
						clipboardPaths={fm.clipboard ? new Set(fm.clipboard.entries.map(e => e.path)) : null}
						clipboardMode={fm.clipboard?.mode ?? null}
						onopen={handleOpen}
						onselect={fm.select}
						oncontextmenu={handleContextMenu}
						onrename={commitRename}
						oncreate={commitCreate}
					/>
				{/if}
				</div>
			</div>
		{/if}
	</div>

	{#if fm.clipboard}
		<div class="status-bar">
			<span class="status-text">
				{fm.clipboard.mode === "cut" ? "Moving" : "Copied"}: {fm.clipboard.entries.length === 1 ? fm.clipboard.entries[0].name : fm.clipboard.entries.length + " items"}
			</span>
			<button class="status-clear" onclick={() => fm.clipboard = null}>Clear</button>
		</div>
	{/if}

	{#if contextMenu}
		<ContextMenu
			x={contextMenu.x}
			y={contextMenu.y}
			items={getContextMenuItems()}
			onclose={() => { contextMenu = null; fm.openWithApps = []; }}
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
