<script lang="ts">
// biome-ignore lint/style/useImportType: component used in template
import Breadcrumb from "$lib/components/Breadcrumb.svelte";

let breadcrumb = $state<ReturnType<typeof Breadcrumb> | null>(null);

export function focusPath() {
	breadcrumb?.focusInput();
}

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
	onopenhelp,
	previewEnabled = false,
	ontogglepreview,
	isDragging = false,
	dropTarget = null,
	ondragoverpath,
	ondroppath,
	ondragleavepath,
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
	onopenhelp: () => void;
	previewEnabled?: boolean;
	ontogglepreview?: () => void;
	isDragging?: boolean;
	dropTarget?: string | null;
	ondragoverpath?: (path: string) => void;
	ondroppath?: (path: string, ctrlKey: boolean) => void;
	ondragleavepath?: () => void;
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
		<Breadcrumb bind:this={breadcrumb} path={currentPath} onnavigate={onnavigate} {isDragging} {dropTarget} {ondragoverpath} {ondroppath} {ondragleavepath} />
	</div>
	<button class="nav-btn icon small" class:active={previewEnabled} onclick={ontogglepreview} title="Toggle preview (P)">
		{"\uf518"}
	</button>
	<button class="nav-btn icon small" class:active={showHidden} onclick={ontogglehidden} title="Toggle hidden files">
		{showHidden ? "\uf06e" : "\uf070"}
	</button>
	<button class="nav-btn icon small" onclick={onopenhelp} title="Keybinds (?)">
		{"\uf059"}
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
		transition: color var(--transition-normal);
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

	.nav-btn.small {
		font-size: 17px;
	}

	.nav-btn.active {
		color: var(--accent);
	}

	.breadcrumb-wrapper {
		flex: 1;
		min-width: 0;
	}
</style>
