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
		fm.setError(`Failed to open: ${String(e)}`);
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
		fm.setError(`Failed to list applications: ${String(e)}`);
	}
}

export async function handleDelete(fm: FileManager) {
	if (!fm.selectedEntry) return;
	const name = fm.selectedEntry.name;
	try {
		await deleteEntry(fm.selectedEntry.path);
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to delete ${name}: ${e}`);
	}
}

export function handleCopy(fm: FileManager) {
	if (!fm.selectedEntry) return;
	fm.clipboard = { entries: [fm.selectedEntry], mode: "copy" };
}

export function handleCut(fm: FileManager) {
	if (!fm.selectedEntry) return;
	fm.clipboard = { entries: [fm.selectedEntry], mode: "cut" };
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
		if (isCut) fm.clipboard = null;
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to ${isCut ? "move" : "paste"}: ${e}`);
	}
}

export function handleRename(fm: FileManager) {
	if (!fm.selectedEntry) return;
	fm.renamingPath = fm.selectedEntry.path;
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
		fm.setError(`Failed to rename: ${e}`);
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
			`Failed to create ${type === "directory" ? "folder" : "file"}: ${e}`,
		);
	}
}

export function handleMoveTo(
	fm: FileManager,
	setFolderPicker: (v: {
		mode: "move" | "copy" | "extract";
		entry: FileEntry;
	}) => void,
) {
	if (!fm.selectedEntry) return;
	setFolderPicker({ mode: "move", entry: fm.selectedEntry });
}

export function handleCopyTo(
	fm: FileManager,
	setFolderPicker: (v: {
		mode: "move" | "copy" | "extract";
		entry: FileEntry;
	}) => void,
) {
	if (!fm.selectedEntry) return;
	setFolderPicker({ mode: "copy", entry: fm.selectedEntry });
}

export async function handleFolderPickerSelect(
	fm: FileManager,
	folderPicker: { mode: "move" | "copy" | "extract"; entry: FileEntry },
	destDir: string,
) {
	const src = folderPicker.entry;
	const mode = folderPicker.mode;

	// Extract mode is handled by 1.3 (dialog/busy state) — this only handles move/copy
	const dest = destDir === "/" ? `/${src.name}` : `${destDir}/${src.name}`;
	try {
		if (mode === "move") {
			await moveEntry(src.path, dest);
		} else {
			await copyEntry(src.path, dest);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to ${mode}: ${String(e)}`);
	}
}

export async function handleRestore(fm: FileManager) {
	if (!fm.selectedEntry) return;
	try {
		await restoreTrash(fm.selectedEntry.name);
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to restore: ${e}`);
	}
}

export async function handleEmptyTrash(fm: FileManager) {
	try {
		await emptyTrash();
		await fm.refresh();
	} catch (e) {
		fm.setError(`Failed to empty trash: ${e}`);
	}
}

export async function handleProperties(
	fm: FileManager,
	setPropertiesData: (
		v: ReturnType<typeof getProperties> extends Promise<infer T> ? T : never,
	) => void,
) {
	if (!fm.selectedEntry) return;
	try {
		const data = await getProperties(fm.selectedEntry.path);
		setPropertiesData(data);
	} catch (e) {
		fm.setError(`Failed to get properties: ${String(e)}`);
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
	fm.select(entry);
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
