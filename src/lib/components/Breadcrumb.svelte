<script lang="ts">
import { listDirectory } from "$lib/commands";
import { pathSegments } from "$lib/utils";

let { path, onnavigate }: { path: string; onnavigate: (path: string) => void } =
	$props();

let segments = $derived(pathSegments(path));
let editing = $state(false);
let editValue = $state("");
let editInput = $state<HTMLInputElement | null>(null);

// Autocomplete state
let suggestions = $state<string[]>([]);
let selectedIndex = $state(-1);
let fetchTimer: ReturnType<typeof setTimeout> | null = null;

function startEditing() {
	editValue = path;
	editing = true;
	suggestions = [];
	selectedIndex = -1;
}

function commit(value?: string) {
	const target = value ?? editValue.trim();
	editing = false;
	suggestions = [];
	if (target && target !== path) {
		onnavigate(target);
	}
}

function cancel() {
	editing = false;
	suggestions = [];
}

function acceptSuggestion(suggestion: string) {
	editValue = suggestion.endsWith("/") ? suggestion : `${suggestion}/`;
	suggestions = [];
	selectedIndex = -1;
	editInput?.focus();
	fetchSuggestions();
}

function getParentAndPrefix(input: string): { parent: string; prefix: string } {
	const lastSlash = input.lastIndexOf("/");
	if (lastSlash === -1) return { parent: "/", prefix: input.toLowerCase() };
	const parent = input.slice(0, lastSlash) || "/";
	const prefix = input.slice(lastSlash + 1).toLowerCase();
	return { parent, prefix };
}

async function fetchSuggestions() {
	const { parent, prefix } = getParentAndPrefix(editValue);
	try {
		const entries = await listDirectory(parent, true);
		const dirs = entries
			.filter((e) => e.is_dir && e.name.toLowerCase().startsWith(prefix))
			.map((e) => (parent === "/" ? `/${e.name}` : `${parent}/${e.name}`))
			.slice(0, 8);
		suggestions = dirs;
		selectedIndex = -1;
	} catch {
		suggestions = [];
	}
}

function scheduleFetch() {
	if (fetchTimer) clearTimeout(fetchTimer);
	fetchTimer = setTimeout(fetchSuggestions, 80);
}

function handleKeydown(e: KeyboardEvent) {
	if (e.key === "Enter") {
		e.preventDefault();
		if (selectedIndex >= 0 && selectedIndex < suggestions.length) {
			acceptSuggestion(suggestions[selectedIndex]);
		} else {
			commit();
		}
	} else if (e.key === "Escape") {
		e.preventDefault();
		if (suggestions.length > 0) {
			suggestions = [];
			selectedIndex = -1;
		} else {
			cancel();
		}
	} else if (e.key === "Tab" && suggestions.length > 0) {
		e.preventDefault();
		const idx = selectedIndex >= 0 ? selectedIndex : 0;
		acceptSuggestion(suggestions[idx]);
	} else if (e.key === "ArrowDown" && suggestions.length > 0) {
		e.preventDefault();
		selectedIndex = Math.min(selectedIndex + 1, suggestions.length - 1);
	} else if (e.key === "ArrowUp" && suggestions.length > 0) {
		e.preventDefault();
		selectedIndex = Math.max(selectedIndex - 1, -1);
	}
}

$effect(() => {
	if (editing && editInput) {
		editInput.focus();
		editInput.select();
	}
});
</script>

{#if editing}
	<div class="edit-wrapper">
		<input
			class="path-input"
			type="text"
			bind:value={editValue}
			bind:this={editInput}
			onkeydown={handleKeydown}
			oninput={scheduleFetch}
			onblur={() => { setTimeout(() => { if (editing) commit(); }, 150); }}
		/>
		{#if suggestions.length > 0}
			<div class="suggestions">
				{#each suggestions as suggestion, i (suggestion)}
					<button
						class="suggestion"
						class:selected={i === selectedIndex}
						onmousedown={(e) => { e.preventDefault(); acceptSuggestion(suggestion); }}
					>
						<span class="suggestion-icon">{"\uF07B"}</span>
						{suggestion.split("/").pop()}
					</button>
				{/each}
			</div>
		{/if}
	</div>
{:else}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<nav class="breadcrumb" onclick={(e) => { if (e.target === e.currentTarget) startEditing(); }}>
		{#each segments as segment, i (segment.path)}
			{#if i > 0}
				<span class="separator">/</span>
			{/if}
			<button
				class="segment"
				class:active={i === segments.length - 1}
				onclick={() => onnavigate(segment.path)}
				ondblclick={(e) => { e.stopPropagation(); startEditing(); }}
			>
				{segment.name}
			</button>
		{/each}
	</nav>
{/if}

<style>
	.breadcrumb {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 4px 8px;
		overflow-x: auto;
		white-space: nowrap;
		min-height: 28px;
		cursor: text;
	}

	.separator {
		color: var(--text-muted);
		font-size: 12px;
	}

	.segment {
		background: none;
		border: none;
		color: var(--text-secondary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 2px 6px;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.segment:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.segment.active {
		color: var(--accent);
	}

	.edit-wrapper {
		position: relative;
		width: 100%;
	}

	.path-input {
		width: 100%;
		background: var(--bg-primary);
		border: 1px solid var(--accent);
		border-radius: var(--radius);
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-mono);
		padding: 4px 8px;
		outline: none;
		box-sizing: border-box;
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.suggestions {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		margin-top: 2px;
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
		z-index: 100;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.suggestion {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 6px 10px;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
	}

	.suggestion:hover,
	.suggestion.selected {
		background: var(--bg-hover);
	}

	.suggestion-icon {
		font-family: var(--font-icon);
		font-size: 14px;
		color: var(--accent);
	}
</style>
