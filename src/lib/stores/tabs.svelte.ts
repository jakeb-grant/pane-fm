import {
	createFileManager,
	type FileManager,
} from "$lib/stores/fileManager.svelte";

interface Tab {
	id: number;
	fm: FileManager;
}

const MAX_TABS = 9;

export function createTabManager() {
	let nextId = 1;
	const initialFm = createFileManager();
	let tabs = $state<Tab[]>([{ id: nextId++, fm: initialFm }]);
	let activeIndex = $state(0);

	const activeFm = $derived(tabs[activeIndex].fm);

	const tabLabels = $derived(
		tabs.map((tab) => {
			const path = tab.fm.currentPath;
			if (path === "trash://") return "Trash";
			if (path === "/") return "/";
			return path.split("/").pop() || "/";
		}),
	);

	type Clipboard = FileManager["clipboard"];

	function leaveTab(): Clipboard {
		const old = tabs[activeIndex].fm;
		const clipboard = old.clipboard;
		old.clearTransient();
		old.clipboard = null;
		return clipboard;
	}

	function enterTab(clipboard: Clipboard) {
		if (clipboard) {
			tabs[activeIndex].fm.clipboard = clipboard;
		}
	}

	function transition(newIndex: number) {
		if (newIndex === activeIndex) return;
		const clipboard = leaveTab();
		activeIndex = newIndex;
		enterTab(clipboard);
	}

	function newTab() {
		if (tabs.length >= MAX_TABS) return;
		const fm = createFileManager();
		const tab: Tab = { id: nextId++, fm };
		const currentDir = activeFm.currentPath;
		const clipboard = leaveTab();
		tabs = [...tabs, tab];
		activeIndex = tabs.length - 1;
		enterTab(clipboard);
		fm.navigate(currentDir);
	}

	function closeTab(index: number) {
		if (index < 0 || index >= tabs.length) return;
		if (tabs.length === 1) {
			tabs[0].fm.navigate(tabs[0].fm.homeDir);
			return;
		}
		const clipboard = tabs[index].fm.clipboard;
		tabs = tabs.filter((_, i) => i !== index);
		if (activeIndex >= tabs.length) {
			activeIndex = tabs.length - 1;
		} else if (activeIndex > index) {
			activeIndex--;
		}
		if (clipboard) {
			tabs[activeIndex].fm.clipboard = clipboard;
		}
	}

	function switchTab(index: number) {
		if (index >= 0 && index < tabs.length) {
			transition(index);
		}
	}

	function nextTab() {
		if (tabs.length <= 1) return;
		transition((activeIndex + 1) % tabs.length);
	}

	function prevTab() {
		if (tabs.length <= 1) return;
		transition((activeIndex - 1 + tabs.length) % tabs.length);
	}

	async function init() {
		await tabs[0].fm.init();
	}

	return {
		get tabs() {
			return tabs;
		},
		get activeIndex() {
			return activeIndex;
		},
		get activeFm() {
			return activeFm;
		},
		get tabLabels() {
			return tabLabels;
		},

		newTab,
		closeTab,
		switchTab,
		nextTab,
		prevTab,
		init,
	};
}

export type TabManager = ReturnType<typeof createTabManager>;
