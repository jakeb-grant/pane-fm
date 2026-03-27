<script lang="ts">
import type { FileEntry } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { formatSize } from "$lib/utils";
import FileIcon from "./FileIcon.svelte";
import { createEditLogic } from "./fileEditLogic.svelte";

let {
	entries,
	cursorPath,
	selectedPaths = new Set(),
	renamingPath = null,
	creatingEntry = null,
	clipboardPaths = null,
	clipboardMode = null,
	isTrash = false,
	dropTarget = null,
	sortBy,
	sortAsc,
	onopen,
	onselect,
	ontoggleselect,
	onselectrange,
	oncontextmenu,
	onsort,
	onrename,
	oncreate,
	ondragstartentries,
	ondropentry,
	ondragoverentry,
	ondragleaveentry,
	ondragleavewindow,
	hideModified = false,
	hideSize = false,
}: {
	entries: FileEntry[];
	cursorPath: string | null;
	selectedPaths?: Set<string>;
	renamingPath?: string | null;
	creatingEntry?: "file" | "directory" | null;
	clipboardPaths?: Set<string> | null;
	clipboardMode?: "copy" | "cut" | null;
	isTrash?: boolean;
	dropTarget?: string | null;
	sortBy: string;
	sortAsc: boolean;
	onopen: (entry: FileEntry) => void;
	onselect: (entry: FileEntry) => void;
	ontoggleselect: (entry: FileEntry) => void;
	onselectrange: (entry: FileEntry) => void;
	oncontextmenu: (e: MouseEvent, entry: FileEntry) => void;
	onsort: (column: string) => void;
	onrename: (entry: FileEntry, newName: string) => void;
	oncreate: (name: string) => void;
	ondragstartentries?: (entries: FileEntry[]) => void;
	ondropentry?: (targetDir: FileEntry, ctrlKey: boolean) => void;
	ondragoverentry?: (entry: FileEntry) => void;
	ondragleaveentry?: () => void;
	ondragleavewindow?: () => void;
	hideModified?: boolean;
	hideSize?: boolean;
} = $props();

const edit = createEditLogic({
	entries: () => entries,
	renamingPath: () => renamingPath,
	creatingEntry: () => creatingEntry,
	onrename: (entry, newName) => onrename(entry, newName),
	oncreate: (name) => oncreate(name),
});

const ROW_HEIGHT = 29;
const OVERSCAN = 10;

let listEl = $state<HTMLDivElement | null>(null);
let scrollTop = $state(0);
let viewHeight = $state(600);

function handleScroll() {
	if (listEl) scrollTop = listEl.scrollTop;
}

let startIdx = $derived(
	Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN),
);
let endIdx = $derived(
	Math.min(
		entries.length,
		Math.ceil((scrollTop + viewHeight) / ROW_HEIGHT) + OVERSCAN,
	),
);
let visibleEntries = $derived(entries.slice(startIdx, endIdx));
let topPad = $derived(startIdx * ROW_HEIGHT);
let bottomPad = $derived((entries.length - endIdx) * ROW_HEIGHT);

// Derived index ensures the effect re-runs when entries reorder (e.g. re-sort)
let cursorIdx = $derived(
	cursorPath ? entries.findIndex((e) => e.path === cursorPath) : -1,
);

// Keep cursor in view — only scroll when it would go off-screen.
// Row positions use tbody coordinates (idx * ROW_HEIGHT).
// The sticky header covers the top 27px of the viewport, so visible
// data runs from scrollTop to scrollTop + clientHeight - headerHeight.
$effect(() => {
	if (cursorIdx === -1 || !listEl) return;
	const thead = listEl.querySelector("thead");
	const headerHeight = thead ? thead.offsetHeight : 0;
	const rowTop = cursorIdx * ROW_HEIGHT;
	const rowBottom = rowTop + ROW_HEIGHT;
	const dataTop = listEl.scrollTop;
	const dataBottom = dataTop + listEl.clientHeight - headerHeight;
	if (rowTop < dataTop) {
		listEl.scrollTop = rowTop;
	} else if (rowBottom > dataBottom) {
		listEl.scrollTop = rowBottom - listEl.clientHeight + headerHeight;
	}
});

// Track container height via ResizeObserver
$effect(() => {
	if (!listEl) return;
	viewHeight = listEl.clientHeight;
	const ro = new ResizeObserver((es) => {
		viewHeight = es[0].contentRect.height;
	});
	ro.observe(listEl);
	return () => ro.disconnect();
});

function sortIndicator(column: string): string {
	if (sortBy !== column) return "";
	return sortAsc ? "▲" : "▼";
}

// Mouse-based drag (HTML5 DnD doesn't work in WebKitGTK/Tauri)
const DRAG_THRESHOLD = 5;
const SCROLL_EDGE = 40;
const SCROLL_SPEED = 8;

let dragOrigin = $state<{ x: number; y: number; entry: FileEntry } | null>(
	null,
);
let dragging = $state(false);
let ghost = $state<HTMLElement | null>(null);

export function cancelDrag() {
	if (ghost) {
		ghost.remove();
		ghost = null;
	}
	dragging = false;
	dragOrigin = null;
}

function handleMouseDown(entry: FileEntry, e: MouseEvent) {
	if (e.button !== 0 || renamingPath === entry.path || isTrash) return;
	e.preventDefault();
	dragOrigin = { x: e.clientX, y: e.clientY, entry };
}

function handleMouseMove(e: MouseEvent) {
	if (!dragOrigin) return;

	if (!dragging) {
		const dx = e.clientX - dragOrigin.x;
		const dy = e.clientY - dragOrigin.y;
		if (Math.abs(dx) < DRAG_THRESHOLD && Math.abs(dy) < DRAG_THRESHOLD) return;

		// Start drag — clear any accidental text selection
		dragging = true;
		window.getSelection()?.removeAllRanges();
		const entry = dragOrigin.entry;
		const dragItems = selectedPaths.has(entry.path)
			? entries.filter((en) => selectedPaths.has(en.path))
			: [entry];
		ondragstartentries?.(dragItems);

		// Clean up any stale ghost before creating a new one
		if (ghost) {
			ghost.remove();
			ghost = null;
		}
		const el = document.createElement("div");
		el.className = "drag-ghost";
		el.textContent =
			dragItems.length === 1 ? "1 item" : `${dragItems.length} items`;
		document.body.appendChild(el);
		ghost = el;
	}

	// Move ghost
	if (ghost) {
		ghost.style.top = `${e.clientY + 12}px`;
		ghost.style.left = `${e.clientX + 12}px`;
	}

	// Detect mouse at window edge → trigger native drag-out
	const EDGE_MARGIN = 3;
	if (
		e.clientX <= EDGE_MARGIN ||
		e.clientY <= EDGE_MARGIN ||
		e.clientX >= window.innerWidth - EDGE_MARGIN ||
		e.clientY >= window.innerHeight - EDGE_MARGIN
	) {
		ondragleavewindow?.();
		return;
	}

	// Auto-scroll at edges
	if (listEl) {
		const rect = listEl.getBoundingClientRect();
		if (e.clientY - rect.top < SCROLL_EDGE) listEl.scrollTop -= SCROLL_SPEED;
		else if (rect.bottom - e.clientY < SCROLL_EDGE)
			listEl.scrollTop += SCROLL_SPEED;
	}

	// Hit-test: only manage drop targets when mouse is over the file list
	const listRect = listEl?.getBoundingClientRect();
	const overList =
		listRect &&
		e.clientX >= listRect.left &&
		e.clientX <= listRect.right &&
		e.clientY >= listRect.top &&
		e.clientY <= listRect.bottom;

	if (overList) {
		const target = document.elementFromPoint(e.clientX, e.clientY);
		const row = target?.closest("tr");
		if (row) {
			const idx = row.dataset.entryIdx;
			if (idx != null) {
				const hovered = entries[Number(idx)];
				if (hovered?.is_dir) {
					ondragoverentry?.(hovered);
					return;
				}
			}
		}
		ondragleaveentry?.();
	}
}

function handleMouseUp(e: MouseEvent) {
	if (!dragging) {
		dragOrigin = null;
		return;
	}

	// Find drop target
	if (ghost) {
		ghost.remove();
		ghost = null;
	}
	const target = document.elementFromPoint(e.clientX, e.clientY);
	const row = target?.closest("tr");
	if (row) {
		const idx = row.dataset.entryIdx;
		if (idx != null) {
			const hovered = entries[Number(idx)];
			if (hovered?.is_dir) {
				ondropentry?.(hovered, e.ctrlKey);
			}
		}
	}

	dragging = false;
	dragOrigin = null;
	ondragleaveentry?.();
}

$effect(() => {
	if (!dragOrigin) return;
	const onMove = (e: MouseEvent) => handleMouseMove(e);
	const onUp = (e: MouseEvent) => handleMouseUp(e);
	const onSelectStart = (e: Event) => e.preventDefault();
	window.addEventListener("mousemove", onMove);
	window.addEventListener("mouseup", onUp);
	document.addEventListener("selectstart", onSelectStart);
	return () => {
		window.removeEventListener("mousemove", onMove);
		window.removeEventListener("mouseup", onUp);
		document.removeEventListener("selectstart", onSelectStart);
		if (ghost) {
			ghost.remove();
			ghost = null;
		}
		dragging = false;
	};
});
</script>

<div class="file-list" bind:this={listEl} onscroll={handleScroll}>
	<table>
		<thead>
			<tr>
				<th class="th-icon"></th>
				<th class="th-name" onclick={() => onsort("name")}>
					<span class="th-content">Name {#if sortBy === "name"}<span class="sort-icon">{sortIndicator("name")}</span>{/if}</span>
				</th>
				{#if !hideSize}
				<th class="th-size" onclick={() => onsort("size")}>
					<span class="th-content">Size {#if sortBy === "size"}<span class="sort-icon">{sortIndicator("size")}</span>{/if}</span>
				</th>
				{/if}
				{#if !hideModified}
				<th class="th-modified" onclick={() => onsort("modified")}>
					<span class="th-content">Modified {#if sortBy === "modified"}<span class="sort-icon">{sortIndicator("modified")}</span>{/if}</span>
				</th>
				{/if}
			</tr>
		</thead>
		<tbody>
			{#if creatingEntry}
				<tr class="creating" class:directory={creatingEntry === "directory"}>
					<td class="td-icon"><FileIcon src={getIconForEntry({ is_dir: creatingEntry === "directory", name: "" })} /></td>
					<td class="td-name">
						<!-- svelte-ignore a11y_autofocus -->
						<input
							class="rename-input"
							type="text"
							bind:value={edit.editValue}
							bind:this={edit.editInput}
							onkeydown={(e) => {
								if (e.key === "Enter") { e.preventDefault(); edit.commitCreateEntry(); }
								if (e.key === "Escape") { e.preventDefault(); oncreate(""); }
							}}
							onblur={edit.commitCreateEntry}
						/>
					</td>
					{#if !hideSize}<td class="td-size"></td>{/if}
					{#if !hideModified}<td class="td-modified"></td>{/if}
				</tr>
			{/if}

			{#if topPad > 0}
				<tr style="height:{topPad}px" aria-hidden="true"><td colspan={2 + (hideSize ? 0 : 1) + (hideModified ? 0 : 1)}></td></tr>
			{/if}

			{#each visibleEntries as entry, vi (entry.path)}
				<tr
					style="height:{ROW_HEIGHT}px"
					data-entry-idx={startIdx + vi}
					class:cursor={cursorPath === entry.path}
					class:selected={selectedPaths.has(entry.path)}
					class:directory={entry.is_dir}
					class:cut={clipboardPaths?.has(entry.path) && clipboardMode === "cut"}
					class:copied={clipboardPaths?.has(entry.path) && clipboardMode === "copy"}
					class:drop-target={dropTarget === entry.path && entry.is_dir}
					ondblclick={() => { if (renamingPath !== entry.path) onopen(entry); }}
					onclick={(e) => {
						if (dragging) return;
						if (renamingPath === entry.path) return;
						if (e.ctrlKey || e.metaKey) { ontoggleselect(entry); }
						else if (e.shiftKey) { onselectrange(entry); }
						else { onselect(entry); }
					}}
					oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e, entry); }}
					onmousedown={(e) => handleMouseDown(entry, e)}
				>
					<td class="td-icon"><FileIcon src={getIconForEntry(entry)} /></td>
					<td class="td-name">
						{#if renamingPath === entry.path}
							<!-- svelte-ignore a11y_autofocus -->
							<input
								class="rename-input"
								type="text"
								bind:value={edit.editValue}
								bind:this={edit.editInput}
								onkeydown={(e) => {
									if (e.key === "Enter") { e.preventDefault(); edit.commitRenameForEntry(entry); }
									if (e.key === "Escape") { e.preventDefault(); onrename(entry, entry.name); }
								}}
								onblur={() => edit.commitRenameForEntry(entry)}
								onclick={(e) => e.stopPropagation()}
								ondblclick={(e) => e.stopPropagation()}
							/>
						{:else}
							{entry.name}
							{#if entry.is_symlink}
								<span class="symlink-badge">link</span>
							{/if}
						{/if}
					</td>
					{#if !hideSize}<td class="td-size">{entry.is_dir ? (entry.children_count != null ? `${entry.children_count} items` : "\u2014") : formatSize(entry.size)}</td>{/if}
					{#if !hideModified}<td class="td-modified">{entry.modified}</td>{/if}
				</tr>
			{/each}

			{#if bottomPad > 0}
				<tr style="height:{bottomPad}px" aria-hidden="true"><td colspan={2 + (hideSize ? 0 : 1) + (hideModified ? 0 : 1)}></td></tr>
			{/if}
		</tbody>
	</table>

	{#if entries.length === 0 && !creatingEntry}
		<div class="empty">Empty directory</div>
	{/if}
</div>

<style>
	.file-list {
		display: flex;
		flex-direction: column;
		flex: 1;
		overflow-y: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		table-layout: fixed;
	}

	/* Header */
	thead {
		position: sticky;
		top: 0;
		z-index: 1;
	}

	th {
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		color: var(--text-muted);
		font-size: 11px;
		font-weight: 500;
		font-family: var(--font-sans);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 4px 12px;
		text-align: left;
		cursor: pointer;
		white-space: nowrap;
		line-height: 1;
	}

	th:hover {
		color: var(--text-primary);
	}

	.th-icon {
		width: 40px;
		cursor: default;
	}

	.th-name {
		width: auto;
	}

	.th-size {
		width: 100px;
	}

	.th-modified {
		width: 150px;
	}

	.th-content {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}

	.sort-icon {
		font-size: 8px;
	}

	/* Rows */
	tbody tr {
		cursor: pointer;
	}

	tbody tr:hover {
		background: var(--bg-surface);
	}

	:global(.hide-cursor) tbody tr:hover:not(.cursor):not(.selected):not(.cut):not(.copied) {
		background: none;
	}

	tr.cursor {
		background: var(--bg-hover);
	}

	tr.selected {
		background: color-mix(in srgb, var(--accent) 8%, transparent);
	}

	tr.selected > td:first-child {
		box-shadow: inset 3px 0 0 var(--accent);
	}

	tr.cursor.selected {
		background: color-mix(in srgb, var(--accent) 14%, var(--bg-hover));
	}

	tr.cursor.selected > td:first-child {
		box-shadow: inset 4px 0 0 var(--accent), 0 0 8px -2px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	tr.copied > td:first-child {
		box-shadow: inset 3px 0 0 var(--success);
	}

	tr.copied {
		background: color-mix(in srgb, var(--success) 6%, transparent);
	}

	tr.cursor.copied {
		background: var(--bg-hover);
	}

	tr.cursor.copied > td:first-child {
		box-shadow: inset 4px 0 0 var(--success), 0 0 8px -2px color-mix(in srgb, var(--success) 30%, transparent);
	}

	tr.cut > td:first-child {
		box-shadow: inset 3px 0 0 var(--warning);
	}

	tr.cut {
		opacity: 0.5;
		background: color-mix(in srgb, var(--warning) 6%, transparent);
	}

	tr.cursor.cut {
		opacity: 1;
		background: var(--bg-hover);
	}

	tr.cursor.cut > td:first-child {
		box-shadow: inset 4px 0 0 var(--warning), 0 0 8px -2px color-mix(in srgb, var(--warning) 30%, transparent);
	}

	tr.creating {
		cursor: default;
	}

	td {
		padding: 6px 12px;
		font-size: 13px;
		font-family: var(--font-sans);
		line-height: 1;
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		user-select: none;
	}

	.td-icon {
		text-align: center;
		line-height: 1;
	}

	.td-name {
		width: auto;
	}

	.td-size {
		color: var(--text-secondary);
	}

	.td-modified {
		color: var(--text-secondary);
	}

	.directory .td-name {
		color: var(--accent);
	}


	.symlink-badge {
		font-size: 10px;
		color: var(--text-muted);
		margin-left: 6px;
		padding: 1px 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	.empty {
		padding: 40px;
		text-align: center;
		color: var(--text-muted);
	}

	tr.drop-target {
		background: color-mix(in srgb, var(--accent) 20%, transparent);
		outline: 1px dashed var(--accent);
		outline-offset: -1px;
	}

	:global(.drag-ghost) {
		position: fixed;
		background: var(--bg-secondary);
		border: 1px solid var(--accent);
		border-radius: var(--radius);
		padding: 4px 10px;
		font-size: 12px;
		color: var(--text-primary);
		font-family: var(--font-sans);
		pointer-events: none;
		white-space: nowrap;
		z-index: 9999;
	}

	.rename-input {
		width: 100%;
		background: var(--bg-primary);
		border: 1px solid var(--accent);
		border-radius: var(--radius);
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 1px 4px;
		outline: none;
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
	}
</style>
