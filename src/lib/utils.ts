export function formatSize(bytes: number): string {
	if (bytes === 0) return "—";
	const units = ["B", "KB", "MB", "GB", "TB"];
	const i = Math.floor(Math.log(bytes) / Math.log(1024));
	const size = bytes / 1024 ** i;
	return `${size.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function parentPath(path: string): string {
	const parts = path.split("/").filter(Boolean);
	if (parts.length <= 1) return "/";
	parts.pop();
	return `/${parts.join("/")}`;
}

export function fuzzyMatch(query: string, text: string): boolean {
	const q = query.toLowerCase();
	const t = text.toLowerCase();
	let qi = 0;
	for (let ti = 0; ti < t.length && qi < q.length; ti++) {
		if (t[ti] === q[qi]) qi++;
	}
	return qi === q.length;
}

export function pathSegments(path: string): { name: string; path: string }[] {
	const parts = path.split("/").filter(Boolean);
	const segments: { name: string; path: string }[] = [{ name: "/", path: "/" }];
	let current = "";
	for (const part of parts) {
		current += `/${part}`;
		segments.push({ name: part, path: current });
	}
	return segments;
}
