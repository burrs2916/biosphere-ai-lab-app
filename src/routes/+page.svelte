<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { t } from '$lib/i18n';

	onMount(() => {
		goto('/lab');
	});
</script>

<div class="redirect">
	<p>{$t('common.redirectingToLab')}</p>
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
