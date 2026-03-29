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

export interface DirStreamBatch {
	entries: FileEntry[];
	done: boolean;
	gen: number;
	path: string;
}

export function streamDirectory(
	path: string,
	showHidden: boolean,
	gen: number,
): Promise<void> {
	return invoke("stream_directory", { path, showHidden, gen });
}

export function cancelStreamDirectory(): Promise<void> {
	return invoke("cancel_stream_directory");
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

export function createSymlink(target: string, link: string): Promise<void> {
	return invoke("create_symlink", { target, link });
}

export function chmodEntry(path: string, mode: number): Promise<void> {
	return invoke("chmod_entry", { path, mode });
}

export function pasteEntries(
	paths: string[],
	dest: string,
	mode: "copy" | "cut",
): Promise<void> {
	return invoke("paste_entries", { paths, dest, mode });
}

export function deleteEntriesPermanently(paths: string[]): Promise<void> {
	return invoke("delete_entries_permanently", { paths });
}

export interface FilePreview {
	content: string;
	truncated: boolean;
	bytes_read: number;
	is_binary: boolean;
}

export function readFilePreview(
	path: string,
	maxBytes = 65536,
): Promise<FilePreview> {
	return invoke("read_file_preview", { path, maxBytes });
}

export interface PdfPreview {
	image_path: string;
	page_count: number;
}

export function readPdfPreview(
	path: string,
	previewPath = "",
): Promise<PdfPreview> {
	return invoke("read_pdf_preview", { path, previewPath });
}

export interface ImageThumbnail {
	image_path: string;
	width: number;
	height: number;
}

export function generateThumbnail(
	path: string,
	maxDim = 800,
	previewPath = "",
): Promise<ImageThumbnail> {
	return invoke("generate_thumbnail", { path, maxDim, previewPath });
}

export function setPreviewPath(path: string): Promise<void> {
	return invoke("set_preview_path", { path });
}

export interface DriveEntry {
	name: string;
	path: string;
	device: string;
	fstype: string;
	removable: boolean;
	mounted: boolean;
	size: string;
}

export function listDrives(): Promise<DriveEntry[]> {
	return invoke("list_drives");
}

export function mountDrive(device: string): Promise<string> {
	return invoke("mount_drive", { device });
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

export function openWithEditor(
	path: string,
	editor?: string | null,
): Promise<void> {
	return invoke("open_with_editor", { path, editor });
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
	light_icons?: boolean;
	editor?: string;
	terminal?: string;
}

export interface CustomAction {
	name: string;
	command: string;
	context: string;
	mime?: string;
	refresh: boolean;
}

export interface AppConfig {
	general: GeneralConfig;
	keybinds: Record<string, string | string[]>;
	chords: Record<string, string[]>;
	actions: CustomAction[];
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

export function runCustomAction(
	command: string,
	cwd: string,
	wait: boolean,
): Promise<void> {
	return invoke("run_custom_action", { command, cwd, wait });
}

export interface SearchResult {
	name: string;
	path: string;
	relative_path: string;
	is_dir: boolean;
	is_symlink: boolean;
}

export function searchFiles(
	root: string,
	query: string,
	showHidden: boolean,
	gen: number,
): Promise<void> {
	return invoke("search_files", { root, query, showHidden, gen });
}

export function cancelSearch(): Promise<void> {
	return invoke("cancel_search");
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

export function showWindow(): Promise<void> {
	return invoke("show_window");
}
