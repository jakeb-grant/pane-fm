<script lang="ts">
let {
	tabs,
	activeIndex,
	onswitch,
	onclose,
	onnew,
}: {
	tabs: { id: number; label: string }[];
	activeIndex: number;
	onswitch: (index: number) => void;
	onclose: (index: number) => void;
	onnew: () => void;
} = $props();
</script>

<div class="tab-bar">
	{#each tabs as tab, i (tab.id)}
		<button
			class="tab"
			class:active={i === activeIndex}
			onclick={() => onswitch(i)}
		>
			<span class="tab-index">{i + 1}</span>
			<span class="tab-label">{tab.label}</span>
			{#if tabs.length > 1}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<span class="tab-close" onclick={(e) => { e.stopPropagation(); onclose(i); }}>×</span>
			{/if}
		</button>
	{/each}
	{#if tabs.length < 9}
		<button class="tab-new" onclick={onnew} title="New tab (t)">+</button>
	{/if}
</div>

<style>
	.tab-bar {
		display: flex;
		align-items: stretch;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
		flex-shrink: 0;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 12px;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--text-muted);
		font-size: 12px;
		font-family: var(--font-sans);
		cursor: pointer;
		white-space: nowrap;
		min-width: 0;
		max-width: 160px;
		transition: color 0.1s, border-color 0.1s;
	}

	.tab:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.tab.active {
		color: var(--accent);
		border-bottom-color: var(--accent);
	}

	.tab-index {
		font-size: 10px;
		font-family: var(--font-mono);
		opacity: 0.5;
		flex-shrink: 0;
	}

	.tab-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tab-close {
		font-size: 14px;
		line-height: 1;
		opacity: 0;
		flex-shrink: 0;
		padding: 0 2px;
		border-radius: 3px;
		transition: opacity 0.1s;
	}

	.tab:hover .tab-close {
		opacity: 0.5;
	}

	.tab-close:hover {
		opacity: 1 !important;
		background: var(--bg-hover);
	}

	.tab-new {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 16px;
		padding: 4px 10px;
		cursor: pointer;
		flex-shrink: 0;
		transition: color 0.1s;
	}

	.tab-new:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}
</style>
