<script lang="ts">
import { pathSegments } from "$lib/utils";

let { path, onnavigate }: { path: string; onnavigate: (path: string) => void } =
	$props();

let segments = $derived(pathSegments(path));
</script>

<nav class="breadcrumb">
	{#each segments as segment, i (segment.path)}
		{#if i > 0}
			<span class="separator">/</span>
		{/if}
		<button
			class="segment"
			class:active={i === segments.length - 1}
			onclick={() => onnavigate(segment.path)}
		>
			{segment.name}
		</button>
	{/each}
</nav>

<style>
	.breadcrumb {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 8px 12px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
		white-space: nowrap;
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
</style>
