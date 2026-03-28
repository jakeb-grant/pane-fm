import {
	createDirectory,
	createFile,
	createSymlink,
	deleteEntriesPermanently,
	deleteEntry,
	emptyTrash,
	type FileEntry,
	getProperties,
	listAppsForMime,
	openDefault,
	openTerminal,
	openWithApp,
	openWithEditor,
	pasteEntries,
	renameEntry,
	restoreTrash,
} from "$lib/commands";
import { isTextPreviewable } from "$lib/constants";
import { errorMessage } from "$lib/errors";
import type { FileManager } from "$lib/stores/fileManager.svelte";
import { parentPath } from "$lib/utils";

export async function handleOpen(
	fm: FileManager,
	entry: FileEntry,
	editor?: string | null,
) {
	if (entry.is_dir) {
		fm.navigate(entry.path);
		return;
	}
	try {
		if (isTextPreviewable(entry.mime_type, entry.name)) {
			await openWithEditor(entry.path, editor);
		} else {
			await openDefault(entry.path);
		}
	} catch (e) {
		// Fall back to xdg-open if $EDITOR fails or isn't set
		if (isTextPreviewable(entry.mime_type, entry.name)) {
			try {
				await openDefault(entry.path);
				return;
			} catch {
				// Both failed — show original error
			}
		}
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

export async function handlePermanentDelete(fm: FileManager) {
	const entries = fm.effectiveSelection;
	if (entries.length === 0) return;
	await deleteEntriesPermanently(entries.map((e) => e.path));
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
	const paths = fm.clipboard.entries.map((e) => e.path);
	const mode = fm.clipboard.mode;
	await pasteEntries(paths, fm.currentPath, mode);
	fm.clipboard = null;
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
	folderPicker: { mode: "move" | "copy" | "extract"; entries: FileEntry[] },
	destDir: string,
) {
	const paths = folderPicker.entries.map((e) => e.path);
	const mode = folderPicker.mode === "move" ? "cut" : "copy";
	await pasteEntries(paths, destDir, mode);
}

export async function handleDrop(
	sourcePaths: string[],
	destDir: string,
	mode: "move" | "copy",
) {
	const filtered = sourcePaths.filter((src) => {
		const name = src.split("/").pop() ?? "";
		const dest = destDir === "/" ? `/${name}` : `${destDir}/${name}`;
		return src !== dest;
	});
	if (filtered.length === 0) return;
	await pasteEntries(filtered, destDir, mode === "move" ? "cut" : "copy");
}

export async function handleDropToTrash(fm: FileManager, paths: string[]) {
	try {
		for (const p of paths) {
			await deleteEntry(p);
		}
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to trash");
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

export async function handleEmptyTrash() {
	await emptyTrash();
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

export async function handleCreateSymlink(fm: FileManager) {
	const entry = fm.cursorEntry;
	if (!entry) return;
	const linkName = `${entry.name} (link)`;
	const linkPath =
		fm.currentPath === "/" ? `/${linkName}` : `${fm.currentPath}/${linkName}`;
	try {
		await createSymlink(entry.path, linkPath);
		await fm.refresh();
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to create symlink");
	}
}

export async function handleOpenInEditor(
	fm: FileManager,
	path: string,
	editor?: string | null,
) {
	try {
		await openWithEditor(path, editor);
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to open in editor");
	}
}

export async function handleOpenTerminal(fm: FileManager, terminal: string) {
	try {
		await openTerminal(fm.currentPath, terminal);
	} catch (e) {
		fm.setError(errorMessage(e) ?? "Failed to open terminal");
	}
}
