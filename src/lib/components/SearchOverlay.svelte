<script lang="ts">
import { tick } from "svelte";
import type { SearchResult } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { isGlobPattern } from "$lib/utils";
import FileIcon from "./FileIcon.svelte";

interface Props {
	results: SearchResult[];
	cursor: number;
	query: string;
	searching: boolean;
	onchange: (query: string) => void;
	onclose: () => void;
	onselect: (result: SearchResult) => void;
	oncursorchange: (index: number) => void;
}

let {
	results,
	cursor,
	query,
	searching,
	onchange,
	onclose,
	onselect,
	oncursorchange,
}: Props = $props();

const ROW_HEIGHT = 29;
let inputEl = $state<HTMLInputElement | null>(null);
let listEl = $state<HTMLDivElement | null>(null);

export async function focusInput() {
	await tick();
	inputEl?.focus();
}

function scrollCursorIntoView(index: number) {
	if (!listEl) return;
	const top = index * ROW_HEIGHT;
	const bottom = top + ROW_HEIGHT;
	if (top < listEl.scrollTop) {
		listEl.scrollTop = top;
	} else if (bottom > listEl.scrollTop + listEl.clientHeight) {
		listEl.scrollTop = bottom - listEl.clientHeight;
	}
}

function moveCursor(delta: number) {
	const next = Math.max(0, Math.min(results.length - 1, cursor + delta));
	oncursorchange(next);
	scrollCursorIntoView(next);
}

function onkeydown(e: KeyboardEvent) {
	if (e.key === "Escape") {
		e.preventDefault();
		onclose();
	} else if (e.key === "ArrowDown" || (e.key === "j" && e.ctrlKey)) {
		e.preventDefault();
		moveCursor(1);
	} else if (e.key === "ArrowUp" || (e.key === "k" && e.ctrlKey)) {
		e.preventDefault();
		moveCursor(-1);
	} else if (e.key === "Enter") {
		e.preventDefault();
		if (results[cursor]) onselect(results[cursor]);
	}
}

function parentDir(relativePath: string): string {
	const idx = relativePath.lastIndexOf("/");
	return idx > 0 ? relativePath.substring(0, idx) : "";
}
</script>

<div class="search-overlay">
	<div class="search-header">
		<span class="search-icon">s</span>
		{#if isGlobPattern(query)}<span class="search-mode">glob</span>{/if}
		<input
			bind:this={inputEl}
			type="text"
			class="search-input"
			placeholder="Search files..."
			value={query}
			oninput={(e) => onchange(e.currentTarget.value)}
			{onkeydown}
		/>
		<span class="search-count">
			{results.length}{#if !searching && results.length >= 500}+{/if}
			{#if searching}&hellip;{/if}
		</span>
		<button class="search-clear" onclick={onclose}>✕</button>
	</div>

	<div class="search-results" bind:this={listEl}>
		{#if results.length === 0 && !searching && query}
			<div class="search-empty">No results found</div>
		{:else if results.length === 0 && !query}
			<div class="search-empty">Type to search files recursively</div>
		{:else}
			{#each results as result, i}
				<button
					class="search-row"
					class:cursor={i === cursor}
					onclick={() => onselect(result)}
					onmouseenter={() => oncursorchange(i)}
				>
					<FileIcon src={getIconForEntry(result)} size={16} />
					<span class="result-name" class:directory={result.is_dir}>{result.name}</span>
					<span class="result-path">{parentDir(result.relative_path)}</span>
				</button>
			{/each}
		{/if}
	</div>

	<div class="search-footer">
		{#if searching}
			<span class="search-status">Searching&hellip;</span>
		{:else if results.length >= 500}
			<span class="search-status">Results capped at 500</span>
		{:else if query}
			<span class="search-status">{results.length} result{results.length !== 1 ? "s" : ""}</span>
		{/if}
		<span class="search-hint"><kbd>Enter</kbd> open <kbd>Esc</kbd> close</span>
	</div>
</div>

<style>
	.search-overlay {
		display: flex;
		flex-direction: column;
		flex: 1;
		overflow: hidden;
	}

	.search-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 12px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.search-icon {
		color: var(--text-muted);
		font-size: 13px;
		font-family: var(--font-mono, monospace);
	}

	.search-mode {
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		color: var(--accent);
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0 4px;
		letter-spacing: 0.03em;
	}

	.search-input {
		flex: 1;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 3px 8px;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		outline: none;
	}

	.search-input:focus {
		border-color: var(--accent);
	}

	.search-count {
		color: var(--text-muted);
		font-size: 12px;
		white-space: nowrap;
	}

	.search-clear {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 14px;
		padding: 2px 4px;
		line-height: 1;
	}

	.search-clear:hover {
		color: var(--text-primary);
	}

	.search-results {
		flex: 1;
		overflow-y: auto;
	}

	.search-empty {
		padding: 24px;
		text-align: center;
		color: var(--text-muted);
		font-size: 13px;
	}

	.search-row {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		height: 29px;
		padding: 0 12px;
		border: none;
		background: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
	}

	.search-row:hover,
	.search-row.cursor {
		background: var(--bg-hover);
	}

	.result-name {
		flex-shrink: 0;
		white-space: nowrap;
	}

	.result-name.directory {
		color: var(--accent);
	}

	.result-path {
		color: var(--text-muted);
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}

	.search-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 12px;
		background: var(--bg-secondary);
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		font-size: 12px;
		color: var(--text-muted);
	}

	.search-status {
		white-space: nowrap;
	}

	.search-hint {
		margin-left: auto;
	}
</style>
