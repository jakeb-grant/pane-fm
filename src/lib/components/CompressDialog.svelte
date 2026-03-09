<script lang="ts">
import { tick } from "svelte";
import { keybindLabel, keybinds, matchesKeybind } from "$lib/keybinds";

let {
	defaultName,
	onconfirm,
	onclose,
}: {
	defaultName: string;
	onconfirm: (name: string, format: string) => void;
	onclose: () => void;
} = $props();

const formats = [
	{ ext: "zip", label: "ZIP" },
	{ ext: "tar.gz", label: "tar.gz" },
	{ ext: "tar.xz", label: "tar.xz" },
	{ ext: "tar.zst", label: "tar.zst" },
	{ ext: "tar.bz2", label: "tar.bz2" },
];

let selectedFormat = $state("zip");
// svelte-ignore state_referenced_locally — intentionally capturing initial prop value
let baseName = $state(defaultName);
let nameInput: HTMLInputElement | undefined = $state();

let fullName = $derived(`${baseName}.${selectedFormat}`);

tick().then(() => {
	if (nameInput) {
		nameInput.focus();
		nameInput.select();
	}
});

function handleSubmit() {
	if (!baseName.trim()) return;
	onconfirm(fullName, selectedFormat);
}
</script>

<svelte:window onkeydown={(e) => {
	const inInput = (e.target as HTMLElement)?.tagName === "INPUT";
	if (matchesKeybind(e, keybinds.escape)) onclose();
	else if (!inInput && matchesKeybind(e, keybinds.confirm)) handleSubmit();
}} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose} onwheel={(e) => e.preventDefault()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="dialog" onclick={(e) => e.stopPropagation()}>
		<h2 class="title">Compress</h2>

		<div class="body">
			<label class="field">
				<span class="field-label">Archive name</span>
				<div class="name-row">
					<input
						class="name-input"
						type="text"
						bind:value={baseName}
						bind:this={nameInput}
						onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); handleSubmit(); } }}
					/>
					<span class="ext-label">.{selectedFormat}</span>
				</div>
			</label>

			<div class="field">
				<span class="field-label">Format</span>
				<div class="format-group">
					{#each formats as fmt (fmt.ext)}
						<button
							class="format-btn"
							class:active={selectedFormat === fmt.ext}
							onclick={() => selectedFormat = fmt.ext}
						>
							{fmt.label}
						</button>
					{/each}
				</div>
			</div>

			<div class="preview">
				<span class="preview-label">Output</span>
				<span class="preview-value">{fullName}</span>
			</div>
		</div>

		<div class="footer">
			<button class="btn cancel" onclick={onclose}>Cancel <kbd>{keybindLabel(keybinds.escape)}</kbd></button>
			<button class="btn confirm" onclick={handleSubmit} disabled={!baseName.trim()}>Compress <kbd>{keybindLabel(keybinds.confirm)}</kbd></button>
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
		width: 380px;
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

	.body {
		padding: 16px 20px;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.field-label {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		font-weight: 500;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 0;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.name-row:focus-within {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent);
	}

	.name-input {
		flex: 1;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 7px 10px;
		outline: none;
		min-width: 0;
	}

	.ext-label {
		font-size: 12px;
		font-family: var(--font-mono);
		color: var(--text-muted);
		padding: 0 10px 0 0;
		flex-shrink: 0;
	}

	.format-group {
		display: flex;
		background: var(--bg-primary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.format-btn {
		flex: 1;
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 11px;
		font-family: var(--font-mono);
		padding: 6px 0;
		cursor: pointer;
		transition: color var(--transition-normal), background var(--transition-normal);
	}

	.format-btn:hover {
		color: var(--text-primary);
		background: var(--bg-surface);
	}

	.format-btn.active {
		color: var(--accent);
		background: var(--bg-surface);
	}

	.preview {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 0;
	}

	.preview-label {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
	}

	.preview-value {
		font-size: 12px;
		font-family: var(--font-mono);
		color: var(--text-secondary);
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

	.btn.confirm:hover {
		opacity: 0.9;
	}

	.btn.confirm:disabled {
		opacity: 0.4;
		cursor: default;
	}

</style>
