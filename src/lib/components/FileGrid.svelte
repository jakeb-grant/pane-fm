<script lang="ts">
import type { FileEntry } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { tick } from "svelte";

let {
	entries,
	selectedPath,
	renamingPath = null,
	creatingEntry = null,
	clipboardPaths = null,
	clipboardMode = null,
	onopen,
	onselect,
	oncontextmenu,
	onrename,
	oncreate,
}: {
	entries: FileEntry[];
	selectedPath: string | null;
	renamingPath?: string | null;
	creatingEntry?: "file" | "directory" | null;
	clipboardPaths?: Set<string> | null;
	clipboardMode?: "copy" | "cut" | null;
	onopen: (entry: FileEntry) => void;
	onselect: (entry: FileEntry) => void;
	oncontextmenu: (e: MouseEvent, entry: FileEntry) => void;
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
</script>

<div class="file-grid">
	{#if creatingEntry}
		<div class="grid-item creating" class:directory={creatingEntry === "directory"}>
			<span class="icon">{creatingEntry === "directory" ? "\uF07B" : "\uF15B"}</span>
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
		</div>
	{/if}

	{#each entries as entry (entry.path)}
		<button
			class="grid-item"
			class:selected={selectedPath === entry.path}
			class:directory={entry.is_dir}
			class:cut={clipboardPaths?.has(entry.path) && clipboardMode === "cut"}
			ondblclick={() => { if (renamingPath !== entry.path) onopen(entry); }}
			onclick={() => { if (renamingPath !== entry.path) onselect(entry); }}
			oncontextmenu={(e) => { e.preventDefault(); oncontextmenu(e, entry); }}
		>
			<span class="icon">{getIconForEntry(entry)}</span>
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
				<span class="name">
					{entry.name}
					{#if entry.is_symlink}
						<span class="symlink-badge">link</span>
					{/if}
				</span>
			{/if}
		</button>
	{/each}

	{#if entries.length === 0 && !creatingEntry}
		<div class="empty">Empty directory</div>
	{/if}
</div>

<style>
	.file-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
		gap: 4px;
		padding: 12px;
		overflow-y: auto;
		flex: 1;
		align-content: start;
	}

	.grid-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 12px 8px;
		background: none;
		border: 1px solid transparent;
		border-radius: var(--radius);
		color: var(--text-primary);
		cursor: pointer;
		font-family: var(--font-sans);
		text-align: center;
	}

	.grid-item:hover {
		background: var(--bg-surface);
	}

	.grid-item.selected {
		background: var(--bg-hover);
		border-color: var(--border);
	}

	.grid-item.cut {
		opacity: 0.45;
	}

	.grid-item.creating {
		cursor: default;
		background: var(--bg-surface);
		border-color: var(--border);
	}

	.icon {
		font-family: var(--font-icon);
		font-size: 32px;
		line-height: 1;
	}

	.directory .icon {
		color: var(--accent);
	}

	.name {
		font-size: 12px;
		word-break: break-all;
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
	}

	.directory .name {
		color: var(--accent);
	}

	.symlink-badge {
		font-size: 9px;
		color: var(--text-muted);
		margin-left: 4px;
		padding: 1px 3px;
		border: 1px solid var(--border);
		border-radius: 3px;
	}

	.empty {
		grid-column: 1 / -1;
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
		font-size: 12px;
		font-family: var(--font-sans);
		padding: 2px 4px;
		outline: none;
		text-align: center;
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
	}
</style>
