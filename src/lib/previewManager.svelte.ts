import { convertFileSrc } from "@tauri-apps/api/core";
import type { FileEntry, FilePreview, PdfPreview } from "$lib/commands";
import {
	generateThumbnail,
	listDirectory,
	readFilePreview,
	readPdfPreview,
	setPreviewPath,
} from "$lib/commands";
import {
	isImagePreviewable,
	isPdfPreviewable,
	isTextPreviewable,
} from "$lib/constants";
import { errorMessage } from "$lib/errors";
import type { HighlightResponse } from "$lib/highlight";
import { type CachedPreview, previewCache } from "$lib/previewCache";
import type { FileManager } from "$lib/stores/fileManager.svelte";

const MAX_HIGHLIGHT_LINES = 200;

function truncateForHighlight(code: string): string {
	let pos = 0;
	for (let i = 0; i < MAX_HIGHLIGHT_LINES && pos < code.length; i++) {
		const nl = code.indexOf("\n", pos);
		if (nl === -1) return code;
		pos = nl + 1;
	}
	return code.slice(0, pos);
}

// biome-ignore lint/suspicious/noEmptyBlockStatements: prefetch failures are intentionally ignored
const noop = () => {};

export function createPreviewManager(getFm: () => FileManager) {
	let previewData = $state<FilePreview | null>(null);
	let pdfPreview = $state<PdfPreview | null>(null);
	let previewLoading = $state(false);
	let previewError = $state<string | null>(null);
	let highlightedHtml = $state<string | null>(null);
	let imagePreviewUrl = $state<string | null>(null);
	let dirPreviewEntries = $state<FileEntry[] | null>(null);

	let previewTimer: ReturnType<typeof setTimeout> | undefined;
	let previewPath = "";
	let activePreviewEntry: FileEntry | null = null;

	const pendingPrefetch = new Map<
		string,
		{ path: string; mtime: string; data: FilePreview }
	>();

	const hlWorker = new Worker(
		new URL("$lib/highlightWorker.ts", import.meta.url),
		{ type: "module" },
	);
	hlWorker.onmessage = (e: MessageEvent<HighlightResponse>) => {
		const { html, path } = e.data;

		if (path === previewPath) {
			highlightedHtml = html;
			previewLoading = false;
			if (activePreviewEntry && previewData) {
				previewCache.set(activePreviewEntry.path, activePreviewEntry.modified, {
					type: "text",
					data: previewData,
					html,
				});
			}
		} else {
			const pending = pendingPrefetch.get(path);
			if (pending) {
				previewCache.set(pending.path, pending.mtime, {
					type: "text",
					data: pending.data,
					html,
				});
				pendingPrefetch.delete(path);
			}
		}
	};

	function clearPreviewState() {
		previewData = null;
		pdfPreview = null;
		highlightedHtml = null;
		imagePreviewUrl = null;
		dirPreviewEntries = null;
		previewError = null;
	}

	function applyCachedPreview(cached: CachedPreview) {
		clearPreviewState();
		previewLoading = false;
		switch (cached.type) {
			case "text":
				previewData = cached.data;
				highlightedHtml = cached.html;
				break;
			case "dir":
				dirPreviewEntries = cached.entries;
				break;
			case "image":
				imagePreviewUrl = cached.url;
				break;
			case "pdf":
				pdfPreview = cached.data;
				break;
			case "none":
				break;
		}
	}

	function prefetchAdjacent(current: FileEntry, currentPath: string) {
		const list = getFm().filteredEntries;
		const idx = list.findIndex((e) => e.path === current.path);
		if (idx < 0) return;

		for (const adj of [list[idx - 1], list[idx + 1]]) {
			if (!adj) continue;
			if (previewCache.get(adj.path, adj.modified)) continue;

			if (adj.is_dir) {
				listDirectory(adj.path, getFm().showHidden)
					.then((entries) =>
						previewCache.set(adj.path, adj.modified, {
							type: "dir",
							entries,
						}),
					)
					.catch(noop);
			} else if (isTextPreviewable(adj.mime_type, adj.name)) {
				readFilePreview(adj.path)
					.then((data) => {
						if (data.is_binary || !data.content) {
							previewCache.set(adj.path, adj.modified, { type: "none" });
							return;
						}
						if (pendingPrefetch.size > 20) pendingPrefetch.clear();
						pendingPrefetch.set(adj.path, {
							path: adj.path,
							mtime: adj.modified,
							data,
						});
						hlWorker.postMessage({
							code: truncateForHighlight(data.content),
							filename: adj.name,
							path: adj.path,
						});
					})
					.catch(noop);
			} else if (isImagePreviewable(adj.mime_type)) {
				if (adj.mime_type === "image/svg+xml") {
					const url = convertFileSrc(adj.path);
					previewCache.set(adj.path, adj.modified, { type: "image", url });
				} else {
					generateThumbnail(adj.path, undefined, currentPath)
						.then((thumb) => {
							const url = convertFileSrc(thumb.image_path);
							previewCache.set(adj.path, adj.modified, {
								type: "image",
								url,
							});
						})
						.catch(noop);
				}
			} else if (isPdfPreviewable(adj.mime_type)) {
				readPdfPreview(adj.path, currentPath)
					.then((data) =>
						previewCache.set(adj.path, adj.modified, { type: "pdf", data }),
					)
					.catch(noop);
			}
		}
	}

	async function loadPreview(entry: FileEntry, forPath: string) {
		if (forPath !== previewPath) return;
		const { mime_type: mime, path, name } = entry;

		if (entry.is_dir) {
			try {
				const entries = await listDirectory(path, getFm().showHidden);
				if (forPath !== previewPath) return;
				dirPreviewEntries = entries;
				previewCache.set(path, entry.modified, { type: "dir", entries });
			} catch {
				if (forPath !== previewPath) return;
			}
		} else if (isTextPreviewable(mime, name)) {
			try {
				const data = await readFilePreview(path);
				if (forPath !== previewPath) return;
				previewData = data;
				if (data.is_binary || !data.content) {
					previewCache.set(path, entry.modified, { type: "none" });
				} else {
					activePreviewEntry = entry;
					hlWorker.postMessage({
						code: truncateForHighlight(data.content),
						filename: name,
						path,
					});
					return;
				}
			} catch (e) {
				if (forPath !== previewPath) return;
				previewError = errorMessage(e) ?? "Failed to load preview";
			}
		} else if (isImagePreviewable(mime)) {
			let url: string;
			if (mime === "image/svg+xml") {
				if (forPath !== previewPath) return;
				url = convertFileSrc(path);
			} else {
				try {
					const thumb = await generateThumbnail(path, undefined, forPath);
					if (forPath !== previewPath) return;
					url = convertFileSrc(thumb.image_path);
				} catch {
					if (forPath !== previewPath) return;
					url = convertFileSrc(path);
				}
			}
			imagePreviewUrl = url;
			previewCache.set(path, entry.modified, { type: "image", url });
		} else if (isPdfPreviewable(mime)) {
			try {
				const data = await readPdfPreview(path, forPath);
				if (forPath !== previewPath) return;
				pdfPreview = data;
				previewCache.set(path, entry.modified, { type: "pdf", data });
			} catch (e) {
				if (forPath !== previewPath) return;
				previewError = errorMessage(e) ?? "Failed to load PDF preview";
			}
		}

		previewLoading = false;
	}

	function track() {
		const entry = getFm().cursorEntry;
		const enabled = getFm().previewEnabled;
		clearTimeout(previewTimer);
		previewPath = entry?.path ?? "";
		setPreviewPath(previewPath);

		if (!enabled || !entry) {
			clearPreviewState();
			previewLoading = false;
			activePreviewEntry = null;
			return;
		}

		const cached = previewCache.get(entry.path, entry.modified);
		if (cached) {
			applyCachedPreview(cached);
			activePreviewEntry = entry;
			prefetchAdjacent(entry, entry.path);
			return;
		}

		clearPreviewState();
		previewLoading = true;
		activePreviewEntry = entry;

		const forPath = entry.path;
		previewTimer = setTimeout(() => {
			loadPreview(entry, forPath).then(() => {
				if (forPath === previewPath) prefetchAdjacent(entry, forPath);
			});
		}, 250);

		return () => clearTimeout(previewTimer);
	}

	return {
		get previewData() {
			return previewData;
		},
		get pdfPreview() {
			return pdfPreview;
		},
		get previewLoading() {
			return previewLoading;
		},
		get previewError() {
			return previewError;
		},
		get highlightedHtml() {
			return highlightedHtml;
		},
		get imagePreviewUrl() {
			return imagePreviewUrl;
		},
		get dirPreviewEntries() {
			return dirPreviewEntries;
		},
		track,
		destroy() {
			clearTimeout(previewTimer);
			hlWorker.terminate();
		},
	};
}

export type PreviewManager = ReturnType<typeof createPreviewManager>;
