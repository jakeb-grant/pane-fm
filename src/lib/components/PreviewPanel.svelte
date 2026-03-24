<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileEntry, FilePreview } from "$lib/commands";
import { isImagePreviewable, isTextPreviewable } from "$lib/constants";
import { getIconForEntry } from "$lib/icons";
import { formatSize } from "$lib/utils";

let {
	entry,
	previewData,
	previewLoading,
	previewError,
	width = 300,
	onresize,
}: {
	entry: FileEntry | null;
	previewData: FilePreview | null;
	previewLoading: boolean;
	previewError: string | null;
	width?: number;
	onresize?: (width: number) => void;
} = $props();

const icon = $derived(entry ? getIconForEntry(entry) : "");
const isText = $derived(
	entry && !entry.is_dir ? isTextPreviewable(entry.mime_type) : false,
);
const isImage = $derived(
	entry && !entry.is_dir ? isImagePreviewable(entry.mime_type) : false,
);
const imageUrl = $derived(isImage && entry ? convertFileSrc(entry.path) : null);
const lines = $derived(previewData?.content.split("\n") ?? []);

let dragging = $state(false);

function onpointerdown(e: PointerEvent) {
	e.preventDefault();
	dragging = true;
	const startX = e.clientX;
	const startWidth = width;

	function onpointermove(e: PointerEvent) {
		// Handle grows leftward, so moving left = wider
		const delta = startX - e.clientX;
		const next = Math.max(150, Math.min(startWidth + delta, 800));
		onresize?.(next);
	}

	function onpointerup() {
		dragging = false;
		window.removeEventListener("pointermove", onpointermove);
		window.removeEventListener("pointerup", onpointerup);
	}

	window.addEventListener("pointermove", onpointermove);
	window.addEventListener("pointerup", onpointerup);
}
</script>

<div class="preview-panel" style:width="{width}px">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="resize-handle" class:active={dragging} {onpointerdown}></div>
	<div class="preview-content">
		{#if !entry}
			<div class="preview-empty">
				<span class="preview-empty-icon">{"\uf15b"}</span>
				<span class="preview-empty-text">No file selected</span>
			</div>
		{:else if entry.is_dir}
			<div class="preview-fallback">
				<span class="preview-icon">{icon}</span>
				<span class="preview-name">{entry.name}</span>
				{#if entry.children_count !== null}
					<span class="preview-meta">{entry.children_count} items</span>
				{/if}
			</div>
		{:else if isImage && imageUrl}
			<div class="preview-image">
				<img src={imageUrl} alt={entry.name} />
				<span class="preview-image-name">{entry.name}</span>
				<span class="preview-meta">{formatSize(entry.size)}</span>
			</div>
		{:else if isText}
			{#if previewLoading}
				<div class="preview-loading">Loading...</div>
			{:else if previewError}
				<div class="preview-fallback">
					<span class="preview-icon">{icon}</span>
					<span class="preview-name">{entry.name}</span>
					<span class="preview-error">{previewError}</span>
				</div>
			{:else if previewData?.is_binary}
				<div class="preview-fallback">
					<span class="preview-icon">{icon}</span>
					<span class="preview-name">{entry.name}</span>
					<span class="preview-meta">Binary file</span>
					<span class="preview-meta">{formatSize(entry.size)}</span>
				</div>
			{:else if previewData}
				<div class="preview-text">
					<pre><code>{#each lines as line, i}<span class="line-num">{i + 1}</span>{line}
{/each}</code></pre>
					{#if previewData.truncated}
						<div class="preview-truncated">Truncated at {formatSize(previewData.bytes_read)}</div>
					{/if}
				</div>
			{/if}
		{:else}
			<div class="preview-fallback">
				<span class="preview-icon">{icon}</span>
				<span class="preview-name">{entry.name}</span>
				<span class="preview-meta">{entry.mime_type}</span>
				<span class="preview-meta">{formatSize(entry.size)}</span>
			</div>
		{/if}
	</div>
</div>

<style>
	.preview-panel {
		min-width: 150px;
		max-width: 800px;
		flex-shrink: 0;
		border-left: 1px solid var(--border);
		background: var(--bg-primary);
		overflow: hidden;
		display: flex;
		position: relative;
	}

	.resize-handle {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 4px;
		cursor: col-resize;
		z-index: 10;
		transition: background var(--transition-normal);
	}

	.resize-handle:hover,
	.resize-handle.active {
		background: var(--accent);
	}

	.preview-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-width: 0;
	}

	.preview-empty,
	.preview-fallback,
	.preview-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		gap: 8px;
		color: var(--text-muted);
	}

	.preview-empty-icon {
		font-family: var(--font-icon);
		font-size: 48px;
		opacity: 0.3;
	}

	.preview-empty-text {
		font-size: 13px;
	}

	.preview-icon {
		font-family: var(--font-icon);
		font-size: 48px;
	}

	.preview-name {
		font-size: 13px;
		color: var(--text-primary);
		word-break: break-all;
		text-align: center;
		padding: 0 12px;
	}

	.preview-meta {
		font-size: 12px;
		color: var(--text-muted);
	}

	.preview-error {
		font-size: 12px;
		color: var(--error);
	}

	.preview-image {
		display: flex;
		flex-direction: column;
		align-items: center;
		height: 100%;
		overflow: hidden;
	}

	.preview-image img {
		flex: 1;
		min-height: 0;
		width: 100%;
		object-fit: contain;
		padding: 8px;
	}

	.preview-image-name {
		font-size: 12px;
		color: var(--text-muted);
		padding: 4px 12px 8px;
		text-align: center;
		word-break: break-all;
	}

	.preview-text {
		flex: 1;
		overflow: auto;
		min-height: 0;
	}

	.preview-text pre {
		margin: 0;
		padding: 8px 0;
		font-size: 12px;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.preview-text code {
		display: block;
	}

	.line-num {
		display: inline-block;
		width: 40px;
		text-align: right;
		padding-right: 8px;
		margin-right: 8px;
		color: var(--text-muted);
		opacity: 0.5;
		user-select: none;
		border-right: 1px solid var(--border);
	}

	.preview-truncated {
		padding: 6px 12px;
		font-size: 11px;
		color: var(--text-muted);
		background: var(--bg-secondary);
		text-align: center;
		border-top: 1px solid var(--border);
	}
</style>
