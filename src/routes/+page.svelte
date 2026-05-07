<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		goto('/lab');
	});
</script>

<div class="redirect">
	<p>正在跳转到 AI Lab...</p>
</div>

<style>
	.redirect {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		color: var(--text-secondary, #9ca3af);
	}
</style>
