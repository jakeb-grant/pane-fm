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
const MAX_PREVIEW_LINES = 200;
const LINE_HEIGHT = 18; // 12px font * 1.5 line-height
const OVERSCAN = 5;

const splitLines = $derived.by(() => {
	if (!highlightedHtml) return { lines: [] as string[], capped: false };
	const lines = highlightedHtml.split("\n");
	const capped = lines.length > MAX_PREVIEW_LINES;
	return { lines: capped ? lines.slice(0, MAX_PREVIEW_LINES) : lines, capped };
});

let codeWrapEl = $state<HTMLDivElement>();
let scrollTop = $state(0);
let viewHeight = $state(300);

function onCodeScroll() {
	if (!codeWrapEl) return;
	scrollTop = codeWrapEl.scrollTop;
	viewHeight = codeWrapEl.clientHeight;
}

// Reset scroll when preview changes
$effect(() => {
	splitLines.lines;
	if (codeWrapEl) {
		codeWrapEl.scrollTop = 0;
		scrollTop = 0;
		viewHeight = codeWrapEl.clientHeight || 300;
	}
});

const visibleRange = $derived.by(() => {
	const total = splitLines.lines.length;
	if (total === 0) return { start: 0, end: 0, total: 0 };
	const start = Math.max(0, Math.floor(scrollTop / LINE_HEIGHT) - OVERSCAN);
	const end = Math.min(
		total,
		Math.ceil((scrollTop + viewHeight) / LINE_HEIGHT) + OVERSCAN,
	);
	return { start, end, total };
});

let gutterEl = $state<HTMLPreElement>();
let codeEl = $state<HTMLElement>();

$effect(() => {
	const { start, end } = visibleRange;
	const lines = splitLines.lines;
	if (!lines.length) {
		if (codeEl) codeEl.innerHTML = "";
		if (gutterEl) gutterEl.textContent = "";
		return;
	}
	const slice = lines.slice(start, end);
	if (codeEl) codeEl.innerHTML = slice.join("\n");
	if (gutterEl) {
		const nums: string[] = [];
		for (let i = start; i < end; i++) nums.push(String(i + 1));
		gutterEl.textContent = nums.join("\n");
	}
});

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
		{:else if previewData && !previewData.is_binary && splitLines.lines.length > 0}
			<div class="preview-text">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="code-wrap" bind:this={codeWrapEl} onscroll={onCodeScroll}>
					<div class="code-virtual" style:height="{splitLines.lines.length * LINE_HEIGHT + 16}px">
						<div class="code-visible" style:transform="translateY({visibleRange.start * LINE_HEIGHT + 8}px)">
							<pre class="gutter" bind:this={gutterEl}></pre>
							<pre class="code-content"><code bind:this={codeEl}></code></pre>
						</div>
					</div>
				</div>
			</div>
			<div class="preview-footer">
				<FileIcon src={icon} size={16} />
				<span class="preview-filename">{entry.name}</span>
				<span class="preview-detail">{formatSize(entry.size)}{#if previewData.truncated || splitLines.capped} (truncated){/if}</span>
			</div>
		{:else if dirPreviewEntries}
			{@const maxDirEntries = 100}
			{@const visibleDirEntries = dirPreviewEntries.length > maxDirEntries ? dirPreviewEntries.slice(0, maxDirEntries) : dirPreviewEntries}
			<div class="preview-dir">
				{#each visibleDirEntries as child (child.path)}
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
		overflow: auto;
		min-height: 0;
	}

	.code-wrap::-webkit-scrollbar-corner {
		background: var(--bg-primary);
	}

	.code-virtual {
		position: relative;
	}

	.code-visible {
		display: flex;
		will-change: transform;
	}

	.code-visible pre {
		margin: 0;
		font-size: 12px;
		line-height: 18px;
		white-space: pre;
	}

	.gutter {
		position: sticky;
		left: 0;
		flex-shrink: 0;
		text-align: right;
		padding: 0 3px;
		color: color-mix(in srgb, var(--text-muted) 50%, transparent);
		user-select: none;
		border-right: 1px solid var(--border);
		background: var(--bg-primary);
		z-index: 1;
	}

	.code-content {
		padding: 0 12px 0 0;
	}

	.code-content code {
		display: block;
		margin-left: 4px;
	}

</style>
