/** Structured error from the Rust backend, serialized via Tauri's InvokeError. */
export type AppError =
	| { kind: "Io"; message: string; path?: string }
	| { kind: "NotFound"; path: string }
	| { kind: "PermissionDenied"; path: string }
	| { kind: "AlreadyExists"; path: string }
	| { kind: "Cancelled" }
	| { kind: "Archive"; message: string }
	| { kind: "Desktop"; message: string }
	| { kind: "Trash"; message: string };

export function isAppError(e: unknown): e is AppError {
	return (
		typeof e === "object" &&
		e !== null &&
		"kind" in e &&
		typeof (e as AppError).kind === "string"
	);
}

export function isCancelled(e: unknown): boolean {
	return isAppError(e) && e.kind === "Cancelled";
}

/** Convert any caught error into a user-friendly message, or null for Cancelled. */
export function errorMessage(e: unknown): string | null {
	if (isCancelled(e)) return null;

	if (!isAppError(e)) return String(e);

	switch (e.kind) {
		case "NotFound":
			return `Not found: ${e.path}`;
		case "PermissionDenied":
			return `Permission denied: ${e.path}`;
		case "AlreadyExists":
			return `Already exists: ${e.path}`;
		case "Io":
			return e.path ? `${e.message} (${e.path})` : e.message;
		case "Archive":
			return e.message;
		case "Desktop":
			return e.message;
		case "Trash":
			return e.message;
		default:
			return String(e);
	}
}
