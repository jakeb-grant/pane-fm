<script lang="ts">
import type { FileEntry } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { formatSize } from "$lib/utils";
import { createEditLogic } from "./fileEditLogic.svelte";

let {
	entries,
	cursorPath,
	selectedPaths = new Set(),
	renamingPath = null,
	creatingEntry = null,
	clipboardPaths = null,
	clipboardMode = null,
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
}: {
	entries: FileEntry[];
	cursorPath: string | null;
	selectedPaths?: Set<string>;
	renamingPath?: string | null;
	creatingEntry?: "file" | "directory" | null;
	clipboardPaths?: Set<string> | null;
	clipboardMode?: "copy" | "cut" | null;
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
} = $props();

const edit = createEditLogic({
	entries: () => entries,
	renamingPath: () => renamingPath,
	creatingEntry: () => creatingEntry,
	onrename: (entry, newName) => onrename(entry, newName),
	oncreate: (name) => oncreate(name),
});

let listEl = $state<HTMLDivElement | null>(null);

$effect(() => {
	if (!cursorPath || !listEl) return;
	const row = listEl.querySelector("tr.cursor");
	row?.scrollIntoView({ block: "nearest" });
});

function sortIndicator(column: string): string {
	if (sortBy !== column) return "";
	return sortAsc ? "▲" : "▼";
}
</script>

<div class="file-list" bind:this={listEl}>
	<table>
		<thead>
			<tr>
				<th class="th-icon"></th>
				<th class="th-name" onclick={() => onsort("name")}>
					<span class="th-content">Name {#if sortBy === "name"}<span class="sort-icon">{sortIndicator("name")}</span>{/if}</span>
				</th>
				<th class="th-size" onclick={() => onsort("size")}>
					<span class="th-content">Size {#if sortBy === "size"}<span class="sort-icon">{sortIndicator("size")}</span>{/if}</span>
				</th>
				<th class="th-modified" onclick={() => onsort("modified")}>
					<span class="th-content">Modified {#if sortBy === "modified"}<span class="sort-icon">{sortIndicator("modified")}</span>{/if}</span>
				</th>
			</tr>
		</thead>
		<tbody>
			{#if creatingEntry}
				<tr class="creating" class:directory={creatingEntry === "directory"}>
					<td class="td-icon">{creatingEntry === "directory" ? "\uF07B" : "\uF15B"}</td>
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
					<td class="td-size"></td>
					<td class="td-modified"></td>
				</tr>
			{/if}

			{#each entries as entry (entry.path)}
				<tr
					class:cursor={cursorPath === entry.path}
					class:selected={selectedPaths.has(entry.path)}
					class:directory={entry.is_dir}
					class:cut={clipboardPaths?.has(entry.path) && clipboardMode === "cut"}
					ondblclick={() => { if (renamingPath !== entry.path) onopen(entry); }}
					onclick={(e) => {
						if (renamingPath === entry.path) return;
						if (e.ctrlKey || e.metaKey) { ontoggleselect(entry); }
						else if (e.shiftKey) { onselectrange(entry); }
						else { onselect(entry); }
					}}
					oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e, entry); }}
				>
					<td class="td-icon">{getIconForEntry(entry)}</td>
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
					<td class="td-size">{entry.is_dir ? `${entry.children_count ?? 0} items` : formatSize(entry.size)}</td>
					<td class="td-modified">{entry.modified}</td>
				</tr>
			{/each}
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
		/* takes remaining space */
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

	tr.cursor {
		background: var(--bg-hover);
	}

	tr.selected {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
	}

	tr.cursor.selected {
		background: color-mix(in srgb, var(--accent) 20%, var(--bg-hover));
	}

	tr.cut {
		opacity: 0.45;
	}

	tr.creating {
		cursor: default;
	}

	td {
		padding: 6px 12px;
		font-size: 13px;
		font-family: var(--font-sans);
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.td-icon {
		text-align: center;
		font-family: var(--font-icon);
		font-size: 16px;
	}

	.td-name {
		/* takes remaining space */
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

	.directory .td-icon {
		color: var(--accent);
	}

	.symlink-badge {
		font-size: 10px;
		color: var(--text-muted);
		margin-left: 6px;
		padding: 1px 4px;
		border: 1px solid var(--border);
		border-radius: 3px;
	}

	.empty {
		padding: 40px;
		text-align: center;
		color: var(--text-muted);
	}

	.rename-input {
		width: 100%;
		background: var(--bg-primary);
		border: 1px solid var(--accent);
		border-radius: 3px;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 1px 4px;
		outline: none;
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
	}
</style>
