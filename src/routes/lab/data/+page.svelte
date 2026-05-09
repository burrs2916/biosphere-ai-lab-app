<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { t } from '$lib/i18n';

	onMount(() => {
		goto('/lab/data/list');
	});
</script>

<div class="redirect">
	<p>{$t('data.redirecting')}</p>
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
