<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileEntry, FilePreview, PdfPreview } from "$lib/commands";
import { isPdfPreviewable } from "$lib/constants";
import { getIconForEntry } from "$lib/icons";
import { overlayFade } from "$lib/transitions";
import { formatSize } from "$lib/utils";
import FileIcon from "./FileIcon.svelte";

let {
	entry,
	previewData,
	pdfPreview,
	previewLoading,
	previewError,
	highlightedHtml = null,
	imagePreviewUrl = null,
	dirPreviewEntries = null,
	width = 300,
	onresize,
}: {
	entry: FileEntry | null;
	previewData: FilePreview | null;
	pdfPreview: PdfPreview | null;
	previewLoading: boolean;
	previewError: string | null;
	highlightedHtml?: string | null;
	imagePreviewUrl?: string | null;
	dirPreviewEntries?: FileEntry[] | null;
	width?: number;
	onresize?: (width: number) => void;
} = $props();

const icon = $derived(entry ? getIconForEntry(entry) : "");
const isPdf = $derived(
	entry && !entry.is_dir ? isPdfPreviewable(entry.mime_type) : false,
);
const pdfImageUrl = $derived(
	isPdf && pdfPreview ? convertFileSrc(pdfPreview.image_path) : null,
);
const hlines = $derived(highlightedHtml ? highlightedHtml.split("\n") : []);

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

<div class="preview-panel" style:width="{width}px" transition:overlayFade>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="resize-handle" class:active={dragging} {onpointerdown}></div>
	<div class="preview-content">
		{#if !entry}
			<div class="preview-center">
				<span class="preview-empty-icon">{"\uf15b"}</span>
				<span class="preview-detail">No file selected</span>
			</div>
		{:else if previewLoading}
			<div class="preview-center">
				<FileIcon src={icon} size={48} />
				<span class="preview-detail">Loading...</span>
			</div>
		{:else if previewError}
			<div class="preview-center">
				<FileIcon src={icon} size={48} />
				<span class="preview-detail preview-error">{previewError}</span>
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
			</div>
		{:else if imagePreviewUrl}
			<div class="preview-visual">
				<img src={imagePreviewUrl} alt={entry.name} decoding="async" />
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{formatSize(entry.size)}</span>
			</div>
		{:else if pdfImageUrl}
			<div class="preview-visual">
				<img src={pdfImageUrl} alt={entry.name} decoding="async" />
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{pdfPreview?.page_count} page{pdfPreview?.page_count !== 1 ? 's' : ''}</span>
			</div>
		{:else if previewData && !previewData.is_binary && hlines.length > 0}
			<div class="preview-text">
				<div class="code-wrap">
					<pre class="gutter">{#each hlines as _, i}{i + 1}
{/each}</pre>
					<pre class="code-content"><code>{#each hlines as line, i}{@html line}
{/each}</code></pre>
				</div>
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{formatSize(entry.size)}{#if previewData.truncated} (truncated){/if}</span>
			</div>
		{:else if dirPreviewEntries}
			<div class="preview-dir">
				{#each dirPreviewEntries as child (child.path)}
					<div class="dir-entry">
						<FileIcon src={getIconForEntry(child)} size={16} />
						<span class="dir-entry-name" class:dir={child.is_dir}>{child.name}</span>
					</div>
				{/each}
				{#if dirPreviewEntries.length === 0}
					<div class="preview-center">
						<span class="preview-detail">Empty directory</span>
					</div>
				{/if}
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{dirPreviewEntries.length} items</span>
			</div>
		{:else}
			<div class="preview-center">
				<FileIcon src={icon} size={48} />
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{entry.mime_type} · {formatSize(entry.size)}</span>
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

	.preview-center {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		color: var(--text-muted);
		min-height: 0;
	}

	.preview-empty-icon {
		font-family: var(--font-icon);
		font-size: 48px;
		opacity: 0.3;
	}

	.preview-error {
		color: var(--danger);
	}

	.preview-visual {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		min-height: 0;
	}

	.preview-visual img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		padding: 8px;
	}

	.preview-dir {
		flex: 1;
		overflow: auto;
		min-height: 0;
		padding: 4px 0;
	}

	.dir-entry {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 2px 12px;
		font-size: 12px;
		color: var(--text-primary);
	}

	.dir-entry-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.dir-entry-name.dir {
		color: var(--accent);
	}

	.preview-footer {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 12px;
		border-top: 1px solid var(--border);
		background: var(--bg-secondary);
		flex-shrink: 0;
	}

	.preview-filename {
		font-size: 12px;
		color: var(--text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
		min-width: 0;
	}

	.preview-detail {
		font-size: 11px;
		color: var(--text-muted);
		flex-shrink: 0;
	}

	.preview-text {
		flex: 1;
		overflow: hidden;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	.code-wrap {
		flex: 1;
		display: flex;
		overflow: auto;
		min-height: 0;
	}

	.code-wrap::-webkit-scrollbar-corner {
		background: var(--bg-primary);
	}

	.code-wrap pre {
		margin: 0;
		font-size: 12px;
		line-height: 1.5;
		white-space: pre;
	}

	.gutter {
		position: sticky;
		left: 0;
		flex-shrink: 0;
		text-align: right;
		padding: 8px 3px;
		color: color-mix(in srgb, var(--text-muted) 50%, transparent);
		user-select: none;
		border-right: 1px solid var(--border);
		background: var(--bg-primary);
		z-index: 1;
	}

	.code-content {
		padding: 8px 12px 8px 0;
	}

	.code-content code {
		display: block;
		margin-left: 4px;
	}

</style>
