import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
	cancelOperation,
	compress,
	extract,
	type FileEntry,
	type FileProperties,
} from "$lib/commands";
import { errorMessage, isCancelled } from "$lib/errors";
import * as ops from "$lib/fileOps";
import type { FileManager } from "$lib/stores/fileManager.svelte";
import { parentPath } from "$lib/utils";

export function createDialogManager(fm: FileManager) {
	// Properties dialog
	let propertiesData = $state<FileProperties | null>(null);

	// Folder picker
	let folderPicker = $state<{
		mode: "move" | "copy" | "extract";
		entries: FileEntry[];
	} | null>(null);

	// Compress dialog
	let compressEntry = $state<FileEntry | null>(null);

	// Context menu
	let contextMenu = $state<{
		x: number;
		y: number;
		entry: FileEntry | null;
	} | null>(null);

	// Busy overlay
	let busyMessage = $state<string | null>(null);
	let busyProgress = $state<{ processed: number; total: number } | null>(null);

	// Progress event listener
	let progressUnlisten: UnlistenFn | null = null;

	async function subscribeProgress() {
		progressUnlisten = await listen<{ processed: number; total: number }>(
			"compress-progress",
			(event) => {
				busyProgress = event.payload;
			},
		);
	}

	function unsubscribeProgress() {
		progressUnlisten?.();
		progressUnlisten = null;
	}

	// --- Dialog actions ---

	function openProperties(data: FileProperties) {
		propertiesData = data;
	}

	function closeProperties() {
		propertiesData = null;
	}

	function openFolderPicker(
		mode: "move" | "copy" | "extract",
		entries: FileEntry[],
	) {
		folderPicker = { mode, entries };
	}

	function closeFolderPicker() {
		folderPicker = null;
	}

	function openCompress(entry: FileEntry) {
		compressEntry = entry;
	}

	function closeCompress() {
		compressEntry = null;
	}

	function openContextMenu(x: number, y: number, entry: FileEntry | null) {
		contextMenu = { x, y, entry };
	}

	function closeContextMenu() {
		contextMenu = null;
		fm.openWithApps = [];
	}

	// --- Busy operation orchestration ---

	async function runBusyOperation(
		label: string,
		operation: () => Promise<void>,
	) {
		if (busyMessage) return;
		busyMessage = label;
		busyProgress = null;
		try {
			await operation();
		} catch (e) {
			if (!isCancelled(e)) {
				const msg = errorMessage(e);
				fm.setError(
					msg ?? `Failed to ${label.toLowerCase().replace("\u2026", "")}`,
				);
			}
		}
		busyMessage = null;
		busyProgress = null;
		await fm.refresh();
	}

	async function handleExtract() {
		if (!fm.cursorEntry) return;
		const entry = fm.cursorEntry;
		const dest = parentPath(entry.path);
		// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
		await runBusyOperation("Extracting\u2026", () => extract(entry.path, dest));
	}

	function handleExtractTo() {
		if (!fm.cursorEntry) return;
		openFolderPicker("extract", [fm.cursorEntry]);
	}

	function handleCompress() {
		if (!fm.cursorEntry) return;
		openCompress(fm.cursorEntry);
	}

	async function handleCompressConfirm(archiveName: string) {
		if (!compressEntry) return;
		const entry = compressEntry;
		closeCompress();
		const dest =
			fm.currentPath === "/"
				? `/${archiveName}`
				: `${fm.currentPath}/${archiveName}`;
		// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
		await runBusyOperation("Compressing\u2026", () =>
			compress([entry.path], dest),
		);
	}

	async function handleCancelOperation() {
		await cancelOperation();
	}

	async function handleFolderPickerSelect(destDir: string) {
		if (!folderPicker) return;
		const fp = folderPicker;
		closeFolderPicker();

		if (fp.mode === "extract") {
			// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
			await runBusyOperation("Extracting\u2026", () =>
				extract(fp.entries[0].path, destDir),
			);
			return;
		}

		await ops.handleFolderPickerSelect(fm, fp, destDir);
	}

	async function handleProperties() {
		if (!fm.cursorEntry) return;
		await ops.handleProperties(fm, openProperties);
	}

	return {
		// State getters
		get propertiesData() {
			return propertiesData;
		},
		get folderPicker() {
			return folderPicker;
		},
		get compressEntry() {
			return compressEntry;
		},
		get contextMenu() {
			return contextMenu;
		},
		get busyMessage() {
			return busyMessage;
		},
		get busyProgress() {
			return busyProgress;
		},

		// Dialog open/close
		openProperties,
		closeProperties,
		openFolderPicker,
		closeFolderPicker,
		openCompress,
		closeCompress,
		openContextMenu,
		closeContextMenu,

		// Orchestration
		handleExtract,
		handleExtractTo,
		handleCompress,
		handleCompressConfirm,
		handleCancelOperation,
		handleFolderPickerSelect,
		handleProperties,

		// Progress lifecycle
		subscribeProgress,
		unsubscribeProgress,
	};
}

export type DialogManager = ReturnType<typeof createDialogManager>;
