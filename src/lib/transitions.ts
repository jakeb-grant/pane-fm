import type { TransitionConfig } from "svelte/transition";

const DURATION = 150;
const DURATION_FAST = 100;

const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
let reduced = motionQuery.matches;
motionQuery.addEventListener("change", (e) => {
	reduced = e.matches;
});

function dur(ms: number): number {
	return reduced ? 0 : ms;
}

export function overlayFade(_node: Element): TransitionConfig {
	return { duration: dur(DURATION), css: (t) => `opacity: ${t}` };
}

export function dialogPop(_node: Element): TransitionConfig {
	return {
		duration: dur(DURATION),
		css: (t) => `opacity: ${t}; transform: scale(${0.95 + 0.05 * t})`,
	};
}

export function menuPop(_node: Element): TransitionConfig {
	return {
		duration: dur(DURATION_FAST),
		css: (t) => `opacity: ${t}; transform: scale(${0.9 + 0.1 * t})`,
	};
}

export function flyDown(_node: Element): TransitionConfig {
	return {
		duration: dur(DURATION),
		css: (t) => `opacity: ${t}; transform: translateY(${(1 - t) * -8}px)`,
	};
}

export function slideDown(_node: Element): TransitionConfig {
	return {
		duration: dur(DURATION_FAST),
		css: (t) => `opacity: ${t}; transform: translateY(${(1 - t) * -4}px)`,
	};
}
