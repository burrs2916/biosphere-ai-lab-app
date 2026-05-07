<script lang="ts">
	import { toastStore } from '$lib/lab/stores/toast';
	import type { ToastType } from '$lib/lab/stores/toast';

	function icon(type: ToastType): string {
		switch (type) {
			case 'success': return '✓';
			case 'error': return '✗';
			case 'warning': return '⚠';
			case 'info': return 'ℹ';
		}
	}

	function remove(id: number) {
		toastStore.remove(id);
	}
</script>

{#if $toastStore.length > 0}
	<div class="toast-container">
		{#each $toastStore as toast (toast.id)}
			<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
			<div class="toast {toast.type}" role="alert" on:click={() => remove(toast.id)} on:keydown={(e) => { if (e.key === 'Enter' || e.key === 'Escape') remove(toast.id); }}>
				<span class="toast-icon">{icon(toast.type)}</span>
				<span class="toast-message">{toast.message}</span>
				<button class="toast-close" on:click|stopPropagation={() => remove(toast.id)}>✕</button>
			</div>
		{/each}
	</div>
{/if}

<style>
	.toast-container {
		position: fixed;
		top: 1rem;
		right: 1rem;
		z-index: 10000;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-width: 400px;
		pointer-events: none;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		font-size: 0.85rem;
		cursor: pointer;
		pointer-events: auto;
		animation: slideIn 0.3s ease;
		backdrop-filter: blur(12px);
		border: 1px solid rgba(255, 255, 255, 0.1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
	}

	.toast.success {
		background: rgba(16, 185, 129, 0.15);
		border-color: rgba(16, 185, 129, 0.3);
		color: #6ee7b7;
	}

	.toast.error {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	.toast.warning {
		background: rgba(245, 158, 11, 0.15);
		border-color: rgba(245, 158, 11, 0.3);
		color: #fcd34d;
	}

	.toast.info {
		background: rgba(59, 130, 246, 0.15);
		border-color: rgba(59, 130, 246, 0.3);
		color: #93c5fd;
	}

	.toast-icon {
		font-size: 1rem;
		flex-shrink: 0;
		font-weight: bold;
	}

	.toast-message {
		flex: 1;
		line-height: 1.4;
	}

	.toast-close {
		background: none;
		border: none;
		color: inherit;
		opacity: 0.5;
		cursor: pointer;
		font-size: 0.8rem;
		padding: 0 0.2rem;
		flex-shrink: 0;
	}

	.toast-close:hover {
		opacity: 1;
	}

	@keyframes slideIn {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}
</style>
