import { tick } from "svelte";
import type { FileEntry } from "$lib/commands";

interface EditLogicParams {
	entries: () => FileEntry[];
	renamingPath: () => string | null;
	creatingEntry: () => "file" | "directory" | null;
	onrename: (entry: FileEntry, newName: string) => void;
	oncreate: (name: string) => void;
}

export function createEditLogic(params: EditLogicParams) {
	let editValue = $state("");
	let editInput = $state<HTMLInputElement>();

	function focusAndSelect(entry?: { is_dir: boolean; name: string }) {
		tick().then(() => {
			if (!editInput) return;
			editInput.focus();
			if (entry && !entry.is_dir) {
				const dotIndex = entry.name.lastIndexOf(".");
				if (dotIndex > 0) {
					editInput.setSelectionRange(0, dotIndex);
					return;
				}
			}
			editInput.select();
		});
	}

	$effect(() => {
		const path = params.renamingPath();
		if (path) {
			const entry = params.entries().find((e) => e.path === path);
			if (entry) {
				editValue = entry.name;
				focusAndSelect(entry);
			}
		}
	});

	$effect(() => {
		const creating = params.creatingEntry();
		if (creating) {
			editValue = creating === "directory" ? "New Folder" : "New File";
			focusAndSelect();
		}
	});

	function commitRenameForEntry(entry: FileEntry) {
		params.onrename(entry, editValue.trim());
	}

	function commitCreateEntry() {
		params.oncreate(editValue.trim());
	}

	return {
		get editValue() {
			return editValue;
		},
		set editValue(v: string) {
			editValue = v;
		},
		get editInput() {
			return editInput;
		},
		set editInput(v: HTMLInputElement | undefined) {
			editInput = v;
		},
		commitRenameForEntry,
		commitCreateEntry,
	};
}
