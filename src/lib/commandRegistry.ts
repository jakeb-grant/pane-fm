import { chords, type KeybindDef, keybindLabel, keybinds } from "$lib/keybinds";

export interface Command {
	id: string;
	label: string;
	category: string;
	keybind: string;
	execute: () => void;
}

export const categories: { label: string; keys: string[] }[] = [
	{
		label: "Navigation",
		keys: [
			"moveDown",
			"moveUp",
			"open",
			"enterDir",
			"goParent",
			"goTop",
			"goBottom",
			"halfPageUp",
			"halfPageDown",
			"historyBack",
			"historyForward",
			"focusPath",
		],
	},
	{
		label: "Selection",
		keys: ["toggleSelect", "selectAll", "visualMode", "escape"],
	},
	{
		label: "File Operations",
		keys: [
			"yank",
			"cut",
			"paste",
			"trash",
			"permanentDelete",
			"rename",
			"newFile",
			"newFolder",
			"cancelClipboard",
		],
	},
	{
		label: "View",
		keys: [
			"toggleHidden",
			"filter",
			"filterNext",
			"filterPrev",
			"properties",
			"openMenu",
			"openTerminal",
			"togglePreview",
			"search",
			"commandPalette",
		],
	},
	{
		label: "Tabs",
		keys: ["newTab", "closeTab"],
	},
];

export const chordCategories: { label: string; keys: string[] }[] = [
	{
		label: "Go To",
		keys: ["goTop", "goHome", "goDownloads", "goTrash", "nextTab", "prevTab"],
	},
	{
		label: "Sort",
		keys: ["sortName", "sortSize", "sortModified"],
	},
	{
		label: "Clipboard",
		keys: ["copyPath", "copyFilename"],
	},
];

export function formatName(camel: string): string {
	return camel
		.replace(/([A-Z])/g, " $1")
		.replace(/^./, (s) => s.toUpperCase())
		.trim();
}

export function allBindLabels(bind: KeybindDef): string {
	if (Array.isArray(bind)) {
		return bind.map((b) => keybindLabel(b as string)).join("  /  ");
	}
	return keybindLabel(bind as string);
}

export function chordLabel(keys: [string, string]): string {
	return `${keys[0]} ${keys[1]}`;
}

export function buildCommandList(
	actions: Record<string, () => void>,
): Command[] {
	const cmds: Command[] = [];

	for (const cat of categories) {
		for (const key of cat.keys) {
			if (!keybinds[key] || !actions[key]) continue;
			cmds.push({
				id: key,
				label: formatName(key),
				category: cat.label,
				keybind: allBindLabels(keybinds[key]),
				execute: actions[key],
			});
		}
	}

	for (const cat of chordCategories) {
		for (const key of cat.keys) {
			if (!chords[key] || !actions[key]) continue;
			cmds.push({
				id: key,
				label: formatName(key),
				category: `Chord: ${cat.label}`,
				keybind: chordLabel(chords[key].keys),
				execute: actions[key],
			});
		}
	}

	// Extra commands passed via actions with special IDs
	if (actions.help) {
		cmds.push({
			id: "help",
			label: "Open Help",
			category: "General",
			keybind: "?",
			execute: actions.help,
		});
	}

	return cmds;
}
