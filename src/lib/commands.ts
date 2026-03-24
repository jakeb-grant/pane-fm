import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
	name: string;
	path: string;
	is_dir: boolean;
	is_symlink: boolean;
	size: number;
	modified: string;
	mime_type: string;
	permissions: number;
	hidden: boolean;
	children_count: number | null;
}

export function listDirectory(
	path: string,
	showHidden: boolean,
): Promise<FileEntry[]> {
	return invoke("list_directory", { path, showHidden });
}

export function getDragIcon(): Promise<string> {
	return invoke("get_drag_icon");
}

export function getHomeDir(): Promise<string> {
	return invoke("get_home_dir");
}

export function createDirectory(path: string): Promise<void> {
	return invoke("create_directory", { path });
}

export function createFile(path: string): Promise<void> {
	return invoke("create_file", { path });
}

export function renameEntry(from: string, to: string): Promise<void> {
	return invoke("rename_entry", { from, to });
}

export function deleteEntry(path: string): Promise<void> {
	return invoke("delete_entry", { path });
}

export function permanentDelete(path: string): Promise<void> {
	return invoke("permanent_delete", { path });
}

export function copyEntry(from: string, to: string): Promise<void> {
	return invoke("copy_entry", { from, to });
}

export function moveEntry(from: string, to: string): Promise<void> {
	return invoke("move_entry", { from, to });
}

export function createSymlink(target: string, link: string): Promise<void> {
	return invoke("create_symlink", { target, link });
}

export interface DriveEntry {
	name: string;
	path: string;
	fstype: string;
	removable: boolean;
}

export function listDrives(): Promise<DriveEntry[]> {
	return invoke("list_drives");
}

export function pathExists(path: string): Promise<boolean> {
	return invoke("path_exists", { path });
}

export function getChildrenCounts(
	paths: string[],
): Promise<Record<string, number>> {
	return invoke("get_children_counts", { paths });
}

export function listTrash(): Promise<FileEntry[]> {
	return invoke("list_trash");
}

export function restoreTrash(name: string): Promise<void> {
	return invoke("restore_trash", { name });
}

export function emptyTrash(): Promise<void> {
	return invoke("empty_trash");
}

export function openDefault(path: string): Promise<void> {
	return invoke("open_default", { path });
}

export interface AppEntry {
	name: string;
	desktop_id: string;
	icon: string;
}

export function listAppsForMime(mimeType: string): Promise<AppEntry[]> {
	return invoke("list_apps_for_mime", { mimeType });
}

export function openWithApp(path: string, desktopId: string): Promise<void> {
	return invoke("open_with_app", { path, desktopId });
}

export function compress(paths: string[], dest: string): Promise<void> {
	return invoke("compress", { paths, dest });
}

export function cancelOperation(): Promise<void> {
	return invoke("cancel_operation");
}

export function extract(archive: string, dest: string): Promise<void> {
	return invoke("extract", { archive, dest });
}

export interface FileProperties {
	name: string;
	path: string;
	size: number;
	is_dir: boolean;
	is_symlink: boolean;
	link_target: string | null;
	mime_type: string;
	permissions: string;
	owner: string;
	group: string;
	created: string | null;
	modified: string | null;
	accessed: string | null;
	contents_count: number | null;
}

export function getProperties(path: string): Promise<FileProperties> {
	return invoke("get_properties", { path });
}

export interface DirStats {
	size: number;
	contents_count: number;
}

export function getDirStats(path: string): Promise<DirStats> {
	return invoke("get_dir_stats", { path });
}

export interface GeneralConfig {
	show_hidden?: boolean;
	sort_by?: string;
	sort_ascending?: boolean;
	theme?: string;
	terminal?: string;
}

export interface AppConfig {
	general: GeneralConfig;
	keybinds: Record<string, string | string[]>;
	chords: Record<string, string[]>;
	warning?: string;
}

export function getConfig(): Promise<AppConfig> {
	return invoke("get_config");
}

export function loadThemeCss(path: string): Promise<string> {
	return invoke("load_theme_css", { path });
}

export function watchTheme(path: string): Promise<void> {
	return invoke("watch_theme", { path });
}

export function openTerminal(path: string, terminal: string): Promise<void> {
	return invoke("open_terminal", { path, terminal });
}

export function watchConfig(): Promise<void> {
	return invoke("watch_config");
}

export function watchDirectory(path: string): Promise<void> {
	return invoke("watch_directory", { path });
}

export function unwatchDirectory(): Promise<void> {
	return invoke("unwatch_directory");
}
