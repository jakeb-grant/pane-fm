<script lang="ts">
import { keybindLabel, keybinds, matchesKeybind } from "$lib/keybinds";
import { formatSize } from "$lib/utils";

let {
	message,
	progress,
	oncancel,
}: {
	message: string;
	progress: { processed: number; total: number } | null;
	oncancel: () => void;
} = $props();
</script>

<svelte:window onkeydown={(e) => {
	if (matchesKeybind(e, keybinds.escape)) oncancel();
}} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="busy-overlay" onwheel={(e) => e.preventDefault()} onclick={(e) => e.stopPropagation()}>
	<div class="busy-card">
		<div class="busy-header">
			<div class="busy-spinner"></div>
			<span class="busy-text">{message}</span>
		</div>
		{#if progress && progress.total > 0}
			<div class="busy-progress-track">
				<div class="busy-progress-bar" style="width: {Math.min(100, (progress.processed / progress.total) * 100)}%"></div>
			</div>
			<span class="busy-detail">{formatSize(progress.processed)} / {formatSize(progress.total)}</span>
		{/if}
		<button class="busy-cancel" onclick={oncancel}>Cancel <kbd>{keybindLabel(keybinds.escape)}</kbd></button>
	</div>
</div>

<style>
	.busy-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 300;
	}

	.busy-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		padding: 20px 24px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		min-width: 280px;
	}

	.busy-header {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.busy-spinner {
		width: 16px;
		height: 16px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.busy-text {
		font-size: 13px;
		color: var(--text-primary);
	}

	.busy-progress-track {
		height: 4px;
		background: var(--bg-surface);
		border-radius: 2px;
		overflow: hidden;
	}

	.busy-progress-bar {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
		transition: width 0.2s ease;
	}

	.busy-detail {
		font-size: 11px;
		color: var(--text-muted);
		text-align: right;
	}

	.busy-cancel {
		align-self: flex-end;
		background: none;
		border: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: 12px;
		font-family: var(--font-sans);
		padding: 4px 14px;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.busy-cancel:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	kbd {
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		padding: 1px 4px;
		border-radius: 3px;
		background: rgba(255, 255, 255, 0.1);
		margin-left: 4px;
		opacity: 0.7;
	}
</style>
