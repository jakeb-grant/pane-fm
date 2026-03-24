export interface Keybind {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
	alt?: boolean;
	meta?: boolean;
}

type KeybindDef = string | Keybind | (string | Keybind)[];

export interface ChordDef {
	keys: [string, string];
}

export type ChordName =
	| "goTop"
	| "goHome"
	| "goDownloads"
	| "goTrash"
	| "nextTab"
	| "prevTab"
	| "sortName"
	| "sortSize"
	| "sortModified"
	| "copyPath"
	| "copyFilename";

export const chords: Record<string, ChordDef> = {
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
};

export let chordPrefixes: Set<string> = new Set(
	Object.values(chords).map((c) => c.keys[0]),
);

export const keybinds: Record<string, KeybindDef> = {
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

	openTerminal: "`",
	togglePreview: "P",

	// Dialog keybinds
	openMenu: "o",
	confirm: "y",
	deny: "n",
	menuDown: ["j", "ArrowDown"],
	menuUp: ["k", "ArrowUp"],
	menuAccept: ["l", "Enter"],
	menuBack: "h",
	menuClose: ["q", "Escape"],
};

const defaultKeybinds: Record<string, KeybindDef> = { ...keybinds };
const defaultChords: Record<string, ChordDef> = { ...chords };

export function resetKeybinds(): void {
	for (const key of Object.keys(keybinds)) delete keybinds[key];
	Object.assign(keybinds, defaultKeybinds);
	for (const key of Object.keys(chords)) delete chords[key];
	Object.assign(chords, defaultChords);
	chordPrefixes = new Set(Object.values(chords).map((c) => c.keys[0]));
}

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

function snakeToCamel(s: string): string {
	return s.replace(/_([a-z])/g, (_, c) => c.toUpperCase());
}

function parseKeybindString(s: string): string | Keybind {
	const parts = s.split("+");
	if (parts.length === 1) return s;
	const key = parts[parts.length - 1];
	const mods = new Set(parts.slice(0, -1).map((m) => m.toLowerCase()));
	return {
		key,
		ctrl: mods.has("ctrl") || undefined,
		shift: mods.has("shift") || undefined,
		alt: mods.has("alt") || undefined,
		meta: mods.has("meta") || undefined,
	};
}

function parseKeybindValue(
	value: string | string[],
): string | Keybind | (string | Keybind)[] {
	if (typeof value === "string") return parseKeybindString(value);
	return value.map(parseKeybindString);
}

export function applyKeybindOverrides(
	keybindOverrides: Record<string, string | string[]>,
	chordOverrides: Record<string, string[]>,
): void {
	for (const [snakeKey, value] of Object.entries(keybindOverrides)) {
		const camelKey = snakeToCamel(snakeKey);
		keybinds[camelKey] = parseKeybindValue(value);
	}

	for (const [snakeKey, keys] of Object.entries(chordOverrides)) {
		const camelKey = snakeToCamel(snakeKey);
		if (keys.length === 2) {
			chords[camelKey] = { keys: [keys[0], keys[1]] };
		}
	}

	chordPrefixes = new Set(Object.values(chords).map((c) => c.keys[0]));
}
