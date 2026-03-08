<script lang="ts">
import { listDirectory, getHomeDir, pathExists } from "$lib/commands";
import type { FileEntry } from "$lib/commands";
import { parentPath, pathSegments } from "$lib/utils";
import { onMount } from "svelte";

let {
	title,
	onselect,
	onclose,
}: {
	title: string;
	onselect: (path: string) => void;
	onclose: () => void;
} = $props();

let currentDir = $state("/");
let folders = $state<FileEntry[]>([]);
let loading = $state(false);
let err = $state<string | null>(null);
let places = $state<{ label: string; icon: string; path: string }[]>([]);

let segments = $derived(pathSegments(currentDir));

async function loadDir(path: string) {
	loading = true;
	err = null;
	try {
		const entries = await listDirectory(path, false);
		folders = entries.filter((e) => e.is_dir);
		currentDir = path;
	} catch (e) {
		err = String(e);
	} finally {
		loading = false;
	}
}

function goUp() {
	const parent = parentPath(currentDir);
	if (parent !== currentDir) {
		loadDir(parent);
	}
}

onMount(async () => {
	let homePath: string;
	try {
		homePath = await getHomeDir();
	} catch {
		homePath = "/home";
	}

	await loadDir(homePath);

	const candidates = [
		{ label: "Home", icon: "\uF015", path: homePath },
		{ label: "Desktop", icon: "\uF108", path: `${homePath}/Desktop` },
		{ label: "Documents", icon: "\uF07B", path: `${homePath}/Documents` },
		{ label: "Downloads", icon: "\uF019", path: `${homePath}/Downloads` },
		{ label: "Pictures", icon: "\uF03E", path: `${homePath}/Pictures` },
		{ label: "Music", icon: "\uF001", path: `${homePath}/Music` },
		{ label: "Videos", icon: "\uF008", path: `${homePath}/Videos` },
	];

	const checks = await Promise.all(candidates.map((c) => pathExists(c.path)));
	places = candidates.filter((_, i) => checks[i]);
});
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="dialog" onclick={(e) => e.stopPropagation()}>
		<div class="toolbar">
			<button class="nav-btn icon" onclick={goUp} disabled={currentDir === "/"} title="Up">
				{"\uF062"}
			</button>
			<nav class="breadcrumb">
				{#each segments as segment, i (segment.path)}
					{#if i > 0}
						<span class="separator">/</span>
					{/if}
					<button
						class="segment"
						class:active={i === segments.length - 1}
						onclick={() => loadDir(segment.path)}
					>
						{segment.name}
					</button>
				{/each}
			</nav>
		</div>

		<div class="body">
			{#if places.length > 0}
				<aside class="sidebar">
					<h3 class="section-label">Places</h3>
					{#each places as item (item.path)}
						<button
							class="sidebar-item"
							class:active={currentDir === item.path}
							onclick={() => loadDir(item.path)}
						>
							<span class="item-icon">{item.icon}</span>
							<span class="item-label">{item.label}</span>
						</button>
					{/each}
				</aside>
			{/if}

			<div class="folder-list">
				{#if loading}
					<div class="empty">Loading...</div>
				{:else if err}
					<div class="empty error">{err}</div>
				{:else if folders.length === 0}
					<div class="empty">No subfolders</div>
				{:else}
					{#each folders as folder (folder.path)}
						<button class="folder-item" ondblclick={() => loadDir(folder.path)} onclick={() => loadDir(folder.path)}>
							<span class="folder-icon">{"\uF07B"}</span>
							<span class="folder-name">{folder.name}</span>
						</button>
					{/each}
				{/if}
			</div>
		</div>

		<div class="footer">
			<span class="footer-title">{title}</span>
			<div class="footer-actions">
				<button class="btn cancel" onclick={onclose}>Cancel</button>
				<button class="btn confirm" onclick={() => onselect(currentDir)}>Select</button>
			</div>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.dialog {
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 560px;
		max-height: 70vh;
		display: flex;
		flex-direction: column;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		overflow: hidden;
	}

	/* Toolbar — mirrors the main app toolbar */
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

	/* Breadcrumb — same as main app */
	.breadcrumb {
		display: flex;
		align-items: center;
		gap: 2px;
		overflow-x: auto;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
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

	/* Body — sidebar + folder list, mirrors main app layout */
	.body {
		flex: 1;
		display: flex;
		overflow: hidden;
		min-height: 0;
	}

	/* Sidebar — mirrors main app sidebar */
	.sidebar {
		width: 160px;
		flex-shrink: 0;
		background: var(--bg-secondary);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 8px;
		overflow-y: auto;
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
		padding: 5px 8px;
		background: none;
		border: none;
		border-radius: var(--radius);
		color: var(--text-secondary);
		font-size: 12px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
		transition: background 0.1s, color 0.1s;
	}

	.sidebar-item:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.sidebar-item.active {
		background: var(--bg-surface);
		color: var(--accent);
	}

	.item-icon {
		font-family: var(--font-icon);
		font-size: 14px;
		width: 18px;
		text-align: center;
		flex-shrink: 0;
	}

	.item-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Folder list — mirrors file list rows */
	.folder-list {
		flex: 1;
		overflow-y: auto;
		padding: 4px;
		min-height: 200px;
	}

	.folder-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 6px 12px;
		background: none;
		border: none;
		border-radius: var(--radius);
		color: var(--accent);
		font-size: 13px;
		font-family: var(--font-sans);
		cursor: pointer;
		text-align: left;
	}

	.folder-item:hover {
		background: var(--bg-surface);
	}

	.folder-icon {
		font-family: var(--font-icon);
		font-size: 16px;
		width: 20px;
		text-align: center;
		flex-shrink: 0;
	}

	.folder-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.empty {
		padding: 40px;
		text-align: center;
		color: var(--text-muted);
		font-size: 13px;
	}

	.empty.error {
		color: var(--danger);
	}

	/* Footer */
	.footer {
		padding: 8px 12px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		border-top: 1px solid var(--border);
		background: var(--bg-secondary);
	}

	.footer-title {
		font-size: 12px;
		color: var(--text-muted);
		font-weight: 500;
	}

	.footer-actions {
		display: flex;
		gap: 8px;
	}

	.btn {
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 5px 14px;
		border-radius: var(--radius);
		cursor: pointer;
		border: 1px solid var(--border);
	}

	.btn.cancel {
		background: none;
		color: var(--text-secondary);
	}

	.btn.cancel:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.btn.confirm {
		background: var(--accent);
		color: var(--bg-primary);
		border-color: var(--accent);
	}

	.btn.confirm:hover {
		opacity: 0.9;
	}
</style>
