<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { experimentStore } from '$lib/lab/stores/experiment';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import type { ExperimentSummary, ExperimentStatus } from '$lib/lab/adapter/types';
	import { t } from '$lib/i18n';

	let loading = true;
	let error: string | null = null;
	let searchQuery = '';
	let statusFilter: ExperimentStatus | 'all' = 'all';
	let sortBy: 'created_at' | 'updated_at' | 'name' | 'status' = 'updated_at';
	let sortDir: 'asc' | 'desc' = 'desc';
	let taskTypeFilter = '';
	let showAdvancedSearch = false;
	let selectedTags: Set<string> = new Set();
	let groupFilter = '';
	let availableGroups: string[] = [];
	let confirmDeleteId: string | null = null;
	let deleting = false;
	let cloneTargetId: string | null = null;
	let cloneName = '';
	let cloning = false;

	let selectedIds: Set<string> = new Set();
	let showCompareModal = false;
	let compareData: Map<string, any> = new Map();
	let compareLoading = false;
	let confirmBatchDelete = false;
	let batchDeleting = false;

	let experiments: ExperimentSummary[] = [];
	let unsubExperiments: (() => void) | null = null;

	onMount(async () => {
		try {
			await experimentStore.refresh();
			const client = getLabClient();
			availableGroups = await client.listExperimentGroups();
		} catch (e: any) {
			error = e?.message || $t('experiments.loadFailed');
		} finally {
			loading = false;
		}

		unsubExperiments = experimentStore.subscribe((map) => {
			experiments = Array.from(map.values());
		});
	});

	onDestroy(() => {
		if (unsubExperiments) {
			unsubExperiments();
			unsubExperiments = null;
		}
	});

	$: allTags = (() => {
		const tags = new Set<string>();
		for (const e of experiments) {
			for (const t of e.tags) tags.add(t);
		}
		return Array.from(tags).sort();
	})();

	$: allTaskTypes = (() => {
		const types = new Set<string>();
		for (const e of experiments) {
			types.add(e.task_type);
		}
		return Array.from(types).sort();
	})();

	$: parsedQuery = parseSearchQuery(searchQuery);

	function parseSearchQuery(query: string): { text: string; tags: string[]; status: string | null; taskType: string | null } {
		const result = { text: '', tags: [] as string[], status: null as string | null, taskType: null as string | null };
		if (!query.trim()) return result;

		const tokens: string[] = [];
		const parts = query.match(/(?:[^\s"]+|"[^"]*")+/g) || [];

		for (const part of parts) {
			if (part.startsWith('tag:')) {
				result.tags.push(part.slice(4).replace(/^"|"$/g, ''));
			} else if (part.startsWith('status:')) {
				result.status = part.slice(7).replace(/^"|"$/g, '');
			} else if (part.startsWith('type:')) {
				result.taskType = part.slice(5).replace(/^"|"$/g, '');
			} else {
				tokens.push(part.replace(/^"|"$/g, ''));
			}
		}

		result.text = tokens.join(' ');
		return result;
	}

	$: filteredExperiments = experiments
		.filter((e) => {
			if (statusFilter !== 'all' && e.status !== statusFilter) return false;
			if (taskTypeFilter && e.task_type !== taskTypeFilter) return false;
			if (groupFilter && e.group !== groupFilter) return false;
			if (selectedTags.size > 0) {
				const hasTag = Array.from(selectedTags).some(t => e.tags.includes(t));
				if (!hasTag) return false;
			}
			if (searchQuery) {
				const pq = parsedQuery;
				if (pq.status && e.status !== pq.status) return false;
				if (pq.taskType && e.task_type !== pq.taskType) return false;
				if (pq.tags.length > 0) {
					const hasTag = pq.tags.some(t => e.tags.includes(t));
					if (!hasTag) return false;
				}
				if (pq.text) {
					const q = pq.text.toLowerCase();
					const matches = e.name.toLowerCase().includes(q) ||
						e.task_type.toLowerCase().includes(q) ||
						e.tags.some(t => t.toLowerCase().includes(q));
					if (!matches) return false;
				}
			}
			return true;
		})
		.sort((a, b) => {
			let cmp = 0;
			switch (sortBy) {
				case 'name':
					cmp = a.name.localeCompare(b.name);
					break;
				case 'status':
					cmp = statusOrder(a.status) - statusOrder(b.status);
					break;
				case 'created_at':
					cmp = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
					break;
				case 'updated_at':
				default:
					cmp = new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime();
					break;
			}
			return sortDir === 'desc' ? -cmp : cmp;
		});

	$: statusCounts = (() => {
		const counts: Record<string, number> = { all: experiments.length };
		for (const e of experiments) {
			counts[e.status] = (counts[e.status] || 0) + 1;
		}
		return counts;
	})();

	function statusOrder(status: ExperimentStatus): number {
		switch (status) {
			case 'running': return 0;
			case 'paused': return 1;
			case 'created': return 2;
			case 'completed': return 3;
			case 'failed': return 4;
			case 'cancelled': return 5;
			case 'archived': return 6;
			default: return 7;
		}
	}

	function statusColor(status: ExperimentStatus): string {
		switch (status) {
			case 'running': return '#10b981';
			case 'completed': return '#3b82f6';
			case 'failed': return '#ef4444';
			case 'paused': return '#f59e0b';
			case 'cancelled': return '#6b7280';
			case 'created': return '#8b5cf6';
			case 'archived': return '#9ca3af';
			default: return '#6b7280';
		}
	}

	function statusLabel(status: ExperimentStatus): string {
		switch (status) {
			case 'running': return $t('experiments.statusRunning');
		case 'completed': return $t('experiments.statusCompleted');
		case 'failed': return $t('experiments.statusFailed');
		case 'paused': return $t('experiments.statusPaused');
		case 'cancelled': return $t('experiments.statusCancelled');
		case 'created': return $t('experiments.statusCreated');
		case 'archived': return $t('experiments.statusArchived');
			default: return status;
		}
	}

	function statusIcon(status: ExperimentStatus): string {
		switch (status) {
			case 'running': return '⚡';
			case 'completed': return '✓';
			case 'failed': return '✕';
			case 'paused': return '⏸';
			case 'cancelled': return '⊘';
			case 'created': return '○';
			case 'archived': return '📁';
			default: return '?';
		}
	}

	function taskTypeLabel(taskType: string): string {
		const labels: Record<string, string> = {
			Classification: $t('experiments.taskClassification'),
			Regression: $t('experiments.taskRegression'),
			Clustering: $t('experiments.taskClustering'),
			Detection: $t('experiments.taskDetection'),
			Segmentation: $t('experiments.taskSegmentation'),
			Generation: $t('experiments.taskGeneration'),
			Reward: $t('experiments.taskReward'),
			Custom: $t('experiments.taskCustom'),
			classification: $t('experiments.taskClassification'),
			regression: $t('experiments.taskRegression'),
			clustering: $t('experiments.taskClustering'),
			detection: $t('experiments.taskDetection'),
			segmentation: $t('experiments.taskSegmentation'),
			generation: $t('experiments.taskGeneration'),
			reward: $t('experiments.taskReward'),
			custom: $t('experiments.taskCustom'),
			image_classification: $t('experiments.taskImageClassification'),
			text_classification: $t('experiments.taskTextClassification'),
			object_detection: $t('experiments.taskObjectDetection'),
		};
		return labels[taskType] || taskType;
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		if (diff < 60000) return $t('time.justNow');
		if (diff < 3600000) return `${Math.floor(diff / 60000)} ${$t('time.minutesAgo')}`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)} ${$t('time.hoursAgo')}`;
		if (diff < 604800000) return `${Math.floor(diff / 86400000)} ${$t('time.daysAgo')}`;
		return d.toLocaleDateString('zh-CN');
	}

	async function deleteExperiment(id: string) {
		deleting = true;
		try {
			const client = getLabClient();
			await client.deleteExperiment(id);
			await experimentStore.refresh();
			confirmDeleteId = null;
		} catch (e: any) {
			error = e?.message || $t('experiments.deleteFailed');
		} finally {
			deleting = false;
		}
	}

	async function cloneExperiment() {
		if (!cloneTargetId || !cloneName.trim()) return;
		cloning = true;
		try {
			const client = getLabClient();
			await client.cloneExperiment(cloneTargetId, cloneName.trim());
			await experimentStore.refresh();
			cloneTargetId = null;
			cloneName = '';
		} catch (e: any) {
			error = e?.message || $t('experiments.cloneFailed');
		} finally {
			cloning = false;
		}
	}

	function startClone(id: string, name: string) {
		cloneTargetId = id;
		cloneName = `${name} (${$t('experiments.copy')})`;
	}

	function toggleSort(field: typeof sortBy) {
		if (sortBy === field) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortBy = field;
			sortDir = 'desc';
		}
	}

	function sortIndicator(field: typeof sortBy): string {
		if (sortBy !== field) return '';
		return sortDir === 'asc' ? ' ↑' : ' ↓';
	}

	function toggleTag(tag: string) {
		const next = new Set(selectedTags);
		if (next.has(tag)) next.delete(tag);
		else next.add(tag);
		selectedTags = next;
	}

	function clearFilters() {
		searchQuery = '';
		statusFilter = 'all';
		taskTypeFilter = '';
		groupFilter = '';
		selectedTags = new Set();
	}

	$: hasActiveFilters = searchQuery !== '' || statusFilter !== 'all' || taskTypeFilter !== '' || groupFilter !== '' || selectedTags.size > 0;

	async function refreshList() {
		loading = true;
		error = null;
		try {
			await experimentStore.refresh();
		} catch (e: any) {
			error = e?.message || $t('experiments.refreshFailed');
		} finally {
			loading = false;
		}
	}

	function toggleSelect(id: string) {
		const next = new Set(selectedIds);
		if (next.has(id)) next.delete(id);
		else if (next.size < 5) next.add(id);
		selectedIds = next;
	}

	function selectAll() {
		const next = new Set<string>();
		for (const exp of filteredExperiments.slice(0, 5)) {
			next.add(exp.id);
		}
		selectedIds = next;
	}

	function clearSelection() {
		selectedIds = new Set();
		confirmBatchDelete = false;
	}

	async function batchDeleteSelected() {
		if (selectedIds.size === 0) return;
		if (!confirmBatchDelete) {
			confirmBatchDelete = true;
			return;
		}
		batchDeleting = true;
		try {
			const client = getLabClient();
			const ids = Array.from(selectedIds);
			await client.batchDeleteExperiments(ids);
			selectedIds = new Set();
			confirmBatchDelete = false;
			await experimentStore.refresh();
		} catch (e: any) {
			error = e?.message || $t('experiments.batchDeleteFailed');
		} finally {
			batchDeleting = false;
		}
	}

	async function compareSelected() {
		if (selectedIds.size < 2) return;
		compareLoading = true;
		showCompareModal = true;
		compareData = new Map();
		try {
			const client = getLabClient();
			for (const id of selectedIds) {
				const detail = await client.getExperimentDetail(id);
				compareData.set(id, detail);
			}
		} catch (e) {
			console.error('Failed to load compare data:', e);
		} finally {
			compareLoading = false;
		}
	}
</script>

<div class="experiments-page">
	<div class="page-header">
		<div>
			<h2>{$t('experiments.title')}</h2>
			<p class="subtitle">{$t('experiments.subtitle')}</p>
		</div>
		<div class="header-actions">
			{#if selectedIds.size >= 2}
				<button class="btn-compare" on:click={compareSelected}>
					📊 {$t('experiments.compareSelected')} ({selectedIds.size})
				</button>
			{:else if selectedIds.size > 0}
				<span class="select-hint">{$t('experiments.selectedCount', { count: selectedIds.size })}，{$t('experiments.selectMore')}</span>
			{/if}
			{#if selectedIds.size > 0}
				<button
					class="btn-batch-delete"
					class:btn-danger-confirm={confirmBatchDelete}
					on:click={batchDeleteSelected}
					disabled={batchDeleting}
				>
					{#if batchDeleting}
						{$t('experiments.deleting')}...
					{:else if confirmBatchDelete}
						⚠️ {$t('experiments.confirmBatchDelete', { count: selectedIds.size })}？
					{:else}
						🗑 {$t('experiments.deleteSelected')} ({selectedIds.size})
					{/if}
				</button>
				<button class="btn-clear-sel" on:click={clearSelection}>{$t('experiments.clearSelection')}</button>
			{/if}
			<button class="btn-refresh" on:click={refreshList} disabled={loading}>
				{loading ? $t('experiments.loading') : $t('experiments.refresh')}
			</button>
			<a href="/lab/train/new" class="btn-new">+ {$t('experiments.newExperiment')}</a>
		</div>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<div class="toolbar">
		<div class="search-row">
			<input
				type="text"
				class="search-input"
				placeholder={$t('experiments.searchPlaceholder')}
				bind:value={searchQuery}
			/>
			<button class="btn-advanced" class:active={showAdvancedSearch} on:click={() => showAdvancedSearch = !showAdvancedSearch}>
				{$t('experiments.advancedFilter')}
			</button>
			{#if hasActiveFilters}
				<button class="btn-clear-filters" on:click={clearFilters}>{$t('experiments.clearFilters')}</button>
			{/if}
		</div>
		{#if showAdvancedSearch}
			<div class="advanced-search">
				{#if availableGroups.length > 0}
					<div class="advanced-row">
						<span class="advanced-label">{$t('experiments.group')}</span>
						<div class="advanced-options">
							<button class="filter-btn" class:active={groupFilter === ''} on:click={() => groupFilter = ''}>{$t('experiments.all')}</button>
							{#each availableGroups as g}
								<button class="filter-btn" class:active={groupFilter === g} on:click={() => groupFilter = g}>
									{g}
								</button>
							{/each}
						</div>
					</div>
				{/if}
				<div class="advanced-row">
				<span class="advanced-label">{$t('experiments.taskType')}</span>
					<div class="advanced-options">
						<button class="filter-btn" class:active={taskTypeFilter === ''} on:click={() => taskTypeFilter = ''}>{$t('experiments.all')}</button>
						{#each allTaskTypes as t}
							<button class="filter-btn" class:active={taskTypeFilter === t} on:click={() => taskTypeFilter = t}>
								{taskTypeLabel(t)}
							</button>
						{/each}
					</div>
				</div>
				{#if allTags.length > 0}
					<div class="advanced-row">
						<span class="advanced-label">{$t('experiments.tags')}</span>
						<div class="advanced-options">
							{#each allTags as tag}
								<button class="filter-btn tag-btn" class:active={selectedTags.has(tag)} on:click={() => toggleTag(tag)}>
									{tag}
								</button>
							{/each}
						</div>
					</div>
				{/if}
				<div class="advanced-hint">
					💡 {$t('experiments.searchSyntax')}: <code>tag:xxx</code> {$t('experiments.filterByTag')}, <code>status:Running</code> {$t('experiments.filterByStatus')}, <code>type:classification</code> {$t('experiments.filterByType')}
				</div>
			</div>
		{/if}
		<div class="filter-group">
			<button class="filter-btn" class:active={statusFilter === 'all'} on:click={() => statusFilter = 'all'}>
				{$t('experiments.all')} {statusCounts.all || 0}
			</button>
			{#each (['running', 'completed', 'failed', 'paused', 'created', 'cancelled', 'archived'] as ExperimentStatus[]) as s}
				{#if statusCounts[s]}
					<button class="filter-btn" class:active={statusFilter === s} on:click={() => statusFilter = s}>
						<span class="filter-dot" style="background: {statusColor(s)}"></span>
						{statusLabel(s)} {statusCounts[s]}
					</button>
				{/if}
			{/each}
		</div>
	</div>

	{#if loading && experiments.length === 0}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>{$t('experiments.loadingList')}</p>
		</div>
	{:else if filteredExperiments.length === 0}
		<div class="empty-state">
			<span class="empty-icon">🔬</span>
			<p class="empty-text">{searchQuery || statusFilter !== 'all' ? $t('experiments.noMatching') : $t('experiments.noExperiments')}</p>
			<p class="empty-hint">
				{#if searchQuery || statusFilter !== 'all'}
					{$t('experiments.tryAdjustFilters')}
				{:else}
					<a href="/lab/train/new">{$t('experiments.createFirstExperiment')}</a>
				{/if}
			</p>
		</div>
	{:else}
		<div class="table-container">
			<table class="experiments-table">
				<thead>
					<tr>
						<th class="check-col">
							<input type="checkbox" checked={selectedIds.size > 0 && selectedIds.size === filteredExperiments.length} on:change={() => { if (selectedIds.size > 0) clearSelection(); else selectAll(); }} />
						</th>
						<th class="sortable" on:click={() => toggleSort('name')}>
							{$t('experiments.name')}{sortIndicator('name')}
						</th>
						<th class="sortable" on:click={() => toggleSort('status')}>
							{$t('experiments.status')}{sortIndicator('status')}
						</th>
						<th>{$t('experiments.taskType')}</th>
						<th>{$t('experiments.tags')}</th>
						<th>{$t('experiments.bestMetric')}</th>
						<th class="sortable" on:click={() => toggleSort('created_at')}>
							{$t('experiments.createdAt')}{sortIndicator('created_at')}
						</th>
						<th class="sortable" on:click={() => toggleSort('updated_at')}>
							{$t('experiments.updatedAt')}{sortIndicator('updated_at')}
						</th>
						<th>{$t('experiments.actions')}</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredExperiments as exp (exp.id)}
						<tr class="experiment-row" class:selected-row={selectedIds.has(exp.id)}>
							<td class="check-col">
								<input type="checkbox" checked={selectedIds.has(exp.id)} on:change={() => toggleSelect(exp.id)} />
							</td>
							<td>
								<a href="/lab/experiments/{exp.id}" class="exp-name">{exp.name}</a>
							</td>
							<td>
								<span
									class="status-badge"
									style="color: {statusColor(exp.status)}; border-color: {statusColor(exp.status)}30; background: {statusColor(exp.status)}10"
								>
									{statusIcon(exp.status)} {statusLabel(exp.status)}
								</span>
							</td>
							<td>
								<span class="task-badge">{taskTypeLabel(exp.task_type)}</span>
							</td>
							<td>
								{#if exp.tags.length > 0}
									<div class="tags-cell">
										{#each exp.tags.slice(0, 3) as tag}
											<span class="tag-chip">{tag}</span>
										{/each}
										{#if exp.tags.length > 3}
											<span class="tag-more">+{exp.tags.length - 3}</span>
										{/if}
									</div>
								{:else}
									<span class="no-tags">-</span>
								{/if}
							</td>
							<td>
								{#if Object.keys(exp.best_metrics).length > 0}
									<div class="metrics-cell">
										{#each Object.entries(exp.best_metrics).slice(0, 2) as [key, val]}
											<span class="metric-chip">
												<span class="metric-key">{key}</span>
												<span class="metric-val">{typeof val === 'number' ? val.toFixed(4) : val}</span>
											</span>
										{/each}
									</div>
								{:else}
									<span class="no-metrics">-</span>
								{/if}
							</td>
							<td class="time-cell">{formatTime(exp.created_at)}</td>
							<td class="time-cell">{formatTime(exp.updated_at)}</td>
							<td>
								<a href="/lab/experiments/{exp.id}" class="action-link">{$t('experiments.view')}</a>
								<button class="action-clone" on:click={() => startClone(exp.id, exp.name)}>{$t('experiments.clone')}</button>
								<button class="action-delete" on:click={() => confirmDeleteId = exp.id}>{$t('experiments.delete')}</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="table-footer">
			<span class="result-count">{$t('experiments.totalCount', { count: filteredExperiments.length })}</span>
		</div>
	{/if}

	{#if confirmDeleteId}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div class="modal-overlay" role="presentation" on:click={() => confirmDeleteId = null} on:keydown={(e) => { if (e.key === 'Escape') confirmDeleteId = null; }}>
			<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
				<h3>{$t('experiments.confirmDelete')}</h3>
				<p>{$t('experiments.confirmDeleteMsg')}</p>
				<div class="modal-actions">
					<button class="btn-cancel" on:click={() => confirmDeleteId = null}>{$t('confirm.cancel')}</button>
					<button class="btn-confirm-delete" on:click={() => deleteExperiment(confirmDeleteId!)} disabled={deleting}>
						{deleting ? $t('experiments.deleting') : $t('experiments.confirmDeleteBtn')}
					</button>
				</div>
			</div>
		</div>
	{/if}

	{#if cloneTargetId}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div class="modal-overlay" role="presentation" on:click={() => { cloneTargetId = null; cloneName = ''; }} on:keydown={(e) => { if (e.key === 'Escape') { cloneTargetId = null; cloneName = ''; } }}>
			<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
				<h3 style="color: #10b981;">{$t('experiments.cloneExperiment')}</h3>
				<p>{$t('experiments.cloneDesc')}</p>
				<div class="modal-actions">
					<label for="clone-name">{$t('experiments.newExperimentName')}</label>
					<input id="clone-name" type="text" bind:value={cloneName} placeholder={$t('experiments.newExperimentNamePlaceholder')} />
				</div>
				<div class="modal-actions">
					<button class="btn-cancel" on:click={() => { cloneTargetId = null; cloneName = ''; }}>{$t('confirm.cancel')}</button>
					<button class="btn-confirm-clone" on:click={cloneExperiment} disabled={cloning || !cloneName.trim()}>
						{cloning ? $t('experiments.cloning') : $t('experiments.confirmClone')}
					</button>
				</div>
			</div>
		</div>
	{/if}

	{#if showCompareModal}
		<div class="modal-overlay" role="presentation" on:click|self={() => showCompareModal = false} on:keydown={(e) => { if (e.key === 'Escape') showCompareModal = false; }}>
			<div class="compare-modal" role="dialog" aria-modal="true" tabindex="-1">
				<div class="compare-header">
					<h3>{$t('experiments.compareExperiments')}</h3>
					<button class="btn-close" on:click={() => showCompareModal = false}>✕</button>
				</div>
				{#if compareLoading}
					<p class="empty-hint">{$t('experiments.loadingCompare')}</p>
				{:else}
					{@const exps = Array.from(compareData.values())}
					<div class="compare-table-wrap">
						<table class="compare-table">
							<thead>
								<tr>
									<th class="prop-col">{$t('experiments.property')}</th>
									{#each exps as exp}
										<th><a href="/lab/experiments/{exp.id}" class="compare-exp-link">{exp.name}</a></th>
									{/each}
								</tr>
							</thead>
							<tbody>
								<tr>
									<td class="prop-col">{$t('experiments.status')}</td>
									{#each exps as exp}
										<td><span class="status-badge" style="color: {statusColor(exp.status)}; border-color: {statusColor(exp.status)}30; background: {statusColor(exp.status)}10">{statusLabel(exp.status)}</span></td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">{$t('experiments.taskType')}</td>
									{#each exps as exp}
										<td>{taskTypeLabel(exp.task_type)}</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">{$t('experiments.learningRate')}</td>
									{#each exps as exp}
										<td>{exp.config?.learning_rate ?? '-'}</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">Epochs</td>
									{#each exps as exp}
										<td>{exp.config?.num_epochs ?? '-'}</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">Batch Size</td>
									{#each exps as exp}
										<td>{exp.config?.batch_size ?? '-'}</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">{$t('experiments.optimizer')}</td>
									{#each exps as exp}
										<td>{exp.config?.optimizer ? JSON.stringify(exp.config.optimizer) : '-'}</td>
									{/each}
								</tr>
								<tr class="metrics-row">
									<td class="prop-col">{$t('experiments.bestMetric')}</td>
									{#each exps as exp}
										<td>
											{#if Object.keys(exp.best_metrics || {}).length > 0}
												{#each Object.entries(exp.best_metrics) as [key, val]}
													<div class="compare-metric">
														<span class="cm-key">{key}:</span>
														<span class="cm-val">{typeof val === 'number' ? val.toFixed(4) : val}</span>
													</div>
												{/each}
											{:else}
												-
											{/if}
										</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">{$t('experiments.tags')}</td>
									{#each exps as exp}
										<td>
											{#if (exp.tags || []).length > 0}
												{#each exp.tags as tag}
													<span class="tag-chip">{tag}</span>
												{/each}
											{:else}
												-
											{/if}
										</td>
									{/each}
								</tr>
								<tr>
									<td class="prop-col">{$t('experiments.createdAt')}</td>
									{#each exps as exp}
										<td>{new Date(exp.created_at).toLocaleString('zh-CN')}</td>
									{/each}
								</tr>
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.experiments-page {
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
	}

	h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin: 0;
	}

	.subtitle {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		margin-top: 0.25rem;
	}

	.header-actions {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.btn-refresh {
		padding: 0.5rem 1rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		cursor: pointer;
		font-size: 0.85rem;
		transition: all 0.2s;
	}

	.btn-refresh:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary, #e5e7eb);
	}

	.btn-refresh:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-compare {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #3b82f6, #2563eb);
		border: none;
		border-radius: 8px;
		color: white;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-compare:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
	}

	.btn-batch-delete {
		padding: 0.5rem 1.25rem;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-batch-delete:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	.btn-danger-confirm {
		background: rgba(239, 68, 68, 0.3) !important;
		border-color: #ef4444 !important;
		animation: pulse-danger 1s infinite;
	}

	@keyframes pulse-danger {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}

	.btn-clear-sel {
		padding: 0.4rem 1rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		cursor: pointer;
	}

	.select-hint {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		padding: 0.4rem 0;
	}

	.check-col {
		width: 40px;
		text-align: center;
	}

	.check-col input[type="checkbox"] {
		accent-color: #10b981;
		cursor: pointer;
	}

	.selected-row {
		background: rgba(16, 185, 129, 0.05) !important;
	}

	.compare-modal {
		background: linear-gradient(135deg, #1a1a2e, #16213e);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: 16px;
		padding: 1.5rem;
		width: 90%;
		max-width: 900px;
		max-height: 80vh;
		overflow-y: auto;
	}

	.compare-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.compare-header h3 {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.btn-close {
		background: none;
		border: none;
		color: var(--text-secondary, #9ca3af);
		font-size: 1.2rem;
		cursor: pointer;
		padding: 0.25rem;
	}

	.btn-close:hover {
		color: var(--text-primary, #e5e7eb);
	}

	.compare-table-wrap {
		overflow-x: auto;
	}

	.compare-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.compare-table th,
	.compare-table td {
		padding: 0.6rem 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		text-align: left;
		vertical-align: top;
	}

	.compare-table th {
		color: var(--text-primary, #e5e7eb);
		font-weight: 600;
		background: rgba(0, 0, 0, 0.2);
		position: sticky;
		top: 0;
	}

	.prop-col {
		color: var(--text-secondary, #9ca3af);
		font-weight: 500;
		white-space: nowrap;
		width: 100px;
		min-width: 100px;
	}

	.compare-exp-link {
		color: #60a5fa;
		text-decoration: none;
		font-weight: 500;
	}

	.compare-exp-link:hover {
		text-decoration: underline;
	}

	.compare-metric {
		display: flex;
		gap: 0.3rem;
		margin-bottom: 0.2rem;
	}

	.cm-key {
		color: var(--text-secondary, #9ca3af);
	}

	.cm-val {
		color: #10b981;
		font-weight: 600;
	}

	.metrics-row td {
		background: rgba(16, 185, 129, 0.03);
	}

	.btn-new {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #10b981, #059669);
		border: none;
		border-radius: 8px;
		color: white;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		text-decoration: none;
		display: inline-block;
	}

	.btn-new:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.error-banner {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.9rem;
		margin-bottom: 1.5rem;
	}

	.toolbar {
		display: flex;
		gap: 1rem;
		flex-direction: column;
		margin-bottom: 1.5rem;
	}

	.search-row {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.search-input {
		flex: 1;
		min-width: 200px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		padding: 0.6rem 1rem;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		transition: border-color 0.2s;
	}

	.search-input:focus {
		border-color: #10b981;
	}

	.search-input::placeholder {
		color: var(--text-secondary, #6b7280);
	}

	.btn-advanced {
		padding: 0.5rem 1rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.btn-advanced:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary, #e5e7eb);
	}

	.btn-advanced.active {
		background: rgba(16, 185, 129, 0.15);
		border-color: #10b981;
		color: #10b981;
	}

	.btn-clear-filters {
		padding: 0.5rem 0.75rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #fca5a5;
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.btn-clear-filters:hover {
		background: rgba(239, 68, 68, 0.2);
	}

	.advanced-search {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.advanced-row {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
	}

	.advanced-label {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		font-weight: 500;
		min-width: 60px;
		padding-top: 0.35rem;
	}

	.advanced-options {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}

	.advanced-hint {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
		padding-top: 0.25rem;
	}

	.advanced-hint code {
		background: rgba(16, 185, 129, 0.1);
		color: #6ee7b7;
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-family: monospace;
	}

	.tag-btn.active {
		background: rgba(59, 130, 246, 0.15);
		border-color: #3b82f6;
		color: #93c5fd;
	}

	.filter-group {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}

	.filter-btn {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		padding: 0.4rem 0.75rem;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		background: rgba(16, 185, 129, 0.1);
		border-color: rgba(16, 185, 129, 0.3);
	}

	.filter-btn.active {
		background: rgba(16, 185, 129, 0.15);
		border-color: #10b981;
		color: #10b981;
	}

	.filter-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		color: var(--text-secondary, #9ca3af);
		gap: 1rem;
	}

	.spinner {
		width: 2rem;
		height: 2rem;
		border: 3px solid rgba(16, 185, 129, 0.2);
		border-top-color: #10b981;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.empty-state {
		text-align: center;
		padding: 4rem 2rem;
	}

	.empty-icon {
		font-size: 3rem;
		margin-bottom: 1rem;
		display: block;
	}

	.empty-text {
		font-size: 1.2rem;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 0.5rem;
	}

	.empty-hint {
		color: var(--text-secondary, #6b7280);
		font-size: 0.9rem;
	}

	.empty-hint a {
		color: #10b981;
		text-decoration: none;
	}

	.empty-hint a:hover {
		text-decoration: underline;
	}

	.table-container {
		background: linear-gradient(135deg, rgba(26, 26, 46, 0.5), rgba(22, 33, 62, 0.5));
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		overflow: hidden;
	}

	.experiments-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.experiments-table th {
		text-align: left;
		padding: 0.75rem 1rem;
		color: var(--text-secondary, #9ca3af);
		font-weight: 500;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		white-space: nowrap;
		user-select: none;
	}

	.experiments-table th.sortable {
		cursor: pointer;
		transition: color 0.2s;
	}

	.experiments-table th.sortable:hover {
		color: #10b981;
	}

	.experiments-table td {
		padding: 0.75rem 1rem;
		color: var(--text-primary, #e5e7eb);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		vertical-align: middle;
	}

	.experiment-row {
		transition: background 0.15s;
	}

	.experiment-row:hover {
		background: rgba(16, 185, 129, 0.03);
	}

	.experiment-row:last-child td {
		border-bottom: none;
	}

	.exp-name {
		color: var(--text-primary, #e5e7eb);
		text-decoration: none;
		font-weight: 500;
		transition: color 0.2s;
	}

	.exp-name:hover {
		color: #10b981;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 500;
		border-width: 1px;
		border-style: solid;
		white-space: nowrap;
	}

	.task-badge {
		background: rgba(139, 92, 246, 0.1);
		color: #a78bfa;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.tags-cell {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
		align-items: center;
	}

	.tag-chip {
		background: rgba(59, 130, 246, 0.1);
		color: #93c5fd;
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		font-size: 0.7rem;
	}

	.tag-more {
		font-size: 0.7rem;
		color: var(--text-secondary, #6b7280);
	}

	.no-tags, .no-metrics {
		color: var(--text-secondary, #6b7280);
	}

	.metrics-cell {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.metric-chip {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-size: 0.75rem;
	}

	.metric-key {
		color: var(--text-secondary, #9ca3af);
	}

	.metric-val {
		color: #6ee7b7;
		font-family: monospace;
		font-weight: 500;
	}

	.time-cell {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		white-space: nowrap;
	}

	.action-link {
		color: #10b981;
		text-decoration: none;
		font-size: 0.85rem;
		font-weight: 500;
		transition: color 0.2s;
		margin-right: 0.75rem;
	}

	.action-link:hover {
		color: #34d399;
		text-decoration: underline;
	}

	.action-delete {
		color: #ef4444;
		background: none;
		border: none;
		font-size: 0.85rem;
		cursor: pointer;
		padding: 0;
		transition: color 0.2s;
		margin-right: 0.75rem;
	}

	.action-delete:hover {
		color: #f87171;
		text-decoration: underline;
	}

	.action-clone {
		color: #10b981;
		background: none;
		border: none;
		font-size: 0.85rem;
		cursor: pointer;
		padding: 0;
		transition: color 0.2s;
		margin-right: 0.75rem;
	}

	.action-clone:hover {
		color: #34d399;
		text-decoration: underline;
	}

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: linear-gradient(135deg, #1a1a2e, #16213e);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 16px;
		padding: 2rem;
		width: 90%;
		max-width: 420px;
	}

	.modal h3 {
		font-size: 1.2rem;
		color: #ef4444;
		margin-bottom: 1rem;
	}

	.modal p {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		line-height: 1.5;
		margin-bottom: 1.5rem;
	}

	.modal-actions {
		display: flex;
		gap: 0.75rem;
		justify-content: flex-end;
	}

	.btn-cancel {
		padding: 0.5rem 1.25rem;
		border-radius: 8px;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-secondary, #1f2937);
		color: var(--text-primary, #e5e7eb);
		cursor: pointer;
		font-size: 0.9rem;
	}

	.btn-cancel:hover {
		border-color: #6b7280;
	}

	.btn-confirm-delete {
		padding: 0.5rem 1.25rem;
		border-radius: 8px;
		border: none;
		background: linear-gradient(135deg, #ef4444, #dc2626);
		color: white;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
	}

	.btn-confirm-delete:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
	}

	.btn-confirm-delete:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.clone-input-group {
		margin-bottom: 1.5rem;
	}

	.clone-input-group label {
		display: block;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		margin-bottom: 0.5rem;
	}

	.clone-input-group input {
		width: 100%;
		padding: 0.6rem 0.8rem;
		border-radius: 8px;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-primary, #111827);
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		box-sizing: border-box;
	}

	.clone-input-group input:focus {
		border-color: #10b981;
		box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.2);
	}

	.btn-confirm-clone {
		padding: 0.5rem 1.25rem;
		border-radius: 8px;
		border: none;
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
	}

	.btn-confirm-clone:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.btn-confirm-clone:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.table-footer {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		margin-top: 0.5rem;
	}

	.result-count {
		color: var(--text-secondary, #6b7280);
		font-size: 0.85rem;
	}
</style>
