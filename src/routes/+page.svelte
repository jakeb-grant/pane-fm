<script lang="ts">
import { onDestroy, onMount, tick } from "svelte";
import BusyOverlay from "$lib/components/BusyOverlay.svelte";
import CompressDialog from "$lib/components/CompressDialog.svelte";
import ContextMenu from "$lib/components/ContextMenu.svelte";
import FileList from "$lib/components/FileList.svelte";
// biome-ignore lint/style/useImportType: component used in template
import FilterBar from "$lib/components/FilterBar.svelte";
import FolderPicker from "$lib/components/FolderPicker.svelte";
import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
import Sidebar from "$lib/components/Sidebar.svelte";
import StatusBar from "$lib/components/StatusBar.svelte";
import Toolbar from "$lib/components/Toolbar.svelte";
import {
	type ContextMenuActions,
	type ContextMenuContext,
	getContextMenuItems,
} from "$lib/contextMenu";
import * as ops from "$lib/fileOps";
import { keybinds, matchesKeybind } from "$lib/keybinds";
import { createDialogManager } from "$lib/stores/dialogs.svelte";
import { createFileManager } from "$lib/stores/fileManager.svelte";

const fm = createFileManager();
const dlg = createDialogManager(fm);

let filterBarVisible = $state(false);
let filterBar = $state<ReturnType<typeof FilterBar> | null>(null);

async function handleWindowKeydown(e: KeyboardEvent) {
	const tag = (e.target as HTMLElement)?.tagName;
	if (tag === "INPUT" || tag === "TEXTAREA") return;

	if (matchesKeybind(e, keybinds.filter)) {
		e.preventDefault();
		filterBarVisible = true;
		await tick();
		filterBar?.focusInput();
		return;
	}

	if (matchesKeybind(e, keybinds.escape)) {
		e.preventDefault();
		if (filterBarVisible) {
			handleFilterClose();
		} else if (fm.selectedPaths.size > 0) {
			fm.clearMultiSelection();
		} else {
			fm.clearSelection();
		}
	} else if (matchesKeybind(e, keybinds.selectAll)) {
		e.preventDefault();
		fm.selectAll();
	} else if (matchesKeybind(e, keybinds.toggleSelect)) {
		e.preventDefault();
		if (fm.cursorEntry) fm.toggleSelect(fm.cursorEntry);
	} else if (matchesKeybind(e, keybinds.moveDown)) {
		e.preventDefault();
		fm.selectRelative(1);
	} else if (matchesKeybind(e, keybinds.moveUp)) {
		e.preventDefault();
		fm.selectRelative(-1);
	} else if (matchesKeybind(e, keybinds.open)) {
		e.preventDefault();
		if (fm.cursorEntry) ops.handleOpen(fm, fm.cursorEntry);
	} else if (matchesKeybind(e, keybinds.goParent)) {
		e.preventDefault();
		fm.goUp();
	} else if (matchesKeybind(e, keybinds.goTop)) {
		e.preventDefault();
		fm.selectByIndex(0);
	} else if (matchesKeybind(e, keybinds.goBottom)) {
		e.preventDefault();
		fm.selectByIndex(fm.filteredEntries.length - 1);
	} else if (matchesKeybind(e, keybinds.toggleHidden)) {
		fm.toggleHidden();
	}
}

function handleFilterClose() {
	fm.clearFilter();
	filterBarVisible = false;
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
		ops.handleMoveTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entries)),
	copyTo: () =>
		ops.handleCopyTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entries)),
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
	return getContextMenuItems(
		ctx,
		{
			isTrash: fm.isTrash,
			showHidden: fm.showHidden,
			clipboard: fm.clipboard,
			openWithApps: fm.openWithApps,
			multiSelectCount:
				fm.selectedPaths.size > 0
					? fm.selectedPaths.size
					: fm.cursorEntry
						? 1
						: 0,
		},
		menuActions,
	);
}

function clipboardText(): string {
	if (!fm.clipboard) return "";
	const mode = fm.clipboard.mode === "cut" ? "Moving" : "Copied";
	const what =
		fm.clipboard.entries.length === 1
			? fm.clipboard.entries[0].name
			: `${fm.clipboard.entries.length} items`;
	return `${mode}: ${what}`;
}

onMount(async () => {
	await fm.init();
	await dlg.subscribeProgress();
});

onDestroy(() => {
	dlg.unsubscribeProgress();
});
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="app">
	<Toolbar
		canGoBack={fm.historyIndex > 0}
		canGoForward={fm.historyIndex < fm.history.length - 1}
		ongoback={fm.goBack}
		ongoforward={fm.goForward}
		ongoup={fm.goUp}
		currentPath={fm.currentPath}
		onnavigate={fm.navigate}
		showHidden={fm.showHidden}
		ontogglehidden={fm.toggleHidden}
	/>

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
				{#if filterBarVisible}
					<FilterBar
						bind:this={filterBar}
						query={fm.filterQuery}
						matchCount={fm.filteredEntries.length}
						totalCount={fm.sortedEntries.length}
						onchange={(q) => fm.setFilterQuery(q)}
						onclose={handleFilterClose}
					/>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="content" oncontextmenu={(e) => ops.handleBgContextMenu(fm, e, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}>
				<FileList
						entries={fm.filteredEntries}
						cursorPath={fm.cursorPath}
						selectedPaths={fm.selectedPaths}
						renamingPath={fm.renamingPath}
						creatingEntry={fm.creatingEntry}
						clipboardPaths={fm.clipboard ? new Set(fm.clipboard.entries.map(e => e.path)) : null}
						clipboardMode={fm.clipboard?.mode ?? null}
						sortBy={fm.sortBy}
						sortAsc={fm.sortAsc}
						onopen={(entry) => ops.handleOpen(fm, entry)}
						onselect={fm.select}
						ontoggleselect={fm.toggleSelect}
						onselectrange={(entry) => { if (fm.cursorEntry) fm.selectRange(fm.cursorEntry, entry); else fm.select(entry); }}
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}
						onsort={fm.handleSort}
						onrename={(entry, name) => ops.commitRename(fm, entry, name)}
						oncreate={(name) => ops.commitCreate(fm, name)}
					/>
				</div>
			</div>
		{/if}
	</div>

	{#if fm.selectedPaths.size > 0}
		<StatusBar text="{fm.selectedPaths.size} {fm.selectedPaths.size === 1 ? 'item' : 'items'} selected" onclear={() => fm.clearMultiSelection()} />
	{:else if fm.clipboard}
		<StatusBar text={clipboardText()} onclear={() => fm.clipboard = null} />
	{/if}

</div>

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

{#if dlg.compressEntries.length > 0}
	<CompressDialog
		defaultName={dlg.compressEntries.length === 1 ? dlg.compressEntries[0].name : "archive"}
		onconfirm={dlg.handleCompressConfirm}
		onclose={dlg.closeCompress}
	/>
{/if}

{#if dlg.busyMessage}
	<BusyOverlay
		message={dlg.busyMessage}
		progress={dlg.busyProgress}
		oncancel={dlg.handleCancelOperation}
	/>
{/if}

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
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

	.main {
		flex: 1 1 0;
		display: flex;
		overflow: hidden;
		background: var(--bg-secondary);
		min-height: 0;
	}

	.loading {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		background: var(--bg-primary);
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-primary);
	}

	.content {
		flex: 1;
		display: flex;
		overflow: hidden;
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
</style>
