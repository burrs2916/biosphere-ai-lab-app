<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	onMount(() => {
		goto('/lab/data/list');
	});
</script>

<div class="redirect">
	<p>正在跳转到数据集列表...</p>
</div>

<style>
	.redirect {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #9ca3af;
		font-size: 0.9rem;
	}
</style>
