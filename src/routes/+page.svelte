<script lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onDestroy, onMount } from "svelte";
import {
	cancelOperation,
	compress,
	extract,
	type FileEntry,
	type FileProperties,
	openWithApp,
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
import * as ops from "$lib/fileOps";
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

function setContextMenu(menu: {
	x: number;
	y: number;
	entry: FileEntry | null;
}) {
	contextMenu = menu;
}

function setFolderPicker(v: {
	mode: "move" | "copy" | "extract";
	entry: FileEntry;
}) {
	folderPicker = v;
}

function setPropertiesData(v: FileProperties) {
	propertiesData = v;
}

// --- Handlers that still live here (busy/progress/dialog orchestration — will move to 1.3) ---

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

function handleExtractTo() {
	if (!fm.selectedEntry) return;
	folderPicker = { mode: "extract", entry: fm.selectedEntry };
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

async function handleFolderPickerSelectWrapper(destDir: string) {
	if (!folderPicker) return;
	const fp = folderPicker;
	folderPicker = null;

	if (fp.mode === "extract") {
		busyMessage = "Extracting\u2026";
		busyProgress = null;
		try {
			await extract(fp.entry.path, destDir);
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

	await ops.handleFolderPickerSelect(fm, fp, destDir);
}

// --- Context menu builder ---

function getContextMenuItems(): MenuEntry[] {
	if (!contextMenu) return [];

	// "Open with" submenu items
	if (fm.openWithApps.length > 0) {
		const items: MenuEntry[] = fm.openWithApps.map((app) => ({
			label: app.name,
			action: () => {
				ops.launchOpenWithApp(
					fm,
					contextMenu?.entry?.path ?? "",
					app.desktop_id,
				);
			},
		}));
		return items;
	}

	if (fm.isTrash) {
		if (contextMenu.entry) {
			return [
				{ label: "Restore", action: () => ops.handleRestore(fm) },
				{
					label: "Delete Permanently",
					action: () => ops.handleDelete(fm),
					danger: true,
				},
			];
		}
		return [
			{
				label: "Empty Trash",
				action: () => ops.handleEmptyTrash(fm),
				danger: true,
			},
		];
	}

	if (contextMenu.entry) {
		const entry = contextMenu.entry;
		const items: MenuEntry[] = [
			{ label: "Open", action: () => ops.handleOpen(fm, entry) },
			{
				label: "Open With\u2026",
				action: () =>
					ops.handleOpenWith(
						fm,
						entry,
						{ x: contextMenu?.x ?? 0, y: contextMenu?.y ?? 0 },
						setContextMenu,
					),
			},
			{ separator: true },
			{ label: "Cut", action: () => ops.handleCut(fm) },
			{ label: "Copy", action: () => ops.handleCopy(fm) },
			{
				label: "Move to\u2026",
				action: () => ops.handleMoveTo(fm, setFolderPicker),
			},
			{
				label: "Copy to\u2026",
				action: () => ops.handleCopyTo(fm, setFolderPicker),
			},
			{ label: "Rename", action: () => ops.handleRename(fm) },
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
			{
				label: "Move to Trash",
				action: () => ops.handleDelete(fm),
				danger: true,
			},
			{ separator: true },
			{
				label: "Properties",
				action: () => ops.handleProperties(fm, setPropertiesData),
			},
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
		items.push(
			{ label: pasteLabel, action: () => ops.handlePaste(fm) },
			{ separator: true },
		);
	}
	items.push(
		{ label: "New Folder", action: () => ops.handleNewFolder(fm) },
		{ label: "New File", action: () => ops.handleNewFile(fm) },
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
							<button class="context-bar-action" onclick={() => ops.handleEmptyTrash(fm)}>Empty Trash</button>
						{/if}
					</div>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="content" oncontextmenu={(e) => ops.handleBgContextMenu(fm, e, setContextMenu)}>
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
						onopen={(entry) => ops.handleOpen(fm, entry)}
						onselect={fm.select}
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, setContextMenu)}
						onsort={fm.handleSort}
						onrename={(entry, name) => ops.commitRename(fm, entry, name)}
						oncreate={(name) => ops.commitCreate(fm, name)}
					/>
				{:else}
					<FileGrid
						entries={fm.sortedEntries}
						selectedPath={fm.selectedPath}
						renamingPath={fm.renamingPath}
						creatingEntry={fm.creatingEntry}
						clipboardPaths={fm.clipboard ? new Set(fm.clipboard.entries.map(e => e.path)) : null}
						clipboardMode={fm.clipboard?.mode ?? null}
						onopen={(entry) => ops.handleOpen(fm, entry)}
						onselect={fm.select}
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, setContextMenu)}
						onrename={(entry, name) => ops.commitRename(fm, entry, name)}
						oncreate={(name) => ops.commitCreate(fm, name)}
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
			onselect={handleFolderPickerSelectWrapper}
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
