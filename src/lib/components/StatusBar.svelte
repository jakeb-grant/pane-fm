<script lang="ts">
let {
	itemCount,
	showHidden,
	previewEnabled,
	ontogglehidden,
	ontogglepreview,
	onopenhelp,
	overlayText = null,
	onclearoverlay,
}: {
	itemCount: number;
	showHidden: boolean;
	previewEnabled: boolean;
	ontogglehidden: () => void;
	ontogglepreview: () => void;
	onopenhelp: () => void;
	overlayText?: string | null;
	onclearoverlay?: () => void;
} = $props();
</script>

<div class="status-bar">
	{#if overlayText}
		<span class="status-text">{overlayText}</span>
		<button class="status-clear" onclick={onclearoverlay}>Clear</button>
	{:else}
		<span class="status-text">{itemCount} {itemCount === 1 ? 'item' : 'items'}</span>
		<div class="status-actions">
			<button class="status-toggle" class:active={previewEnabled} onclick={ontogglepreview} title="Toggle preview (P)">
				Preview
			</button>
			<button class="status-toggle" class:active={showHidden} onclick={ontogglehidden} title="Toggle hidden files (.)">
				Hidden
			</button>
			<button class="status-toggle" onclick={onopenhelp} title="Keyboard shortcuts (?)">
				<kbd>?</kbd>
			</button>
		</div>
	{/if}
</div>

<style>
	.status-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 12px;
		background: var(--bg-secondary);
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		min-height: 28px;
	}

	.status-text {
		font-size: 12px;
		color: var(--text-secondary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-actions {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.status-toggle {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		cursor: pointer;
		padding: 2px 8px;
		border-radius: var(--radius);
		transition: color var(--transition-normal), background var(--transition-normal);
	}

	.status-toggle:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.status-toggle.active {
		color: var(--accent);
	}

	.status-clear {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		cursor: pointer;
		padding: 2px 6px;
		border-radius: var(--radius);
	}

	.status-clear:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}
</style>
