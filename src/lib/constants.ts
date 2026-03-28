import { extToLang, nameToLang } from "$lib/highlight";

export const archiveExtensions =
	/\.(zip|tar|tar\.gz|tgz|tar\.xz|tar\.bz2|tar\.zst)$/i;

const textAppMimes = new Set([
	"application/json",
	"application/xml",
	"application/javascript",
	"application/toml",
	"application/yaml",
	"application/x-yaml",
	"application/x-sh",
	"application/x-shellscript",
]);

const extraTextExtensions = [
	"qml",
	"svelte",
	"vue",
	"astro",
	"mts",
	"cts",
	"cfg",
	"env",
	"properties",
	"hrl",
	"fs",
	"fsi",
	"fsx",
	"cljc",
	"edn",
	"sbt",
	"groovy",
	"gradle",
	"scm",
	"rkt",
	"csh",
	"tcsh",
	"ps1",
	"psm1",
	"mk",
	"just",
	"gitignore",
	"gitattributes",
	"gitmodules",
	"editorconfig",
	"eslintrc",
	"prettierrc",
	"babelrc",
	"lock",
	"sum",
	"mod",
	"service",
	"timer",
	"socket",
	"mount",
	"desktop",
	"kdl",
	"ron",
	"pkl",
];

const textExtensions = new Set([
	...Object.keys(extToLang),
	...Object.keys(nameToLang),
	...extraTextExtensions,
]);

export function isTextPreviewable(mime: string, filename?: string): boolean {
	if (mime.startsWith("text/") || textAppMimes.has(mime)) return true;
	if (filename) {
		const ext = filename.split(".").pop()?.toLowerCase() ?? "";
		if (textExtensions.has(ext)) return true;
		const base = filename.toLowerCase();
		if (textExtensions.has(base)) return true;
		// Extensionless files with unknown MIME — likely text (PKGBUILD, LICENSE, etc.)
		if (mime === "application/octet-stream" && !filename.includes("."))
			return true;
	}
	return false;
}

const imagePreviewMimes = new Set([
	"image/png",
	"image/jpeg",
	"image/gif",
	"image/webp",
	"image/svg+xml",
	"image/avif",
	"image/bmp",
]);

export function isImagePreviewable(mime: string): boolean {
	return imagePreviewMimes.has(mime);
}

export function isPdfPreviewable(mime: string): boolean {
	return mime === "application/pdf";
}
