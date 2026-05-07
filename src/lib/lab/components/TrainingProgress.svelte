<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { progressStore, formatETA } from '$lib/lab/stores/progress';

	export let sessionId: string;
	export let compact: boolean = false;

	let progress: import('$lib/lab/stores/progress').TrainingProgress | undefined;
	let unsubProgress: (() => void) | null = null;

	onMount(() => {
		unsubProgress = progressStore.subscribe((map) => {
			progress = map.get(sessionId);
			if (!progress) {
				progress = progressStore.getByExperimentId(sessionId);
			}
		});
	});

	onDestroy(() => {
		if (unsubProgress) {
			unsubProgress();
			unsubProgress = null;
		}
	});

	$: progressPercent = progress ? Math.round(progress.progress * 100) : 0;

	$: etaLabel = progress ? formatETA(progress.estimatedTimeRemaining) : '';

	$: barColor = (() => {
		if (!progress) return '#6b7280';
		if (progressPercent >= 100) return '#3b82f6';
		return '#10b981';
	})();

	$: epochSpeed = (() => {
		if (!progress || progress.epochTimes.length === 0) return null;
		const avg = progress.epochTimes.reduce((a, b) => a + b, 0) / progress.epochTimes.length;
		return avg.toFixed(1);
	})();

	$: lossFromMessage = (() => {
		if (!progress?.message) return null;
		const lossMatch = progress.message.match(/loss:\s*([\d.]+)/);
		const valLossMatch = progress.message.match(/val_loss:\s*([\d.]+)/);
		return {
			trainLoss: lossMatch ? parseFloat(lossMatch[1]) : null,
			valLoss: valLossMatch ? parseFloat(valLossMatch[1]) : null,
		};
	})();
</script>

{#if progress}
	<div class="progress-container" class:compact>
		{#if !compact}
			<div class="progress-header">
				<span class="progress-label">
					{#if progress.totalEpochs > 0}
						Epoch {progress.currentEpoch}/{progress.totalEpochs}
					{:else}
						{progressPercent}%
					{/if}
				</span>
				<div class="progress-stats">
					{#if etaLabel && progress.estimatedTimeRemaining !== null && progress.estimatedTimeRemaining > 0}
						<span class="stat-chip eta">
							⏱ 剩余 {etaLabel}
						</span>
					{:else if progressPercent >= 100}
						<span class="stat-chip done">✓ 已完成</span>
					{/if}
					{#if epochSpeed}
						<span class="stat-chip speed">
							⚡ {epochSpeed}s/epoch
						</span>
					{/if}
					{#if lossFromMessage && lossFromMessage.trainLoss !== null}
						<span class="stat-chip loss">
							📉 loss={lossFromMessage.trainLoss.toFixed(4)}
						</span>
					{/if}
					{#if lossFromMessage && lossFromMessage.valLoss !== null}
						<span class="stat-chip val-loss">
							📊 val_loss={lossFromMessage.valLoss.toFixed(4)}
						</span>
					{/if}
				</div>
			</div>
		{/if}
		<div class="progress-bar-track">
			<div
				class="progress-bar-fill"
				style="width: {progressPercent}%; background: {barColor};"
			></div>
			{#if !compact && progress.totalEpochs > 0}
				<div class="progress-epoch-markers">
					{#each Array(Math.min(progress.totalEpochs, 20)) as _, i}
						<div
							class="epoch-marker"
							class:completed={(i + 1) <= progress.currentEpoch}
							style="left: {((i + 1) / progress.totalEpochs) * 100}%"
						></div>
					{/each}
				</div>
			{/if}
		</div>
		{#if !compact && progress.message}
			<div class="progress-message">{progress.message}</div>
		{/if}
	</div>
{/if}

<style>
	.progress-container {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.progress-container.compact {
		gap: 0;
	}

	.progress-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 0.8rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.progress-label {
		color: var(--text-primary, #e5e7eb);
		font-weight: 600;
		font-size: 0.9rem;
	}

	.progress-stats {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}

	.stat-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.2rem;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-size: 0.72rem;
		font-family: monospace;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--text-secondary, #9ca3af);
	}

	.stat-chip.eta {
		color: #f59e0b;
		border-color: rgba(245, 158, 11, 0.2);
		background: rgba(245, 158, 11, 0.05);
	}

	.stat-chip.done {
		color: #3b82f6;
		border-color: rgba(59, 130, 246, 0.2);
		background: rgba(59, 130, 246, 0.05);
	}

	.stat-chip.speed {
		color: #8b5cf6;
		border-color: rgba(139, 92, 246, 0.2);
		background: rgba(139, 92, 246, 0.05);
	}

	.stat-chip.loss {
		color: #10b981;
		border-color: rgba(16, 185, 129, 0.2);
		background: rgba(16, 185, 129, 0.05);
	}

	.stat-chip.val-loss {
		color: #3b82f6;
		border-color: rgba(59, 130, 246, 0.2);
		background: rgba(59, 130, 246, 0.05);
	}

	.progress-bar-track {
		height: 8px;
		background: rgba(255, 255, 255, 0.08);
		border-radius: 4px;
		overflow: hidden;
		position: relative;
	}

	.compact .progress-bar-track {
		height: 4px;
	}

	.progress-bar-fill {
		height: 100%;
		border-radius: 4px;
		transition: width 0.5s ease, background 0.3s ease;
		position: relative;
	}

	.progress-bar-fill::after {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
		animation: shimmer 2s ease-in-out infinite;
	}

	@keyframes shimmer {
		0% { transform: translateX(-100%); }
		100% { transform: translateX(100%); }
	}

	.progress-epoch-markers {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
	}

	.epoch-marker {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 1px;
		background: rgba(255, 255, 255, 0.15);
		transform: translateX(-0.5px);
	}

	.epoch-marker.completed {
		background: rgba(255, 255, 255, 0.3);
	}

	.progress-message {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
		font-family: monospace;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
