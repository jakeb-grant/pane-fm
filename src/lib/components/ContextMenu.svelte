<script lang="ts">
export interface MenuItem {
	label: string;
	action: () => void;
	danger?: boolean;
	separator?: false;
}

export interface MenuSeparator {
	separator: true;
}

export type MenuEntry = MenuItem | MenuSeparator;

let {
	x,
	y,
	items,
	onclose,
}: {
	x: number;
	y: number;
	items: MenuEntry[];
	onclose: () => void;
} = $props();

function handleClick(action: () => void) {
	action();
	onclose();
}

let menuStyle = $derived(() => {
	// Keep menu within viewport
	const pad = 8;
	const menuWidth = 220;
	const menuHeight = items.length * 30;
	const left = Math.min(x, window.innerWidth - menuWidth - pad);
	const top = Math.min(y, window.innerHeight - menuHeight - pad);
	return `left: ${left}px; top: ${top}px`;
});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="context-overlay" onclick={onclose} onwheel={(e) => e.preventDefault()} oncontextmenu={(e) => { e.preventDefault(); onclose(); }}></div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="context-menu"
	style={menuStyle()}
	onclick={(e) => e.stopPropagation()}
	onkeydown={(e) => e.key === "Escape" && onclose()}
>
	{#each items as item, i (i)}
		{#if item.separator}
			<div class="separator"></div>
		{:else}
			<button
				class="menu-item"
				class:danger={item.danger}
				onclick={() => handleClick(item.action)}
			>
				{item.label}
			</button>
		{/if}
	{/each}
</div>

<style>
	.context-overlay {
		position: fixed;
		inset: 0;
		z-index: 99;
	}

	.context-menu {
		position: fixed;
		z-index: 100;
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 4px;
		min-width: 200px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.menu-item {
		display: block;
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 6px 12px;
		border-radius: calc(var(--radius) - 2px);
		cursor: pointer;
	}

	.menu-item:hover {
		background: var(--bg-hover);
	}

	.menu-item.danger {
		color: var(--danger);
	}

	.menu-item.danger:hover {
		background: color-mix(in srgb, var(--danger) 15%, transparent);
	}

	.separator {
		height: 1px;
		background: var(--border);
		margin: 4px 8px;
	}
</style>
