<script lang="ts">
import { keybindLabel, keybinds, matchesKeybind } from "$lib/keybinds";
import { dialogPop, overlayFade } from "$lib/transitions";

let {
	title,
	message,
	confirmLabel = "Confirm",
	danger = false,
	onconfirm,
	onclose,
}: {
	title: string;
	message: string;
	confirmLabel?: string;
	danger?: boolean;
	onconfirm: () => void;
	onclose: () => void;
} = $props();
</script>

<svelte:window onkeydown={(e) => {
	if (matchesKeybind(e, keybinds.confirm) || e.key === "Enter") onconfirm();
	else if (matchesKeybind(e, keybinds.deny) || matchesKeybind(e, keybinds.escape)) onclose();
}} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()} transition:overlayFade>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="dialog" onclick={(e) => e.stopPropagation()} transition:dialogPop>
		<h2 class="title">{title}</h2>
		<p class="message">{message}</p>
		<div class="footer">
			<button class="btn cancel" onclick={onclose}>Cancel <kbd>{keybindLabel(keybinds.deny)}</kbd></button>
			<button class="btn confirm" class:danger onclick={onconfirm}>{confirmLabel} <kbd>Enter/{keybindLabel(keybinds.confirm)}</kbd></button>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: var(--overlay-bg);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: var(--z-dialog);
	}

	.dialog {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 360px;
		box-shadow: var(--shadow-lg);
		overflow: hidden;
	}

	.title {
		margin: 0;
		padding: 16px 20px 0;
		font-size: 15px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.message {
		margin: 0;
		padding: 12px 20px 16px;
		font-size: 13px;
		color: var(--text-secondary);
		line-height: 1.5;
	}

	.footer {
		padding: 0 20px 16px;
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.btn {
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 5px 14px;
		border-radius: var(--radius);
		cursor: pointer;
		border: 1px solid var(--border);
	}

	.btn.cancel {
		background: none;
		color: var(--text-secondary);
	}

	.btn.cancel:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.btn.confirm {
		background: var(--accent);
		color: var(--bg-primary);
		border-color: var(--accent);
	}

	.btn.confirm.danger {
		background: var(--danger);
		border-color: var(--danger);
		color: white;
	}

	.btn.confirm:hover {
		opacity: 0.9;
	}

</style>
