<script lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onDestroy, onMount } from "svelte";
import {
	chmodEntry,
	type DirStats,
	type FileProperties,
	getDirStats,
} from "$lib/commands";
import { errorMessage } from "$lib/errors";
import { getIconForEntry } from "$lib/icons";
import { keybindLabel, keybinds, matchesKeybind } from "$lib/keybinds";
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
let unlisten: UnlistenFn | null = null;

let editMode = $state(Number.parseInt(properties.permissions, 8));
const originalMode = Number.parseInt(properties.permissions, 8);
let saving = $state(false);
let permError = $state<string | null>(null);

const octalDisplay = $derived(editMode.toString(8).padStart(3, "0"));
const isDirty = $derived(editMode !== originalMode);

const permRows = [
	{ label: "Owner", r: 0o400, w: 0o200, x: 0o100 },
	{ label: "Group", r: 0o040, w: 0o020, x: 0o010 },
	{ label: "Other", r: 0o004, w: 0o002, x: 0o001 },
] as const;

function toggleBit(bit: number) {
	// biome-ignore lint/suspicious/noBitwiseOperators: XOR toggles permission bits
	editMode ^= bit;
}

async function applyPermissions() {
	saving = true;
	permError = null;
	try {
		await chmodEntry(properties.path, editMode);
	} catch (e) {
		permError = errorMessage(e) ?? "Failed to change permissions";
	}
	saving = false;
}

const iconEntry = $derived({
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
});

onMount(async () => {
	if (properties.is_dir) {
		dirStatsLoading = true;
		unlisten = await listen<DirStats>("dir-stats-progress", (event) => {
			dirSize = event.payload.size;
			dirCount = event.payload.contents_count;
		});
		getDirStats(properties.path)
			.then((stats) => {
				dirSize = stats.size;
				dirCount = stats.contents_count;
				dirStatsLoading = false;
			})
			.catch(() => {
				dirStatsLoading = false;
			});
	}
});

onDestroy(() => {
	unlisten?.();
});
</script>

<svelte:window onkeydown={(e) => {
	if (matchesKeybind(e, keybinds.menuClose) || matchesKeybind(e, keybinds.menuAccept)) onclose();
}} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
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
							{formatSize(dirSize)}{#if dirStatsLoading}<span class="calculating"> …</span>{/if}
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
							{dirCount.toLocaleString()} items{#if dirStatsLoading}<span class="calculating"> …</span>{/if}
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
				<span class="value mono">{octalDisplay}</span>
			</div>
			<div class="perms-grid">
				{#each permRows as row}
					<div class="perms-row">
						<span class="perms-label">{row.label}</span>
						<div class="perms-buttons">
							{#each [{ label: "r", bit: row.r }, { label: "w", bit: row.w }, { label: "x", bit: row.x }] as perm}
								<button
									class="perm-btn"
									class:active={!!(editMode & perm.bit)}
									onclick={() => toggleBit(perm.bit)}
								>{perm.label}</button>
							{/each}
						</div>
					</div>
				{/each}
			</div>
			{#if isDirty}
				<button class="apply-btn" onclick={applyPermissions} disabled={saving}>
					{saving ? "Applying\u2026" : "Apply"}
				</button>
			{/if}
			{#if permError}
				<span class="perms-error">{permError}</span>
			{/if}
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
			<button class="close-btn" onclick={onclose}>Close <kbd>q/Esc</kbd></button>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: var(--overlay-bg);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: var(--z-dialog);
	}

	.dialog {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 380px;
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: var(--shadow-lg);
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

	.perms-grid {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: 6px 0;
	}

	.perms-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.perms-label {
		font-size: 12px;
		color: var(--text-muted);
		width: 48px;
	}

	.perms-buttons {
		display: flex;
		gap: 4px;
	}

	.perm-btn {
		width: 28px;
		height: 24px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--bg-surface);
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.1s, color 0.1s, border-color 0.1s;
	}

	.perm-btn:hover {
		border-color: var(--accent);
	}

	.perm-btn.active {
		background: var(--accent);
		color: var(--bg-primary);
		border-color: var(--accent);
	}

	.apply-btn {
		margin-top: 6px;
		background: var(--accent);
		border: none;
		color: var(--bg-primary);
		font-size: 12px;
		font-family: var(--font-sans);
		padding: 4px 12px;
		border-radius: var(--radius);
		cursor: pointer;
		width: 100%;
	}

	.apply-btn:hover {
		opacity: 0.9;
	}

	.apply-btn:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.perms-error {
		display: block;
		margin-top: 4px;
		font-size: 11px;
		color: var(--danger);
	}

</style>
