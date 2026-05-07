<script lang="ts">
	import { i18n } from '$lib/i18n';
	import type { Locale } from '$lib/i18n';

	function toggleLocale() {
		const current = $i18n;
		const next: Locale = current === 'zh-CN' ? 'en' : 'zh-CN';
		i18n.setLocale(next);
	}

	function getLocaleLabel(locale: Locale): string {
		return locale === 'zh-CN' ? '中' : 'EN';
	}
</script>

<button
	class="locale-switch"
	on:click={toggleLocale}
	title={$i18n === 'zh-CN' ? 'Switch to English' : '切换到中文'}
>
	<span class="locale-label">{getLocaleLabel($i18n)}</span>
	<span class="locale-divider">/</span>
	<span class="locale-alt">{getLocaleLabel($i18n === 'zh-CN' ? 'en' : 'zh-CN')}</span>
</button>

<style>
	.locale-switch {
		display: flex;
		align-items: center;
		gap: 0.15rem;
		padding: 0.25rem 0.5rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		color: #9ca3af;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s ease;
		user-select: none;
	}

	.locale-switch:hover {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(16, 185, 129, 0.3);
		color: #e5e7eb;
	}

	.locale-switch:active {
		transform: scale(0.95);
	}

	.locale-label {
		color: #10b981;
		font-weight: 600;
	}

	.locale-divider {
		opacity: 0.4;
	}

	.locale-alt {
		opacity: 0.6;
	}
</style>
