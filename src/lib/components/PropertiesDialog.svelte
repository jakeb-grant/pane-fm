<script lang="ts">
import { onMount } from "svelte";
import { type FileProperties, getDirStats } from "$lib/commands";
import { getIconForEntry } from "$lib/icons";
import { formatSize } from "$lib/utils";

let {
	properties,
	onclose,
}: {
	properties: FileProperties;
	onclose: () => void;
} = $props();

let dirSize = $state<number | null>(null);
let dirCount = $state<number | null>(null);
let dirStatsLoading = $state(false);

const iconEntry = {
	name: properties.name,
	path: properties.path,
	is_dir: properties.is_dir,
	is_symlink: properties.is_symlink,
	mime_type: properties.mime_type,
	size: properties.size,
	modified: properties.modified ?? "",
	permissions: 0,
	hidden: properties.name.startsWith("."),
	children_count: null,
};

onMount(() => {
	if (properties.is_dir) {
		dirStatsLoading = true;
		getDirStats(properties.path).then((stats) => {
			dirSize = stats.size;
			dirCount = stats.contents_count;
			dirStatsLoading = false;
		}).catch(() => {
			dirStatsLoading = false;
		});
	}
});
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="dialog" onclick={(e) => e.stopPropagation()}>
		<div class="header">
			<span class="file-icon">{getIconForEntry(iconEntry)}</span>
			<h2 class="file-name">{properties.name}</h2>
		</div>

		<div class="props">
			<div class="row">
				<span class="label">Type</span>
				<span class="value">{properties.is_dir ? "Directory" : properties.mime_type}</span>
			</div>
			<div class="row">
				<span class="label">Location</span>
				<span class="value path">{properties.path}</span>
			</div>
			<div class="row">
				<span class="label">Size</span>
				<span class="value">
					{#if properties.is_dir}
						{#if dirSize != null}
							{formatSize(dirSize)}
						{:else}
							<span class="calculating">Calculating…</span>
						{/if}
					{:else}
						{formatSize(properties.size)}
					{/if}
				</span>
			</div>
			{#if properties.is_dir}
				<div class="row">
					<span class="label">Contents</span>
					<span class="value">
						{#if dirCount != null}
							{dirCount.toLocaleString()} items
						{:else}
							<span class="calculating">Calculating…</span>
						{/if}
					</span>
				</div>
			{/if}
			{#if properties.is_symlink && properties.link_target}
				<div class="row">
					<span class="label">Link Target</span>
					<span class="value path">{properties.link_target}</span>
				</div>
			{/if}

			<div class="divider"></div>

			{#if properties.created}
				<div class="row">
					<span class="label">Created</span>
					<span class="value">{properties.created}</span>
				</div>
			{/if}
			{#if properties.modified}
				<div class="row">
					<span class="label">Modified</span>
					<span class="value">{properties.modified}</span>
				</div>
			{/if}
			{#if properties.accessed}
				<div class="row">
					<span class="label">Accessed</span>
					<span class="value">{properties.accessed}</span>
				</div>
			{/if}

			<div class="divider"></div>

			<div class="row">
				<span class="label">Permissions</span>
				<span class="value mono">{properties.permissions}</span>
			</div>
			<div class="row">
				<span class="label">Owner</span>
				<span class="value">{properties.owner}</span>
			</div>
			<div class="row">
				<span class="label">Group</span>
				<span class="value">{properties.group}</span>
			</div>
		</div>

		<div class="footer">
			<button class="close-btn" onclick={onclose}>Close</button>
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
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 380px;
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
	}

	.header {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 20px 20px 12px;
	}

	.file-icon {
		font-family: var(--font-icon);
		font-size: 28px;
		color: var(--accent);
	}

	.file-name {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: var(--text-primary);
		word-break: break-all;
	}

	.props {
		padding: 0 20px 12px;
	}

	.row {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		padding: 4px 0;
		gap: 16px;
	}

	.label {
		font-size: 12px;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.value {
		font-size: 12px;
		color: var(--text-primary);
		text-align: right;
		word-break: break-all;
	}

	.calculating {
		color: var(--text-muted);
		font-style: italic;
	}

	.value.path {
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--text-secondary);
	}

	.value.mono {
		font-family: var(--font-mono);
	}

	.divider {
		height: 1px;
		background: var(--border);
		margin: 8px 0;
	}

	.footer {
		padding: 12px 20px 16px;
		display: flex;
		justify-content: flex-end;
	}

	.close-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 6px 16px;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.close-btn:hover {
		background: var(--bg-hover);
	}
</style>
