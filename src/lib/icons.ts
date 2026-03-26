import {
	DEFAULT_FILE,
	DEFAULT_FOLDER,
	EXT_ICONS,
	FOLDER_ICONS,
	LIGHT_EXT_ICONS,
	LIGHT_FOLDER_ICONS,
	LIGHT_NAME_ICONS,
	NAME_ICONS,
} from "./icons.gen";

export { DEFAULT_FOLDER } from "./icons.gen";

export type IconMode = "dark" | "light";

let currentMode: IconMode = "dark";

export function setIconMode(mode: IconMode) {
	currentMode = mode;
}

export function getIconForEntry(entry: {
	is_dir: boolean;
	name: string;
}): string {
	const nameLower = entry.name.toLowerCase();
	const isLight = currentMode === "light";

	if (entry.is_dir) {
		if (isLight) {
			const lightIcon = LIGHT_FOLDER_ICONS[nameLower];
			if (lightIcon) return lightIcon;
		}
		return FOLDER_ICONS[nameLower] ?? DEFAULT_FOLDER;
	}

	// Exact filename match
	if (isLight) {
		const lightIcon = LIGHT_NAME_ICONS[nameLower];
		if (lightIcon) return lightIcon;
	}
	const nameIcon = NAME_ICONS[nameLower];
	if (nameIcon) return nameIcon;

	// Compound extension (e.g., spec.ts, d.ts)
	const parts = entry.name.split(".");
	if (parts.length > 2) {
		const compoundExt = parts.slice(-2).join(".").toLowerCase();
		if (isLight) {
			const lightIcon = LIGHT_EXT_ICONS[compoundExt];
			if (lightIcon) return lightIcon;
		}
		const compoundIcon = EXT_ICONS[compoundExt];
		if (compoundIcon) return compoundIcon;
	}

	// Simple extension
	const ext = parts.pop()?.toLowerCase() ?? "";
	if (isLight) {
		const lightIcon = LIGHT_EXT_ICONS[ext];
		if (lightIcon) return lightIcon;
	}
	const extIcon = EXT_ICONS[ext];
	if (extIcon) return extIcon;

	return DEFAULT_FILE;
}
