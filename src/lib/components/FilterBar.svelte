<script lang="ts">
import { tick } from "svelte";
import { keybinds, matchesKeybind } from "$lib/keybinds";

interface Props {
	query: string;
	matchCount: number;
	totalCount: number;
	onchange: (query: string) => void;
	onclose: () => void;
	onmovedown: () => void;
	onmoveup: () => void;
	onopen: () => void;
	onaccept: () => void;
}

let {
	query,
	matchCount,
	totalCount,
	onchange,
	onclose,
	onmovedown,
	onmoveup,
	onopen,
	onaccept,
}: Props = $props();

let inputEl = $state<HTMLInputElement | null>(null);

export async function focusInput() {
	await tick();
	inputEl?.focus();
}

function onkeydown(e: KeyboardEvent) {
	if (e.key === "Escape") {
		e.preventDefault();
		onclose();
	} else if (e.key === "ArrowDown" || (e.key === "j" && e.ctrlKey)) {
		e.preventDefault();
		onmovedown();
	} else if (e.key === "ArrowUp" || (e.key === "k" && e.ctrlKey)) {
		e.preventDefault();
		onmoveup();
	} else if (e.key === "Enter") {
		e.preventDefault();
		onopen();
	} else if (matchesKeybind(e, keybinds.filter)) {
		e.preventDefault();
		inputEl?.blur();
		if (query) {
			onaccept();
		} else {
			onclose();
		}
	}
}
</script>

<div class="filter-bar">
	<span class="filter-icon">/</span>
	<input
		bind:this={inputEl}
		type="text"
		class="filter-input"
		placeholder="Filter..."
		value={query}
		oninput={(e) => onchange(e.currentTarget.value)}
		{onkeydown}
	/>
	<span class="filter-count">{matchCount} of {totalCount}</span>
	<button class="filter-clear" onclick={onclose}>✕</button>
</div>

<style>
	.filter-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 12px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.filter-icon {
		color: var(--text-muted);
		font-size: 13px;
		font-family: var(--font-mono, monospace);
	}

	.filter-input {
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

	.filter-input:focus {
		border-color: var(--accent);
	}

	.filter-count {
		color: var(--text-muted);
		font-size: 12px;
		white-space: nowrap;
	}

	.filter-clear {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font-size: 14px;
		padding: 2px 4px;
		line-height: 1;
	}

	.filter-clear:hover {
		color: var(--text-primary);
	}
</style>
