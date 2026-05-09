<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { t, i18n } from '$lib/i18n';
	import { datasetRegistryStore } from '$lib/lab/stores/dataset';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { taskManagerStore } from '$lib/lab/stores/taskManager';
	import type { DatasetRegistration, ColumnProfile, DatasetVersionRecord } from '$lib/lab/adapter/types';
	import Skeleton from '$lib/lab/components/Skeleton.svelte';
	import ColumnHistogram from '$lib/lab/components/ColumnHistogram.svelte';
	import ColumnBarChart from '$lib/lab/components/ColumnBarChart.svelte';
	import MissingValueHeatmap from '$lib/lab/components/MissingValueHeatmap.svelte';
	import DataTable from '$lib/lab/components/DataTable.svelte';
	import type { DataPreview } from '$lib/lab/adapter/types';
	import LineageGraph from '$lib/lab/components/LineageGraph.svelte';
	import DataRecipeBuilder from '$lib/lab/components/DataRecipeBuilder.svelte';
	import DataVersionControl from '$lib/lab/components/DataVersionControl.svelte';
	import LabelQualityPanel from '$lib/lab/components/LabelQualityPanel.svelte';
	import DataAnalysisPanel from '$lib/lab/components/DataAnalysisPanel.svelte';
	import DatasetCardPanel from '$lib/lab/components/DatasetCardPanel.svelte';
	import AdvancedDataTools from '$lib/lab/components/AdvancedDataTools.svelte';
	import NotificationStack from '$lib/lab/components/NotificationStack.svelte';
	import TaskProgressPanel from '$lib/lab/components/TaskProgressPanel.svelte';
	import QualityScorePanel from '$lib/lab/components/QualityScorePanel.svelte';
	import HealthCheckPanel from '$lib/lab/components/HealthCheckPanel.svelte';
	import VersionDiffPanel from '$lib/lab/components/VersionDiffPanel.svelte';
	import TrainingReadinessDashboard from '$lib/lab/components/TrainingReadinessDashboard.svelte';
	import KeyboardShortcuts from '$lib/lab/components/KeyboardShortcuts.svelte';
	import { uxStore } from '$lib/lab/stores/uxStore';
	import { localizeError } from '$lib/lab/utils/errorLocalizer';

	let dataset: DatasetRegistration | null = null;
	let profiles: ColumnProfile[] = [];
	let versionHistory: DatasetVersionRecord[] = [];
	let loading = true;
	let error: string | null = null;
	let versionLoading = false;

	let showEditDesc = false;
	let editDescription = '';
	let showAddTag = false;
	let newTag = '';
	let showNewVersion = false;
	let newVersionNote = '';
	let creatingVersion = false;

	let confirmArchive = false;
	let confirmDelete = false;
	let archiving = false;
	let deleting = false;

	let qualityScore: any = null;
	let qualityLoading = false;
	let qualityError: string | null = null;

	let readinessData: any = null;
	let readinessLoading = false;

	let diffFromVersion = '';
	let diffToVersion = '';
	let diffResult: any = null;
	let diffLoading = false;
	let diffError: string | null = null;

	let previewData: DataPreview | null = null;
	let previewLoading = false;
	let previewPage = 0;
	let previewPageSize = 50;
	let previewSortCol: string | null = null;
	let previewSortDir: 'asc' | 'desc' = 'asc';

	let showExport = false;
	let exportTargetFormat: 'csv' | 'json' | 'parquet' = 'csv';
	let exporting = false;
	let exportResult: any = null;
	let exportError: string | null = null;

	let healthCheckRunning = false;
	let healthCheckResults: {
		readiness?: any;
		validation?: any;
		integrity?: any;
		leakage?: any;
		sufficiency?: any;
		dedup?: any;
		splits?: any[];
	} = {};
	let healthCheckError: string | null = null;

	let versionDiffData: any = null;
	let versionDiffLoading = false;

	let showSplitModal = false;
	let splitName = 'default_split';
	let splitTrainRatio = 0.7;
	let splitValRatio = 0.15;
	let splitTestRatio = 0.15;
	let splitShuffle = true;
	let splitSeed = 42;
	let splitCreating = false;
	let splitError: string | null = null;

	let showDedupModal = false;
	let dedupThreshold = 0.8;
	let dedupNumPerm = 128;
	let dedupNGram = 5;
	let dedupRunning = false;
	let dedupResult: any = null;
	let dedupError: string | null = null;

	let lineageGraph: any = null;
	let lineageLoading = false;
	let activeDetailTab = 'overview';
	let chartFilterColumn: string | null = null;
	let chartFilterValue: string | null = null;
	let chartFilterRange: [number, number] | null = null;

	function handleHistogramRangeSelect(e: CustomEvent) {
		const detail = e.detail;
		if (!detail) {
			chartFilterColumn = null;
			chartFilterRange = null;
		} else {
			chartFilterColumn = detail.columnName;
			chartFilterRange = [detail.start, detail.end];
			chartFilterValue = null;
		}
	}

	function handleBarValueSelect(e: CustomEvent) {
		const detail = e.detail;
		if (!detail) {
			chartFilterColumn = null;
			chartFilterValue = null;
		} else {
			chartFilterColumn = detail.columnName;
			chartFilterValue = detail.value;
			chartFilterRange = null;
		}
	}

	function clearChartFilter() {
		chartFilterColumn = null;
		chartFilterValue = null;
		chartFilterRange = null;
	}

	async function loadLineage() {
		if (!dataset) return;
		lineageLoading = true;
		try {
			const client = getLabClient();
			lineageGraph = await client.datasetLineage(dataset.id);
		} catch (e) {
			lineageGraph = null;
		} finally {
			lineageLoading = false;
		}
	}

	$: datasetId = $page.params.id;

	const columnTypeIcons: Record<string, string> = {
		Integer: '🔢',
		Float: '📊',
		String: '📝',
		Boolean: '✅',
		Categorical: '🏷️',
		DateTime: '📅',
		Unknown: '❓',
	};

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString(i18n.getLocale() === 'zh-CN' ? 'zh-CN' : 'en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	function formatSize(mb: number): string {
		if (mb < 1) return `${(mb * 1024).toFixed(0)} KB`;
		if (mb < 1024) return `${mb.toFixed(1)} MB`;
		return `${(mb / 1024).toFixed(1)} GB`;
	}

	function formatPct(v: number, total: number): string {
		if (total === 0) return '0%';
		return ((v / total) * 100).toFixed(1) + '%';
	}

	async function loadDataset() {
		loading = true;
		error = null;
		try {
			const id = datasetId;
			if (!id) {
				error = $t('dataset.detail.invalidId');
				loading = false;
				return;
			}
			await datasetRegistryStore.loadDataset(id);
			let current: DatasetRegistration | null = null;
			let currentProfiles: ColumnProfile[] = [];
			const unsub = datasetRegistryStore.subscribe((s) => {
				current = s.currentDataset;
				currentProfiles = s.currentProfiles || [];
			});
			unsub();
			if (current) {
				dataset = current;
				profiles = currentProfiles;
				loadPreview(0, previewPageSize);
			} else {
				error = $t('dataset.detail.notFound');
			}
		} catch (e: any) {
			error = e?.toString() || $t('dataset.detail.loadFailed');
		} finally {
			loading = false;
		}
	}

	async function loadVersionHistory() {
		const id = datasetId;
		if (!id) return;
		versionLoading = true;
		try {
			const client = getLabClient();
			versionHistory = await client.datasetVersionHistory(id);
		} catch (e) {
			versionHistory = [];
		} finally {
			versionLoading = false;
		}
	}

	async function loadQualityScore() {
		const id = datasetId;
		if (!id) return;
		qualityLoading = true;
		qualityError = null;

		const taskId = taskManagerStore.createTask(
			$t('dataset.detail.qualityAssessing'),
			$t('dataset.detail.qualityAssessingDesc'),
			false,
			2
		);

		try {
			const client = getLabClient();
			taskManagerStore.advanceStep(taskId, $t('dataset.detail.analyzingIntegrity'));
			qualityScore = await client.datasetQualityScore(id);
			taskManagerStore.completeTask(taskId, `${$t('quality.overallScore')}: ${qualityScore.overall_score.toFixed(0)}`);
		} catch (e: any) {
			qualityError = e?.toString() || $t('dataset.detail.loadFailed');
			taskManagerStore.failTask(taskId, qualityError || $t('common.error'));
		} finally {
			qualityLoading = false;
		}
	}

	async function loadReadiness() {
		const id = datasetId;
		if (!id) return;
		readinessLoading = true;
		try {
			const client = getLabClient();
			readinessData = await client.datasetReadinessScore(id);
		} catch (e: any) {
			uxStore.error($t('readiness.title'), localizeError(e).message);
		} finally {
			readinessLoading = false;
		}
	}

	async function loadVersionDiff() {
		const id = datasetId;
		if (!id || !diffFromVersion || !diffToVersion) return;
		diffLoading = true;
		diffError = null;
		diffResult = null;
		try {
			const client = getLabClient();
			diffResult = await client.datasetVersionDiff(id, diffFromVersion, diffToVersion);
		} catch (e: any) {
			diffError = e?.toString() || $t('dataset.detail.versionDiffFailed');
		} finally {
			diffLoading = false;
		}
	}

	let rowDiffData: any = null;
	let rowDiffLoading = false;
	let rowDiffOffset = 0;
	const ROW_DIFF_LIMIT = 20;

	async function loadRowDiff(fromVersion: string, toVersion: string) {
		const id = datasetId;
		if (!id) return;
		rowDiffLoading = true;
		try {
			const client = getLabClient();
			rowDiffData = await client.datasetRowDiff(id, fromVersion, toVersion, rowDiffOffset, ROW_DIFF_LIMIT);
		} catch (e: any) {
			uxStore.error($t('dataset.detail.rowDiffFailed'), localizeError(e).message);
		} finally {
			rowDiffLoading = false;
		}
	}

	async function rollbackToVersion(version: string) {
		const id = datasetId;
		if (!id || !dataset) return;
		try {
			const client = getLabClient();
			await client.dataVersionCheckout(dataset.path, version);
			uxStore.success($t('dataset.detail.rollbackSuccess'), `v${version}`);
			await loadDataset();
		} catch (e: any) {
			uxStore.error($t('dataset.detail.rollbackFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		}
	}

	async function saveDescription() {
		if (!dataset) return;
		await datasetRegistryStore.setDescription(dataset.id, editDescription);
		dataset = { ...dataset, description: editDescription };
		showEditDesc = false;
	}

	async function loadPreview(page: number = 0, pageSize: number = 50) {
		if (!dataset) return;
		previewLoading = true;
		previewPage = page;
		previewPageSize = pageSize;
		try {
			const client = getLabClient();
			const result = await client.previewData(
				{ path: dataset.path, format: dataset.format, has_header: true, delimiter: null, encoding: null, max_rows: null, custom_params: {} },
				page * pageSize,
				pageSize
			);
			previewData = result;
		} catch (e: any) {
			previewData = null;
		} finally {
			previewLoading = false;
		}
	}

	function handlePreviewPageChange(page: number, pageSize: number) {
		loadPreview(page, pageSize);
	}

	function handlePreviewSort(col: string, dir: 'asc' | 'desc') {
		previewSortCol = col;
		previewSortDir = dir;
	}

	async function addTag() {
		if (!dataset || !newTag.trim()) return;
		await datasetRegistryStore.addTag(dataset.id, newTag.trim());
		dataset = { ...dataset, tags: [...dataset.tags, newTag.trim()] };
		newTag = '';
		showAddTag = false;
	}

	async function removeTag(tag: string) {
		if (!dataset) return;
		await datasetRegistryStore.removeTag(dataset.id, tag);
		dataset = { ...dataset, tags: dataset.tags.filter((t) => t !== tag) };
	}

	async function createNewVersion() {
		if (!dataset) return;
		creatingVersion = true;
		try {
			const client = getLabClient();
			const updated = await client.datasetNewVersionWithNote(dataset.id, newVersionNote);
			dataset = updated;
			newVersionNote = '';
			showNewVersion = false;
			await loadVersionHistory();
		} catch (e: any) {
			console.error('Failed to create version:', e);
		} finally {
			creatingVersion = false;
		}
	}

	async function archiveDataset() {
		if (!dataset) return;
		archiving = true;
		try {
			await datasetRegistryStore.archiveDataset(dataset.id);
			dataset = { ...dataset, status: 'archived' };
			confirmArchive = false;
			uxStore.success($t('dataset.detail.archiveSuccess'), `"${dataset.name}"`);
	} catch (e: any) {
		console.error('Failed to archive:', e);
		uxStore.error($t('dataset.detail.archiveFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		} finally {
			archiving = false;
		}
	}

	async function deleteDataset() {
		if (!dataset) return;
		deleting = true;
		try {
			await datasetRegistryStore.deleteDataset(dataset.id);
			uxStore.success($t('dataset.detail.deleteSuccess'), `"${dataset.name}"`);
			goto('/lab/data/list');
		} catch (e: any) {
			console.error('Failed to delete:', e);
			uxStore.error($t('dataset.detail.deleteFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		} finally {
			deleting = false;
		}
	}

	async function exportDataset() {
		if (!dataset) return;
		exporting = true;
		exportError = null;
		exportResult = null;

		const taskId = taskManagerStore.createTask(
			$t('dataset.detail.exportTask'),
			$t('dataset.detail.exportTaskDesc'),
			false,
			2
		);

		try {
			const client = getLabClient();
			taskManagerStore.advanceStep(taskId, $t('dataset.detail.readingData'));
			const result = await client.exportDataset(dataset.id, exportTargetFormat, null);
			exportResult = result;
			taskManagerStore.completeTask(taskId, result.message);
			uxStore.success($t('dataset.detail.exportSuccess'), `${exportTargetFormat.toUpperCase()}`);
		} catch (e: any) {
			exportError = e?.toString() || $t('dataset.detail.exportFailed');
			taskManagerStore.failTask(taskId, exportError || $t('common.error'));
			uxStore.error($t('dataset.detail.exportFailed'), localizeError(exportError).message + '\n💡 ' + localizeError(exportError).suggestion);
		} finally {
			exporting = false;
		}
	}

	async function runHealthCheck() {
		if (!dataset) return;
		healthCheckRunning = true;
		healthCheckError = null;
		healthCheckResults = {};

		const taskId = taskManagerStore.createTask(
			$t('dataset.detail.healthCheckTitle'),
			$t('dataset.detail.healthCheckDesc'),
			false,
			6
		);

		try {
			const client = getLabClient();
			const id = dataset.id;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.validatingIntegrity'));
			const integrity = await client.validateDatasetIntegrity(id).catch(() => null);
			healthCheckResults.integrity = integrity;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.validatingFormat'));
			const validation = await client.validateDataset(id).catch(() => null);
			healthCheckResults.validation = validation;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.detectingLeakage'));
			const leakage = await client.datasetCheckLeakage(id).catch(() => null);
			healthCheckResults.leakage = leakage;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.checkingSufficiency'));
			const sufficiency = await client.datasetCheckSufficiency(id).catch(() => null);
			healthCheckResults.sufficiency = sufficiency;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.computingReadiness'));
			const readiness = await client.datasetReadinessScore(id).catch(() => null);
			healthCheckResults.readiness = readiness;

			taskManagerStore.advanceStep(taskId, $t('dataset.detail.loadingSplits'));
			const splits = await client.listDatasetSplits(id).catch(() => []);
			healthCheckResults.splits = splits;

			taskManagerStore.completeTask(taskId, $t('dataset.detail.healthCheckComplete'));
			uxStore.success($t('dataset.detail.checkComplete'), `"${dataset.name}"`);
		} catch (e: any) {
			healthCheckError = e?.toString() || $t('dataset.detail.healthCheckFailed');
			taskManagerStore.failTask(taskId, healthCheckError || $t('common.error'));
			uxStore.error($t('dataset.detail.checkFailed'), localizeError(healthCheckError).message + '\n💡 ' + localizeError(healthCheckError).suggestion);
		} finally {
			healthCheckRunning = false;
		}
	}

	async function createSplit() {
		if (!dataset) return;
		splitCreating = true;
		splitError = null;

		const taskId = taskManagerStore.createTask(
			$t('dataset.detail.createSplitTask'),
			$t('dataset.detail.createSplitDesc'),
			false,
			2
		);

		try {
			const client = getLabClient();
			const result = await client.createDatasetSplit(dataset.id, splitName, {
				train_ratio: splitTrainRatio,
				val_ratio: splitValRatio,
				test_ratio: splitTestRatio,
				shuffle: splitShuffle,
				seed: splitSeed,
			});
			showSplitModal = false;
			taskManagerStore.completeTask(taskId, $t('dataset.detail.splitCreated'));
			await runHealthCheck();
		} catch (e: any) {
			splitError = e?.toString() || $t('dataset.detail.splitFailed');
			taskManagerStore.failTask(taskId, splitError || $t('common.error'));
		} finally {
			splitCreating = false;
		}
	}

	async function runDedup() {
		if (!dataset) return;
		dedupRunning = true;
		dedupError = null;
		dedupResult = null;

		const taskId = taskManagerStore.createTask(
			$t('dataset.detail.dedupTask'),
			$t('dataset.detail.dedupDesc'),
			false,
			2
		);

		try {
			const client = getLabClient();
			taskManagerStore.advanceStep(taskId, $t('dataset.detail.computingMinhash'));
			const result = await client.datasetDedup(dataset.id, {
				similarity_threshold: dedupThreshold,
				num_perm: dedupNumPerm,
				n_gram: dedupNGram,
			});
			dedupResult = result;
			taskManagerStore.completeTask(taskId, $t('dataset.detail.dedupRemoved', { count: result.duplicates_removed }));
		} catch (e: any) {
			dedupError = e?.toString() || $t('dataset.detail.dedupFailed');
			taskManagerStore.failTask(taskId, dedupError || $t('common.error'));
		} finally {
			dedupRunning = false;
		}
	}

	onMount(() => {
		loadDataset();
		loadVersionHistory();
		loadQualityScore();
		loadLineage();
	});
</script>

<div class="detail-page">
	<div class="page-header">
		<button class="btn-back" on:click={() => goto('/lab/data/list')}>{$t('dataset.detail.backToList')}</button>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="skeleton-detail">
			<Skeleton width="200px" height="24px" />
			<Skeleton width="120px" height="14px" marginTop="8px" />
			<div class="skeleton-grid">
				<Skeleton width="100%" height="60px" />
				<Skeleton width="100%" height="60px" />
				<Skeleton width="100%" height="60px" />
				<Skeleton width="100%" height="60px" />
			</div>
		</div>
	{:else if dataset}
		<div class="detail-content">
			<div class="detail-header">
				<div class="header-left">
					<h2>{dataset.name}</h2>
					<div class="header-meta">
						<span class="status-badge {dataset.status}">
							{dataset.status === 'active' ? $t('dataset.detail.active') : $t('dataset.detail.archived')}
						</span>
						<span class="meta-item">{$t('dataset.detail.version')} {dataset.version}</span>
						<span class="meta-item">ID: {dataset.id.slice(0, 8)}</span>
					</div>
				</div>
				<div class="header-actions">
					<button class="btn-primary" on:click={() => (showExport = true)}>{$t('dataset.detail.export')}</button>
					{#if dataset.status === 'active'}
						<button class="btn-secondary" on:click={() => (confirmArchive = true)}>{$t('dataset.detail.archive')}</button>
					{/if}
					<button class="btn-danger-outline" on:click={() => (confirmDelete = true)}>{$t('dataset.detail.delete')}</button>
				</div>
			</div>

			<div class="detail-tabs">
				{#each [
					{ id: 'overview', label: $t('dataset.detail.tabOverview') },
					{ id: 'readiness', label: $t('dataset.detail.tabReadiness') },
					{ id: 'lineage', label: $t('dataset.detail.tabLineage') },
					{ id: 'recipe', label: $t('dataset.detail.tabRecipe') },
					{ id: 'version', label: $t('dataset.detail.tabVersion') },
					{ id: 'label', label: $t('dataset.detail.tabLabel') },
					{ id: 'analysis', label: $t('dataset.detail.tabAnalysis') },
					{ id: 'card', label: $t('dataset.detail.tabCard') },
					{ id: 'tools', label: $t('dataset.detail.tabTools') },
				] as tab}
					<button class="detail-tab" class:active={activeDetailTab === tab.id} on:click={() => (activeDetailTab = tab.id)}>
						{tab.label}
					</button>
				{/each}
			</div>

			{#if activeDetailTab === 'overview'}
			{#if dataset.description || showEditDesc}
				<div class="info-card">
					<div class="card-header">
						<h4>{$t('dataset.detail.description')}</h4>
						{#if !showEditDesc}
							<button class="btn-link" on:click={() => { editDescription = dataset?.description || ''; showEditDesc = true; }}>{$t('dataset.detail.edit')}</button>
						{/if}
					</div>
					{#if showEditDesc}
						<div class="edit-desc">
							<textarea bind:value={editDescription} placeholder={$t('dataset.detail.addDescPlaceholder')} class="textarea" rows="3"></textarea>
							<div class="edit-actions">
								<button class="btn-sm" on:click={() => (showEditDesc = false)}>{$t('dataset.detail.cancel')}</button>
								<button class="btn-sm btn-primary-sm" on:click={saveDescription}>{$t('dataset.detail.save')}</button>
							</div>
						</div>
					{:else}
						<p class="desc-text">{dataset.description || $t('dataset.detail.noDesc')}</p>
					{/if}
				</div>
			{/if}

			{#if profiles.length > 0}
				<div class="info-card">
					<div class="card-header">
						<h4>{$t('dataset.detail.dataViz')}</h4>
						<span class="viz-hint">{$t('dataset.detail.vizHint')}</span>
					</div>

					<div class="viz-section">
						<h5 class="viz-subtitle">{$t('dataset.detail.missing')}</h5>
						<MissingValueHeatmap profiles={profiles} width={Math.min(600, profiles.length * 60 + 140)} height={160} />
					</div>

					{#if chartFilterColumn}
						<div class="chart-filter-banner">
							<span>🔍 {$t('dataset.detail.filteredBy')} <strong>{chartFilterColumn}</strong>
								{#if chartFilterRange}
									{$t('dataset.detail.range')} {chartFilterRange[0].toFixed(2)} – {chartFilterRange[1].toFixed(2)}
								{:else if chartFilterValue}
									= {chartFilterValue}
								{/if}
							</span>
							<button class="clear-chart-filter" on:click={clearChartFilter}>{$t('dataset.detail.clearFilter')}</button>
						</div>
					{/if}

					<div class="viz-grid">
						{#each profiles.filter(p => p.column_type === 'integer' || p.column_type === 'float') as col}
							<div class="viz-item" class:active-viz={chartFilterColumn === col.name}>
								<h5 class="viz-subtitle">{col.name} ({col.column_type})</h5>
								<ColumnHistogram
									columnName={col.name}
									values={col.top_values?.flatMap(([v, c]: [string, number]) => Array(c).fill(parseFloat(v))).filter((v: number) => !isNaN(v)) || []}
									width={280}
									height={160}
									selectedRange={chartFilterColumn === col.name ? chartFilterRange : null}
									on:rangeselect={(e) => {
										if (e.detail) {
											chartFilterColumn = col.name;
											chartFilterRange = [e.detail.start, e.detail.end];
											chartFilterValue = null;
										} else {
											clearChartFilter();
										}
									}}
								/>
							</div>
						{/each}

						{#each profiles.filter(p => p.column_type === 'categorical' || p.column_type === 'string' || p.column_type === 'boolean') as col}
							<div class="viz-item" class:active-viz={chartFilterColumn === col.name}>
								<h5 class="viz-subtitle">{col.name} ({col.column_type})</h5>
								<ColumnBarChart
									columnName={col.name}
									topValues={col.top_values || []}
									width={280}
									height={160}
									selectedValue={chartFilterColumn === col.name ? chartFilterValue : null}
									on:valueselect={(e) => {
										if (e.detail) {
											chartFilterColumn = col.name;
											chartFilterValue = e.detail;
											chartFilterRange = null;
										} else {
											clearChartFilter();
										}
									}}
								/>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<div class="info-card">
				<div class="card-header">
					<h4>{$t('dataset.detail.dataPreview')}</h4>
					{#if !previewData && !previewLoading}
						<button class="btn-link" on:click={() => loadPreview(0, previewPageSize)}>{$t('tools.load')}</button>
					{/if}
				</div>
				{#if previewLoading || previewData}
					<DataTable
						columns={previewData?.columns || []}
						columnTypes={previewData?.column_types || []}
						rows={previewData?.rows || []}
						totalRows={previewData?.total_rows || 0}
						loading={previewLoading}
						pageSize={previewPageSize}
						currentPage={previewPage}
						columnProfiles={profiles}
						onPageChange={handlePreviewPageChange}
						onSortChange={handlePreviewSort}
					/>
				{/if}
			</div>

			<div class="info-card">
				<div class="card-header">
					<h4>{$t('dataset.detail.qualityScoreTitle')}</h4>
					<button class="btn-link" on:click={loadQualityScore} disabled={qualityLoading}>
						{qualityLoading ? '...' : '🔄'}
					</button>
				</div>
				{#if qualityLoading}
					<div class="loading-row"><Skeleton width="100%" height="80px" /></div>
				{:else if qualityError}
					<div class="quality-error">{qualityError}</div>
				{:else if qualityScore}
					<QualityScorePanel {qualityScore} />
				{:else}
					<div class="quality-empty">
						<p>{$t('healthCheck.clickToCheck')}</p>
					</div>
				{/if}
			</div>

			<div class="info-card">
				<div class="card-header">
					<h4>🏥 {$t('healthCheck.checking')}</h4>
					<div class="header-actions-row">
						<button class="btn-link" on:click={runHealthCheck} disabled={healthCheckRunning}>
							{healthCheckRunning ? '...' : '🔄'}
						</button>
						<button class="btn-link" on:click={() => (showSplitModal = true)}>📐 {$t('dataset.detail.createSplit')}</button>
						<button class="btn-link" on:click={() => (showDedupModal = true)}>🔍 {$t('dataset.detail.dedupTitle')}</button>
					</div>
				</div>

				<HealthCheckPanel
					{healthCheckRunning}
					{healthCheckResults}
					{healthCheckError}
				/>
			</div>

			<div class="info-card">
				<div class="card-header">
					<h4>{$t('dataset.detail.tags')}</h4>
					<button class="btn-link" on:click={() => (showAddTag = !showAddTag)}>+ {$t('dataset.detail.edit')}</button>
				</div>
				<div class="tags-list">
					{#each dataset.tags as tag}
						<span class="tag">
							{tag}
							<button class="tag-remove" on:click={() => removeTag(tag)}>✕</button>
						</span>
					{/each}
					{#if dataset.tags.length === 0}
						<span class="no-data">{$t('common.noData')}</span>
					{/if}
				</div>
				{#if showAddTag}
					<div class="add-tag-row">
						<input type="text" bind:value={newTag} placeholder={$t('dataset.detail.tagPlaceholder')} class="input-sm" on:keydown={(e) => e.key === 'Enter' && addTag()} />
						<button class="btn-sm btn-primary-sm" on:click={addTag} disabled={!newTag.trim()}>{$t('dataset.detail.addTag')}</button>
					</div>
				{/if}
			</div>

			<div class="info-card">
				<div class="card-header">
					<h4>{$t('dataset.detail.columnProfile')}</h4>
				</div>
				<div class="info-grid">
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.format')}</span>
						<span class="info-value">{dataset.format.toUpperCase()}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.size')}</span>
						<span class="info-value">{formatSize(dataset.memory_size_mb)}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.rows')}</span>
						<span class="info-value">{dataset.rows.toLocaleString()}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.columns')}</span>
						<span class="info-value">{dataset.columns}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.type')}</span>
						<span class="info-value path-value">{dataset.path}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.createdAt')}</span>
						<span class="info-value">{formatDate(dataset.created_at)}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.updatedAt')}</span>
						<span class="info-value">{formatDate(dataset.updated_at)}</span>
					</div>
					<div class="info-item">
						<span class="info-label">{$t('dataset.detail.experiments')}</span>
						<span class="info-value">{dataset.experiment_ids?.length || 0}</span>
					</div>
				</div>
			</div>

			{#if profiles.length > 0}
				<div class="info-card">
					<div class="card-header">
						<h4>{$t('dataset.detail.columnProfile')} ({profiles.length})</h4>
					</div>
					<div class="profiles-table-wrapper">
						<table class="profiles-table">
							<thead>
								<tr>
									<th>{$t('dataset.detail.colName')}</th>
									<th>{$t('dataset.detail.colType')}</th>
									<th>{$t('dataset.detail.notNull')}</th>
									<th>{$t('dataset.detail.nullCount')}</th>
									<th>{$t('dataset.detail.unique')}</th>
									<th>{$t('dataset.detail.mean')}</th>
									<th>{$t('dataset.detail.min')}</th>
									<th>{$t('dataset.detail.max')}</th>
								</tr>
							</thead>
							<tbody>
								{#each profiles as col}
									<tr>
										<td class="col-name-cell">
											<span class="col-icon">{columnTypeIcons[col.column_type] || '📌'}</span>
											{col.name}
										</td>
										<td>
											<span class="type-badge {col.column_type.toLowerCase()}">{col.column_type}</span>
										</td>
										<td class="num-cell">{col.total_count - col.null_count}</td>
										<td class="num-cell">
											<span class:high-null={col.null_count > 0 && col.total_count > 0 && col.null_count / col.total_count > 0.1}>
												{col.null_count > 0 ? `${col.null_count} (${formatPct(col.null_count, col.total_count)})` : '0'}
											</span>
										</td>
										<td class="num-cell">{col.distinct_count}</td>
										<td class="num-cell">{col.mean_value !== null && col.mean_value !== undefined ? col.mean_value.toFixed(2) : '-'}</td>
										<td class="num-cell">{col.min_value !== null && col.min_value !== undefined ? String(col.min_value) : '-'}</td>
										<td class="num-cell">{col.max_value !== null && col.max_value !== undefined ? String(col.max_value) : '-'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}

			<div class="info-card">
				<div class="card-header">
					<h4>{$t('dataset.detail.versionHistory')}</h4>
					<button class="btn-link" on:click={() => (showNewVersion = true)}>+ {$t('dataset.detail.newVersion')}</button>
				</div>
				{#if versionLoading}
					<div class="loading-row"><Skeleton width="100%" height="40px" /></div>
				{:else if versionHistory.length > 0}
					<div class="version-list">
						{#each versionHistory as v, i}
							<div class="version-item" class:is-current={i === 0}>
								<div class="version-dot" class:current={i === 0}></div>
								{#if i < versionHistory.length - 1}
									<div class="version-line"></div>
								{/if}
								<div class="version-info">
									<div class="version-header">
										<span class="version-tag">{v.version}</span>
										{#if i === 0}
											<span class="current-badge">{$t('dataset.detail.current')}</span>
										{/if}
										<span class="version-date">{formatDate(v.created_at)}</span>
									</div>
									<div class="version-stats">
										{v.rows.toLocaleString()} × {v.columns} · {formatSize(v.memory_size_mb)}
									</div>
									{#if v.change_note}
										<div class="version-note">📝 {v.change_note}</div>
									{/if}
									{#if i > 0}
										<div class="version-actions">
											<button class="btn-rollback" on:click={() => rollbackToVersion(v.version)}>
												↩ {$t('dataset.detail.rollbackToVersion')}
											</button>
										</div>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<span class="no-data">{$t('dataset.detail.noVersionHistory')}</span>
				{/if}

				{#if versionHistory.length >= 2}
					<div class="diff-section">
						<div class="diff-header">
							<h5>{$t('dataset.detail.versionDiff')}</h5>
						</div>
						<div class="diff-selectors">
							<select bind:value={diffFromVersion} class="diff-select">
								<option value="">-- {$t('dataset.detail.selectBaseVersion')} --</option>
								{#each versionHistory as v}
									<option value={v.version}>{v.version}</option>
								{/each}
							</select>
							<span class="diff-arrow">→</span>
							<select bind:value={diffToVersion} class="diff-select">
								<option value="">-- {$t('dataset.detail.selectDiffVersion')} --</option>
								{#each versionHistory as v}
									<option value={v.version}>{v.version}</option>
								{/each}
							</select>
							<button class="btn-diff" on:click={loadVersionDiff} disabled={!diffFromVersion || !diffToVersion || diffLoading}>
								{diffLoading ? '...' : $t('dataset.detail.compare')}
							</button>
						</div>
						{#if diffError}
							<div class="diff-error">{diffError}</div>
						{/if}
						{#if diffResult}
							<div class="diff-result">
								<div class="diff-summary">
									<div class="diff-stat {diffResult.rows_added > 0 ? 'added' : diffResult.rows_added < 0 ? 'removed' : ''}">
										<span class="diff-stat-label">{$t('dataset.detail.rowChanges')}</span>
										<span class="diff-stat-value">
											{diffResult.rows_added > 0 ? '+' : ''}{diffResult.rows_added.toLocaleString()}
										</span>
									</div>
									<div class="diff-stat {diffResult.columns_added?.length > 0 ? 'added' : ''}">
										<span class="diff-stat-label">{$t('dataset.detail.addedCols')}</span>
										<span class="diff-stat-value">{diffResult.columns_added?.length || 0}</span>
									</div>
									<div class="diff-stat {diffResult.columns_removed?.length > 0 ? 'removed' : ''}">
										<span class="diff-stat-label">{$t('dataset.detail.removedCols')}</span>
										<span class="diff-stat-value">{diffResult.columns_removed?.length || 0}</span>
									</div>
									<div class="diff-stat {diffResult.columns_type_changed?.length > 0 ? 'changed' : ''}">
										<span class="diff-stat-label">{$t('dataset.detail.typeChanges')}</span>
										<span class="diff-stat-value">{diffResult.columns_type_changed?.length || 0}</span>
									</div>
									<div class="diff-stat">
										<span class="diff-stat-label">{$t('dataset.detail.schemaCompat')}</span>
										<span class="diff-stat-value">
											{diffResult.schema_compatible ? '✅' : '⚠️'} {diffResult.schema_compatible ? $t('dataset.detail.compatible') : $t('dataset.detail.incompatible')}
										</span>
									</div>
								</div>
								{#if diffResult.columns_added?.length > 0}
									<div class="diff-detail">
										<span class="diff-detail-label">{$t('dataset.detail.addedCols')}:</span>
										{#each diffResult.columns_added as col}
											<span class="diff-tag added">+ {col}</span>
										{/each}
									</div>
								{/if}
								{#if diffResult.columns_removed?.length > 0}
									<div class="diff-detail">
										<span class="diff-detail-label">{$t('dataset.detail.removedCols')}:</span>
										{#each diffResult.columns_removed as col}
											<span class="diff-tag removed">- {col}</span>
										{/each}
									</div>
								{/if}
								{#if diffResult.columns_type_changed?.length > 0}
									<div class="diff-detail">
										<span class="diff-detail-label">{$t('dataset.detail.typeChanges')}:</span>
										{#each diffResult.columns_type_changed as change}
											<span class="diff-tag changed">
												{change.column_name}: {change.from_type} → {change.to_type}
											</span>
										{/each}
									</div>
								{/if}

								<div class="row-diff-section">
									<button class="btn-row-diff" on:click={() => loadRowDiff(diffFromVersion, diffToVersion)} disabled={rowDiffLoading}>
										{rowDiffLoading ? '...' : '📋 ' + $t('dataset.detail.viewRowDiff')}
									</button>

									{#if rowDiffData}
										<div class="row-diff-result">
											{#if rowDiffData.summary}
												<div class="row-diff-summary">
													{#if rowDiffData.summary.added > 0}
														<span class="stat-added">+{rowDiffData.summary.added} {$t('dataset.detail.rowsAdded')}</span>
													{/if}
													{#if rowDiffData.summary.removed > 0}
														<span class="stat-removed">-{rowDiffData.summary.removed} {$t('dataset.detail.rowsRemoved')}</span>
													{/if}
													{#if rowDiffData.summary.modified > 0}
														<span class="stat-modified">~{rowDiffData.summary.modified} {$t('dataset.detail.rowsModified')}</span>
													{/if}
												</div>
											{/if}
											{#if rowDiffData.rows?.length > 0}
												<div class="row-diff-table-wrapper">
													<table class="row-diff-table">
														<thead>
															<tr>
																<th>{$t('dataset.detail.colType')}</th>
																{#each rowDiffData.columns || [] as col}
																	<th>{col}</th>
																{/each}
															</tr>
														</thead>
														<tbody>
															{#each rowDiffData.rows as row}
																<tr class="diff-row-{row.diff_type || 'unchanged'}">
																	<td class="diff-type-cell">
																		{#if row.diff_type === 'added'}➕{:else if row.diff_type === 'removed'}➖{:else if row.diff_type === 'modified'}✏️{:else}—{/if}
																	</td>
																	{#each row.values || [] as val}
																		<td>{val != null ? String(val) : ''}</td>
																	{/each}
																</tr>
															{/each}
														</tbody>
													</table>
												</div>
												{#if rowDiffData.total > rowDiffData.rows.length}
													<div class="row-diff-more">
														{$t('dataset.detail.showingDiff', { shown: rowDiffData.rows.length, total: rowDiffData.total })}
														<button class="btn-load-more" on:click={() => { rowDiffOffset += ROW_DIFF_LIMIT; loadRowDiff(diffFromVersion, diffToVersion); }}>
															{$t('dataset.detail.loadMore')}
														</button>
													</div>
												{/if}
											{/if}
										</div>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
			{/if}

			{#if activeDetailTab === 'readiness'}
			<div class="info-card">
				<div class="card-header">
					<h4>🎯 {$t('readiness.title')}</h4>
					<button class="btn-link" on:click={loadReadiness} disabled={readinessLoading}>
						{readinessLoading ? '...' : '🔄'}
					</button>
				</div>
				<TrainingReadinessDashboard
					{datasetId}
					{qualityScore}
					healthCheckResults={healthCheckResults}
					readinessScore={readinessData}
					loading={readinessLoading}
				/>
			</div>
			{/if}

			{#if activeDetailTab === 'lineage'}
			<div class="info-card">
				<LineageGraph graph={lineageGraph} />
			</div>
			{/if}

			{#if activeDetailTab === 'recipe'}
			<div class="info-card">
				<DataRecipeBuilder availableDatasets={dataset ? [dataset] : []} />
			</div>
			{/if}

			{#if activeDetailTab === 'version'}
			<div class="info-card">
				<DataVersionControl datasetPath={dataset?.path || ''} datasetId={dataset?.id || ''} />
			</div>
			{/if}

			{#if activeDetailTab === 'label'}
			<div class="info-card">
				<LabelQualityPanel datasetId={dataset?.id || ''} />
			</div>
			{/if}

			{#if activeDetailTab === 'analysis'}
			<div class="info-card">
				<DataAnalysisPanel datasetId={dataset?.id || ''} />
			</div>
			{/if}

			{#if activeDetailTab === 'card'}
			<div class="info-card">
				<DatasetCardPanel datasetId={dataset?.id || ''} />
			</div>
			{/if}

			{#if activeDetailTab === 'tools'}
			<div class="info-card">
				<AdvancedDataTools datasetId={dataset?.id || ''} />
			</div>
			{/if}
		</div>
	{/if}
</div>

{#if showNewVersion}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (showNewVersion = false)} on:keydown={(e) => { if (e.key === 'Escape') showNewVersion = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.createNewVersion')}</h3>
			<div class="modal-body">
				<label for="version-note">{$t('dataset.detail.versionNote')}</label>
				<input id="version-note" type="text" bind:value={newVersionNote} placeholder={$t('dataset.detail.versionNotePlaceholder')} class="input" />
			</div>
			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (showNewVersion = false)}>{$t('dataset.detail.cancel')}</button>
				<button class="btn-primary" on:click={createNewVersion} disabled={creatingVersion}>
					{creatingVersion ? '...' : $t('dataset.detail.createVersion')}
				</button>
			</div>
		</div>
	</div>
{/if}

<NotificationStack />
<TaskProgressPanel />
<KeyboardShortcuts onNavigateBack={() => goto('/lab/data/list')} />

{#if confirmArchive}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (confirmArchive = false)} on:keydown={(e) => { if (e.key === 'Escape') confirmArchive = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.confirmArchive')}</h3>
			<p>{$t('dataset.detail.confirmArchiveDesc')}</p>
			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (confirmArchive = false)}>{$t('dataset.detail.cancel')}</button>
				<button class="btn-danger" on:click={archiveDataset} disabled={archiving}>
					{archiving ? '...' : $t('dataset.detail.confirmArchive')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if confirmDelete}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (confirmDelete = false)} on:keydown={(e) => { if (e.key === 'Escape') confirmDelete = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.confirmDelete')}</h3>
			<p>{$t('dataset.detail.confirmDeleteDesc')}</p>
			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (confirmDelete = false)}>{$t('dataset.detail.cancel')}</button>
				<button class="btn-danger" on:click={deleteDataset} disabled={deleting}>
					{deleting ? '...' : $t('dataset.detail.confirmDelete')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if showExport}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (showExport = false)} on:keydown={(e) => { if (e.key === 'Escape') showExport = false; }}>
		<div class="modal modal-export" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.exportDataset')}</h3>
			<p class="export-ds-name">{dataset?.name}</p>

			<div class="export-format-selector">
			<span class="export-format-label">{$t('dataset.detail.selectExportFormat')}:</span>
				<div class="format-options">
					<label class="format-option" class:active={exportTargetFormat === 'csv'}>
						<input type="radio" bind:group={exportTargetFormat} value="csv" />
						<span class="format-icon">📄</span>
						<span class="format-name">CSV</span>
						<span class="format-desc">{$t('dataset.detail.csvDesc')}</span>
					</label>
					<label class="format-option" class:active={exportTargetFormat === 'json'}>
						<input type="radio" bind:group={exportTargetFormat} value="json" />
						<span class="format-icon">📋</span>
						<span class="format-name">JSON</span>
						<span class="format-desc">{$t('dataset.detail.jsonDesc')}</span>
					</label>
					<label class="format-option" class:active={exportTargetFormat === 'parquet'}>
						<input type="radio" bind:group={exportTargetFormat} value="parquet" />
						<span class="format-icon">📦</span>
						<span class="format-name">Parquet</span>
						<span class="format-desc">{$t('dataset.detail.parquetDesc')}</span>
					</label>
				</div>
			</div>

			{#if exportError}
				<div class="export-error">{exportError}</div>
			{/if}

			{#if exportResult}
				<div class="export-result">
					<div class="export-result-header">{exportResult.success ? '✅ ' + $t('dataset.detail.exportSuccess') : '❌ ' + $t('dataset.detail.exportFailed')}</div>
					<div class="export-result-details">
						<div class="export-detail-row">
							<span>{$t('dataset.detail.sourceFormat')}:</span>
							<span class="export-detail-val">{exportResult.source_format?.toUpperCase()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.targetFormat')}:</span>
							<span class="export-detail-val">{exportResult.target_format?.toUpperCase()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.outputPath')}:</span>
							<span class="export-detail-val export-path">{exportResult.output_path}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.fileSize')}:</span>
							<span class="export-detail-val">{(exportResult.file_size_bytes / 1024).toFixed(1)} KB</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.exportedRows')}:</span>
							<span class="export-detail-val">{exportResult.rows_exported?.toLocaleString()}</span>
						</div>
					</div>
				</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (showExport = false)}>{$t('dataset.detail.close')}</button>
				<button class="btn-primary" on:click={exportDataset} disabled={exporting}>
					{exporting ? '...' : $t('dataset.detail.startExport')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if showSplitModal}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (showSplitModal = false)} on:keydown={(e) => { if (e.key === 'Escape') showSplitModal = false; }}>
		<div class="modal modal-split" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.createSplitTitle')}</h3>
			<p class="export-ds-name">{dataset?.name}</p>

			<div class="form-group">
				<label for="split-name-input">{$t('dataset.detail.splitName')}</label>
				<input id="split-name-input" class="input" type="text" bind:value={splitName} placeholder="default_split" />
			</div>

			<div class="split-ratio-config">
				<span class="ratio-label">{$t('dataset.detail.splitRatio')}</span>
				<div class="ratio-sliders">
					<div class="ratio-row">
						<span class="ratio-label">{$t('dataset.detail.trainSet')}</span>
						<input type="range" min="0" max="100" bind:value={splitTrainRatio} step="1" />
						<span class="ratio-val">{splitTrainRatio}%</span>
					</div>
					<div class="ratio-row">
						<span class="ratio-label">{$t('dataset.detail.valSet')}</span>
						<input type="range" min="0" max="100" bind:value={splitValRatio} step="1" />
						<span class="ratio-val">{splitValRatio}%</span>
					</div>
					<div class="ratio-row">
						<span class="ratio-label">{$t('dataset.detail.testSet')}</span>
						<input type="range" min="0" max="100" bind:value={splitTestRatio} step="1" />
						<span class="ratio-val">{splitTestRatio}%</span>
					</div>
				</div>
				<div class="ratio-bar">
					<div class="ratio-seg train" style="width: {splitTrainRatio}%"></div>
					<div class="ratio-seg val" style="width: {splitValRatio}%"></div>
					<div class="ratio-seg test" style="width: {splitTestRatio}%"></div>
				</div>
			</div>

			<div class="form-row">
				<label class="checkbox-label">
					<input type="checkbox" bind:checked={splitShuffle} />
					{$t('dataset.detail.randomShuffle')}
				</label>
				<div class="form-group-sm">
					<label for="split-seed">{$t('dataset.detail.randomSeed')}</label>
					<input id="split-seed" class="input input-sm" type="number" bind:value={splitSeed} />
				</div>
			</div>

			{#if splitError}
				<div class="export-error">{splitError}</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (showSplitModal = false)}>{$t('dataset.detail.cancel')}</button>
				<button class="btn-primary" on:click={createSplit} disabled={splitCreating}>
					{splitCreating ? '...' : $t('dataset.detail.createSplit')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if showDedupModal}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (showDedupModal = false)} on:keydown={(e) => { if (e.key === 'Escape') showDedupModal = false; }}>
		<div class="modal modal-dedup" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.detail.dedupTitle')}</h3>
			<p class="export-ds-name">{dataset?.name}</p>

			<div class="form-group">
				<label for="dedup-threshold">{$t('dataset.detail.similarityThreshold')}</label>
				<input id="dedup-threshold" class="input" type="number" bind:value={dedupThreshold} min="0" max="1" step="0.05" />
				<span class="input-hint">{$t('dataset.detail.thresholdHint')}</span>
			</div>

			<div class="form-row">
				<div class="form-group">
					<label for="dedup-perm">{$t('dataset.detail.minhashPerms')}</label>
					<input id="dedup-perm" class="input" type="number" bind:value={dedupNumPerm} min="32" max="512" step="32" />
				</div>
				<div class="form-group">
					<label for="dedup-ngram">{$t('dataset.detail.ngramSize')}</label>
					<input id="dedup-ngram" class="input" type="number" bind:value={dedupNGram} min="1" max="10" />
				</div>
			</div>

			{#if dedupError}
				<div class="export-error">{dedupError}</div>
			{/if}

			{#if dedupResult}
				<div class="export-result">
					<div class="export-result-header">✅ {$t('dataset.detail.dedupComplete')}</div>
					<div class="export-result-details">
						<div class="export-detail-row">
							<span>{$t('dataset.detail.originalRows')}:</span>
							<span class="export-detail-val">{dedupResult.original_rows?.toLocaleString()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.duplicatesFound')}:</span>
							<span class="export-detail-val">{dedupResult.duplicates_found?.toLocaleString()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.removed')}:</span>
							<span class="export-detail-val">{dedupResult.duplicates_removed?.toLocaleString()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.remainingRows')}:</span>
							<span class="export-detail-val">{dedupResult.remaining_rows?.toLocaleString()}</span>
						</div>
						<div class="export-detail-row">
							<span>{$t('dataset.detail.dedupRate')}:</span>
							<span class="export-detail-val">{(dedupResult.dedup_ratio * 100).toFixed(2)}%</span>
						</div>
					</div>
				</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (showDedupModal = false)}>{$t('dataset.detail.close')}</button>
				<button class="btn-warn" on:click={runDedup} disabled={dedupRunning}>
					{dedupRunning ? '...' : $t('dataset.detail.startDedup')}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.detail-page { padding: 0; }

	.page-header { margin-bottom: 1rem; }
	.btn-back {
		background: none;
		border: none;
		color: #9ca3af;
		font-size: 0.85rem;
		cursor: pointer;
		padding: 0;
	}
	.btn-back:hover { color: #d1d5db; }

	.error-banner {
		padding: 0.65rem 0.85rem;
		background: rgba(239,68,68,0.1);
		border: 1px solid rgba(239,68,68,0.25);
		border-radius: 6px;
		color: #fca5a5;
		font-size: 0.82rem;
		margin-bottom: 1rem;
	}

	.skeleton-detail { display: flex; flex-direction: column; gap: 0.5rem; }
	.skeleton-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
		margin-top: 0.5rem;
	}

	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 0.75rem;
	}
	.detail-header h2 { margin: 0 0 0.4rem 0; font-size: 1.3rem; }
	.header-meta { display: flex; align-items: center; gap: 0.65rem; font-size: 0.8rem; color: #9ca3af; }
	.header-actions { display: flex; gap: 0.5rem; }

	.detail-tabs {
		display: flex; gap: 0.25rem; margin-bottom: 1rem;
		border-bottom: 1px solid rgba(148,163,184,0.15); padding-bottom: 0.5rem;
		flex-wrap: wrap;
	}
	.detail-tab {
		padding: 0.35rem 0.65rem; border: 1px solid transparent; border-radius: 6px 6px 0 0;
		background: none; color: #9ca3af; font-size: 0.78rem; cursor: pointer;
		transition: all 0.15s;
	}
	.detail-tab:hover { color: #d1d5db; background: rgba(255,255,255,0.03); }
	.detail-tab.active {
		color: #93c5fd; background: rgba(59,130,246,0.08);
		border-color: rgba(59,130,246,0.2); border-bottom-color: transparent;
		font-weight: 600;
	}

	.status-badge {
		display: inline-block;
		padding: 0.1rem 0.45rem;
		border-radius: 3px;
		font-size: 0.72rem;
		font-weight: 500;
	}
	.status-badge.active { background: rgba(16,185,129,0.12); color: #6ee7b7; }
	.status-badge.archived { background: rgba(107,114,128,0.12); color: #9ca3af; }

	.info-card {
		background: rgba(255,255,255,0.015);
		border: 1px solid rgba(107,114,128,0.15);
		border-radius: 8px;
		padding: 1rem;
		margin-bottom: 0.85rem;
	}
	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.65rem;
	}
	.card-header h4 { margin: 0; font-size: 0.9rem; color: #e5e7eb; }

	.btn-link {
		background: none;
		border: none;
		color: #60a5fa;
		font-size: 0.78rem;
		cursor: pointer;
		padding: 0;
	}
	.btn-link:hover { color: #93c5fd; }

	.desc-text { color: #9ca3af; font-size: 0.84rem; margin: 0; line-height: 1.5; }
	.no-data { color: #6b7280; font-size: 0.8rem; }

	.edit-desc { display: flex; flex-direction: column; gap: 0.5rem; }
	.textarea {
		width: 100%;
		padding: 0.5rem;
		background: rgba(255,255,255,0.04);
		border: 1px solid rgba(107,114,128,0.2);
		border-radius: 5px;
		color: #e5e7eb;
		font-size: 0.84rem;
		resize: vertical;
		outline: none;
		box-sizing: border-box;
	}
	.textarea:focus { border-color: #3b82f6; }
	.edit-actions { display: flex; justify-content: flex-end; gap: 0.4rem; }

	.tags-list { display: flex; flex-wrap: wrap; gap: 0.4rem; }
	.tag {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.15rem 0.5rem;
		background: rgba(59,130,246,0.1);
		border: 1px solid rgba(59,130,246,0.2);
		border-radius: 4px;
		font-size: 0.75rem;
		color: #93c5fd;
	}
	.tag-remove {
		background: none;
		border: none;
		color: #93c5fd;
		cursor: pointer;
		font-size: 0.65rem;
		padding: 0;
		line-height: 1;
	}
	.tag-remove:hover { color: #fca5a5; }

	.add-tag-row {
		display: flex;
		gap: 0.4rem;
		margin-top: 0.5rem;
	}
	.input-sm {
		flex: 1;
		padding: 0.3rem 0.5rem;
		background: rgba(255,255,255,0.04);
		border: 1px solid rgba(107,114,128,0.2);
		border-radius: 4px;
		color: #e5e7eb;
		font-size: 0.8rem;
		outline: none;
	}
	.input-sm:focus { border-color: #3b82f6; }

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 0.65rem;
	}
	.info-item { display: flex; flex-direction: column; gap: 0.15rem; }
	.info-label { font-size: 0.7rem; color: #6b7280; text-transform: uppercase; letter-spacing: 0.03em; }
	.info-value { font-size: 0.84rem; color: #d1d5db; }
	.path-value { font-family: monospace; font-size: 0.75rem; word-break: break-all; }

	.profiles-table-wrapper { overflow-x: auto; }
	.profiles-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.8rem;
	}
	.profiles-table th {
		text-align: left;
		padding: 0.4rem 0.5rem;
		color: #9ca3af;
		font-weight: 500;
		font-size: 0.7rem;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		border-bottom: 1px solid rgba(107,114,128,0.15);
		white-space: nowrap;
	}
	.profiles-table td {
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid rgba(107,114,128,0.06);
		color: #d1d5db;
	}
	.profiles-table tbody tr:hover { background: rgba(59,130,246,0.03); }
	.col-name-cell { display: flex; align-items: center; gap: 0.35rem; font-weight: 500; }
	.col-icon { font-size: 0.85rem; }
	.num-cell { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
	.high-null { color: #fca5a5; }

	.type-badge {
		display: inline-block;
		padding: 0.08rem 0.35rem;
		border-radius: 3px;
		font-size: 0.68rem;
		font-weight: 500;
		background: rgba(107,114,128,0.12);
		color: #9ca3af;
	}

	.version-list { position: relative; padding-left: 1.25rem; }
	.version-item { position: relative; padding-bottom: 0.85rem; }
	.version-item:last-child { padding-bottom: 0; }
	.version-dot {
		position: absolute;
		left: -1.25rem;
		top: 0.3rem;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #3b82f6;
	}
	.version-dot.current {
		background: #3b82f6;
		box-shadow: 0 0 6px rgba(59,130,246,0.4);
	}
	.version-line {
		position: absolute;
		left: calc(-1.25rem + 3px);
		top: 0.85rem;
		bottom: 0;
		width: 2px;
		background: rgba(107,114,128,0.2);
	}
	.version-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.15rem; }
	.version-tag {
		font-size: 0.8rem;
		font-weight: 500;
		color: #e5e7eb;
		background: rgba(59,130,246,0.1);
		padding: 0.08rem 0.4rem;
		border-radius: 3px;
	}
	.current-badge {
		font-size: 0.6rem; padding: 0.08rem 0.3rem; border-radius: 3px;
		background: rgba(59,130,246,0.15); color: #93c5fd; font-weight: 500;
	}
	.version-date { font-size: 0.72rem; color: #6b7280; }
	.version-stats { font-size: 0.75rem; color: #9ca3af; }
	.version-note { font-size: 0.75rem; color: #6b7280; margin-top: 0.15rem; }
	.version-actions { margin-top: 0.3rem; }
	.btn-rollback {
		padding: 0.2rem 0.5rem; border-radius: 4px;
		background: none; border: 1px solid rgba(234,179,8,0.25);
		color: #fbbf24; font-size: 0.68rem; cursor: pointer;
	}
	.btn-rollback:hover { background: rgba(234,179,8,0.06); }
	.btn-row-diff {
		margin-top: 0.75rem; padding: 0.35rem 0.65rem; border-radius: 4px;
		background: rgba(59,130,246,0.1); border: 1px solid rgba(59,130,246,0.2);
		color: #93c5fd; font-size: 0.75rem; cursor: pointer;
	}
	.btn-row-diff:hover:not(:disabled) { background: rgba(59,130,246,0.16); }
	.btn-row-diff:disabled { opacity: 0.4; cursor: not-allowed; }
	.row-diff-result { margin-top: 0.5rem; }
	.row-diff-summary { display: flex; gap: 0.75rem; margin-bottom: 0.5rem; }
	.stat-added { color: #6ee7b7; font-size: 0.78rem; }
	.stat-removed { color: #fca5a5; font-size: 0.78rem; }
	.stat-modified { color: #93c5fd; font-size: 0.78rem; }
	.row-diff-table-wrapper { overflow-x: auto; }
	.row-diff-table {
		width: 100%; border-collapse: collapse; font-size: 0.72rem;
	}
	.row-diff-table th {
		text-align: left; padding: 0.3rem 0.5rem;
		background: rgba(255,255,255,0.03); color: #94a3b8;
		border-bottom: 1px solid rgba(107,114,128,0.15);
	}
	.row-diff-table td {
		padding: 0.25rem 0.5rem; border-bottom: 1px solid rgba(107,114,128,0.08);
		color: #d1d5db; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
	}
	.diff-type-cell { width: 28px; text-align: center; }
	.diff-row-added td { background: rgba(16,185,129,0.04); }
	.diff-row-removed td { background: rgba(239,68,68,0.04); }
	.diff-row-modified td { background: rgba(59,130,246,0.04); }
	.row-diff-more {
		display: flex; align-items: center; gap: 0.5rem;
		padding: 0.4rem 0; font-size: 0.72rem; color: #94a3b8;
	}
	.btn-load-more {
		padding: 0.2rem 0.5rem; border-radius: 4px;
		background: rgba(255,255,255,0.04); border: 1px solid rgba(107,114,128,0.2);
		color: #d1d5db; font-size: 0.68rem; cursor: pointer;
	}
	.btn-load-more:hover { background: rgba(255,255,255,0.08); }

	.loading-row { padding: 0.5rem 0; }

	.btn-sm {
		padding: 0.25rem 0.55rem;
		font-size: 0.75rem;
		border-radius: 4px;
		border: 1px solid rgba(107,114,128,0.25);
		background: rgba(255,255,255,0.04);
		color: #d1d5db;
		cursor: pointer;
	}
	.btn-sm:hover { background: rgba(255,255,255,0.08); }
	.btn-primary-sm { background: #3b82f6; border-color: #3b82f6; color: #fff; }
	.btn-primary-sm:hover { background: #2563eb; }

	.btn-danger-outline {
		padding: 0.35rem 0.75rem;
		font-size: 0.8rem;
		border-radius: 5px;
		border: 1px solid rgba(239,68,68,0.3);
		background: transparent;
		color: #fca5a5;
		cursor: pointer;
	}
	.btn-danger-outline:hover { background: rgba(239,68,68,0.1); }

	.btn-warn {
		padding: 0.4rem 0.85rem;
		font-size: 0.82rem;
		border-radius: 5px;
		border: 1px solid rgba(245,158,11,0.3);
		background: rgba(245,158,11,0.1);
		color: #fcd34d;
		cursor: pointer;
	}
	.btn-warn:hover { background: rgba(245,158,11,0.18); }

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.modal {
		background: #1f2937;
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 10px;
		padding: 1.5rem;
		max-width: 440px;
		width: 90%;
	}
	.modal h3 { margin: 0 0 0.85rem 0; color: #e5e7eb; font-size: 1rem; }
	.modal p { color: #9ca3af; font-size: 0.84rem; margin: 0 0 1.25rem 0; }
	.form-group { margin-bottom: 0.85rem; }
	.form-group label { display: block; font-size: 0.78rem; color: #9ca3af; margin-bottom: 0.3rem; }
	.input {
		width: 100%;
		padding: 0.45rem 0.6rem;
		background: rgba(255,255,255,0.05);
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 5px;
		color: #e5e7eb;
		font-size: 0.85rem;
		outline: none;
		box-sizing: border-box;
	}
	.input:focus { border-color: #3b82f6; }
	.modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; }

	.modal-export { max-width: 520px; }
	.export-ds-name { color: #9ca3af; font-size: 0.85rem; margin-bottom: 1rem; }
	.export-format-selector { margin-bottom: 1rem; }
	.export-format-label { display: block; font-size: 0.85rem; color: #d1d5db; margin-bottom: 0.75rem; }
	.format-options { display: flex; gap: 0.75rem; }
	.format-option {
		flex: 1;
		display: flex; flex-direction: column; align-items: center; gap: 0.3rem;
		padding: 0.75rem 0.5rem;
		border: 1.5px solid rgba(75,85,99,0.3);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		background: rgba(15,23,42,0.4);
	}
	.format-option:hover { border-color: rgba(59,130,246,0.4); background: rgba(59,130,246,0.06); }
	.format-option.active { border-color: #3b82f6; background: rgba(59,130,246,0.1); }
	.format-option input { display: none; }
	.format-icon { font-size: 1.3rem; }
	.format-name { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
	.format-desc { font-size: 0.7rem; color: #9ca3af; }
	.export-error { padding: 0.5rem 0.75rem; background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; color: #fca5a5; font-size: 0.8rem; margin-bottom: 0.75rem; }
	.export-result { padding: 0.75rem; background: rgba(16,185,129,0.06); border: 1px solid rgba(16,185,129,0.2); border-radius: 8px; margin-bottom: 0.75rem; }
	.export-result-header { font-weight: 600; font-size: 0.9rem; color: #10b981; margin-bottom: 0.5rem; }
	.export-result-details { display: flex; flex-direction: column; gap: 0.3rem; }
	.export-detail-row { display: flex; justify-content: space-between; font-size: 0.8rem; color: #d1d5db; }
	.export-detail-val { color: #e5e7eb; font-weight: 500; }
	.export-path { font-family: monospace; font-size: 0.72rem; color: #93c5fd; max-width: 260px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.quality-overview { display: flex; gap: 1.25rem; align-items: flex-start; }
	.quality-score-circle {
		width: 80px; height: 80px;
		border-radius: 50%;
		border: 3px solid var(--score-color, #10b981);
		display: flex; flex-direction: column;
		align-items: center; justify-content: center;
		flex-shrink: 0;
		background: rgba(15,23,42,0.6);
	}
	.score-number { font-size: 1.5rem; font-weight: 700; color: #e5e7eb; line-height: 1; }
	.score-grade { font-size: 0.65rem; color: var(--score-color, #10b981); margin-top: 2px; }

	.quality-dimensions { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; }
	.dimension-bar { min-height: 0; }
	.dim-label { display: flex; justify-content: space-between; font-size: 0.72rem; color: #9ca3af; margin-bottom: 2px; }
	.dim-score { font-weight: 600; color: #d1d5db; }
	.dim-track { height: 6px; background: rgba(255,255,255,0.06); border-radius: 3px; overflow: hidden; }
	.dim-fill { height: 100%; border-radius: 3px; transition: width 0.5s ease; }

	.quality-issues { margin-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.06); padding-top: 0.75rem; }
	.quality-issues h5 { font-size: 0.75rem; color: #9ca3af; margin: 0 0 0.5rem 0; }
	.issue-item { display: flex; gap: 0.5rem; padding: 0.4rem 0; font-size: 0.75rem; }
	.issue-item.error { color: #fca5a5; }
	.issue-item.warning { color: #fde68a; }
	.issue-icon { flex-shrink: 0; }
	.issue-desc { margin-bottom: 2px; }
	.issue-suggestion { font-size: 0.7rem; color: #6b7280; }

	.quality-recommendations { margin-top: 0.75rem; border-top: 1px solid rgba(255,255,255,0.06); padding-top: 0.75rem; }
	.quality-recommendations h5 { font-size: 0.75rem; color: #9ca3af; margin: 0 0 0.5rem 0; }
	.rec-item { font-size: 0.72rem; color: #6ee7b7; padding: 0.2rem 0; }

	.quality-error { color: #fca5a5; font-size: 0.8rem; padding: 0.5rem 0; }
	.quality-empty { color: #6b7280; font-size: 0.8rem; text-align: center; padding: 1rem 0; }

	.diff-section { margin-top: 1rem; border-top: 1px solid rgba(255,255,255,0.06); padding-top: 0.75rem; }
	.diff-header h5 { font-size: 0.78rem; color: #9ca3af; margin: 0 0 0.5rem 0; }
	.diff-selectors { display: flex; gap: 0.5rem; align-items: center; }
	.diff-select {
		flex: 1;
		padding: 0.35rem 0.5rem;
		background: rgba(255,255,255,0.04);
		border: 1px solid rgba(107,114,128,0.2);
		border-radius: 4px;
		color: #d1d5db;
		font-size: 0.72rem;
		outline: none;
	}
	.diff-select:focus { border-color: #3b82f6; }
	.diff-arrow { color: #6b7280; font-size: 0.8rem; flex-shrink: 0; }
	.btn-diff {
		padding: 0.35rem 0.75rem;
		background: #3b82f6;
		border: none;
		border-radius: 4px;
		color: #fff;
		font-size: 0.72rem;
		cursor: pointer;
		flex-shrink: 0;
	}
	.btn-diff:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn-diff:hover:not(:disabled) { background: #2563eb; }

	.diff-error { color: #fca5a5; font-size: 0.72rem; margin-top: 0.5rem; }
	.diff-result { margin-top: 0.75rem; }
	.diff-summary { display: flex; gap: 0.5rem; flex-wrap: wrap; }
	.diff-stat {
		padding: 0.35rem 0.6rem;
		background: rgba(255,255,255,0.03);
		border-radius: 4px;
		border: 1px solid rgba(255,255,255,0.06);
		display: flex; flex-direction: column; gap: 2px;
	}
	.diff-stat.added { border-color: rgba(16,185,129,0.3); }
	.diff-stat.removed { border-color: rgba(239,68,68,0.3); }
	.diff-stat.changed { border-color: rgba(245,158,11,0.3); }
	.diff-stat-label { font-size: 0.6rem; color: #6b7280; }
	.diff-stat-value { font-size: 0.78rem; font-weight: 600; color: #d1d5db; }
	.diff-stat.added .diff-stat-value { color: #34d399; }
	.diff-stat.removed .diff-stat-value { color: #f87171; }
	.diff-stat.changed .diff-stat-value { color: #fbbf24; }

	.diff-detail { margin-top: 0.5rem; display: flex; gap: 0.35rem; flex-wrap: wrap; align-items: center; }
	.diff-detail-label { font-size: 0.68rem; color: #6b7280; }
	.diff-tag {
		padding: 0.15rem 0.4rem;
		border-radius: 3px;
		font-size: 0.65rem;
	}
	.diff-tag.added { background: rgba(16,185,129,0.12); color: #34d399; }
	.diff-tag.removed { background: rgba(239,68,68,0.12); color: #f87171; }
	.diff-tag.changed { background: rgba(245,158,11,0.12); color: #fbbf24; }

	.viz-section { margin-bottom: 1rem; }
	.viz-subtitle { font-size: 0.75rem; color: #9ca3af; margin: 0 0 0.5rem 0; font-weight: 500; }
	.viz-grid { display: flex; flex-wrap: wrap; gap: 1rem; }
	.viz-item { flex: 0 0 auto; border-radius: 8px; padding: 0.5rem; transition: all 0.2s; border: 1px solid transparent; }
	.viz-item.active-viz { border-color: rgba(59, 130, 246, 0.3); background: rgba(59, 130, 246, 0.05); }
	.viz-hint { font-size: 0.7rem; color: #6b7280; }
	.chart-filter-banner {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.4rem 0.75rem;
		background: rgba(59, 130, 246, 0.08);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: 6px;
		margin-bottom: 0.75rem;
		font-size: 0.78rem;
		color: #93c5fd;
	}
	.chart-filter-banner strong { color: #e2e8f0; }
	.clear-chart-filter {
		background: none;
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 4px;
		color: #fca5a5;
		font-size: 0.72rem;
		padding: 0.15rem 0.5rem;
		cursor: pointer;
	}
	.clear-chart-filter:hover { background: rgba(239, 68, 68, 0.1); }

	.header-actions-row { display: flex; gap: 0.5rem; align-items: center; }
	.health-error { padding: 0.5rem 0.75rem; background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3); border-radius: 6px; color: #fca5a5; font-size: 0.8rem; margin-bottom: 0.75rem; }
	.health-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 0.75rem; }
	.health-item { background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 8px; padding: 0.75rem; }
	.health-item-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.4rem; }
	.health-icon { font-size: 1rem; }
	.health-label { font-size: 0.82rem; color: #d1d5db; font-weight: 500; flex: 1; }
	.health-score { font-size: 0.8rem; font-weight: 700; padding: 0.15rem 0.5rem; border-radius: 4px; }
	.health-score.good { background: rgba(16,185,129,0.15); color: #34d399; }
	.health-score.warn { background: rgba(245,158,11,0.15); color: #fbbf24; }
	.health-score.bad { background: rgba(239,68,68,0.15); color: #f87171; }
	.health-level { font-size: 0.75rem; color: #9ca3af; margin-bottom: 0.4rem; }
	.health-issues { display: flex; flex-direction: column; gap: 0.25rem; margin-top: 0.4rem; }
	.health-issue { font-size: 0.72rem; padding: 0.2rem 0.4rem; border-radius: 4px; }
	.health-issue.warning { background: rgba(245,158,11,0.08); color: #fbbf24; }
	.health-issue.error { background: rgba(239,68,68,0.08); color: #fca5a5; }
	.health-details { display: flex; gap: 0.4rem; flex-wrap: wrap; font-size: 0.72rem; color: #9ca3af; }
	.detail-chip { padding: 0.1rem 0.4rem; border-radius: 3px; font-size: 0.7rem; }
	.detail-chip.pass { background: rgba(16,185,129,0.1); color: #34d399; }
	.detail-chip.fail { background: rgba(239,68,68,0.1); color: #f87171; }
	.health-risk { font-size: 0.75rem; color: #9ca3af; }
	.health-risk strong { color: #e5e7eb; }
	.health-splits { grid-column: 1 / -1; }
	.split-row { display: flex; justify-content: space-between; align-items: center; padding: 0.3rem 0; border-top: 1px solid rgba(255,255,255,0.04); font-size: 0.75rem; }
	.split-name { color: #93c5fd; font-weight: 500; }
	.split-ratio { color: #9ca3af; font-family: monospace; font-size: 0.7rem; }

	.modal-split { max-width: 480px; }
	.modal-dedup { max-width: 440px; }
	.split-ratio-config { margin-bottom: 0.75rem; }
	.split-ratio-config > .ratio-label { display: block; font-size: 0.8rem; color: #9ca3af; margin-bottom: 0.5rem; }
	.ratio-sliders { display: flex; flex-direction: column; gap: 0.4rem; margin-bottom: 0.5rem; }
	.ratio-row { display: flex; align-items: center; gap: 0.5rem; }
	.ratio-label { width: 50px; font-size: 0.75rem; color: #d1d5db; }
	.ratio-row input[type="range"] { flex: 1; accent-color: #3b82f6; }
	.ratio-val { width: 36px; text-align: right; font-size: 0.75rem; color: #e5e7eb; font-weight: 600; }
	.ratio-bar { display: flex; height: 8px; border-radius: 4px; overflow: hidden; }
	.ratio-seg.train { background: #3b82f6; }
	.ratio-seg.val { background: #f59e0b; }
	.ratio-seg.test { background: #10b981; }
	.form-group-sm { display: flex; align-items: center; gap: 0.4rem; }
	.form-group-sm label { font-size: 0.75rem; color: #9ca3af; white-space: nowrap; }
	.input-sm { width: 70px; }
	.input-hint { display: block; font-size: 0.68rem; color: #6b7280; margin-top: 0.2rem; }
	.btn-warn { padding: 0.4rem 0.9rem; border: none; border-radius: 6px; background: #f59e0b; color: #000; font-size: 0.8rem; font-weight: 600; cursor: pointer; }
	.btn-warn:hover { background: #d97706; }
	.btn-warn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
