<script lang="ts">
import {
	allBindLabels,
	categories,
	chordCategories,
	chordLabel,
	formatName,
} from "$lib/commandRegistry";
import { chords, keybinds, matchesKeybind } from "$lib/keybinds";

let {
	onclose,
}: {
	onclose: () => void;
} = $props();

let dialogEl = $state<HTMLDivElement | null>(null);
</script>

<svelte:window
	onkeydown={(e) => {
		if (matchesKeybind(e, keybinds.menuClose) || e.key === "?") {
			onclose();
		} else if (matchesKeybind(e, keybinds.menuDown)) {
			e.preventDefault();
			dialogEl?.scrollBy({ top: 60 });
		} else if (matchesKeybind(e, keybinds.menuUp)) {
			e.preventDefault();
			dialogEl?.scrollBy({ top: -60 });
		}
	}}
/>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="overlay" onclick={onclose}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="dialog" bind:this={dialogEl} onclick={(e) => e.stopPropagation()}>
		<div class="header">
			<h2>Keybinds</h2>
		</div>

		<div class="sections">
			{#each categories as cat}
				<div class="section">
					<h3 class="section-title">{cat.label}</h3>
					{#each cat.keys as key}
						{#if keybinds[key]}
							<div class="row">
								<span class="label">{formatName(key)}</span>
								<kbd>{allBindLabels(keybinds[key])}</kbd>
							</div>
						{/if}
					{/each}
					{#if cat.label === "Tabs"}
						<div class="row">
							<span class="label">Switch to Tab</span>
							<kbd>1 - 9</kbd>
						</div>
					{/if}
				</div>
			{/each}

			<div class="divider"></div>
			<h3 class="section-title chords-heading">Chords</h3>

			{#each chordCategories as cat}
				<div class="section">
					<h3 class="section-subtitle">{cat.label}</h3>
					{#each cat.keys as key}
						{#if chords[key]}
							<div class="row">
								<span class="label">{formatName(key)}</span>
								<kbd>{chordLabel(chords[key].keys)}</kbd>
							</div>
						{/if}
					{/each}
				</div>
			{/each}
		</div>

		<div class="sections hint">
			<div class="row">
				<span class="label">Help</span>
				<kbd>?</kbd>
			</div>
		</div>

		<div class="footer">
			<button class="close-btn" onclick={onclose}>Close <kbd>q/Esc</kbd></button>
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
		width: 460px;
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: var(--shadow-lg);
	}

	.header {
		padding: 20px 20px 8px;
	}

	.header h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.sections {
		padding: 0 20px 12px;
	}

	.section {
		margin-bottom: 12px;
	}

	.section-title {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--accent);
		margin: 0 0 4px;
	}

	.chords-heading {
		margin-bottom: 8px;
	}

	.section-subtitle {
		font-size: 11px;
		font-weight: 500;
		color: var(--text-muted);
		margin: 0 0 2px;
	}

	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 3px 0;
	}

	.label {
		font-size: 12px;
		color: var(--text-primary);
	}

	.divider {
		height: 1px;
		background: var(--border);
		margin: 8px 0;
	}

	.footer {
		padding: 12px 20px 16px;
		display: flex;
		justify-content: flex-end;
	}

	.close-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-primary);
		font-size: 13px;
		font-family: var(--font-sans);
		padding: 6px 16px;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.close-btn:hover {
		background: var(--bg-hover);
	}
</style>
