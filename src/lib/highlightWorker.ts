import type { HighlightRequest, HighlightResponse } from "./highlight";
import { highlightCode } from "./highlight";

self.onmessage = (e: MessageEvent<HighlightRequest>) => {
	const { code, filename, gen } = e.data;
	const html = highlightCode(code, filename);
	self.postMessage({ html, gen } satisfies HighlightResponse);
};
