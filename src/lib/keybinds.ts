export const keybinds = {
	filter: "/",
	moveDown: ["j", "ArrowDown"],
	moveUp: ["k", "ArrowUp"],
	open: ["l", "ArrowRight", "Enter"],
	goParent: ["h", "ArrowLeft"],
	goTop: "Home",
	goBottom: "End",
	toggleHidden: ".",
} as const;

export function matchesKeybind(
	key: string,
	bind: string | readonly string[],
): boolean {
	return Array.isArray(bind) ? bind.includes(key) : key === bind;
}
