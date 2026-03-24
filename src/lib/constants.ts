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

export function isTextPreviewable(mime: string): boolean {
	return mime.startsWith("text/") || textAppMimes.has(mime);
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
