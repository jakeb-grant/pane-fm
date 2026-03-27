<script lang="ts">
import { onMount } from "svelte";
import type { Command } from "$lib/commandRegistry";
import { flyDown, overlayFade } from "$lib/transitions";
import { fuzzyMatch } from "$lib/utils";

interface Props {
	commands: Command[];
	onclose: () => void;
	onexecute: (command: Command) => void;
}

let { commands, onclose, onexecute }: Props = $props();

let query = $state("");
let cursor = $state(0);
let keyboardNav = $state(false);
let lastMousePos = { x: 0, y: 0 };
let inputEl = $state<HTMLInputElement | null>(null);
let listEl = $state<HTMLDivElement | null>(null);

function handleRowMouse(e: MouseEvent, idx: number) {
	if (e.clientX === lastMousePos.x && e.clientY === lastMousePos.y) return;
	lastMousePos = { x: e.clientX, y: e.clientY };
	keyboardNav = false;
	cursor = idx;
}

const filtered = $derived.by(() => {
	if (!query) return commands;
	return commands.filter((cmd) => fuzzyMatch(query, cmd.label));
});

// Reset cursor when query changes
$effect(() => {
	query;
	cursor = 0;
});

// Group filtered commands by category with flat indices (only when unfiltered)
const grouped = $derived.by(() => {
	if (query) return null;
	const groups: { category: string; items: { cmd: Command; idx: number }[] }[] =
		[];
	let lastCat = "";
	for (let i = 0; i < filtered.length; i++) {
		const cmd = filtered[i];
		if (cmd.category !== lastCat) {
			groups.push({ category: cmd.category, items: [] });
			lastCat = cmd.category;
		}
		groups[groups.length - 1].items.push({ cmd, idx: i });
	}
	return groups;
});

onMount(() => {
	inputEl?.focus();
});

function moveCursor(delta: number) {
	const next = Math.max(0, Math.min(filtered.length - 1, cursor + delta));
	cursor = next;
	keyboardNav = true;
}

// Keep cursor row in view — only scroll when navigating via keyboard
$effect(() => {
	if (!keyboardNav || cursor < 0 || !listEl) return;
	const row = listEl.querySelectorAll(".palette-row")[cursor] as
		| HTMLElement
		| undefined;
	if (!row) return;
	const listRect = listEl.getBoundingClientRect();
	const rowRect = row.getBoundingClientRect();
	if (rowRect.top < listRect.top) {
		listEl.scrollTop -= listRect.top - rowRect.top;
	} else if (rowRect.bottom > listRect.bottom) {
		listEl.scrollTop += rowRect.bottom - listRect.bottom;
	}
});

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
		if (filtered[cursor]) onexecute(filtered[cursor]);
	}
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose} transition:overlayFade>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="palette" onclick={(e) => e.stopPropagation()} transition:flyDown>
		<div class="palette-header">
			<span class="palette-icon">&gt;</span>
			<input
				bind:this={inputEl}
				type="text"
				class="palette-input"
				placeholder="Type a command..."
				value={query}
				oninput={(e) => { query = e.currentTarget.value; }}
				{onkeydown}
			/>
		</div>

		{#snippet cmdrow(cmd: Command, idx: number)}
			<button
				class="palette-row"
				class:cursor={idx === cursor}
				onclick={() => onexecute(cmd)}
				onmousemove={(e) => handleRowMouse(e, idx)}
			>
				<span class="cmd-label">{cmd.label}</span>
				{#if cmd.keybind}
					<kbd>{cmd.keybind}</kbd>
				{/if}
			</button>
		{/snippet}

		<div class="palette-list" bind:this={listEl}>
			{#if grouped && !query}
				{#each grouped as group}
					<div class="category-header">{group.category}</div>
					{#each group.items as item}
						{@render cmdrow(item.cmd, item.idx)}
					{/each}
				{/each}
			{:else}
				{#each filtered as cmd, i}
					{@render cmdrow(cmd, i)}
				{/each}
				{#if filtered.length === 0}
					<div class="palette-empty">No matching commands</div>
				{/if}
			{/if}
		</div>

		<div class="palette-footer">
			<span class="palette-hint"><kbd>Enter</kbd> run <kbd>Esc</kbd> close</span>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: var(--overlay-bg);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 15vh;
		z-index: var(--z-dialog);
	}

	.palette {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 500px;
		max-height: 60vh;
		display: flex;
		flex-direction: column;
		box-shadow: var(--shadow-lg);
	}

	.palette-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.palette-icon {
		color: var(--accent);
		font-size: 14px;
		font-weight: 600;
		font-family: var(--font-mono, monospace);
	}

	.palette-input {
		flex: 1;
		background: transparent;
		border: none;
		color: var(--text-primary);
		font-size: 14px;
		font-family: var(--font-sans);
		outline: none;
	}

	.palette-list {
		flex: 1;
		overflow-y: auto;
		padding: 4px 0;
	}

	.category-header {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--accent);
		padding: 8px 16px 4px;
	}

	.category-header:first-child {
		padding-top: 4px;
	}

	.palette-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		width: 100%;
		height: 29px;
		padding: 0 16px;
		border: none;
		background: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
	}

	.palette-row:hover,
	.palette-row.cursor {
		background: var(--bg-hover);
	}

	.cmd-label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.palette-empty {
		padding: 16px;
		text-align: center;
		color: var(--text-muted);
		font-size: 13px;
	}

	.palette-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		padding: 6px 16px;
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		font-size: 12px;
		color: var(--text-muted);
	}

	.palette-hint {
		display: flex;
		align-items: center;
		gap: 4px;
	}
</style>
