<script lang="ts">
import Breadcrumb from "$lib/components/Breadcrumb.svelte";

let {
	canGoBack,
	canGoForward,
	ongoback,
	ongoforward,
	ongoup,
	currentPath,
	onnavigate,
	showHidden,
	ontogglehidden,
}: {
	canGoBack: boolean;
	canGoForward: boolean;
	ongoback: () => void;
	ongoforward: () => void;
	ongoup: () => void;
	currentPath: string;
	onnavigate: (path: string) => void;
	showHidden: boolean;
	ontogglehidden: () => void;
} = $props();
</script>

<div class="toolbar">
	<button class="nav-btn icon" onclick={ongoback} disabled={!canGoBack} title="Back">
		{"\uf060"}
	</button>
	<button class="nav-btn icon" onclick={ongoforward} disabled={!canGoForward} title="Forward">
		{"\uf061"}
	</button>
	<button class="nav-btn icon" onclick={ongoup} title="Up">{"\uf062"}</button>
	<div class="breadcrumb-wrapper">
		<Breadcrumb path={currentPath} onnavigate={onnavigate} />
	</div>
	<button class="nav-btn icon" class:active={showHidden} onclick={ontogglehidden} title="Toggle hidden files">
		{showHidden ? "\uf06e" : "\uf070"}
	</button>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 6px 24px 6px 16px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
	}

	.nav-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 22px;
		padding: 4px 6px;
		cursor: pointer;
		flex-shrink: 0;
		transition: color 0.15s;
	}

	.nav-btn.icon {
		font-family: var(--font-icon);
	}

	.nav-btn:hover:not(:disabled) {
		color: var(--text-primary);
	}

	.nav-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.nav-btn.active {
		color: var(--accent);
	}

	.breadcrumb-wrapper {
		flex: 1;
		min-width: 0;
	}
</style>
