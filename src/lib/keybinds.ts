export interface Keybind {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
	alt?: boolean;
	meta?: boolean;
}

type KeybindDef = string | Keybind | readonly (string | Keybind)[];

export interface ChordDef {
	keys: readonly [string, string];
}

export const chords = {
	goTop: { keys: ["g", "g"] },
	goHome: { keys: ["g", "h"] },
	goDownloads: { keys: ["g", "d"] },
	goTrash: { keys: ["g", "x"] },
	nextTab: { keys: ["g", "t"] },
	prevTab: { keys: ["g", "T"] },
	sortName: { keys: [",", "n"] },
	sortSize: { keys: [",", "s"] },
	sortModified: { keys: [",", "m"] },
	copyPath: { keys: ["c", "c"] },
	copyFilename: { keys: ["c", "f"] },
} as const satisfies Record<string, ChordDef>;

export type ChordName = keyof typeof chords;

export const chordPrefixes: Set<string> = new Set(
	Object.values(chords).map((c) => c.keys[0]),
);

export const keybinds = {
	filter: "/",
	moveDown: ["j", "ArrowDown"],
	moveUp: ["k", "ArrowUp"],
	open: ["l", "Enter"],
	enterDir: "ArrowRight",
	goParent: ["h", "ArrowLeft"],
	goTop: "Home",
	goBottom: ["G", "End"],
	toggleHidden: ".",
	toggleSelect: " ",
	selectAll: { key: "a", ctrl: true },
	escape: "Escape",
	yank: "y",
	cut: "x",
	paste: "p",
	trash: "d",
	permanentDelete: { key: "D", shift: true },
	rename: "r",
	newFile: "a",
	newFolder: { key: "A", shift: true },
	cancelClipboard: ["Y", "X"],
	historyBack: "H",
	historyForward: "L",
	visualMode: "v",
	filterNext: "n",
	filterPrev: { key: "N", shift: true },
	properties: "i",
	focusPath: { key: "l", ctrl: true },
	halfPageUp: { key: "u", ctrl: true },
	halfPageDown: { key: "d", ctrl: true },

	// Tab keybinds
	newTab: "t",
	closeTab: "q",

	// Dialog keybinds
	openMenu: "o",
	confirm: "y",
	deny: "n",
	menuDown: ["j", "ArrowDown"],
	menuUp: ["k", "ArrowUp"],
	menuAccept: ["l", "Enter"],
	menuBack: "h",
	menuClose: ["q", "Escape"],
} as const satisfies Record<string, KeybindDef>;

function matchesSingle(e: KeyboardEvent, bind: string | Keybind): boolean {
	if (typeof bind === "string") {
		return e.key === bind && !e.ctrlKey && !e.altKey && !e.metaKey;
	}
	return (
		e.key === bind.key &&
		!!e.ctrlKey === !!bind.ctrl &&
		!!e.altKey === !!bind.alt &&
		!!e.metaKey === !!bind.meta
	);
}

export function keybindLabel(bind: KeybindDef): string {
	const single = Array.isArray(bind) ? bind[0] : bind;
	if (typeof single === "string") return single;
	const parts: string[] = [];
	if (single.ctrl) parts.push("Ctrl");
	if (single.alt) parts.push("Alt");
	if (single.meta) parts.push("Meta");
	if (single.shift) parts.push("Shift");
	parts.push(single.key);
	return parts.join("+");
}

export function matchesKeybind(e: KeyboardEvent, bind: KeybindDef): boolean {
	if (typeof bind === "string" || !Array.isArray(bind)) {
		return matchesSingle(e, bind as string | Keybind);
	}
	return bind.some((b) => matchesSingle(e, b));
}
