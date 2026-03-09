export interface Keybind {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
	alt?: boolean;
	meta?: boolean;
}

type KeybindDef = string | Keybind | readonly (string | Keybind)[];

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
	rename: "r",
	newFile: "a",
	newFolder: { key: "A", shift: true },
	cancelClipboard: ["Y", "X"],
	historyBack: "H",
	historyForward: "L",
	properties: "i",
	focusPath: { key: "l", ctrl: true },
	halfPageUp: { key: "u", ctrl: true },
	halfPageDown: { key: "d", ctrl: true },
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

export function matchesKeybind(e: KeyboardEvent, bind: KeybindDef): boolean {
	if (typeof bind === "string" || !Array.isArray(bind)) {
		return matchesSingle(e, bind as string | Keybind);
	}
	return bind.some((b) => matchesSingle(e, b));
}
