import {
	type FileEntry,
	getChildrenCounts,
	getHomeDir,
	listDirectory,
	listDrives,
	listTrash,
	type SearchResult,
} from "$lib/commands";
import { errorMessage } from "$lib/errors";
import { fuzzyMatch, globMatch, isGlobPattern, parentPath } from "$lib/utils";

let configDefaults: {
	showHidden?: boolean;
	sortBy?: string;
	sortAscending?: boolean;
} = {};

export function setConfigDefaults(opts: typeof configDefaults): void {
	configDefaults = opts;
}

export function loadPreference<T>(key: string, fallback: T): T {
	if (typeof window === "undefined") return fallback;
	const stored = localStorage.getItem(`hyprfiles.${key}`);
	if (stored === null) return fallback;
	try {
		return JSON.parse(stored) as T;
	} catch {
		return fallback;
	}
}

export function savePreference(key: string, value: unknown): void {
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

	// Remembers last cursor position per directory
	const cursorMemory = new Map<string, string>();

	// Directory listing cache for instant navigation
	const MAX_PREFETCH_CHILDREN = 10000;
	const PREFETCH_CONCURRENCY = 4;
	const SIBLING_CACHE_TTL = 30000;
	const dirCache = new Map<
		string,
		{ entries: FileEntry[]; showHid: boolean; time: number; ancestor: boolean }
	>();

	function getCachedListing(path: string): FileEntry[] | null {
		const cached = dirCache.get(path);
		if (!cached) return null;
		if (cached.showHid !== showHidden) return null;
		// Ancestors never expire; siblings expire after TTL
		if (!cached.ancestor && Date.now() - cached.time > SIBLING_CACHE_TTL) {
			dirCache.delete(path);
			return null;
		}
		return cached.entries;
	}

	function cacheListing(path: string, list: FileEntry[], ancestor = false) {
		const existing = dirCache.get(path);
		dirCache.set(path, {
			entries: list,
			showHid: showHidden,
			time: Date.now(),
			ancestor: ancestor || (existing?.ancestor ?? false),
		});
	}

	// biome-ignore lint/suspicious/noEmptyBlockStatements: prefetch failures are intentionally ignored
	const noopCatch = () => {};

	// Dedup in-flight prefetch requests so navigate() can join an existing fetch
	const inFlight = new Map<string, Promise<FileEntry[]>>();

	function prefetchDirectory(path: string, ancestor = false): Promise<void> {
		if (path === "trash://" || getCachedListing(path)) return Promise.resolve();
		const existing = inFlight.get(path);
		if (existing) return existing.then(noopCatch);
		const p = listDirectory(path, showHidden);
		inFlight.set(path, p);
		return p
			.then((list) => cacheListing(path, list, ancestor))
			.catch(noopCatch)
			.finally(() => inFlight.delete(path));
	}

	// Staggered prefetch for all subdirectories in current listing
	let prefetchAbort: (() => void) | null = null;

	function prefetchSubdirectories(list: FileEntry[], parentDir: string) {
		if (prefetchAbort) {
			prefetchAbort();
			prefetchAbort = null;
		}

		const dirs = list.filter(
			(e) =>
				e.is_dir &&
				!getCachedListing(e.path) &&
				(e.children_count === null ||
					e.children_count <= MAX_PREFETCH_CHILDREN),
		);

		let ancestor = parentDir;
		while (true) {
			const up = parentPath(ancestor);
			if (up === ancestor) break;
			prefetchDirectory(up, true);
			ancestor = up;
		}

		if (dirs.length === 0) return;

		let cancelled = false;
		let idx = 0;
		prefetchAbort = () => {
			cancelled = true;
		};

		function nextBatch() {
			if (cancelled || idx >= dirs.length) return;
			const batch = dirs.slice(idx, idx + PREFETCH_CONCURRENCY);
			idx += PREFETCH_CONCURRENCY;
			Promise.allSettled(batch.map((e) => prefetchDirectory(e.path))).then(
				() => {
					if (!cancelled) nextBatch();
				},
			);
		}

		nextBatch();
	}

	// File state
	let entries = $state<FileEntry[]>([]);
	let drives = $state<{ name: string; path: string; icon: string }[]>([]);
	let sortBy = $state(
		loadPreference("sortBy", configDefaults.sortBy ?? "name"),
	);
	let sortAsc = $state(
		loadPreference("sortAsc", configDefaults.sortAscending ?? true),
	);
	let showHidden = $state(
		loadPreference("showHidden", configDefaults.showHidden ?? false),
	);

	// Cursor state (the focused/highlighted entry)
	let cursorPath = $state<string | null>(null);
	let cursorEntry = $state<FileEntry | null>(null);

	// Multi-selection state
	let selectedPaths = $state<Set<string>>(new Set());

	// Visual mode state
	let visualAnchor = $state<string | null>(null);

	// Edit state
	let renamingPath = $state<string | null>(null);
	let creatingEntry = $state<"file" | "directory" | null>(null);

	// Clipboard state
	let clipboard = $state<{ entries: FileEntry[]; mode: "copy" | "cut" } | null>(
		null,
	);

	// Drag and drop state
	let dragEntries = $state<FileEntry[]>([]);
	let dropTarget = $state<string | null>(null);
	const isDragging = $derived(dragEntries.length > 0);

	// Filter state
	let filterQuery = $state("");

	// Search state
	let searchOpen = $state(false);
	let searchQuery = $state("");
	let searchResults = $state<SearchResult[]>([]);
	let searchCursor = $state(0);
	let searchDone = $state(true);
	let searchGen = $state(0);

	// Preview panel state
	let previewEnabled = $state(loadPreference("previewEnabled", false));
	let previewWidth = $state(loadPreference("previewWidth", 300));
	let previewWidthSaveTimer: ReturnType<typeof setTimeout> | undefined;

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

	const filteredEntries = $derived.by(() => {
		if (!filterQuery) return sortedEntries;
		if (isGlobPattern(filterQuery)) {
			return sortedEntries.filter((e) => globMatch(filterQuery, e.name));
		}
		return sortedEntries.filter((e) => fuzzyMatch(filterQuery, e.name));
	});

	const effectiveSelection = $derived.by(() => {
		if (selectedPaths.size > 0) {
			return filteredEntries.filter((e) => selectedPaths.has(e.path));
		}
		return cursorEntry ? [cursorEntry] : [];
	});

	const isTrash = $derived(currentPath === "trash://");
	const visualMode = $derived(visualAnchor !== null);

	// Actions
	function applyEntries(
		path: string,
		list: FileEntry[],
		addToHistory: boolean,
		selectAfter: string | null,
	) {
		entries = list;
		currentPath = path;

		if (addToHistory) {
			history = [...history.slice(0, historyIndex + 1), path];
			historyIndex = history.length - 1;
		}

		const remembered = cursorMemory.get(path);
		const target = selectAfter
			? list.find((e) => e.path === selectAfter || e.name === selectAfter)
			: remembered
				? list.find((e) => e.path === remembered)
				: list[0];
		if (target) select(target);
		else if (list[0]) select(list[0]);

		const dirPaths = list.filter((e) => e.is_dir).map((e) => e.path);
		if (dirPaths.length > 0) {
			fetchChildrenCounts(path, dirPaths);
		}
	}

	async function backgroundRefresh(path: string) {
		try {
			const fresh =
				path === "trash://"
					? await listTrash()
					: await (inFlight.get(path) ?? listDirectory(path, showHidden));
			if (currentPath !== path) return;
			cacheListing(path, fresh);
			entries = fresh;
			if (cursorPath) {
				const entry = fresh.find((e) => e.path === cursorPath);
				if (entry) {
					cursorEntry = entry;
				} else if (fresh.length > 0) {
					select(fresh[0]);
				} else {
					cursorPath = null;
					cursorEntry = null;
				}
			}
			const dirPaths = fresh.filter((e) => e.is_dir).map((e) => e.path);
			if (dirPaths.length > 0) fetchChildrenCounts(path, dirPaths);
		} catch {
			// Silent — cached data remains displayed
		}
	}

	async function navigate(
		path: string,
		addToHistory = true,
		selectAfter: string | null = null,
	) {
		// Remember cursor position before leaving
		if (cursorPath && currentPath) {
			cursorMemory.set(currentPath, cursorPath);
		}

		cursorPath = null;
		cursorEntry = null;
		selectedPaths = new Set();
		visualAnchor = null;
		filterQuery = "";
		error = null;

		// Cache hit — render instantly, background refresh for freshness
		const cached = getCachedListing(path);
		if (cached) {
			loading = false;
			applyEntries(path, cached, addToHistory, selectAfter);
			backgroundRefresh(path);
			prefetchSubdirectories(cached, path);
			return;
		}

		// Cache miss — full load with loading state
		loading = true;

		try {
			// Join an in-flight prefetch if one exists, otherwise start a new fetch
			const list =
				path === "trash://"
					? await listTrash()
					: await (inFlight.get(path) ?? listDirectory(path, showHidden));
			cacheListing(path, list, true);
			applyEntries(path, list, addToHistory, selectAfter);
			prefetchSubdirectories(list, path);
		} catch (e) {
			error = errorMessage(e) ?? String(e);
		} finally {
			loading = false;
		}
	}

	async function fetchChildrenCounts(forPath: string, dirPaths: string[]) {
		try {
			const counts = await getChildrenCounts(dirPaths);
			// Only apply if we're still on the same directory
			if (currentPath !== forPath) return;
			entries = entries.map((e) =>
				e.path in counts ? { ...e, children_count: counts[e.path] } : e,
			);
		} catch {
			// Non-critical — leave children_count as null
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
			const previousDir = currentPath;
			navigate(parent, true, previousDir);
		}
	}

	function refresh() {
		dirCache.delete(currentPath);
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
		dirCache.clear();
		navigate(currentPath, false);
	}

	function moveCursor(entry: FileEntry) {
		cursorPath = entry.path;
		cursorEntry = entry;
	}

	function select(entry: FileEntry) {
		moveCursor(entry);
		selectedPaths = new Set();
	}

	function clearSelection() {
		cursorPath = null;
		cursorEntry = null;
		selectedPaths = new Set();
		visualAnchor = null;
	}

	function selectByIndex(index: number) {
		const list = filteredEntries;
		if (list.length === 0) return;
		const clamped = Math.max(0, Math.min(index, list.length - 1));
		moveCursor(list[clamped]);
		if (visualAnchor) {
			recomputeVisualRange(list);
		}
	}

	function selectRelative(delta: number) {
		const list = filteredEntries;
		if (list.length === 0) return;
		const currentIndex = list.findIndex((e) => e.path === cursorPath);
		const next = currentIndex === -1 ? 0 : currentIndex + delta;
		selectByIndex(next);
	}

	function selectRelativeWrap(delta: number) {
		const list = filteredEntries;
		if (list.length === 0) return;
		const currentIndex = list.findIndex((e) => e.path === cursorPath);
		const next =
			currentIndex === -1
				? 0
				: (((currentIndex + delta) % list.length) + list.length) % list.length;
		selectByIndex(next);
	}

	function recomputeVisualRange(list: FileEntry[]) {
		const anchorIdx = list.findIndex((e) => e.path === visualAnchor);
		const curIdx = list.findIndex((e) => e.path === cursorPath);
		if (anchorIdx === -1 || curIdx === -1) return;
		const start = Math.min(anchorIdx, curIdx);
		const end = Math.max(anchorIdx, curIdx);
		const next = new Set<string>();
		for (let i = start; i <= end; i++) {
			next.add(list[i].path);
		}
		selectedPaths = next;
	}

	function enterVisualMode() {
		if (!cursorPath) return;
		visualAnchor = cursorPath;
		selectedPaths = new Set([cursorPath]);
	}

	function exitVisualMode() {
		visualAnchor = null;
	}

	function toggleSelect(entry: FileEntry) {
		const next = new Set(selectedPaths);
		if (next.has(entry.path)) {
			next.delete(entry.path);
		} else {
			next.add(entry.path);
		}
		selectedPaths = next;
		cursorPath = entry.path;
		cursorEntry = entry;
	}

	function selectRange(from: FileEntry, to: FileEntry) {
		const list = filteredEntries;
		const fromIdx = list.findIndex((e) => e.path === from.path);
		const toIdx = list.findIndex((e) => e.path === to.path);
		if (fromIdx === -1 || toIdx === -1) return;
		const start = Math.min(fromIdx, toIdx);
		const end = Math.max(fromIdx, toIdx);
		const next = new Set(selectedPaths);
		for (let i = start; i <= end; i++) {
			next.add(list[i].path);
		}
		selectedPaths = next;
		cursorPath = to.path;
		cursorEntry = to;
	}

	function selectAll() {
		selectedPaths = new Set(filteredEntries.map((e) => e.path));
	}

	function clearMultiSelection() {
		selectedPaths = new Set();
	}

	function setError(msg: string | null) {
		error = msg;
	}

	async function init(path?: string) {
		try {
			homeDir = await getHomeDir();
			await navigate(path ?? homeDir);
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
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		get cursorPath() {
			return cursorPath;
		},
		get cursorEntry() {
			return cursorEntry;
		},
		get selectedPaths() {
			return selectedPaths;
		},
		get effectiveSelection() {
			return effectiveSelection;
		},
		// Backward-compat aliases (removed when consumers migrate)
		get selectedPath() {
			return cursorPath;
		},
		get selectedEntry() {
			return cursorEntry;
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
		get filteredEntries() {
			return filteredEntries;
		},
		get filterQuery() {
			return filterQuery;
		},
		get isTrash() {
			return isTrash;
		},
		get visualMode() {
			return visualMode;
		},
		get dragEntries() {
			return dragEntries;
		},
		get dropTarget() {
			return dropTarget;
		},
		get previewEnabled() {
			return previewEnabled;
		},
		get previewWidth() {
			return previewWidth;
		},
		get isDragging() {
			return isDragging;
		},
		get searchOpen() {
			return searchOpen;
		},
		get searchQuery() {
			return searchQuery;
		},
		get searchResults() {
			return searchResults;
		},
		get searchCursor() {
			return searchCursor;
		},
		set searchCursor(v: number) {
			searchCursor = v;
		},
		get searchDone() {
			return searchDone;
		},
		get searchGen() {
			return searchGen;
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
		setFilterQuery(query: string) {
			filterQuery = query;
		},
		clearFilter() {
			filterQuery = "";
		},
		setSearchQuery(q: string) {
			searchQuery = q;
		},
		openSearch() {
			searchOpen = true;
			searchQuery = "";
			searchResults = [];
			searchCursor = 0;
			searchDone = true;
		},
		closeSearch() {
			searchOpen = false;
			searchQuery = "";
			searchResults = [];
			searchCursor = 0;
			searchDone = true;
		},
		appendSearchResults(gen: number, batch: SearchResult[]) {
			if (gen !== searchGen) return;
			searchResults = [...searchResults, ...batch];
		},
		markSearchDone(gen: number) {
			if (gen !== searchGen) return;
			searchDone = true;
		},
		resetSearchResults() {
			searchGen++;
			searchResults = [];
			searchCursor = 0;
			searchDone = false;
		},
		navigate,
		goBack,
		goForward,
		goUp,
		refresh,
		handleSort,
		toggleHidden,
		select,
		clearSelection,
		selectByIndex,
		selectRelative,
		selectRelativeWrap,
		toggleSelect,
		selectRange,
		selectAll,
		clearMultiSelection,
		enterVisualMode,
		exitVisualMode,
		clearTransient() {
			selectedPaths = new Set();
			visualAnchor = null;
			renamingPath = null;
			creatingEntry = null;
			openWithApps = [];
		},
		startDrag(entries: FileEntry[]) {
			dragEntries = entries;
			dropTarget = null;
		},
		setDropTarget(path: string | null) {
			dropTarget = path;
		},
		endDrag() {
			dragEntries = [];
			dropTarget = null;
		},
		togglePreview() {
			previewEnabled = !previewEnabled;
			savePreference("previewEnabled", previewEnabled);
		},
		setPreviewWidth(w: number) {
			previewWidth = w;
			clearTimeout(previewWidthSaveTimer);
			previewWidthSaveTimer = setTimeout(
				() => savePreference("previewWidth", w),
				300,
			);
		},
		prefetchDirectory,
		setError,
		init,
		applyConfigDefaults() {
			sortBy = loadPreference("sortBy", configDefaults.sortBy ?? "name");
			sortAsc = loadPreference("sortAsc", configDefaults.sortAscending ?? true);
			showHidden = loadPreference(
				"showHidden",
				configDefaults.showHidden ?? false,
			);
		},
	};
}

export type FileManager = ReturnType<typeof createFileManager>;
