<script lang="ts">
import { keybindLabel, keybinds, matchesKeybind } from "$lib/keybinds";

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
	if (matchesKeybind(e, keybinds.confirm)) onconfirm();
	else if (matchesKeybind(e, keybinds.deny) || matchesKeybind(e, keybinds.escape)) onclose();
}} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="dialog" onclick={(e) => e.stopPropagation()}>
		<h2 class="title">{title}</h2>
		<p class="message">{message}</p>
		<div class="footer">
			<button class="btn cancel" onclick={onclose}>Cancel <kbd>{keybindLabel(keybinds.deny)}</kbd></button>
			<button class="btn confirm" class:danger onclick={onconfirm}>{confirmLabel} <kbd>{keybindLabel(keybinds.confirm)}</kbd></button>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.dialog {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 2);
		width: 360px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
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
	}

	.btn.confirm:hover {
		opacity: 0.9;
	}

	kbd {
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		padding: 1px 4px;
		border-radius: 3px;
		background: rgba(255, 255, 255, 0.1);
		margin-left: 4px;
		opacity: 0.7;
	}
</style>
