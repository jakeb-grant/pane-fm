<script lang="ts">
import { pathExists } from "$lib/commands";

let {
	currentPath,
	onnavigate,
	drives,
	homeDir,
}: {
	currentPath: string;
	onnavigate: (path: string) => void;
	drives: { name: string; path: string; icon: string }[];
	homeDir: string;
} = $props();

let places = $state<{ label: string; icon: string; path: string }[]>([]);

$effect(() => {
	const dir = homeDir;
	if (dir === "/") return;

	const candidates = [
		{ label: "Home", icon: "\uF015", path: dir },
		{ label: "Desktop", icon: "\uF108", path: `${dir}/Desktop` },
		{ label: "Documents", icon: "\uF07B", path: `${dir}/Documents` },
		{ label: "Downloads", icon: "\uF019", path: `${dir}/Downloads` },
		{ label: "Pictures", icon: "\uF03E", path: `${dir}/Pictures` },
		{ label: "Music", icon: "\uF001", path: `${dir}/Music` },
		{ label: "Videos", icon: "\uF008", path: `${dir}/Videos` },
	];

	Promise.all(candidates.map((c) => pathExists(c.path))).then((checks) => {
		places = candidates.filter((_, i) => checks[i]);
	});
});

const system = [{ label: "Trash", icon: "\uF1F8", path: "trash://" }];
</script>

<aside class="sidebar">
	<section>
		<h3 class="section-label">Places</h3>
		{#each places as item (item.path)}
			<button
				class="sidebar-item"
				class:active={currentPath === item.path}
				onclick={() => onnavigate(item.path)}
			>
				<span class="item-icon">{item.icon}</span>
				<span class="item-label">{item.label}</span>
			</button>
		{/each}
	</section>

	{#if drives.length > 0}
		<section>
			<h3 class="section-label">Drives</h3>
			{#each drives as drive (drive.path)}
				<button
					class="sidebar-item"
					class:active={currentPath === drive.path}
					onclick={() => onnavigate(drive.path)}
				>
					<span class="item-icon">{drive.icon}</span>
					<span class="item-label">{drive.name}</span>
				</button>
			{/each}
		</section>
	{/if}

	<section>
		<h3 class="section-label">System</h3>
		{#each system as item (item.path)}
			<button
				class="sidebar-item"
				class:active={currentPath === item.path}
				onclick={() => onnavigate(item.path)}
			>
				<span class="item-icon">{item.icon}</span>
				<span class="item-label">{item.label}</span>
			</button>
		{/each}
	</section>
</aside>

<style>
	.sidebar {
		width: 200px;
		flex-shrink: 0;
		background: var(--bg-secondary);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px 0;
		overflow-y: auto;
	}

	section {
		display: flex;
		flex-direction: column;
		padding: 0 8px;
	}

	section + section {
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}

	.section-label {
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-muted);
		padding: 4px 8px 6px;
		margin: 0;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 6px 8px;
		background: none;
		border: none;
		border-radius: var(--radius);
		color: var(--text-secondary);
		font-size: 13px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
		transition: background var(--transition-fast), color var(--transition-fast);
	}

	.sidebar-item:hover:not(:disabled) {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.sidebar-item.active {
		background: var(--bg-surface);
		color: var(--accent);
	}

	.sidebar-item:disabled {
		opacity: 0.35;
		cursor: default;
	}

	.item-icon {
		font-family: var(--font-icon);
		font-size: 15px;
		width: 20px;
		text-align: center;
		flex-shrink: 0;
	}

	.item-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
