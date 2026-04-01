<script lang="ts">
import { mountDrive, pathExists, unmountDrive } from "$lib/commands";
import { errorMessage } from "$lib/errors";

let {
	currentPath,
	onnavigate,
	drives,
	homeDir,
	onrefreshdrives,
	onerror,
	collapsed = false,
	isDragging = false,
	dropTarget = null,
	ondragover,
	ondrop,
	ondragleave,
}: {
	currentPath: string;
	onnavigate: (path: string) => void;
	drives: {
		name: string;
		path: string;
		device: string;
		icon: string;
		mounted: boolean;
		size: string;
	}[];
	homeDir: string;
	onrefreshdrives: () => void;
	onerror: (msg: string) => void;
	collapsed?: boolean;
	isDragging?: boolean;
	dropTarget?: string | null;
	ondragover?: (path: string) => void;
	ondrop?: (path: string, ctrlKey: boolean) => void;
	ondragleave?: () => void;
} = $props();

let places = $state<{ label: string; icon: string; path: string }[]>([]);
let mounting = $state<string | null>(null);

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

let ejecting = $state<string | null>(null);

async function handleMount(drive: { device: string }) {
	if (mounting) return;
	mounting = drive.device;
	try {
		const mountPoint = await mountDrive(drive.device);
		onrefreshdrives();
		if (mountPoint) onnavigate(mountPoint);
	} catch (e) {
		onerror(errorMessage(e) ?? "Mount failed");
	} finally {
		mounting = null;
	}
}

async function handleEject(
	e: MouseEvent,
	drive: { device: string; path: string },
) {
	e.stopPropagation();
	if (ejecting) return;
	ejecting = drive.device;
	try {
		if (drive.path && currentPath.startsWith(drive.path)) {
			onnavigate(homeDir);
		}
		await unmountDrive(drive.device);
		onrefreshdrives();
	} catch (err) {
		onerror(errorMessage(err) ?? "Eject failed");
	} finally {
		ejecting = null;
	}
}
</script>

<aside class="sidebar" class:collapsed>
	<section>
		{#if !collapsed}<h3 class="section-label">Places</h3>{/if}
		{#each places as item (item.path)}
			<button
				class="sidebar-item"
				class:active={currentPath === item.path}
				class:drop-target={dropTarget === item.path}
				title={collapsed ? item.label : undefined}
				onclick={() => onnavigate(item.path)}
				onmouseenter={() => { if (isDragging) ondragover?.(item.path); }}
				onmouseleave={() => { if (isDragging) ondragleave?.(); }}
				onmouseup={(e) => { if (isDragging) ondrop?.(item.path, e.ctrlKey); }}
			>
				<span class="item-icon">{item.icon}</span>
				{#if !collapsed}<span class="item-label">{item.label}</span>{/if}
			</button>
		{/each}
	</section>

	{#if drives.length > 0}
		<section>
			{#if !collapsed}<h3 class="section-label">Drives</h3>{/if}
			{#each drives as drive (drive.device)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="drive-row"
					class:active={drive.mounted && currentPath === drive.path}
					class:unmounted={!drive.mounted}
					class:drop-target={dropTarget === drive.path}
					onmouseenter={() => { if (isDragging && drive.mounted) ondragover?.(drive.path); }}
					onmouseleave={() => { if (isDragging) ondragleave?.(); }}
					onmouseup={(e) => { if (isDragging && drive.mounted) ondrop?.(drive.path, e.ctrlKey); }}
				>
					<button
						class="sidebar-item drive-item"
						disabled={mounting === drive.device || ejecting === drive.device}
						title={collapsed ? drive.name : undefined}
						onclick={() => drive.mounted ? onnavigate(drive.path) : handleMount(drive)}
					>
						<span class="item-icon">{drive.icon}</span>
						{#if !collapsed}
							<span class="item-label">{drive.name}</span>
							{#if !drive.mounted}
								<span class="item-hint">{drive.size}</span>
							{/if}
						{/if}
					</button>
					{#if drive.mounted && !collapsed}
						<button
							class="eject-btn"
							title="Safely remove"
							disabled={ejecting === drive.device}
							onclick={(e) => handleEject(e, drive)}
						>{"\uF052"}</button>
					{/if}
				</div>
			{/each}
		</section>
	{/if}

	<section>
		{#if !collapsed}<h3 class="section-label">System</h3>{/if}
		{#each system as item (item.path)}
			<button
				class="sidebar-item"
				class:active={currentPath === item.path}
				class:drop-target={dropTarget === item.path}
				title={collapsed ? item.label : undefined}
				onclick={() => onnavigate(item.path)}
				onmouseenter={() => { if (isDragging) ondragover?.(item.path); }}
				onmouseleave={() => { if (isDragging) ondragleave?.(); }}
				onmouseup={(e) => { if (isDragging) ondrop?.(item.path, e.ctrlKey); }}
			>
				<span class="item-icon">{item.icon}</span>
				{#if !collapsed}<span class="item-label">{item.label}</span>{/if}
			</button>
		{/each}
	</section>
</aside>

<style>
	.sidebar {
		width: fit-content;
		max-width: 200px;
		flex-shrink: 0;
		transition: max-width var(--transition-normal), padding var(--transition-normal);
		background: var(--bg-secondary);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 8px 0;
		overflow-y: auto;
	}

	.sidebar.collapsed {
		max-width: none;
		padding: 8px 4px;
	}

	section {
		display: flex;
		flex-direction: column;
		padding: 0 8px;
	}

	.collapsed section {
		padding: 0 4px;
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

	.collapsed .sidebar-item {
		justify-content: center;
		padding: 6px;
		gap: 0;
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

	.sidebar-item.drop-target {
		background: color-mix(in srgb, var(--accent) 20%, transparent);
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
		flex: 1;
		min-width: 0;
	}

	.item-hint {
		font-size: 11px;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.drive-row {
		display: flex;
		align-items: center;
		border-radius: var(--radius);
		transition: background var(--transition-fast);
	}

	.drive-row:hover {
		background: var(--bg-surface);
	}

	.drive-row.active {
		background: var(--bg-surface);
	}

	.drive-row.unmounted {
		opacity: 0.5;
	}

	.drive-row.unmounted:hover {
		opacity: 0.8;
	}

	.drive-row.drop-target {
		background: color-mix(in srgb, var(--accent) 20%, transparent);
	}

	.drive-item {
		flex: 1;
		min-width: 0;
	}

	.drive-row:hover .sidebar-item {
		background: none;
	}

	.drive-row.active .sidebar-item {
		background: none;
		color: var(--accent);
	}

	.eject-btn {
		background: none;
		border: none;
		padding: 2px 6px 2px 0;
		font-family: var(--font-icon);
		font-size: 12px;
		color: var(--text-muted);
		cursor: pointer;
		flex-shrink: 0;
		opacity: 0;
		transition: opacity var(--transition-fast), color var(--transition-fast);
	}

	.drive-row:hover .eject-btn {
		opacity: 1;
	}

	.eject-btn:hover {
		color: var(--text-primary);
	}

	.eject-btn:disabled {
		opacity: 0.35;
		cursor: default;
	}
</style>
