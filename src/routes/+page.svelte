<script lang="ts">
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { onDestroy, onMount, tick } from "svelte";
import { buildCommandList, type Command } from "$lib/commandRegistry";
import type {
	CustomAction,
	FileEntry,
	FilePreview,
	PdfPreview,
	SearchResult,
} from "$lib/commands";
import {
	type AppConfig,
	cancelSearch,
	generateThumbnail,
	getConfig,
	getDragIcon,
	listDirectory,
	loadThemeCss,
	readFilePreview,
	readPdfPreview,
	runCustomAction,
	searchFiles,
	setPreviewGen,
	showWindow,
	unwatchDirectory,
	watchConfig,
	watchDirectory,
	watchTheme,
} from "$lib/commands";
import BusyOverlay from "$lib/components/BusyOverlay.svelte";
// biome-ignore lint/style/useImportType: component used in template
import CommandPalette from "$lib/components/CommandPalette.svelte";
import CompressDialog from "$lib/components/CompressDialog.svelte";
import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
import ContextMenu from "$lib/components/ContextMenu.svelte";
// biome-ignore lint/style/useImportType: component used in template
import FileList from "$lib/components/FileList.svelte";
// biome-ignore lint/style/useImportType: component used in template
import FilterBar from "$lib/components/FilterBar.svelte";
import FolderPicker from "$lib/components/FolderPicker.svelte";
import HelpDialog from "$lib/components/HelpDialog.svelte";
import PreviewPanel from "$lib/components/PreviewPanel.svelte";
import PropertiesDialog from "$lib/components/PropertiesDialog.svelte";
// biome-ignore lint/style/useImportType: component used in template
import SearchOverlay from "$lib/components/SearchOverlay.svelte";
import Sidebar from "$lib/components/Sidebar.svelte";
import StatusBar from "$lib/components/StatusBar.svelte";
import TabBar from "$lib/components/TabBar.svelte";
// biome-ignore lint/style/useImportType: component used in template
import Toolbar from "$lib/components/Toolbar.svelte";
import {
	isImagePreviewable,
	isPdfPreviewable,
	isTextPreviewable,
} from "$lib/constants";
import {
	type ContextMenuActions,
	type ContextMenuContext,
	getContextMenuItems,
} from "$lib/contextMenu";
import { errorMessage } from "$lib/errors";
import * as ops from "$lib/fileOps";
import type { HighlightResponse } from "$lib/highlight";
import { setIconMode } from "$lib/icons";
import {
	applyKeybindOverrides,
	type ChordName,
	chordPrefixes,
	chords,
	keybinds,
	matchesKeybind,
	resetKeybinds,
} from "$lib/keybinds";
import { type CachedPreview, previewCache } from "$lib/previewCache";
import { createDialogManager } from "$lib/stores/dialogs.svelte";
import { setConfigDefaults } from "$lib/stores/fileManager.svelte";
import { createTabManager } from "$lib/stores/tabs.svelte";
import { isGlobPattern, parentPath } from "$lib/utils";

let themeUnlisten: UnlistenFn | null = null;
let searchUnlisten: UnlistenFn | null = null;
let searchDebounce: ReturnType<typeof setTimeout> | undefined;
let editorApp: string | null = null;
let terminalApp: string | null = null;
let customActions: CustomAction[] = [];
let dirWatchUnlisten: UnlistenFn | null = null;
let configUnlisten: UnlistenFn | null = null;
let currentThemeName: string | null = null;

function applyBgOpacity(css: string): string {
	const opacityMatch = css.match(/--bg-opacity:\s*([\d.]+)%/);
	if (!opacityMatch) return css;
	const opacity = opacityMatch[1];
	if (opacity === "100") return css;

	const bgVars = [
		"--bg-primary",
		"--bg-secondary",
		"--bg-surface",
		"--bg-hover",
	];
	const overrides: string[] = [];
	for (const v of bgVars) {
		const re = new RegExp(`${v}:\\s*(#[0-9a-fA-F]{6})(?![0-9a-fA-F-])`);
		const m = css.match(re);
		if (m) {
			overrides.push(
				`${v}: color-mix(in srgb, ${m[1]} ${opacity}%, transparent)`,
			);
		}
	}
	if (overrides.length === 0) return css;
	return `${css}\n:root {\n\t${overrides.join(";\n\t")};\n}`;
}

function applyThemeCss(css: string) {
	let el = document.getElementById("pane-fm-theme");
	if (!el) {
		el = document.createElement("style");
		el.id = "pane-fm-theme";
		document.head.appendChild(el);
	}
	el.textContent = applyBgOpacity(css);
}

async function applyConfig(config: AppConfig) {
	if (config.warning) {
		tabs.activeFm.setError(config.warning);
		return;
	}

	resetKeybinds();
	applyKeybindOverrides(config.keybinds, config.chords);

	setConfigDefaults({
		showHidden: config.general.show_hidden ?? undefined,
		sortBy: config.general.sort_by ?? undefined,
		sortAscending: config.general.sort_ascending ?? undefined,
	});
	for (const tab of tabs.tabs) {
		tab.fm.applyConfigDefaults();
	}
	editorApp = config.general.editor ?? null;
	terminalApp = config.general.terminal ?? null;
	customActions = config.actions ?? [];
	setIconMode(config.general.light_icons ? "light" : "dark");

	const newTheme = config.general.theme ?? null;
	if (newTheme !== currentThemeName) {
		currentThemeName = newTheme;
		if (newTheme) {
			try {
				const css = await loadThemeCss(newTheme);
				applyThemeCss(css);
				await watchTheme(newTheme);
				if (!themeUnlisten) {
					themeUnlisten = await listen<string>("theme-changed", (e) => {
						applyThemeCss(e.payload);
					});
				}
			} catch {
				applyThemeCss("");
			}
		} else {
			applyThemeCss("");
		}
	}
}

const tabs = createTabManager();
const dlg = createDialogManager(() => fm);
let fm = $derived(tabs.activeFm);

let filterBarVisible = $state(false);
let filterBar = $state<ReturnType<typeof FilterBar> | null>(null);
let searchOverlay = $state<ReturnType<typeof SearchOverlay> | null>(null);
let toolbar = $state<ReturnType<typeof Toolbar> | null>(null);
let fileList = $state<ReturnType<typeof FileList> | null>(null);

// Sync filter bar visibility when switching tabs
let prevTabIndex = tabs.activeIndex;
$effect(() => {
	const idx = tabs.activeIndex;
	if (idx !== prevTabIndex) {
		prevTabIndex = idx;
		filterBarVisible = !!fm.filterQuery;
	}
});

// Persist tab paths when the active tab navigates
$effect(() => {
	if (fm.currentPath) tabs.persistTabs();
});

// Prefetch directory under cursor for instant Enter/l navigation
$effect(() => {
	const entry = fm.cursorEntry;
	if (entry?.is_dir) fm.prefetchDirectory(entry.path);
});

// Preview panel state
let previewData = $state<FilePreview | null>(null);
let pdfPreview = $state<PdfPreview | null>(null);
let previewLoading = $state(false);
let previewError = $state<string | null>(null);
let highlightedHtml = $state<string | null>(null);
let imagePreviewUrl = $state<string | null>(null);
let dirPreviewEntries = $state<FileEntry[] | null>(null);
let previewTimer: ReturnType<typeof setTimeout> | undefined;
let previewGen = 0;

const MAX_HIGHLIGHT_LINES = 200;
function truncateForHighlight(code: string): string {
	let pos = 0;
	for (let i = 0; i < MAX_HIGHLIGHT_LINES && pos < code.length; i++) {
		const nl = code.indexOf("\n", pos);
		if (nl === -1) return code;
		pos = nl + 1;
	}
	return code.slice(0, pos);
}

let prefetchGen = 0;
const pendingPrefetch = new Map<
	number,
	{ path: string; mtime: string; data: FilePreview }
>();

const hlWorker = new Worker(
	new URL("$lib/highlightWorker.ts", import.meta.url),
	{ type: "module" },
);
hlWorker.onmessage = (e: MessageEvent<HighlightResponse>) => {
	const { html, gen } = e.data;

	if (gen === previewGen) {
		highlightedHtml = html;
		previewLoading = false;
		if (activePreviewEntry && previewData) {
			previewCache.set(activePreviewEntry.path, activePreviewEntry.modified, {
				type: "text",
				data: previewData,
				html,
			});
		}
	} else {
		const pending = pendingPrefetch.get(gen);
		if (pending) {
			previewCache.set(pending.path, pending.mtime, {
				type: "text",
				data: pending.data,
				html,
			});
			pendingPrefetch.delete(gen);
		}
	}
};

let activePreviewEntry: FileEntry | null = null;

function clearPreviewState() {
	previewData = null;
	pdfPreview = null;
	highlightedHtml = null;
	imagePreviewUrl = null;
	dirPreviewEntries = null;
	previewError = null;
}

function applyCachedPreview(cached: CachedPreview) {
	clearPreviewState();
	previewLoading = false;
	switch (cached.type) {
		case "text":
			previewData = cached.data;
			highlightedHtml = cached.html;
			break;
		case "dir":
			dirPreviewEntries = cached.entries;
			break;
		case "image":
			imagePreviewUrl = cached.url;
			break;
		case "pdf":
			pdfPreview = cached.data;
			break;
		case "none":
			break;
	}
}

// biome-ignore lint/suspicious/noEmptyBlockStatements: prefetch failures are intentionally ignored
const noop = () => {};

function prefetchAdjacent(current: FileEntry, gen: number) {
	const list = fm.filteredEntries;
	const idx = list.findIndex((e) => e.path === current.path);
	if (idx < 0) return;

	for (const adj of [list[idx - 1], list[idx + 1]]) {
		if (!adj) continue;
		if (previewCache.get(adj.path, adj.modified)) continue;

		if (adj.is_dir) {
			listDirectory(adj.path, fm.showHidden)
				.then((entries) =>
					previewCache.set(adj.path, adj.modified, {
						type: "dir",
						entries,
					}),
				)
				.catch(noop);
		} else if (isTextPreviewable(adj.mime_type, adj.name)) {
			readFilePreview(adj.path)
				.then((data) => {
					if (data.is_binary || !data.content) {
						previewCache.set(adj.path, adj.modified, { type: "none" });
						return;
					}
					const pgen = --prefetchGen;
					// Cap pending map to prevent leaks from dropped worker messages
					if (pendingPrefetch.size > 20) pendingPrefetch.clear();
					pendingPrefetch.set(pgen, {
						path: adj.path,
						mtime: adj.modified,
						data,
					});
					hlWorker.postMessage({
						code: truncateForHighlight(data.content),
						filename: adj.name,
						gen: pgen,
					});
				})
				.catch(noop);
		} else if (isImagePreviewable(adj.mime_type)) {
			if (adj.mime_type === "image/svg+xml") {
				const url = convertFileSrc(adj.path);
				previewCache.set(adj.path, adj.modified, { type: "image", url });
			} else {
				generateThumbnail(adj.path, undefined, gen)
					.then((thumb) => {
						const url = convertFileSrc(thumb.image_path);
						previewCache.set(adj.path, adj.modified, {
							type: "image",
							url,
						});
						const img = new Image();
						img.src = url;
					})
					.catch(noop);
			}
		} else if (isPdfPreviewable(adj.mime_type)) {
			readPdfPreview(adj.path, gen)
				.then((data) =>
					previewCache.set(adj.path, adj.modified, { type: "pdf", data }),
				)
				.catch(noop);
		}
	}
}

async function loadPreview(entry: FileEntry, gen: number) {
	if (gen !== previewGen) return;
	const { mime_type: mime, path, name } = entry;

	if (entry.is_dir) {
		try {
			const entries = await listDirectory(path, fm.showHidden);
			if (gen !== previewGen) return;
			dirPreviewEntries = entries;
			previewCache.set(path, entry.modified, { type: "dir", entries });
		} catch {
			if (gen !== previewGen) return;
		}
	} else if (isTextPreviewable(mime, name)) {
		try {
			const data = await readFilePreview(path);
			if (gen !== previewGen) return;
			previewData = data;
			if (data.is_binary || !data.content) {
				previewCache.set(path, entry.modified, { type: "none" });
			} else {
				activePreviewEntry = entry;
				hlWorker.postMessage({
					code: truncateForHighlight(data.content),
					filename: name,
					gen,
				});
				return; // Worker callback handles previewLoading + caching
			}
		} catch (e) {
			if (gen !== previewGen) return;
			previewError = errorMessage(e) ?? "Failed to load preview";
		}
	} else if (isImagePreviewable(mime)) {
		let url: string;
		if (mime === "image/svg+xml") {
			if (gen !== previewGen) return;
			url = convertFileSrc(path);
		} else {
			try {
				const thumb = await generateThumbnail(path, undefined, gen);
				if (gen !== previewGen) return;
				url = convertFileSrc(thumb.image_path);
			} catch {
				if (gen !== previewGen) return;
				url = convertFileSrc(path);
			}
		}
		imagePreviewUrl = url;
		previewCache.set(path, entry.modified, { type: "image", url });
	} else if (isPdfPreviewable(mime)) {
		try {
			const data = await readPdfPreview(path, gen);
			if (gen !== previewGen) return;
			pdfPreview = data;
			previewCache.set(path, entry.modified, { type: "pdf", data });
		} catch (e) {
			if (gen !== previewGen) return;
			previewError = errorMessage(e) ?? "Failed to load PDF preview";
		}
	}

	previewLoading = false;
}

$effect(() => {
	const entry = fm.cursorEntry;
	const enabled = fm.previewEnabled;
	clearTimeout(previewTimer);
	const gen = ++previewGen;
	setPreviewGen(gen);

	if (!enabled || !entry) {
		clearPreviewState();
		previewLoading = false;
		activePreviewEntry = null;
		return;
	}

	const cached = previewCache.get(entry.path, entry.modified);
	if (cached) {
		applyCachedPreview(cached);
		activePreviewEntry = entry;
		prefetchAdjacent(entry, gen);
		return;
	}

	clearPreviewState();
	previewLoading = true;
	activePreviewEntry = entry;

	previewTimer = setTimeout(() => {
		loadPreview(entry, gen).then(() => {
			if (gen === previewGen) prefetchAdjacent(entry, gen);
		});
	}, 250);

	return () => clearTimeout(previewTimer);
});

let mouseCursorHidden = $state(false);
let contentEl = $state<HTMLDivElement | null>(null);
let contentWidth = $state(1200);
const compact = $derived(contentWidth < 700);
const narrow = $derived(contentWidth < 500);

$effect(() => {
	if (!contentEl) return;
	contentWidth = contentEl.clientWidth;
	const ro = new ResizeObserver(([e]) => {
		contentWidth = e.contentRect.width;
	});
	ro.observe(contentEl);
	return () => ro.disconnect();
});

let lastMousePos = { x: 0, y: 0 };
let pendingChord = $state<string | null>(null);
let chordTimer: ReturnType<typeof setTimeout> | null = null;

function clearChord() {
	pendingChord = null;
	if (chordTimer) {
		clearTimeout(chordTimer);
		chordTimer = null;
	}
}

function matchChord(prefix: string, second: string): ChordName | null {
	for (const [name, def] of Object.entries(chords)) {
		if (def.keys[0] === prefix && def.keys[1] === second)
			return name as ChordName;
	}
	return null;
}

function executeChord(name: ChordName) {
	switch (name) {
		case "goTop":
			fm.selectByIndex(0);
			break;
		case "goBottom":
			fm.selectByIndex(fm.filteredEntries.length - 1);
			break;
		case "goHome":
			fm.navigate(fm.homeDir);
			break;
		case "goDownloads":
			fm.navigate(`${fm.homeDir}/Downloads`);
			break;
		case "goTrash":
			fm.navigate("trash://");
			break;
		case "sortName":
			fm.handleSort("name");
			break;
		case "sortSize":
			fm.handleSort("size");
			break;
		case "sortModified":
			fm.handleSort("modified");
			break;
		case "copyPath":
			if (fm.cursorEntry) navigator.clipboard.writeText(fm.cursorEntry.path);
			break;
		case "copyFilename":
			if (fm.cursorEntry) navigator.clipboard.writeText(fm.cursorEntry.name);
			break;
		case "nextTab":
			tabs.nextTab();
			break;
		case "prevTab":
			tabs.prevTab();
			break;
	}
}

function isDialogOpen() {
	return !!(
		dlg.contextMenu ||
		dlg.propertiesData ||
		dlg.folderPicker ||
		dlg.compressEntries.length > 0 ||
		dlg.confirmDialog ||
		dlg.busyMessage ||
		dlg.helpOpen ||
		dlg.paletteOpen
	);
}

async function handleWindowKeydown(e: KeyboardEvent) {
	// Command palette toggle — works in inputs but not over confirm/busy dialogs
	if (matchesKeybind(e, keybinds.commandPalette)) {
		e.preventDefault();
		if (dlg.paletteOpen) dlg.closePalette();
		else if (!dlg.busyMessage && !dlg.confirmDialog) dlg.openPalette();
		return;
	}

	// Let dialogs handle their own keys
	if (isDialogOpen()) return;

	if (matchesKeybind(e, keybinds.focusPath)) {
		e.preventDefault();
		toolbar?.focusPath();
		return;
	}
	if (matchesKeybind(e, keybinds.halfPageUp)) {
		e.preventDefault();
		fm.selectRelative(-15);
		mouseCursorHidden = true;
		return;
	}
	if (matchesKeybind(e, keybinds.halfPageDown)) {
		e.preventDefault();
		fm.selectRelative(15);
		mouseCursorHidden = true;
		return;
	}

	const tag = (e.target as HTMLElement)?.tagName;
	if (tag === "INPUT" || tag === "TEXTAREA") return;

	// Chord handling: two-key sequences like gg, gh, ,s
	// Ignore modifier-only keys so Shift/Ctrl don't break chords like gT
	if (
		pendingChord &&
		(e.key === "Shift" ||
			e.key === "Control" ||
			e.key === "Alt" ||
			e.key === "Meta")
	) {
		return;
	}
	if (pendingChord) {
		const chord = matchChord(pendingChord, e.key);
		clearChord();
		if (chord) {
			e.preventDefault();
			executeChord(chord);
			mouseCursorHidden = true;
			return;
		}
		// No match — fall through so second key works as normal single key
	}

	if (!e.ctrlKey && !e.altKey && !e.metaKey && chordPrefixes.has(e.key)) {
		// Check if this key is ONLY a chord prefix (no single-key bind uses it)
		// g, comma, c are not single-key binds so we can safely intercept
		const isSingleBind = Object.values(keybinds).some((bind) => {
			if (typeof bind === "string") return bind === e.key;
			if (Array.isArray(bind))
				return bind.some((b) => typeof b === "string" && b === e.key);
			return false;
		});
		if (!isSingleBind) {
			e.preventDefault();
			pendingChord = e.key;
			chordTimer = setTimeout(clearChord, 500);
			return;
		}
	}

	// Tab keybinds — check digit keys for tab switching (1-9)
	if (!e.ctrlKey && !e.altKey && !e.metaKey && e.key >= "1" && e.key <= "9") {
		const idx = Number.parseInt(e.key, 10) - 1;
		if (idx < tabs.tabs.length) {
			e.preventDefault();
			tabs.switchTab(idx);
			mouseCursorHidden = true;
			return;
		}
	}

	let handled = true;

	if (matchesKeybind(e, keybinds.newTab)) {
		e.preventDefault();
		tabs.newTab();
	} else if (matchesKeybind(e, keybinds.closeTab)) {
		e.preventDefault();
		tabs.closeTab(tabs.activeIndex);
	} else if (matchesKeybind(e, keybinds.filter)) {
		e.preventDefault();
		if (filterBarVisible) {
			if (fm.filterQuery) {
				// Lock: dismiss bar, keep filter active
				filterBarVisible = false;
			} else {
				// Empty filter: close entirely
				handleFilterClose();
			}
		} else {
			filterBarVisible = true;
			await tick();
			filterBar?.focusInput();
		}
		return;
	} else if (fm.filterQuery && matchesKeybind(e, keybinds.filterNext)) {
		e.preventDefault();
		fm.selectRelativeWrap(1);
	} else if (fm.filterQuery && matchesKeybind(e, keybinds.filterPrev)) {
		e.preventDefault();
		fm.selectRelativeWrap(-1);
	} else if (matchesKeybind(e, keybinds.escape)) {
		e.preventDefault();
		if (fm.searchOpen) {
			handleSearchClose();
		} else if (fm.visualMode) {
			fm.exitVisualMode();
		} else if (filterBarVisible || fm.filterQuery) {
			handleFilterClose();
		} else if (fm.clipboard) {
			fm.clipboard = null;
		} else if (fm.selectedPaths.size > 0) {
			fm.clearMultiSelection();
		} else {
			fm.clearSelection();
		}
	} else if (matchesKeybind(e, keybinds.selectAll)) {
		e.preventDefault();
		fm.selectAll();
	} else if (matchesKeybind(e, keybinds.visualMode)) {
		e.preventDefault();
		if (fm.visualMode) fm.exitVisualMode();
		else fm.enterVisualMode();
	} else if (matchesKeybind(e, keybinds.toggleSelect)) {
		e.preventDefault();
		if (fm.cursorEntry) fm.toggleSelect(fm.cursorEntry);
	} else if (matchesKeybind(e, keybinds.moveDown)) {
		e.preventDefault();
		fm.selectRelative(1);
	} else if (matchesKeybind(e, keybinds.moveUp)) {
		e.preventDefault();
		fm.selectRelative(-1);
	} else if (matchesKeybind(e, keybinds.open)) {
		e.preventDefault();
		if (fm.cursorEntry) ops.handleOpen(fm, fm.cursorEntry, editorApp);
	} else if (matchesKeybind(e, keybinds.enterDir)) {
		e.preventDefault();
		if (fm.cursorEntry?.is_dir) fm.navigate(fm.cursorEntry.path);
	} else if (matchesKeybind(e, keybinds.goParent)) {
		e.preventDefault();
		fm.goUp();
	} else if (matchesKeybind(e, keybinds.toggleHidden)) {
		fm.toggleHidden();
	} else if (matchesKeybind(e, keybinds.yank)) {
		ops.handleCopy(fm);
	} else if (matchesKeybind(e, keybinds.cut)) {
		ops.handleCut(fm);
	} else if (matchesKeybind(e, keybinds.paste)) {
		dlg.handlePaste();
	} else if (matchesKeybind(e, keybinds.permanentDelete)) {
		dlg.handlePermanentDelete();
	} else if (matchesKeybind(e, keybinds.trash)) {
		dlg.handleDelete();
	} else if (matchesKeybind(e, keybinds.rename)) {
		e.preventDefault();
		ops.handleRename(fm);
	} else if (matchesKeybind(e, keybinds.newFolder)) {
		e.preventDefault();
		ops.handleNewFolder(fm);
	} else if (matchesKeybind(e, keybinds.newFile)) {
		e.preventDefault();
		ops.handleNewFile(fm);
	} else if (matchesKeybind(e, keybinds.openMenu)) {
		e.preventDefault();
		if (fm.cursorEntry && contentEl) {
			const row = contentEl.querySelector("tr.cursor");
			if (row) {
				const rect = row.getBoundingClientRect();
				dlg.openContextMenu(
					rect.left + rect.width / 2,
					rect.top + rect.height,
					fm.cursorEntry,
				);
			}
		}
	} else if (matchesKeybind(e, keybinds.openBgMenu)) {
		e.preventDefault();
		if (contentEl) {
			const rect = contentEl.getBoundingClientRect();
			dlg.openContextMenu(
				rect.left + rect.width / 2,
				rect.top + rect.height / 2,
				null,
			);
		}
	} else if (matchesKeybind(e, keybinds.properties)) {
		dlg.handleProperties();
	} else if (matchesKeybind(e, keybinds.cancelClipboard)) {
		fm.clipboard = null;
	} else if (matchesKeybind(e, keybinds.historyBack)) {
		fm.goBack();
	} else if (matchesKeybind(e, keybinds.historyForward)) {
		fm.goForward();
	} else if (matchesKeybind(e, keybinds.togglePreview)) {
		fm.togglePreview();
	} else if (matchesKeybind(e, keybinds.previewShrink)) {
		fm.setPreviewWidth(Math.max(150, fm.previewWidth - 15));
	} else if (matchesKeybind(e, keybinds.previewGrow)) {
		fm.setPreviewWidth(Math.min(800, fm.previewWidth + 15));
	} else if (matchesKeybind(e, keybinds.search)) {
		e.preventDefault();
		if (!fm.searchOpen) {
			fm.openSearch();
			await tick();
			searchOverlay?.focusInput();
		}
	} else if (matchesKeybind(e, keybinds.openInEditor)) {
		if (fm.cursorEntry)
			ops.handleOpenInEditor(fm, fm.cursorEntry.path, editorApp);
	} else if (matchesKeybind(e, keybinds.openTerminal)) {
		if (terminalApp) ops.handleOpenTerminal(fm, terminalApp);
		else fm.setError("No terminal configured — set 'terminal' in config.toml");
	} else if (e.key === "?") {
		e.preventDefault();
		dlg.openHelp();
	} else {
		handled = false;
	}

	if (handled) mouseCursorHidden = true;
}

function restoreFocus() {
	tick().then(() => contentEl?.focus());
}

function handleFilterClose() {
	fm.clearFilter();
	filterBarVisible = false;
}

// --- Search wiring ---

function handleSearchInput(query: string) {
	fm.setSearchQuery(query);
	clearTimeout(searchDebounce);
	if (!query) {
		cancelSearch();
		fm.resetSearchResults();
		return;
	}
	searchDebounce = setTimeout(() => {
		fm.resetSearchResults();
		searchFiles(fm.currentPath, query, fm.showHidden, fm.searchGen);
	}, 250);
}

async function handleSearchClose() {
	clearTimeout(searchDebounce);
	await cancelSearch();
	fm.closeSearch();
}

function handleSearchSelect(result: SearchResult) {
	handleSearchClose();
	const dir = parentPath(result.path);
	fm.navigate(dir, true, result.path);
}

// --- Command palette wiring ---

function buildCommands(): Command[] {
	return buildCommandList({
		// Navigation
		moveDown: () => fm.selectRelative(1),
		moveUp: () => fm.selectRelative(-1),
		open: () => {
			if (fm.cursorEntry) ops.handleOpen(fm, fm.cursorEntry, editorApp);
		},
		enterDir: () => {
			if (fm.cursorEntry?.is_dir) fm.navigate(fm.cursorEntry.path);
		},
		goParent: () => fm.goUp(),
		goTop: () => fm.selectByIndex(0),
		goBottom: () => fm.selectByIndex(fm.filteredEntries.length - 1),
		halfPageUp: () => fm.selectRelative(-15),
		halfPageDown: () => fm.selectRelative(15),
		historyBack: () => fm.goBack(),
		historyForward: () => fm.goForward(),
		focusPath: () => toolbar?.focusPath(),
		// Selection
		toggleSelect: () => {
			if (fm.cursorEntry) fm.toggleSelect(fm.cursorEntry);
		},
		selectAll: () => fm.selectAll(),
		visualMode: () => {
			if (fm.visualMode) fm.exitVisualMode();
			else fm.enterVisualMode();
		},
		escape: () => fm.clearSelection(),
		// File Operations
		yank: () => ops.handleCopy(fm),
		cut: () => ops.handleCut(fm),
		paste: () => dlg.handlePaste(),
		trash: () => dlg.handleDelete(),
		permanentDelete: () => dlg.handlePermanentDelete(),
		rename: () => ops.handleRename(fm),
		newFile: () => ops.handleNewFile(fm),
		newFolder: () => ops.handleNewFolder(fm),
		cancelClipboard: () => {
			fm.clipboard = null;
		},
		// View
		toggleHidden: () => fm.toggleHidden(),
		filter: () => {
			filterBarVisible = true;
			tick().then(() => filterBar?.focusInput());
		},
		filterNext: () => fm.selectRelativeWrap(1),
		filterPrev: () => fm.selectRelativeWrap(-1),
		properties: () => dlg.handleProperties(),
		openMenu: () => {
			if (fm.cursorEntry && contentEl) {
				const row = contentEl.querySelector("tr.cursor");
				if (row) {
					const rect = row.getBoundingClientRect();
					dlg.openContextMenu(
						rect.left + rect.width / 2,
						rect.top + rect.height,
						fm.cursorEntry,
					);
				}
			}
		},
		previewShrink: () =>
			fm.setPreviewWidth(Math.max(150, fm.previewWidth - 15)),
		previewGrow: () => fm.setPreviewWidth(Math.min(800, fm.previewWidth + 15)),
		openInEditor: () => {
			if (fm.cursorEntry)
				ops.handleOpenInEditor(fm, fm.cursorEntry.path, editorApp);
		},
		openTerminal: () => {
			if (terminalApp) ops.handleOpenTerminal(fm, terminalApp);
			else
				fm.setError("No terminal configured — set 'terminal' in config.toml");
		},
		togglePreview: () => fm.togglePreview(),
		search: () => {
			fm.openSearch();
			tick().then(() => searchOverlay?.focusInput());
		},
		commandPalette: () => dlg.openPalette(),
		// Tabs
		newTab: () => tabs.newTab(),
		closeTab: () => tabs.closeTab(tabs.activeIndex),
		// Chords
		goHome: () => fm.navigate(fm.homeDir),
		goDownloads: () => fm.navigate(`${fm.homeDir}/Downloads`),
		goTrash: () => fm.navigate("trash://"),
		sortName: () => fm.handleSort("name"),
		sortSize: () => fm.handleSort("size"),
		sortModified: () => fm.handleSort("modified"),
		copyPath: () => {
			if (fm.cursorEntry) navigator.clipboard.writeText(fm.cursorEntry.path);
		},
		copyFilename: () => {
			if (fm.cursorEntry) navigator.clipboard.writeText(fm.cursorEntry.name);
		},
		nextTab: () => tabs.nextTab(),
		prevTab: () => tabs.prevTab(),
		// Extra
		help: () => dlg.openHelp(),
	});
}

// --- Context menu wiring ---

const menuActions: ContextMenuActions = {
	open: (entry) => ops.handleOpen(fm, entry, editorApp),
	openWith: (entry, pos) =>
		ops.handleOpenWith(fm, entry, pos, (menu) =>
			dlg.openContextMenu(menu.x, menu.y, menu.entry),
		),
	cut: () => ops.handleCut(fm),
	copy: () => ops.handleCopy(fm),
	paste: () => dlg.handlePaste(),
	rename: () => ops.handleRename(fm),
	moveTo: () =>
		ops.handleMoveTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entries)),
	copyTo: () =>
		ops.handleCopyTo(fm, (v) => dlg.openFolderPicker(v.mode, v.entries)),
	delete: () => dlg.handleDelete(),
	extract: dlg.handleExtract,
	extractTo: dlg.handleExtractTo,
	compress: dlg.handleCompress,
	properties: dlg.handleProperties,
	restore: () => ops.handleRestore(fm),
	emptyTrash: () => dlg.handleEmptyTrash(),
	newFolder: () => ops.handleNewFolder(fm),
	newFile: () => ops.handleNewFile(fm),
	toggleHidden: () => fm.toggleHidden(),
	launchApp: (filePath, desktopId) =>
		ops.launchOpenWithApp(fm, filePath, desktopId),
	createSymlink: () => ops.handleCreateSymlink(fm),
	openInEditor: (path: string) => ops.handleOpenInEditor(fm, path, editorApp),
	openTerminal: () => {
		if (terminalApp) ops.handleOpenTerminal(fm, terminalApp);
		else fm.setError("No terminal configured — set 'terminal' in config.toml");
	},
	runCustomAction: async (action) => {
		const entry = dlg.contextMenu?.entry;
		let cmd = action.command;
		if (entry) {
			const dot = entry.name.lastIndexOf(".");
			const nameNoExt = dot > 0 ? entry.name.slice(0, dot) : entry.name;
			cmd = cmd.replaceAll("%f", entry.path);
			cmd = cmd.replaceAll("%n", nameNoExt);
			const selected = fm.effectiveSelection.map((e) => e.path);
			cmd = cmd.replaceAll("%F", selected.join(" "));
		}
		cmd = cmd.replaceAll("%d", fm.currentPath);
		try {
			await runCustomAction(cmd, fm.currentPath, action.refresh);
			if (action.refresh) await fm.refresh();
		} catch (e) {
			fm.setError(errorMessage(e) ?? "Custom action failed");
		}
	},
};

function buildMenuItems() {
	if (!dlg.contextMenu) return [];
	const ctx: ContextMenuContext = dlg.contextMenu.entry
		? {
				kind: "entry",
				entry: dlg.contextMenu.entry,
				x: dlg.contextMenu.x,
				y: dlg.contextMenu.y,
			}
		: { kind: "background" };
	return getContextMenuItems(
		ctx,
		{
			isTrash: fm.isTrash,
			showHidden: fm.showHidden,
			clipboard: fm.clipboard,
			openWithApps: fm.openWithApps,
			terminal: terminalApp,
			customActions,
			multiSelectCount:
				fm.selectedPaths.size > 0
					? fm.selectedPaths.size
					: fm.cursorEntry
						? 1
						: 0,
		},
		menuActions,
	);
}

function clipboardText(): string {
	if (!fm.clipboard) return "";
	const mode = fm.clipboard.mode === "cut" ? "Moving" : "Copied";
	const what =
		fm.clipboard.entries.length === 1
			? fm.clipboard.entries[0].name
			: `${fm.clipboard.entries.length} items`;
	return `${mode}: ${what}`;
}

// --- Drag and drop wiring ---

function handleDragStart(draggedEntries: FileEntry[]) {
	fm.startDrag(draggedEntries);
}

let nativeDragPending = false;

function handleNativeDragOut() {
	if (!fm.isDragging || !dragIconPath || nativeDragPending) return;
	nativeDragPending = true;
	const paths = fm.dragEntries.map((en) => en.path);
	fileList?.cancelDrag();
	fm.endDrag();
	setTimeout(() => {
		startDrag({ item: paths, icon: dragIconPath }, () => {
			nativeDragPending = false;
			fm.refresh();
		}).catch(() => {
			nativeDragPending = false;
		});
	}, 0);
}

function handleDropOnEntry(targetDir: FileEntry, ctrlKey: boolean) {
	if (!fm.isDragging || !targetDir.is_dir) return;
	if (fm.dragEntries.some((en) => en.path === targetDir.path)) {
		fm.endDrag();
		return;
	}
	const mode = ctrlKey ? "copy" : "move";
	const paths = fm.dragEntries.map((en) => en.path);
	fm.endDrag();
	dlg.handleDrop(paths, targetDir.path, mode);
}

function handleDropOnTarget(path: string, ctrlKey: boolean) {
	if (!fm.isDragging) return;
	const draggedPaths = fm.dragEntries.map((en) => en.path);
	const draggedNames = fm.dragEntries.map((en) => en.name);
	fm.endDrag();
	if (path === "trash://") {
		const label =
			draggedNames.length === 1
				? draggedNames[0]
				: `${draggedNames.length} items`;
		dlg.confirm({
			title: "Move to Trash",
			message: `Move ${label} to trash?`,
			confirmLabel: "Move to Trash",
			danger: true,
			onconfirm: async () => {
				dlg.closeConfirm();
				await ops.handleDropToTrash(fm, draggedPaths);
			},
		});
		return;
	}
	const mode = ctrlKey ? "copy" : "move";
	dlg.handleDrop(draggedPaths, path, mode);
}

function handleDragOverTarget(path: string) {
	fm.setDropTarget(path);
}

function handleDragLeaveTarget() {
	fm.setDropTarget(null);
}

$effect(() => {
	const path = fm.currentPath;
	if (path === "trash://") {
		// biome-ignore lint/suspicious/noEmptyBlockStatements: fire-and-forget
		unwatchDirectory().catch(() => {});
		return;
	}
	// biome-ignore lint/suspicious/noEmptyBlockStatements: fire-and-forget
	watchDirectory(path).catch(() => {});
});

let dragIconPath = "";
let dropUnlisten: (() => void) | null = null;

onMount(async () => {
	try {
		const config = await getConfig();
		await applyConfig(config);
	} catch {
		// Config load failed — continue with defaults
	}

	let configDebounceTimer: ReturnType<typeof setTimeout> | null = null;
	await watchConfig();
	configUnlisten = await listen<AppConfig>("config-changed", (e) => {
		if (configDebounceTimer) clearTimeout(configDebounceTimer);
		configDebounceTimer = setTimeout(() => applyConfig(e.payload), 300);
	});

	await tabs.init();
	showWindow();
	await dlg.subscribeProgress();

	searchUnlisten = await listen<{
		results: SearchResult[];
		done: boolean;
		gen: number;
	}>("search-results", (event) => {
		const { results, done, gen } = event.payload;
		if (results.length > 0) fm.appendSearchResults(gen, results);
		if (done) fm.markSearchDone(gen);
	});

	getDragIcon()
		.then((p) => {
			dragIconPath = p;
		})
		.catch(() => {
			// Icon not available in dev — drag-out disabled
		});

	dropUnlisten = await getCurrentWebview().onDragDropEvent((event) => {
		if (event.payload.type === "drop" && event.payload.paths.length > 0) {
			dlg.handleDrop(event.payload.paths, fm.currentPath, "copy");
		}
	});

	let dirRefreshTimer: ReturnType<typeof setTimeout> | null = null;
	dirWatchUnlisten = await listen<string>("directory-changed", (event) => {
		if (event.payload === fm.currentPath) {
			if (dirRefreshTimer) clearTimeout(dirRefreshTimer);
			dirRefreshTimer = setTimeout(() => fm.refresh(), 300);
		}
	});
});

onDestroy(() => {
	dlg.unsubscribeProgress();
	clearTimeout(searchDebounce);
	hlWorker.terminate();
	searchUnlisten?.();
	themeUnlisten?.();
	configUnlisten?.();
	dropUnlisten?.();
	dirWatchUnlisten?.();
	// biome-ignore lint/suspicious/noEmptyBlockStatements: fire-and-forget
	unwatchDirectory().catch(() => {});
});
</script>

<svelte:window onkeydown={handleWindowKeydown} onmousemove={(e) => { if (e.screenX !== lastMousePos.x || e.screenY !== lastMousePos.y) { lastMousePos = { x: e.screenX, y: e.screenY }; mouseCursorHidden = false; }}} />

<div class="app" class:hide-cursor={mouseCursorHidden}>
	<Sidebar currentPath={fm.currentPath} onnavigate={(path) => fm.navigate(path)} drives={fm.drives} homeDir={fm.homeDir} onrefreshdrives={() => fm.refreshDrives()} onerror={(msg) => fm.setError(msg)} collapsed={narrow} isDragging={fm.isDragging} dropTarget={fm.dropTarget} ondragover={handleDragOverTarget} ondrop={handleDropOnTarget} ondragleave={handleDragLeaveTarget} />

	<div class="main-column">
		{#if tabs.tabs.length > 1}
			<TabBar
				tabs={tabs.tabs.map((tab, i) => ({ id: tab.id, label: tabs.tabLabels[i], path: tab.fm.currentPath }))}
				activeIndex={tabs.activeIndex}
				onswitch={tabs.switchTab}
				onclose={tabs.closeTab}
				onnew={tabs.newTab}
				isDragging={fm.isDragging}
				dropTarget={fm.dropTarget}
				ondragover={handleDragOverTarget}
				ondrop={handleDropOnTarget}
				ondragleave={handleDragLeaveTarget}
			/>
		{/if}
		<Toolbar
			bind:this={toolbar}
			canGoBack={fm.historyIndex > 0}
			canGoForward={fm.historyIndex < fm.history.length - 1}
			ongoback={fm.goBack}
			ongoforward={fm.goForward}
			ongoup={fm.goUp}
			currentPath={fm.currentPath}
			onnavigate={fm.navigate}
			isDragging={fm.isDragging}
			dropTarget={fm.dropTarget}
			ondragoverpath={handleDragOverTarget}
			ondroppath={handleDropOnTarget}
			ondragleavepath={handleDragLeaveTarget}
		/>

		{#if fm.error}
			<div class="error-bar">
				<span>{fm.error}</span>
				<button onclick={() => fm.setError(null)}>✕</button>
			</div>
		{/if}

		{#if fm.loading}
			<div class="loading">Loading...</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="content-wrapper">
				{#if fm.searchOpen}
					<SearchOverlay
						bind:this={searchOverlay}
						results={fm.searchResults}
						cursor={fm.searchCursor}
						query={fm.searchQuery}
						searching={!fm.searchDone}
						onchange={handleSearchInput}
						onclose={handleSearchClose}
						onselect={handleSearchSelect}
						oncursorchange={(i) => { fm.searchCursor = i; }}
					/>
				{:else}
				{#if fm.isTrash}
					<div class="context-bar">
						<span class="context-bar-text">
							{fm.entries.length === 0 ? "Trash is empty" : `${fm.entries.length} ${fm.entries.length === 1 ? "item" : "items"} in trash`}
						</span>
						{#if fm.entries.length > 0}
							<button class="context-bar-action" onclick={() => dlg.handleEmptyTrash()}>Empty Trash</button>
						{/if}
					</div>
				{/if}
				{#if filterBarVisible || fm.filterQuery}
					<FilterBar
						bind:this={filterBar}
						query={fm.filterQuery}
						matchCount={fm.filteredEntries.length}
						totalCount={fm.sortedEntries.length}
						isGlob={isGlobPattern(fm.filterQuery)}
						onchange={(q) => fm.setFilterQuery(q)}
						onclose={handleFilterClose}
						onmovedown={() => fm.selectRelative(1)}
						onmoveup={() => fm.selectRelative(-1)}
						onopen={() => { if (fm.cursorEntry) ops.handleOpen(fm, fm.cursorEntry, editorApp); }}
						onaccept={() => { filterBarVisible = false; }}
					/>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="content" bind:this={contentEl} tabindex="-1" oncontextmenu={(e) => ops.handleBgContextMenu(fm, e, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}>
				<FileList
						bind:this={fileList}
						entries={fm.filteredEntries}
						cursorPath={fm.cursorPath}
						selectedPaths={fm.selectedPaths}
						renamingPath={fm.renamingPath}
						creatingEntry={fm.creatingEntry}
						clipboardPaths={fm.clipboard ? new Set(fm.clipboard.entries.map(e => e.path)) : null}
						clipboardMode={fm.clipboard?.mode ?? null}
						isTrash={fm.isTrash}
						dropTarget={fm.dropTarget}
						sortBy={fm.sortBy}
						sortAsc={fm.sortAsc}
						hideModified={compact}
						hideSize={narrow}
						onopen={(entry) => ops.handleOpen(fm, entry, editorApp)}
						onselect={fm.select}
						ontoggleselect={fm.toggleSelect}
						onselectrange={(entry) => { if (fm.cursorEntry) fm.selectRange(fm.cursorEntry, entry); else fm.select(entry); }}
						oncontextmenu={(e, entry) => ops.handleContextMenu(fm, e, entry, (menu) => dlg.openContextMenu(menu.x, menu.y, menu.entry))}
						onsort={fm.handleSort}
						onrename={(entry, name) => ops.commitRename(fm, entry, name)}
						oncreate={(name) => ops.commitCreate(fm, name)}
						ondragstartentries={handleDragStart}
						ondropentry={handleDropOnEntry}
						ondragoverentry={(entry) => { if (!fm.dragEntries.some((en) => en.path === entry.path)) fm.setDropTarget(entry.path); }}
						ondragleaveentry={() => fm.setDropTarget(null)}
						ondragleavewindow={handleNativeDragOut}
					/>
				{#if fm.previewEnabled && !compact}
					<PreviewPanel
						entry={fm.cursorEntry}
						{previewData}
						{pdfPreview}
						{previewLoading}
						{previewError}
						{highlightedHtml}
						{imagePreviewUrl}
						{dirPreviewEntries}
						width={fm.previewWidth}
						onresize={(w) => fm.setPreviewWidth(w)}
					/>
				{/if}
				</div>
				{/if}
			</div>
		{/if}

		<StatusBar
			itemCount={fm.filteredEntries.length}
			showHidden={fm.showHidden}
			previewEnabled={fm.previewEnabled}
			ontogglehidden={fm.toggleHidden}
			ontogglepreview={() => fm.togglePreview()}
			onopenhelp={dlg.openHelp}
			overlayText={fm.visualMode
				? `VISUAL — ${fm.selectedPaths.size} ${fm.selectedPaths.size === 1 ? 'item' : 'items'}`
				: fm.selectedPaths.size > 0
					? `${fm.selectedPaths.size} ${fm.selectedPaths.size === 1 ? 'item' : 'items'} selected`
					: fm.clipboard
						? clipboardText()
						: null}
			onclearoverlay={() => {
				if (fm.visualMode) fm.exitVisualMode();
				else if (fm.selectedPaths.size > 0) fm.clearMultiSelection();
				else if (fm.clipboard) fm.clipboard = null;
			}}
		/>
	</div>
</div>

{#if dlg.contextMenu}
	<ContextMenu
		x={dlg.contextMenu.x}
		y={dlg.contextMenu.y}
		items={buildMenuItems()}
		onclose={() => { dlg.closeContextMenu(); restoreFocus(); }}
	/>
{/if}

{#if dlg.propertiesData}
	<PropertiesDialog
		properties={dlg.propertiesData}
		onclose={() => { dlg.closeProperties(); restoreFocus(); }}
	/>
{/if}

{#if dlg.folderPicker}
	<FolderPicker
		title={dlg.folderPicker.mode === "move" ? "Move to\u2026" : dlg.folderPicker.mode === "extract" ? "Extract to\u2026" : "Copy to\u2026"}
		onselect={dlg.handleFolderPickerSelect}
		onclose={() => { dlg.closeFolderPicker(); restoreFocus(); }}
	/>
{/if}

{#if dlg.compressEntries.length > 0}
	<CompressDialog
		defaultName={dlg.compressEntries.length === 1 ? dlg.compressEntries[0].name : "archive"}
		onconfirm={dlg.handleCompressConfirm}
		onclose={() => { dlg.closeCompress(); restoreFocus(); }}
	/>
{/if}

{#if dlg.confirmDialog}
	<ConfirmDialog
		title={dlg.confirmDialog.title}
		message={dlg.confirmDialog.message}
		confirmLabel={dlg.confirmDialog.confirmLabel}
		danger={dlg.confirmDialog.danger}
		onconfirm={dlg.confirmDialog.onconfirm}
		onclose={() => { dlg.closeConfirm(); restoreFocus(); }}
	/>
{/if}

{#if dlg.busyMessage}
	<BusyOverlay
		message={dlg.busyMessage}
		progress={dlg.busyProgress}
		mode={dlg.progressMode}
		oncancel={dlg.handleCancelOperation}
	/>
{/if}

{#if dlg.helpOpen}
	<HelpDialog onclose={() => { dlg.closeHelp(); restoreFocus(); }} />
{/if}

{#if dlg.paletteOpen}
	<CommandPalette
		commands={buildCommands()}
		onclose={() => { dlg.closePalette(); restoreFocus(); }}
		onexecute={(cmd) => { dlg.closePalette(); restoreFocus(); cmd.execute(); }}
	/>
{/if}

<style>
	.app {
		display: flex;
		height: 100vh;
	}

	.main-column {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.app.hide-cursor {
		cursor: none;
	}

	.app.hide-cursor :global(*) {
		cursor: none !important;
	}

	.error-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 12px;
		background: color-mix(in srgb, var(--danger) 15%, var(--bg-secondary));
		color: var(--danger);
		font-size: 13px;
	}

	.error-bar button {
		background: none;
		border: none;
		color: var(--danger);
		cursor: pointer;
		font-size: 14px;
	}

	.loading {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
		background: var(--bg-primary);
	}

	.content-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: var(--bg-primary);
	}

	.content {
		flex: 1;
		display: flex;
		overflow: hidden;
	}

	.context-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 12px;
		background: var(--bg-secondary);
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.context-bar-text {
		font-size: 12px;
		color: var(--text-muted);
	}

	.context-bar-action {
		background: none;
		border: none;
		color: var(--danger);
		font-size: 12px;
		font-family: var(--font-sans);
		cursor: pointer;
		padding: 2px 8px;
		border-radius: var(--radius);
		transition: background var(--transition-normal);
	}

	.context-bar-action:hover {
		background: color-mix(in srgb, var(--danger) 15%, transparent);
	}
</style>
