<script lang="ts">
let { src, size = 16 }: { src: string; size?: number } = $props();
</script>

<img {src} width={size} height={size} alt="" class="file-icon" draggable="false" />

<style>
	.file-icon {
		display: inline-block;
		vertical-align: middle;
		flex-shrink: 0;
	}
</style>
