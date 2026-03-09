import {
	copyEntry,
	createDirectory,
	createFile,
	deleteEntry,
	emptyTrash,
	type FileEntry,
	getProperties,
	listAppsForMime,
	moveEntry,
	openDefault,
	openWithApp,
	renameEntry,
	restoreTrash,
} from "$lib/commands";
import { errorMessage } from "$lib/errors";
import type { FileManager } from "$lib/stores/fileManager.svelte";
import { parentPath } from "$lib/utils";

export async function handleOpen(fm: FileManager, entry: FileEntry) {
	if (entry.is_dir) {
		fm.navigate(entry.path);
		return;
	}
	try {
		await openDefault(entry.path);
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to open file");
	}
}

export async function handleOpenWith(
	fm: FileManager,
	entry: FileEntry,
	position: { x: number; y: number },
	setContextMenu: (menu: {
		x: number;
		y: number;
		entry: FileEntry | null;
	}) => void,
) {
	try {
		const apps = await listAppsForMime(entry.mime_type);
		if (apps.length === 0) {
			fm.setError("No applications found for this file type");
			return;
		}
		fm.openWithApps = apps;
		setContextMenu({ x: position.x, y: position.y, entry });
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to list applications");
	}
}

export async function handleDelete(fm: FileManager) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	try {
		for (const entry of entries) {
			await deleteEntry(entry.path);
		}
		await fm.refresh();
	} catch (e) {
		const label =
			entries.length === 1 ? entries[0].name : `${entries.length} items`;
		fm.setError(errorMessage(e) ?? `Failed to delete ${label}`);
	}
}

export function handleCopy(fm: FileManager) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	fm.clipboard = { entries, mode: "copy" };
	fm.clearMultiSelection();
}

export function handleCut(fm: FileManager) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	fm.clipboard = { entries, mode: "cut" };
	fm.clearMultiSelection();
}

export async function handlePaste(fm: FileManager) {
	if (!fm.clipboard) return;
	const isCut = fm.clipboard.mode === "cut";

	try {
		for (const src of fm.clipboard.entries) {
			const destPath =
				fm.currentPath === "/"
					? `/${src.name}`
					: `${fm.currentPath}/${src.name}`;
			if (isCut) {
				await moveEntry(src.path, destPath);
			} else {
				await copyEntry(src.path, destPath);
			}
		}
		fm.clipboard = null;
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? `Failed to ${isCut ? "move" : "paste"}`);
	}
}

export function handleRename(fm: FileManager) {
	if (!fm.cursorEntry) return;
	fm.renamingPath = fm.cursorEntry.path;
}

export async function commitRename(
	fm: FileManager,
	entry: FileEntry,
	newName: string,
) {
	fm.renamingPath = null;
	if (!newName || newName === entry.name) return;

	const parent = parentPath(entry.path);
	const newPath = parent === "/" ? `/${newName}` : `${parent}/${newName}`;

	try {
		await renameEntry(entry.path, newPath);
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to rename");
	}
}

export function handleNewFolder(fm: FileManager) {
	fm.creatingEntry = "directory";
}

export function handleNewFile(fm: FileManager) {
	fm.creatingEntry = "file";
}

export async function commitCreate(fm: FileManager, name: string) {
	const type = fm.creatingEntry;
	fm.creatingEntry = null;
	if (!name || !type) return;

	const path =
		fm.currentPath === "/" ? `/${name}` : `${fm.currentPath}/${name}`;
	try {
		if (type === "directory") {
			await createDirectory(path);
		} else {
			await createFile(path);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(
			errorMessage(e) ??
				`Failed to create ${type === "directory" ? "folder" : "file"}`,
		);
	}
}

export function handleMoveTo(
	fm: FileManager,
	setFolderPicker: (v: {
		mode: "move" | "copy" | "extract";
		entries: FileEntry[];
	}) => void,
) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	setFolderPicker({ mode: "move", entries });
}

export function handleCopyTo(
	fm: FileManager,
	setFolderPicker: (v: {
		mode: "move" | "copy" | "extract";
		entries: FileEntry[];
	}) => void,
) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	setFolderPicker({ mode: "copy", entries });
}

export async function handleFolderPickerSelect(
	fm: FileManager,
	folderPicker: { mode: "move" | "copy" | "extract"; entries: FileEntry[] },
	destDir: string,
) {
	const mode = folderPicker.mode;

	// Extract mode is handled by dialogs.svelte.ts — this only handles move/copy
	try {
		for (const src of folderPicker.entries) {
			const dest = destDir === "/" ? `/${src.name}` : `${destDir}/${src.name}`;
			if (mode === "move") {
				await moveEntry(src.path, dest);
			} else {
				await copyEntry(src.path, dest);
			}
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? `Failed to ${mode}`);
	}
}

export async function handleRestore(fm: FileManager) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	try {
		for (const entry of entries) {
			await restoreTrash(entry.name);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to restore");
	}
}

export async function handleEmptyTrash(fm: FileManager) {
	try {
		await emptyTrash();
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to empty trash");
	}
}

export async function handleProperties(
	fm: FileManager,
	setPropertiesData: (
		v: ReturnType<typeof getProperties> extends Promise<infer T> ? T : never,
	) => void,
) {
	if (!fm.cursorEntry) return;
	try {
		const data = await getProperties(fm.cursorEntry.path);
		setPropertiesData(data);
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to get properties");
	}
}

export function handleContextMenu(
	fm: FileManager,
	e: MouseEvent,
	entry: FileEntry,
	setContextMenu: (menu: {
		x: number;
		y: number;
		entry: FileEntry | null;
	}) => void,
) {
	e.stopPropagation();
	if (!fm.selectedPaths.has(entry.path)) {
		fm.select(entry);
	}
	fm.openWithApps = [];
	setContextMenu({ x: e.clientX, y: e.clientY, entry });
}

export function handleBgContextMenu(
	fm: FileManager,
	e: MouseEvent,
	setContextMenu: (menu: {
		x: number;
		y: number;
		entry: FileEntry | null;
	}) => void,
) {
	e.preventDefault();
	fm.clearSelection();
	setContextMenu({ x: e.clientX, y: e.clientY, entry: null });
}

export function launchOpenWithApp(
	fm: FileManager,
	filePath: string,
	desktopId: string,
) {
	openWithApp(filePath, desktopId);
	fm.openWithApps = [];
}
