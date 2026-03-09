import type { FileEntry } from "$lib/commands";
import type { MenuEntry } from "$lib/components/ContextMenu.svelte";
import { archiveExtensions } from "$lib/constants";

// --- Discriminated union for context menu context ---

export interface EntryContext {
	kind: "entry";
	entry: FileEntry;
	x: number;
	y: number;
}

export interface BackgroundContext {
	kind: "background";
}

export type ContextMenuContext = EntryContext | BackgroundContext;

// --- Action callbacks the builder needs ---

export interface ContextMenuActions {
	open: (entry: FileEntry) => void;
	openWith: (entry: FileEntry, position: { x: number; y: number }) => void;
	cut: () => void;
	copy: () => void;
	paste: () => void;
	rename: () => void;
	moveTo: () => void;
	copyTo: () => void;
	delete: () => void;
	extract: () => void;
	extractTo: () => void;
	compress: () => void;
	properties: () => void;
	restore: () => void;
	emptyTrash: () => void;
	newFolder: () => void;
	newFile: () => void;
	toggleHidden: () => void;
	launchApp: (filePath: string, desktopId: string) => void;
}

// --- State the builder reads ---

export interface ContextMenuState {
	isTrash: boolean;
	showHidden: boolean;
	clipboard: { entries: FileEntry[]; mode: "copy" | "cut" } | null;
	openWithApps: Array<{ name: string; desktop_id: string; icon: string }>;
	multiSelectCount: number;
}

// --- Builders ---

function isArchive(entry: FileEntry): boolean {
	return !entry.is_dir && archiveExtensions.test(entry.name);
}

function buildOpenWithItems(
	ctx: EntryContext,
	state: ContextMenuState,
	actions: ContextMenuActions,
): MenuEntry[] {
	return state.openWithApps.map((app) => ({
		label: app.name,
		action: () => actions.launchApp(ctx.entry.path, app.desktop_id),
	}));
}

function buildTrashEntryItems(actions: ContextMenuActions): MenuEntry[] {
	return [
		{ label: "Restore", action: actions.restore },
		{ label: "Delete Permanently", action: actions.delete, danger: true },
	];
}

function buildTrashBgItems(actions: ContextMenuActions): MenuEntry[] {
	return [{ label: "Empty Trash", action: actions.emptyTrash, danger: true }];
}

function buildEntryItems(
	ctx: EntryContext,
	state: ContextMenuState,
	actions: ContextMenuActions,
): MenuEntry[] {
	const { entry } = ctx;
	const multi = state.multiSelectCount > 1;
	const n = state.multiSelectCount;
	const items: MenuEntry[] = [];

	if (!multi) {
		items.push(
			{ label: "Open", action: () => actions.open(entry) },
			{
				label: "Open With\u2026",
				action: () => actions.openWith(entry, { x: ctx.x, y: ctx.y }),
			},
			{ separator: true },
		);
	}

	items.push(
		{ label: multi ? `Cut ${n} Items` : "Cut", action: actions.cut },
		{ label: multi ? `Copy ${n} Items` : "Copy", action: actions.copy },
		{
			label: multi ? `Move ${n} Items to\u2026` : "Move to\u2026",
			action: actions.moveTo,
		},
		{
			label: multi ? `Copy ${n} Items to\u2026` : "Copy to\u2026",
			action: actions.copyTo,
		},
	);

	if (!multi) {
		items.push({ label: "Rename", action: actions.rename });
	}

	items.push({ separator: true });

	if (!multi && isArchive(entry)) {
		items.push(
			{ label: "Extract Here", action: actions.extract },
			{ label: "Extract to Folder\u2026", action: actions.extractTo },
		);
	}

	items.push(
		{
			// biome-ignore lint/security/noSecrets: ellipsis character, not a secret
			label: multi ? `Compress ${n} Items\u2026` : "Compress\u2026",
			action: actions.compress,
		},
		{ separator: true },
		{
			label: multi ? `Move ${n} Items to Trash` : "Move to Trash",
			action: actions.delete,
			danger: true,
		},
	);

	if (!multi) {
		items.push(
			{ separator: true },
			{ label: "Properties", action: actions.properties },
		);
	}

	return items;
}

function buildBgItems(
	state: ContextMenuState,
	actions: ContextMenuActions,
): MenuEntry[] {
	const items: MenuEntry[] = [];

	if (state.clipboard) {
		const pasteLabel =
			state.clipboard.entries.length === 1
				? `Paste \u201C${state.clipboard.entries[0].name}\u201D`
				: `Paste ${state.clipboard.entries.length} items`;
		items.push(
			{ label: pasteLabel, action: actions.paste },
			{ separator: true },
		);
	}

	items.push(
		{ label: "New Folder", action: actions.newFolder },
		{ label: "New File", action: actions.newFile },
		{ separator: true },
		{
			label: state.showHidden ? "Hide Hidden Files" : "Show Hidden Files",
			action: actions.toggleHidden,
		},
	);

	return items;
}

// --- Main entry point ---

export function getContextMenuItems(
	ctx: ContextMenuContext,
	state: ContextMenuState,
	actions: ContextMenuActions,
): MenuEntry[] {
	if (state.isTrash) {
		if (ctx.kind === "entry") return buildTrashEntryItems(actions);
		return buildTrashBgItems(actions);
	}

	if (ctx.kind === "entry") {
		// "Open With" submenu mode
		if (state.openWithApps.length > 0) {
			return buildOpenWithItems(ctx, state, actions);
		}
		return buildEntryItems(ctx, state, actions);
	}

	return buildBgItems(state, actions);
}
