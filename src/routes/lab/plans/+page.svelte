<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { t } from '$lib/i18n';

	interface PlanSummary {
		id: string;
		name: string;
		version: string;
		description: string | null;
		plan_type: string;
		phases_count: number;
		datasets_count: number;
		total_estimated_tokens: number;
		total_estimated_steps: number;
		estimated_gpu_hours: number | null;
		modified_at: string;
	}

	let plans: PlanSummary[] = [];
	let loading = true;
	let error: string | null = null;
	let deleteConfirm: string | null = null;
	let deleting = false;

	const planTypeLabels = (type: string): string => {
		const map: Record<string, string> = {
			Pretraining: $t('plans.pretraining'),
			SFT: $t('plans.sft'),
			RLHF: 'RLHF',
			DPO: 'DPO',
			ContinuedPretraining: $t('plans.continuedPretraining'),
		};
		return map[type] || type;
	};

	function formatTokens(tokens: number): string {
		if (tokens >= 1_000_000_000_000) return (tokens / 1_000_000_000_000).toFixed(1) + 'T';
		if (tokens >= 1_000_000_000) return (tokens / 1_000_000_000).toFixed(1) + 'B';
		if (tokens >= 1_000_000) return (tokens / 1_000_000).toFixed(1) + 'M';
		return tokens.toLocaleString();
	}

	function formatGpuHours(hours: number | null): string {
		if (hours === null || hours === undefined) return '-';
		if (hours >= 1000) return (hours / 1000).toFixed(1) + 'K h';
		return hours.toFixed(1) + ' h';
	}

	function formatDate(dateStr: string): string {
		try {
			const d = new Date(dateStr);
			return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
		} catch {
			return dateStr;
		}
	}

	async function loadPlans() {
		loading = true;
		error = null;
		try {
			const client = getLabClient();
			plans = await client.trainingPlanList();
		} catch (e: any) {
			error = e?.toString() || $t('plans.loadFailed');
		} finally {
			loading = false;
		}
	}

	async function deletePlan(planId: string) {
		deleting = true;
		try {
			const client = getLabClient();
			await client.trainingPlanDelete(planId);
			plans = plans.filter((p) => p.id !== planId);
			deleteConfirm = null;
		} catch (e: any) {
			error = e?.toString() || $t('plans.deleteFailed');
		} finally {
			deleting = false;
		}
	}

	function viewPlan(planId: string) {
		goto(`/lab/plan?id=${planId}`);
	}

	onMount(() => {
		loadPlans();
	});
</script>

<div class="plans-page">
	<div class="page-header">
		<div>
			<h2>{$t('plans.title')}</h2>
			<p class="desc">{$t('plans.desc')}</p>
		</div>
		<div class="header-actions">
			<button class="btn-secondary" on:click={loadPlans} disabled={loading}>🔄 {$t('plans.refresh')}</button>
			<button class="btn-primary" on:click={() => goto('/lab/plan')}>+ {$t('plans.createPlan')}</button>
		</div>
	</div>

	{#if error}
		<div class="error-banner">
			<span>{error}</span>
			<button class="error-close" on:click={() => (error = null)}>✕</button>
		</div>
	{/if}

	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
			<span>{$t('plans.loadingPlans')}</span>
		</div>
	{:else if plans.length === 0}
		<div class="empty-state">
			<div class="empty-icon">📋</div>
			<h3>{$t('plans.noPlans')}</h3>
			<p>{$t('plans.noPlansDesc')}</p>
			<button class="btn-primary" on:click={() => goto('/lab/plan')}>{$t('plans.createFirstPlan')}</button>
		</div>
	{:else}
		<div class="plans-table-wrapper">
			<table class="plans-table">
				<thead>
					<tr>
						<th>{$t('plans.planName')}</th>
						<th>{$t('plans.type')}</th>
						<th>{$t('plans.phases')}</th>
						<th>{$t('plans.datasets')}</th>
						<th>{$t('plans.estimatedTokens')}</th>
						<th>{$t('plans.estimatedSteps')}</th>
						<th>{$t('plans.gpuHours')}</th>
						<th>{$t('plans.updatedAt')}</th>
						<th>{$t('plans.actions')}</th>
					</tr>
				</thead>
				<tbody>
					{#each plans as plan (plan.id)}
						<tr>
							<td>
								<div class="plan-name-cell">
									<span class="plan-name">{plan.name}</span>
									<span class="plan-version">v{plan.version}</span>
									{#if plan.description}
										<span class="plan-desc">{plan.description}</span>
									{/if}
								</div>
							</td>
							<td>
								<span class="plan-type-badge {plan.plan_type.toLowerCase()}">
									{planTypeLabels(plan.plan_type)}
								</span>
							</td>
							<td class="num-cell">{plan.phases_count}</td>
							<td class="num-cell">{plan.datasets_count}</td>
							<td class="num-cell">{formatTokens(plan.total_estimated_tokens)}</td>
							<td class="num-cell">{plan.total_estimated_steps.toLocaleString()}</td>
							<td class="num-cell">{formatGpuHours(plan.estimated_gpu_hours)}</td>
							<td class="date-cell">{formatDate(plan.modified_at)}</td>
							<td class="actions-cell">
								<button class="btn-sm" on:click={() => viewPlan(plan.id)}>{$t('plans.view')}</button>
								<button class="btn-sm btn-danger" on:click={() => (deleteConfirm = plan.id)}>{$t('plans.delete')}</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

{#if deleteConfirm}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (deleteConfirm = null)} on:keydown={(e) => { if (e.key === 'Escape') deleteConfirm = null; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('plans.confirmDelete')}</h3>
			<p>{$t('plans.confirmDeleteMsg', { name: plans.find((p) => p.id === deleteConfirm)?.name || '' })}</p>
			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (deleteConfirm = null)}>{$t('confirm.cancel')}</button>
				<button class="btn-danger" on:click={() => deleteConfirm && deletePlan(deleteConfirm)} disabled={deleting}>
					{deleting ? $t('plans.deleting') : $t('plans.confirmDeleteBtn')}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.plans-page {
		padding: 0;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
	}
	.page-header h2 { margin: 0 0 0.25rem 0; font-size: 1.4rem; }
	.desc { color: #9ca3af; font-size: 0.85rem; margin: 0; }
	.header-actions { display: flex; gap: 0.5rem; }

	.error-banner {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 6px;
		color: #fca5a5;
		font-size: 0.85rem;
		margin-bottom: 1rem;
	}
	.error-close {
		background: none;
		border: none;
		color: #fca5a5;
		cursor: pointer;
		font-size: 1rem;
		padding: 0 0.25rem;
	}

	.loading-state {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 3rem;
		color: #9ca3af;
	}
	.spinner {
		width: 20px;
		height: 20px;
		border: 2px solid rgba(255,255,255,0.1);
		border-top-color: #3b82f6;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty-state {
		text-align: center;
		padding: 3rem 1rem;
		color: #9ca3af;
	}
	.empty-icon { font-size: 3rem; margin-bottom: 1rem; }
	.empty-state h3 { color: #d1d5db; margin: 0 0 0.5rem 0; }
	.empty-state p { margin: 0 0 1.5rem 0; font-size: 0.9rem; }

	.plans-table-wrapper {
		overflow-x: auto;
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
	}
	.plans-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}
	.plans-table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		color: #9ca3af;
		font-weight: 500;
		font-size: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		border-bottom: 1px solid rgba(107, 114, 128, 0.2);
		background: rgba(255, 255, 255, 0.02);
		white-space: nowrap;
	}
	.plans-table td {
		padding: 0.6rem 0.75rem;
		border-bottom: 1px solid rgba(107, 114, 128, 0.1);
		color: #d1d5db;
		vertical-align: middle;
	}
	.plans-table tbody tr:hover {
		background: rgba(59, 130, 246, 0.04);
	}
	.plans-table tbody tr:last-child td {
		border-bottom: none;
	}

	.plan-name-cell {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.plan-name { font-weight: 500; color: #e5e7eb; }
	.plan-version { font-size: 0.7rem; color: #6b7280; }
	.plan-desc { font-size: 0.75rem; color: #9ca3af; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.plan-type-badge {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
		white-space: nowrap;
	}
	.plan-type-badge.pretraining { background: rgba(59, 130, 246, 0.15); color: #93c5fd; }
	.plan-type-badge.sft { background: rgba(16, 185, 129, 0.15); color: #6ee7b7; }
	.plan-type-badge.rlhf { background: rgba(245, 158, 11, 0.15); color: #fcd34d; }
	.plan-type-badge.dpo { background: rgba(139, 92, 246, 0.15); color: #c4b5fd; }
	.plan-type-badge.continuedpretraining { background: rgba(236, 72, 153, 0.15); color: #f9a8d4; }

	.num-cell { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
	.date-cell { white-space: nowrap; color: #9ca3af; font-size: 0.8rem; }
	.actions-cell { white-space: nowrap; display: flex; gap: 0.35rem; }

	.btn-sm {
		padding: 0.25rem 0.6rem;
		font-size: 0.75rem;
		border-radius: 4px;
		border: 1px solid rgba(107, 114, 128, 0.3);
		background: rgba(255, 255, 255, 0.05);
		color: #d1d5db;
		cursor: pointer;
		transition: all 0.15s;
	}
	.btn-sm:hover { background: rgba(255, 255, 255, 0.1); }
	.btn-danger { color: #fca5a5; border-color: rgba(239, 68, 68, 0.3); }
	.btn-danger:hover { background: rgba(239, 68, 68, 0.15); }

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.modal {
		background: #1f2937;
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 10px;
		padding: 1.5rem;
		max-width: 420px;
		width: 90%;
	}
	.modal h3 { margin: 0 0 0.75rem 0; color: #e5e7eb; }
	.modal p { color: #9ca3af; font-size: 0.9rem; margin: 0 0 1.25rem 0; }
	.modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
</style>
