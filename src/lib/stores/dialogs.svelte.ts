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

export function createDialogManager(getFm: () => FileManager) {
	// Properties dialog
	let propertiesData = $state<FileProperties | null>(null);

	// Folder picker
	let folderPicker = $state<{
		mode: "move" | "copy" | "extract";
		entries: FileEntry[];
	} | null>(null);

	// Compress dialog
	let compressEntries = $state<FileEntry[]>([]);

	// Context menu
	let contextMenu = $state<{
		x: number;
		y: number;
		entry: FileEntry | null;
	} | null>(null);

	// Confirm dialog
	let confirmDialog = $state<{
		title: string;
		message: string;
		confirmLabel: string;
		danger: boolean;
		onconfirm: () => void;
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

	function openCompress(entries: FileEntry[]) {
		compressEntries = entries;
	}

	function closeCompress() {
		compressEntries = [];
	}

	function openContextMenu(x: number, y: number, entry: FileEntry | null) {
		contextMenu = { x, y, entry };
	}

	function closeContextMenu() {
		contextMenu = null;
		getFm().openWithApps = [];
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
				getFm().setError(
					msg ?? `Failed to ${label.toLowerCase().replace("\u2026", "")}`,
				);
			}
		}
		busyMessage = null;
		busyProgress = null;
		await getFm().refresh();
	}

	async function handleExtract() {
		const fm = getFm();
		if (!fm.cursorEntry) return;
		const entry = fm.cursorEntry;
		const dest = parentPath(entry.path);
		// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
		await runBusyOperation("Extracting\u2026", () => extract(entry.path, dest));
	}

	function handleExtractTo() {
		const fm = getFm();
		if (!fm.cursorEntry) return;
		openFolderPicker("extract", [fm.cursorEntry]);
	}

	function handleCompress() {
		const entries = getFm().effectiveSelection;
		if (entries.length === 0) return;
		openCompress(entries);
	}

	async function handleCompressConfirm(archiveName: string) {
		if (compressEntries.length === 0) return;
		const paths = compressEntries.map((e) => e.path);
		closeCompress();
		const currentPath = getFm().currentPath;
		const dest =
			currentPath === "/" ? `/${archiveName}` : `${currentPath}/${archiveName}`;
		// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
		await runBusyOperation("Compressing\u2026", () => compress(paths, dest));
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

		await ops.handleFolderPickerSelect(getFm(), fp, destDir);
	}

	function confirm(opts: {
		title: string;
		message: string;
		confirmLabel: string;
		danger?: boolean;
		onconfirm: () => void;
	}) {
		confirmDialog = {
			title: opts.title,
			message: opts.message,
			confirmLabel: opts.confirmLabel,
			danger: opts.danger ?? false,
			onconfirm: opts.onconfirm,
		};
	}

	function closeConfirm() {
		confirmDialog = null;
	}

	function handleDelete() {
		const entries = getFm().effectiveSelection;
		if (entries.length === 0) return;
		const label =
			entries.length === 1 ? entries[0].name : `${entries.length} items`;
		confirm({
			title: "Move to Trash",
			message: `Move ${label} to trash?`,
			confirmLabel: "Move to Trash",
			danger: true,
			onconfirm: async () => {
				closeConfirm();
				await ops.handleDelete(getFm());
			},
		});
	}

	function handlePermanentDelete() {
		const entries = getFm().effectiveSelection;
		if (entries.length === 0) return;
		const label =
			entries.length === 1 ? entries[0].name : `${entries.length} items`;
		confirm({
			title: "Permanently Delete",
			message: `Permanently delete ${label}? This cannot be undone.`,
			confirmLabel: "Delete Forever",
			danger: true,
			onconfirm: async () => {
				closeConfirm();
				await ops.handlePermanentDelete(getFm());
			},
		});
	}

	function handleEmptyTrash() {
		confirm({
			title: "Empty Trash",
			message: "Permanently delete all items in trash? This cannot be undone.",
			confirmLabel: "Empty Trash",
			danger: true,
			onconfirm: async () => {
				closeConfirm();
				await ops.handleEmptyTrash(getFm());
			},
		});
	}

	async function handleProperties() {
		const fm = getFm();
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
		get compressEntries() {
			return compressEntries;
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
		get confirmDialog() {
			return confirmDialog;
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
		confirm,
		closeConfirm,

		// Orchestration
		handleDelete,
		handlePermanentDelete,
		handleEmptyTrash,
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
