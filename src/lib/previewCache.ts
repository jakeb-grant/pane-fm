import type { FileEntry, FilePreview, PdfPreview } from "$lib/commands";

export type CachedPreview =
	| { type: "dir"; entries: FileEntry[] }
	| { type: "text"; data: FilePreview; html: string }
	| { type: "image"; url: string }
	| { type: "pdf"; data: PdfPreview }
	| { type: "none" };

interface CacheEntry {
	preview: CachedPreview;
	mtime: string;
	byteSize: number;
}

const MAX_BYTES = 5 * 1024 * 1024;

function estimateSize(preview: CachedPreview): number {
	switch (preview.type) {
		case "text":
			return preview.data.content.length + preview.html.length;
		case "dir":
			return preview.entries.length * 200;
		case "image":
			return preview.url.length;
		case "pdf":
			return 100;
		case "none":
			return 50;
	}
}

class PreviewCache {
	private map = new Map<string, CacheEntry>();
	private totalBytes = 0;

	get(path: string, mtime: string): CachedPreview | null {
		const entry = this.map.get(path);
		if (!entry || entry.mtime !== mtime) return null;
		// Move to end (most recently used)
		this.map.delete(path);
		this.map.set(path, entry);
		return entry.preview;
	}

	set(path: string, mtime: string, preview: CachedPreview): void {
		const old = this.map.get(path);
		if (old) {
			this.totalBytes -= old.byteSize;
			this.map.delete(path);
		}

		const byteSize = estimateSize(preview);

		while (this.totalBytes + byteSize > MAX_BYTES && this.map.size > 0) {
			const oldest = this.map.keys().next().value;
			if (oldest === undefined) break;
			const entry = this.map.get(oldest);
			if (entry) this.totalBytes -= entry.byteSize;
			this.map.delete(oldest);
		}

		this.map.set(path, { preview, mtime, byteSize });
		this.totalBytes += byteSize;
	}

	clear(): void {
		this.map.clear();
		this.totalBytes = 0;
	}
}

export const previewCache = new PreviewCache();
