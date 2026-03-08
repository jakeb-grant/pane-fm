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
	sortBy,
	sortAsc,
	onsort,
	viewMode,
	onviewtoggle,
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
	sortBy: string;
	sortAsc: boolean;
	onsort: (column: string) => void;
	viewMode: "list" | "grid";
	onviewtoggle: () => void;
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
	<div class="toolbar-group">
		<div class="sort-control">
			{#each [["name", "Name"], ["size", "Size"], ["modified", "Date"]] as [key, label] (key)}
				<button class="sort-btn" class:active={sortBy === key} onclick={() => onsort(key)}>
					{label}<span class="sort-arrow" class:visible={sortBy === key}>{sortAsc ? "\u25b2" : "\u25bc"}</span>
				</button>
			{/each}
		</div>
		<button class="nav-btn icon" class:active={viewMode === "grid"} onclick={onviewtoggle} title="Toggle view mode">
			{viewMode === "list" ? "\uf00a" : "\uf00b"}
		</button>
		<button class="nav-btn icon" class:active={showHidden} onclick={ontogglehidden} title="Toggle hidden files">
			{showHidden ? "\uf06e" : "\uf070"}
		</button>
	</div>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 6px 8px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
	}

	.nav-btn {
		background: none;
		border: 1px solid transparent;
		color: var(--text-secondary);
		font-size: 16px;
		width: 32px;
		height: 32px;
		border-radius: var(--radius);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.nav-btn.icon {
		font-family: var(--font-icon);
	}

	.nav-btn:hover:not(:disabled) {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.nav-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.nav-btn.active {
		color: var(--accent);
	}

	.toolbar-group {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.sort-control {
		display: flex;
		align-items: center;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.sort-btn {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-sans);
		padding: 5px 10px;
		cursor: pointer;
		transition: color 0.15s, background 0.15s;
		letter-spacing: 0.03em;
		white-space: nowrap;
	}

	.sort-btn:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.sort-btn.active {
		color: var(--accent);
		background: var(--bg-surface);
	}

	.sort-arrow {
		font-size: 8px;
		margin-left: 3px;
		opacity: 0;
		display: inline-block;
		width: 8px;
	}

	.sort-arrow.visible {
		opacity: 0.7;
	}

	.breadcrumb-wrapper {
		flex: 1;
		min-width: 0;
	}
</style>
