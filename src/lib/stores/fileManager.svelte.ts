import {
	type FileEntry,
	getHomeDir,
	listDirectory,
	listDrives,
	listTrash,
} from "$lib/commands";
import { errorMessage } from "$lib/errors";
import { parentPath } from "$lib/utils";

function loadPreference<T>(key: string, fallback: T): T {
	if (typeof window === "undefined") return fallback;
	const stored = localStorage.getItem(`hyprfiles.${key}`);
	if (stored === null) return fallback;
	try {
		return JSON.parse(stored) as T;
	} catch {
		return fallback;
	}
}

function savePreference(key: string, value: unknown): void {
	if (typeof window === "undefined") return;
	localStorage.setItem(`hyprfiles.${key}`, JSON.stringify(value));
}

export function createFileManager() {
	// Navigation state
	let currentPath = $state("/");
	let history = $state<string[]>([]);
	let historyIndex = $state(-1);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// File state
	let entries = $state<FileEntry[]>([]);
	let drives = $state<{ name: string; path: string; icon: string }[]>([]);
	let sortBy = $state(loadPreference("sortBy", "name"));
	let sortAsc = $state(loadPreference("sortAsc", true));
	let showHidden = $state(loadPreference("showHidden", false));
	let viewMode = $state<"list" | "grid">(loadPreference("viewMode", "list"));

	// Selection state
	let selectedPath = $state<string | null>(null);
	let selectedEntry = $state<FileEntry | null>(null);

	// Edit state
	let renamingPath = $state<string | null>(null);
	let creatingEntry = $state<"file" | "directory" | null>(null);

	// Clipboard state
	let clipboard = $state<{ entries: FileEntry[]; mode: "copy" | "cut" } | null>(
		null,
	);

	// Open With state
	let openWithApps = $state<
		Array<{ name: string; desktop_id: string; icon: string }>
	>([]);

	// Home directory (shared with sidebar)
	let homeDir = $state<string>("/");

	// Derived
	const sortedEntries = $derived.by(() => {
		const nameCmp = (a: FileEntry, b: FileEntry) =>
			a.name.toLowerCase().localeCompare(b.name.toLowerCase());

		const sorted = [...entries];
		sorted.sort((a, b) => {
			if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;

			if (a.is_dir && b.is_dir && sortBy === "size") {
				const cmp = (a.children_count ?? 0) - (b.children_count ?? 0);
				const directed = sortAsc ? cmp : -cmp;
				return directed !== 0 ? directed : nameCmp(a, b);
			}

			let cmp = 0;
			if (sortBy === "name") {
				cmp = nameCmp(a, b);
			} else if (sortBy === "size") {
				cmp = a.size - b.size;
			} else if (sortBy === "modified") {
				cmp = a.modified.localeCompare(b.modified);
			}

			const directed = sortAsc ? cmp : -cmp;
			return directed !== 0 ? directed : nameCmp(a, b);
		});
		return sorted;
	});

	const isTrash = $derived(currentPath === "trash://");

	// Actions
	async function navigate(path: string, addToHistory = true) {
		loading = true;
		error = null;
		selectedPath = null;
		selectedEntry = null;

		try {
			if (path === "trash://") {
				entries = await listTrash();
			} else {
				entries = await listDirectory(path, showHidden);
			}
			currentPath = path;

			if (addToHistory) {
				history = [...history.slice(0, historyIndex + 1), path];
				historyIndex = history.length - 1;
			}
		} catch (e) {
			error = errorMessage(e) ?? String(e);
		} finally {
			loading = false;
		}
	}

	function goBack() {
		if (historyIndex > 0) {
			historyIndex--;
			navigate(history[historyIndex], false);
		}
	}

	function goForward() {
		if (historyIndex < history.length - 1) {
			historyIndex++;
			navigate(history[historyIndex], false);
		}
	}

	function goUp() {
		const parent = parentPath(currentPath);
		if (parent !== currentPath) {
			navigate(parent);
		}
	}

	function refresh() {
		return navigate(currentPath, false);
	}

	function handleSort(column: string) {
		if (sortBy === column) {
			sortAsc = !sortAsc;
		} else {
			sortBy = column;
			sortAsc = true;
		}
		savePreference("sortBy", sortBy);
		savePreference("sortAsc", sortAsc);
	}

	function toggleHidden() {
		showHidden = !showHidden;
		savePreference("showHidden", showHidden);
		navigate(currentPath, false);
	}

	function setViewMode(mode: "list" | "grid") {
		viewMode = mode;
		savePreference("viewMode", viewMode);
	}

	function select(entry: FileEntry) {
		selectedPath = entry.path;
		selectedEntry = entry;
	}

	function clearSelection() {
		selectedPath = null;
		selectedEntry = null;
	}

	function setError(msg: string | null) {
		error = msg;
	}

	async function init() {
		try {
			homeDir = await getHomeDir();
			await navigate(homeDir);
		} catch {
			await navigate("/");
		}

		try {
			const d = await listDrives();
			drives = d.map((drive) => ({
				name: drive.name,
				path: drive.path,
				icon: drive.removable ? "\uF0A0" : "\uF0A0",
			}));
		} catch {
			drives = [];
		}
	}

	return {
		// Read-only state (via getters)
		get currentPath() {
			return currentPath;
		},
		get entries() {
			return entries;
		},
		get drives() {
			return drives;
		},
		get sortBy() {
			return sortBy;
		},
		get sortAsc() {
			return sortAsc;
		},
		get showHidden() {
			return showHidden;
		},
		get viewMode() {
			return viewMode;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		get selectedPath() {
			return selectedPath;
		},
		get selectedEntry() {
			return selectedEntry;
		},
		get renamingPath() {
			return renamingPath;
		},
		get creatingEntry() {
			return creatingEntry;
		},
		get clipboard() {
			return clipboard;
		},
		get openWithApps() {
			return openWithApps;
		},
		get homeDir() {
			return homeDir;
		},
		get sortedEntries() {
			return sortedEntries;
		},
		get isTrash() {
			return isTrash;
		},
		get history() {
			return history;
		},
		get historyIndex() {
			return historyIndex;
		},

		// Writable state (via setters)
		set renamingPath(v: string | null) {
			renamingPath = v;
		},
		set creatingEntry(v: "file" | "directory" | null) {
			creatingEntry = v;
		},
		set clipboard(v: { entries: FileEntry[]; mode: "copy" | "cut" } | null) {
			clipboard = v;
		},
		set openWithApps(v: Array<{
			name: string;
			desktop_id: string;
			icon: string;
		}>) {
			openWithApps = v;
		},

		// Actions
		navigate,
		goBack,
		goForward,
		goUp,
		refresh,
		handleSort,
		toggleHidden,
		setViewMode,
		select,
		clearSelection,
		setError,
		init,
	};
}

export type FileManager = ReturnType<typeof createFileManager>;
