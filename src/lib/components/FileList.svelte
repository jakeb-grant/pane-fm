<script lang="ts">
import { tick } from "svelte";
import type { FileEntry } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { formatSize } from "$lib/utils";

let {
	entries,
	selectedPath,
	renamingPath = null,
	creatingEntry = null,
	clipboardPaths = null,
	clipboardMode = null,
	sortBy,
	sortAsc,
	onopen,
	onselect,
	oncontextmenu,
	onsort,
	onrename,
	oncreate,
}: {
	entries: FileEntry[];
	selectedPath: string | null;
	renamingPath?: string | null;
	creatingEntry?: "file" | "directory" | null;
	clipboardPaths?: Set<string> | null;
	clipboardMode?: "copy" | "cut" | null;
	sortBy: string;
	sortAsc: boolean;
	onopen: (entry: FileEntry) => void;
	onselect: (entry: FileEntry) => void;
	oncontextmenu: (e: MouseEvent, entry: FileEntry) => void;
	onsort: (column: string) => void;
	onrename: (entry: FileEntry, newName: string) => void;
	oncreate: (name: string) => void;
} = $props();

let editValue = $state("");
let editInput: HTMLInputElement | undefined = $state();

function focusAndSelect(entry?: { is_dir: boolean; name: string }) {
	tick().then(() => {
		if (!editInput) return;
		editInput.focus();
		if (entry && !entry.is_dir) {
			const dotIndex = entry.name.lastIndexOf(".");
			if (dotIndex > 0) {
				editInput.setSelectionRange(0, dotIndex);
				return;
			}
		}
		editInput.select();
	});
}

$effect(() => {
	if (renamingPath) {
		const entry = entries.find((e) => e.path === renamingPath);
		if (entry) {
			editValue = entry.name;
			focusAndSelect(entry);
		}
	}
});

$effect(() => {
	if (creatingEntry) {
		editValue = creatingEntry === "directory" ? "New Folder" : "New File";
		focusAndSelect();
	}
});

function commitRenameForEntry(entry: FileEntry) {
	onrename(entry, editValue.trim());
}

function commitCreateEntry() {
	oncreate(editValue.trim());
}

function sortIndicator(column: string): string {
	if (sortBy !== column) return "";
	return sortAsc ? " \u25B2" : " \u25BC";
}
</script>

<div class="file-list">
	<div class="header">
		<button class="col col-icon" disabled aria-label="Icon column">&nbsp;</button>
		<button class="col col-name" onclick={() => onsort("name")}>
			Name{sortIndicator("name")}
		</button>
		<button class="col col-size" onclick={() => onsort("size")}>
			Size{sortIndicator("size")}
		</button>
		<button class="col col-modified" onclick={() => onsort("modified")}>
			Modified{sortIndicator("modified")}
		</button>
	</div>

	<div class="entries">
		{#if creatingEntry}
			<div class="row creating" class:directory={creatingEntry === "directory"}>
				<span class="col col-icon">{creatingEntry === "directory" ? "\uF07B" : "\uF15B"}</span>
				<span class="col col-name">
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="rename-input"
						type="text"
						bind:value={editValue}
						bind:this={editInput}
						onkeydown={(e) => {
							if (e.key === "Enter") { e.preventDefault(); commitCreateEntry(); }
							if (e.key === "Escape") { e.preventDefault(); oncreate(""); }
						}}
						onblur={commitCreateEntry}
					/>
				</span>
				<span class="col col-size"></span>
				<span class="col col-modified"></span>
			</div>
		{/if}

		{#each entries as entry (entry.path)}
			<button
				class="row"
				class:selected={selectedPath === entry.path}
				class:directory={entry.is_dir}
				class:cut={clipboardPaths?.has(entry.path) && clipboardMode === "cut"}
				ondblclick={() => { if (renamingPath !== entry.path) onopen(entry); }}
				onclick={() => { if (renamingPath !== entry.path) onselect(entry); }}
				oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e, entry); }}
			>
				<span class="col col-icon">{getIconForEntry(entry)}</span>
				<span class="col col-name">
					{#if renamingPath === entry.path}
						<!-- svelte-ignore a11y_autofocus -->
						<input
							class="rename-input"
							type="text"
							bind:value={editValue}
							bind:this={editInput}
							onkeydown={(e) => {
								if (e.key === "Enter") { e.preventDefault(); commitRenameForEntry(entry); }
								if (e.key === "Escape") { e.preventDefault(); onrename(entry, entry.name); }
							}}
							onblur={() => commitRenameForEntry(entry)}
							onclick={(e) => e.stopPropagation()}
							ondblclick={(e) => e.stopPropagation()}
						/>
					{:else}
						{entry.name}
						{#if entry.is_symlink}
							<span class="symlink-badge">link</span>
						{/if}
					{/if}
				</span>
				<span class="col col-size">{entry.is_dir ? `${entry.children_count ?? 0} items` : formatSize(entry.size)}</span>
				<span class="col col-modified">{entry.modified}</span>
			</button>
		{/each}

		{#if entries.length === 0 && !creatingEntry}
			<div class="empty">Empty directory</div>
		{/if}
	</div>
</div>

<style>
	.file-list {
		display: flex;
		flex-direction: column;
		flex: 1;
		overflow: hidden;
	}

	.header {
		display: flex;
		border-bottom: 1px solid var(--border);
		background: var(--bg-secondary);
		position: sticky;
		top: 0;
	}

	.header button {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 6px 12px;
		cursor: pointer;
		text-align: left;
	}

	.header button:hover:not(:disabled) {
		color: var(--text-primary);
	}

	.entries {
		flex: 1;
		overflow-y: auto;
	}

	.row {
		display: flex;
		width: 100%;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 0;
		cursor: pointer;
		text-align: left;
	}

	.row:hover {
		background: var(--bg-surface);
	}

	.row.selected {
		background: var(--bg-hover);
	}

	.row.cut {
		opacity: 0.45;
	}

	.row.creating {
		cursor: default;
	}

	.col {
		padding: 6px 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.col-icon {
		width: 40px;
		flex-shrink: 0;
		text-align: center;
		font-family: var(--font-icon);
		font-size: 16px;
	}

	.col-name {
		flex: 1;
		min-width: 0;
	}

	.col-size {
		width: 100px;
		flex-shrink: 0;
		text-align: right;
		color: var(--text-secondary);
	}

	.col-modified {
		width: 150px;
		flex-shrink: 0;
		color: var(--text-secondary);
	}

	.directory .col-name {
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
