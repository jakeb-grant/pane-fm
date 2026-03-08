<script lang="ts">
import { onDestroy, onMount } from "svelte";
import Breadcrumb from "$lib/components/Breadcrumb.svelte";
import CompressDialog from "$lib/components/CompressDialog.svelte";
import ContextMenu from "$lib/components/ContextMenu.svelte";
import FileGrid from "$lib/components/FileGrid.svelte";
import FileList from "$lib/components/FileList.svelte";
import FolderPicker from "$lib/components/FolderPicker.svelte";
import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
import Sidebar from "$lib/components/Sidebar.svelte";
import {
	type ContextMenuActions,
	type ContextMenuContext,
	getContextMenuItems,
} from "$lib/contextMenu";
import * as ops from "$lib/fileOps";
import { createDialogManager } from "$lib/stores/dialogs.svelte";
import { createFileManager } from "$lib/stores/fileManager.svelte";
import { formatSize } from "$lib/utils";

const fm = createFileManager();
const dlg = createDialogManager(fm);

function formatBusyProgress(processed: number, total: number): string {
	return `${formatSize(processed)} / ${formatSize(total)}`;
}

// --- Context menu wiring ---

const menuActions: ContextMenuActions = {
	open: (entry) => ops.handleOpen(fm, entry),
	openWith: (entry, pos) =>
		ops.handleOpenWith(fm, entry, pos, (menu) =>
			dlg.openContextMenu(menu.x, menu.y, menu.entry),
		),
	cut: () => ops.handleCut(fm),
	copy: () => ops.handleCopy(fm),
	paste: () => ops.handlePaste(fm),
	rename: () => ops.handleRename(fm),
	moveTo: () =>
		ops.handleMoveTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entry)),
	copyTo: () =>
		ops.handleCopyTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entry)),
	delete: () => ops.handleDelete(fm),
	extract: dlg.handleExtract,
	extractTo: dlg.handleExtractTo,
	compress: dlg.handleCompress,
	properties: dlg.handleProperties,
	restore: () => ops.handleRestore(fm),
	emptyTrash: () => ops.handleEmptyTrash(fm),
	newFolder: () => ops.handleNewFolder(fm),
	newFile: () => ops.handleNewFile(fm),
	toggleHidden: fm.toggleHidden,
	launchApp: (filePath, desktopId) =>
		ops.launchOpenWithApp(fm, filePath, desktopId),
};

function buildMenuItems() {
	if (!dlg.contextMenu) return [];
	const ctx: ContextMenuContext = dlg.contextMenu.entry
		? {
				kind: "entry",
				entry: dlg.contextMenu.entry,
				x: dlg.contextMenu.x,
				y: dlg.contextMenu.y,
			}
		: { kind: "background" };
	return getContextMenuItems(ctx, fm, menuActions);
}

onMount(async () => {
	await fm.init();
	await dlg.subscribeProgress();
});

onDestroy(() => {
	dlg.unsubscribeProgress();
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
				<div class="content" oncontextmenu={(e) => ops.handleBgContextMenu(fm, e, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}>
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
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}
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
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}
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

	{#if dlg.contextMenu}
		<ContextMenu
			x={dlg.contextMenu.x}
			y={dlg.contextMenu.y}
			items={buildMenuItems()}
			onclose={dlg.closeContextMenu}
		/>
	{/if}

	{#if dlg.propertiesData}
		<PropertiesDialog
			properties={dlg.propertiesData}
			onclose={dlg.closeProperties}
		/>
	{/if}

	{#if dlg.folderPicker}
		<FolderPicker
			title={dlg.folderPicker.mode === "move" ? "Move to\u2026" : dlg.folderPicker.mode === "extract" ? "Extract to\u2026" : "Copy to\u2026"}
			onselect={dlg.handleFolderPickerSelect}
			onclose={dlg.closeFolderPicker}
		/>
	{/if}

	{#if dlg.compressEntry}
		<CompressDialog
			defaultName={dlg.compressEntry.name}
			onconfirm={dlg.handleCompressConfirm}
			onclose={dlg.closeCompress}
		/>
	{/if}

	{#if dlg.busyMessage}
		<div class="busy-overlay">
			<div class="busy-card">
				<div class="busy-header">
					<div class="busy-spinner"></div>
					<span class="busy-text">{dlg.busyMessage}</span>
				</div>
				{#if dlg.busyProgress && dlg.busyProgress.total > 0}
					<div class="busy-progress-track">
						<div class="busy-progress-bar" style="width: {Math.min(100, (dlg.busyProgress.processed / dlg.busyProgress.total) * 100)}%"></div>
					</div>
					<span class="busy-detail">{formatBusyProgress(dlg.busyProgress.processed, dlg.busyProgress.total)}</span>
				{/if}
				<button class="busy-cancel" onclick={dlg.handleCancelOperation}>Cancel</button>
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
