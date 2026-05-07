<script lang="ts">
	export let width = '100%';
	export let height = '16px';
	export let marginTop = '0';
	export let borderRadius = '4px';
</script>

<div
	class="skeleton"
	style="width: {width}; height: {height}; margin-top: {marginTop}; border-radius: {borderRadius};"
></div>

<style>
	.skeleton {
		background: linear-gradient(90deg, rgba(255,255,255,0.04) 25%, rgba(255,255,255,0.08) 50%, rgba(255,255,255,0.04) 75%);
		background-size: 200% 100%;
		animation: shimmer 1.5s ease-in-out infinite;
	}
	@keyframes shimmer {
		0% { background-position: 200% 0; }
		100% { background-position: -200% 0; }
	}
</style>
