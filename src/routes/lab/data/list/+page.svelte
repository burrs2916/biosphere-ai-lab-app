<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { datasetRegistryStore } from '$lib/lab/stores/dataset';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import type { DatasetSummary, DataFormat } from '$lib/lab/adapter/types';
	import Skeleton from '$lib/lab/components/Skeleton.svelte';
	import ConfirmDialog from '$lib/lab/components/ConfirmDialog.svelte';
	import NotificationStack from '$lib/lab/components/NotificationStack.svelte';
	import TaskProgressPanel from '$lib/lab/components/TaskProgressPanel.svelte';
	import EmptyStateGuide from '$lib/lab/components/EmptyStateGuide.svelte';
	import KeyboardShortcuts from '$lib/lab/components/KeyboardShortcuts.svelte';
	import { uxStore } from '$lib/lab/stores/uxStore';
	import { localizeError } from '$lib/lab/utils/errorLocalizer';
	import { t } from '$lib/i18n';

	let datasets: DatasetSummary[] = [];
	let loading = true;
	let error: string | null = null;

	let searchQuery = '';
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let statusFilter: 'active' | 'archived' | 'all' = 'active';
	let formatFilter: DataFormat | 'all' = 'all';

	let selectedIds = new Set<string>();
	let batchDeleting = false;
	let batchArchiving = false;
	let batchRestoring = false;

	let confirmDialog: {
		show: boolean;
		title: string;
		message: string;
		confirmLabel: string;
		danger: boolean;
		onConfirm: () => void;
	} = {
		show: false,
		title: '',
		message: '',
		confirmLabel: $t('confirm.ok'),
		danger: false,
		onConfirm: () => {},
	};

	let showRegisterModal = false;
	let regName = '';
	let regPath = '';
	let regFormat: DataFormat = 'csv';
	let regDescription = '';
	let registering = false;
	let regError: string | null = null;
	let regNameTouched = false;
	let regPathTouched = false;

	$: regNameError = (() => {
		if (!regNameTouched) return '';
		if (!regName.trim()) return $t('dataset.register.nameRequired');
		if (regName.trim().length < 2) return $t('dataset.register.nameMinLength');
		if (regName.trim().length > 64) return $t('dataset.register.nameMaxLength');
		if (!/^[a-zA-Z0-9_\-\u4e00-\u9fff]+$/.test(regName.trim())) return $t('dataset.register.nameInvalidChars');
		if (datasets.some(d => d.name === regName.trim())) return $t('dataset.register.nameDuplicate');
		return '';
	})();

	$: regPathError = (() => {
		if (!regPathTouched) return '';
		if (!regPath.trim()) return $t('dataset.register.pathRequired');
		if (regPath.trim().includes(' ')) return $t('dataset.register.pathNoSpaces');
		const ext = regPath.trim().split('.').pop()?.toLowerCase();
		const validExts: Record<string, boolean> = { csv: true, json: true, parquet: true, txt: true, jsonl: true, tsv: true };
		if (ext && !validExts[ext]) return $t('dataset.register.pathUnsupportedFormat') + `: .${ext}`;
		return '';
	})();

	$: regFormValid = !regNameError && !regPathError && regName.trim() && regPath.trim();

	let showAdvancedFilters = false;
	let sizeFilter: 'all' | 'small' | 'medium' | 'large' | 'huge' = 'all';
	let rowsFilter: 'all' | 'tiny' | 'small' | 'medium' | 'large' = 'all';
	let qualityFilter: 'all' | 'excellent' | 'good' | 'fair' | 'poor' = 'all';
	let sortBy: string = 'updated';
	let sortDir: 'asc' | 'desc' = 'desc';

	let unsub: (() => void) | null = null;
	let autoRefreshTimer: ReturnType<typeof setInterval> | null = null;
	const AUTO_REFRESH_INTERVAL = 30000;

	const formatOptions: { value: DataFormat | 'all'; labelKey: string }[] = [
		{ value: 'all', labelKey: 'dataset.allFormats' },
		{ value: 'csv', labelKey: 'CSV' },
		{ value: 'json', labelKey: 'JSON' },
		{ value: 'parquet', labelKey: 'Parquet' },
		{ value: 'text', labelKey: 'Text' },
		{ value: 'image', labelKey: 'Image' },
		{ value: 'huggingface', labelKey: 'HuggingFace' },
	];

	$: filteredDatasets = datasets
		.filter((d) => {
			if (statusFilter !== 'all' && d.status !== statusFilter) return false;
			if (formatFilter !== 'all' && d.format !== formatFilter) return false;
			if (searchQuery && !d.name.toLowerCase().includes(searchQuery.toLowerCase()) && !d.id.toLowerCase().includes(searchQuery.toLowerCase())) return false;
			if (sizeFilter !== 'all') {
			const mb = d.memory_size_mb || 0;
			if (sizeFilter === 'small' && mb >= 10) return false;
			if (sizeFilter === 'medium' && (mb < 10 || mb >= 100)) return false;
			if (sizeFilter === 'large' && (mb < 100 || mb >= 1000)) return false;
			if (sizeFilter === 'huge' && mb < 1000) return false;
		}
			if (rowsFilter !== 'all') {
			const rows = d.rows || 0;
				if (rowsFilter === 'tiny' && rows >= 1000) return false;
				if (rowsFilter === 'small' && (rows < 1000 || rows >= 10000)) return false;
				if (rowsFilter === 'medium' && (rows < 10000 || rows >= 100000)) return false;
				if (rowsFilter === 'large' && rows < 100000) return false;
			}
			if (qualityFilter !== 'all') {
			const hasMissing = d.has_missing_values ?? true;
			if (qualityFilter === 'excellent' && hasMissing) return false;
			if (qualityFilter === 'good' && hasMissing) return false;
			if (qualityFilter === 'fair' && !hasMissing) return false;
			if (qualityFilter === 'poor' && !hasMissing) return false;
		}
			return true;
		})
		.sort((a, b) => {
			let cmp = 0;
			switch (sortBy) {
				case 'name': cmp = a.name.localeCompare(b.name); break;
				case 'size': cmp = (a.memory_size_mb || 0) - (b.memory_size_mb || 0); break;
			case 'rows': cmp = (a.rows || 0) - (b.rows || 0); break;
			case 'quality': cmp = Number(a.has_missing_values ?? true) - Number(b.has_missing_values ?? true); break;
				default: cmp = new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(); break;
			}
			return sortDir === 'desc' ? -cmp : cmp;
		});

	$: hasAdvancedFilters = sizeFilter !== 'all' || rowsFilter !== 'all' || qualityFilter !== 'all';

	$: activeCount = datasets.filter((d) => d.status === 'active').length;
	$: archivedCount = datasets.filter((d) => d.status === 'archived').length;
	$: allSelected = filteredDatasets.length > 0 && filteredDatasets.every((d) => selectedIds.has(d.id));
	$: someSelected = selectedIds.size > 0;

	function onSearchInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			searchQuery = searchQuery;
		}, 200);
	}

	function toggleSelectAll() {
		if (allSelected) {
			selectedIds = new Set();
		} else {
			selectedIds = new Set(filteredDatasets.map((d) => d.id));
		}
	}

	function toggleSelect(id: string) {
		const next = new Set(selectedIds);
		if (next.has(id)) {
			next.delete(id);
		} else {
			next.add(id);
		}
		selectedIds = next;
	}

	function clearSelection() {
		selectedIds = new Set();
	}

	async function loadDatasets() {
		loading = true;
		error = null;
		try {
			await datasetRegistryStore.fetchDatasets();
			uxStore.markRefresh();
		} catch (e: any) {
			error = e?.toString() || $t('dataset.loadError');
		} finally {
			loading = false;
		}
	}

	function showBatchConfirm(type: 'archive' | 'delete' | 'restore') {
		const ids = [...selectedIds];
		const count = ids.length;
		if (count === 0) return;

		const configs = {
			archive: {
				title: $t('dataset.batch.archiveTitle'),
				message: $t('dataset.batch.archiveMessage', { count }),
				confirmLabel: $t('dataset.confirmArchive'),
				danger: false,
			},
			delete: {
				title: $t('dataset.batch.deleteTitle'),
				message: $t('dataset.batch.deleteMessage', { count }),
				confirmLabel: $t('dataset.confirmDelete'),
				danger: true,
			},
			restore: {
				title: $t('dataset.batch.restoreTitle'),
				message: $t('dataset.batch.restoreMessage', { count }),
				confirmLabel: $t('dataset.confirmRestore'),
				danger: false,
			},
		};

		const cfg = configs[type];
		confirmDialog = {
			show: true,
			...cfg,
			onConfirm: async () => {
				confirmDialog.show = false;
				if (type === 'archive') await executeBatchArchive(ids);
				else if (type === 'delete') await executeBatchDelete(ids);
				else if (type === 'restore') await executeBatchRestore(ids);
			},
		};
	}

	function showSingleConfirm(type: 'archive' | 'delete' | 'restore', ds: DatasetSummary) {
		const configs = {
			archive: {
				title: $t('dataset.single.archiveTitle'),
				message: $t('dataset.single.archiveMessage', { name: ds.name }),
				confirmLabel: $t('dataset.confirmArchive'),
				danger: false,
			},
			delete: {
				title: $t('dataset.single.deleteTitle'),
				message: $t('dataset.single.deleteMessage', { name: ds.name }),
				confirmLabel: $t('dataset.confirmDelete'),
				danger: true,
			},
			restore: {
				title: $t('dataset.single.restoreTitle'),
				message: $t('dataset.single.restoreMessage', { name: ds.name }),
				confirmLabel: $t('dataset.confirmRestore'),
				danger: false,
			},
		};

		const cfg = configs[type];
		confirmDialog = {
			show: true,
			...cfg,
			onConfirm: async () => {
				confirmDialog.show = false;
				if (type === 'archive') await datasetRegistryStore.archiveDataset(ds.id);
				else if (type === 'delete') await datasetRegistryStore.deleteDataset(ds.id);
				else if (type === 'restore') await datasetRegistryStore.restoreDataset(ds.id);
			},
		};
	}

	async function executeBatchArchive(ids: string[]) {
		batchArchiving = true;
		const optId = uxStore.addOptimisticUpdate('batch-archive', ids.join(','), {
			ids,
			previousStatuses: ids.map(id => {
				const ds = datasets.find(d => d.id === id);
				return ds ? { id, status: ds.status } : null;
			}).filter(Boolean),
		});
		let successCount = 0;
		let failCount = 0;
		try {
			for (const id of ids) {
				try {
					await datasetRegistryStore.archiveDataset(id);
					successCount++;
				} catch {
					failCount++;
				}
			}
			clearSelection();
			uxStore.commitOptimisticUpdate(optId);
			if (failCount === 0) {
				uxStore.success($t('dataList.batchArchiveDone'), $t('dataList.batchArchiveSuccess', { count: successCount }));
			} else {
				uxStore.warning($t('dataList.batchArchivePartial'), $t('dataList.batchPartialSuccess', { success: successCount, fail: failCount }));
			}
		} catch (e: any) {
			uxStore.rollbackOptimisticUpdate(optId);
			uxStore.error($t('dataList.batchArchiveFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		} finally {
			batchArchiving = false;
		}
	}

	async function executeBatchDelete(ids: string[]) {
		batchDeleting = true;
		const optId = uxStore.addOptimisticUpdate('batch-delete', ids.join(','), {
			ids,
			datasets: ids.map(id => datasets.find(d => d.id === id)).filter(Boolean),
		});
		let successCount = 0;
		let failCount = 0;
		try {
			for (const id of ids) {
				try {
					await datasetRegistryStore.deleteDataset(id);
					successCount++;
				} catch {
					failCount++;
				}
			}
			clearSelection();
			uxStore.commitOptimisticUpdate(optId);
			if (failCount === 0) {
				uxStore.success($t('dataList.batchDeleteDone'), $t('dataList.batchDeleteSuccess', { count: successCount }));
			} else {
				uxStore.warning($t('dataList.batchDeletePartial'), $t('dataList.batchPartialSuccess', { success: successCount, fail: failCount }));
			}
		} catch (e: any) {
			uxStore.rollbackOptimisticUpdate(optId);
			uxStore.error($t('dataList.batchDeleteFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		} finally {
			batchDeleting = false;
		}
	}

	async function executeBatchRestore(ids: string[]) {
		batchRestoring = true;
		const optId = uxStore.addOptimisticUpdate('batch-restore', ids.join(','), {
			ids,
			previousStatuses: ids.map(id => {
				const ds = datasets.find(d => d.id === id);
				return ds ? { id, status: ds.status } : null;
			}).filter(Boolean),
		});
		let successCount = 0;
		let failCount = 0;
		try {
			for (const id of ids) {
				try {
					await datasetRegistryStore.restoreDataset(id);
					successCount++;
				} catch {
					failCount++;
				}
			}
			clearSelection();
			uxStore.commitOptimisticUpdate(optId);
			if (failCount === 0) {
				uxStore.success($t('dataList.batchRestoreDone'), $t('dataList.batchRestoreSuccess', { count: successCount }));
			} else {
				uxStore.warning($t('dataList.batchRestorePartial'), $t('dataList.batchPartialSuccess', { success: successCount, fail: failCount }));
			}
		} catch (e: any) {
			uxStore.rollbackOptimisticUpdate(optId);
			uxStore.error($t('dataList.batchRestoreFailed'), localizeError(e).message + '\n💡 ' + localizeError(e).suggestion);
		} finally {
			batchRestoring = false;
		}
	}

	async function selectFile() {
		const client = getLabClient();
		const path = await client.selectFile([
			{ name: 'Data Files', extensions: ['csv', 'json', 'parquet'] },
			{ name: 'All Files', extensions: ['*'] },
		]);
		if (path) {
			regPath = path;
			regPathTouched = true;
			const ext = path.split('.').pop()?.toLowerCase();
			if (ext === 'csv') regFormat = 'csv';
			else if (ext === 'json') regFormat = 'json';
			else if (ext === 'parquet') regFormat = 'parquet';
		}
	}

	async function registerDataset() {
		regNameTouched = true;
		regPathTouched = true;
		if (!regName.trim() || !regPath.trim()) return;

		registering = true;
		regError = null;
		try {
			await datasetRegistryStore.registerDataset(regName.trim(), regFormat, regPath.trim());
			showRegisterModal = false;
			regName = '';
			regPath = '';
			regNameTouched = false;
			regPathTouched = false;
			uxStore.success($t('dataList.registerSuccess'), $t('dataList.registerSuccessMsg', { name: regName.trim() }));
		} catch (e: any) {
			regError = e?.toString() || $t('dataList.registerFailed');
			uxStore.error($t('dataList.registerFailed'), localizeError(regError).message + '\n💡 ' + localizeError(regError).suggestion);
		} finally {
			registering = false;
		}
	}

	function viewDetail(id: string) {
		goto(`/lab/data/${id}`);
	}

	function formatIcon(format: DataFormat): string {
		switch (format) {
			case 'csv': return '📄';
			case 'json': return '📋';
			case 'parquet': return '📦';
			case 'image': return '🖼';
			case 'text': return '📝';
			case 'huggingface': return '🤗';
			default: return '📎';
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	function formatSize(mb: number): string {
		if (mb < 1) return `${(mb * 1024).toFixed(0)} KB`;
		if (mb < 1024) return `${mb.toFixed(1)} MB`;
		return `${(mb / 1024).toFixed(1)} GB`;
	}

	onMount(() => {
		unsub = datasetRegistryStore.subscribe((s) => {
			datasets = [...s.datasets];
		});
		loadDatasets();
		autoRefreshTimer = setInterval(() => {
			datasetRegistryStore.fetchDatasets();
		}, AUTO_REFRESH_INTERVAL);
		datasetRegistryStore.startListening();
	});

	onDestroy(() => {
		if (unsub) unsub();
		if (debounceTimer) clearTimeout(debounceTimer);
		if (autoRefreshTimer) clearInterval(autoRefreshTimer);
		datasetRegistryStore.stopListening();
	});
</script>

<div class="list-page">
	<div class="page-header">
		<div>
			<h2>{$t('dataset.listTitle')}</h2>
			<p class="desc">
				{$t('dataset.listDesc')}
				{#if $uxStore.lastRefreshTime}
					<span class="refresh-time">· {$t('dataset.lastRefresh')}: {new Date($uxStore.lastRefreshTime).toLocaleTimeString('zh-CN')}</span>
				{/if}
			</p>
		</div>
		<div class="header-actions">
			<button class="btn-secondary" on:click={loadDatasets} disabled={loading}>
				{loading ? $t('dataset.refreshing') : '🔄 ' + $t('dataset.refresh')}
			</button>
			<button class="btn-primary" on:click={() => (showRegisterModal = true)}>+ {$t('dataset.registerDataset')}</button>
		</div>
	</div>

	{#if error}
		<div class="error-banner">
			<span>{error}</span>
			<button class="error-close" on:click={() => (error = null)}>✕</button>
		</div>
	{/if}

	<div class="toolbar">
		<div class="search-box">
			<span class="search-icon">🔍</span>
			<input
				type="text"
				placeholder={$t('dataset.search')}
				bind:value={searchQuery}
				on:input={onSearchInput}
				class="search-input"
			/>
			{#if searchQuery}
				<button class="search-clear" on:click={() => (searchQuery = '')}>✕</button>
			{/if}
		</div>
		<div class="filter-group">
			<select bind:value={formatFilter} class="filter-select">
				{#each formatOptions as opt}
					<option value={opt.value}>{$t(opt.labelKey)}</option>
				{/each}
			</select>
			<button class="btn-filter-toggle" class:active={showAdvancedFilters} on:click={() => (showAdvancedFilters = !showAdvancedFilters)}>
				⚙️ {$t('dataset.filter')}
				{#if hasAdvancedFilters}
					<span class="filter-badge">!</span>
				{/if}
			</button>
		</div>
	</div>

	{#if showAdvancedFilters}
		<div class="advanced-filters">
			<div class="filter-row">
				<label class="filter-label">{$t('dataset.sizeFilter')}</label>
				<select bind:value={sizeFilter} class="filter-select-sm">
					<option value="all">{$t('dataset.all')}</option>
					<option value="small">&lt;10MB</option>
					<option value="medium">10-100MB</option>
					<option value="large">100MB-1GB</option>
					<option value="huge">&gt;1GB</option>
				</select>

				<label class="filter-label">{$t('dataset.rowsFilter')}</label>
				<select bind:value={rowsFilter} class="filter-select-sm">
					<option value="all">{$t('dataset.all')}</option>
					<option value="tiny">&lt;1K</option>
					<option value="small">1K-10K</option>
					<option value="medium">10K-100K</option>
					<option value="large">&gt;100K</option>
				</select>

				<label class="filter-label">{$t('dataset.qualityFilter')}</label>
				<select bind:value={qualityFilter} class="filter-select-sm">
					<option value="all">{$t('dataset.all')}</option>
					<option value="excellent">{$t('dataset.qualityExcellent')}</option>
					<option value="good">{$t('dataset.qualityGood')}</option>
					<option value="fair">{$t('dataset.qualityFair')}</option>
					<option value="poor">{$t('dataset.qualityPoor')}</option>
				</select>

				<label class="filter-label">{$t('dataset.sortBy')}</label>
				<select bind:value={sortBy} class="filter-select-sm">
					<option value="updated">{$t('dataset.updated')}</option>
					<option value="name">{$t('dataset.name')}</option>
					<option value="size">{$t('dataset.size')}</option>
					<option value="rows">{$t('dataset.rows')}</option>
					<option value="quality">{$t('dataset.quality')}</option>
				</select>

				<button class="btn-sort-dir" on:click={() => (sortDir = sortDir === 'asc' ? 'desc' : 'asc')} title={sortDir === 'asc' ? $t('dataset.ascending') : $t('dataset.descending')}>
					{sortDir === 'asc' ? '↑' : '↓'}
				</button>

				{#if hasAdvancedFilters}
					<button class="btn-clear-filters" on:click={() => { sizeFilter = 'all'; rowsFilter = 'all'; qualityFilter = 'all'; }}>{$t('dataset.clearFilters')}</button>
				{/if}
			</div>
		</div>
	{/if}

	<div class="status-tabs">
		<button class="status-tab" class:active={statusFilter === 'active'} on:click={() => (statusFilter = 'active')}>
			{$t('dataset.active')} <span class="tab-count">{activeCount}</span>
		</button>
		<button class="status-tab" class:active={statusFilter === 'archived'} on:click={() => (statusFilter = 'archived')}>
			{$t('dataset.archived')} <span class="tab-count">{archivedCount}</span>
		</button>
		<button class="status-tab" class:active={statusFilter === 'all'} on:click={() => (statusFilter = 'all')}>
			{$t('dataset.all')} <span class="tab-count">{datasets.length}</span>
		</button>
	</div>

	{#if someSelected}
		<div class="batch-bar">
			<span class="batch-info">{$t('dataset.selected')} {selectedIds.size} {$t('dataset.items')}</span>
			<button class="btn-sm" on:click={clearSelection} disabled={batchArchiving || batchDeleting || batchRestoring}>{$t('dataset.clearSelection')}</button>
			{#if statusFilter === 'archived'}
				<button class="btn-sm btn-restore" on:click={() => showBatchConfirm('restore')} disabled={batchRestoring}>
					{batchRestoring ? $t('dataset.restoring') : '🔄 ' + $t('dataset.batchRestore')}
				</button>
			{:else}
				<button class="btn-sm btn-warn" on:click={() => showBatchConfirm('archive')} disabled={batchArchiving}>
					{batchArchiving ? $t('dataset.archiving') : '📦 ' + $t('dataset.batchArchive')}
				</button>
			{/if}
			<button class="btn-sm btn-danger" on:click={() => showBatchConfirm('delete')} disabled={batchDeleting}>
				{batchDeleting ? $t('dataset.deleting') : '🗑 ' + $t('dataset.batchDelete')}
			</button>
			{#if batchArchiving || batchDeleting || batchRestoring}
				<div class="batch-progress">
					<div class="batch-progress-bar"></div>
				</div>
			{/if}
		</div>
	{/if}

	{#if loading}
		<div class="skeleton-list">
			{#each Array(5) as _}
				<div class="skeleton-row">
					<Skeleton width="20px" height="20px" />
					<Skeleton width="32px" height="32px" />
					<div class="skeleton-info">
						<Skeleton width="140px" height="14px" />
						<Skeleton width="80px" height="12px" marginTop="6px" />
					</div>
					<Skeleton width="60px" height="22px" />
					<Skeleton width="70px" height="14px" />
					<Skeleton width="100px" height="14px" />
				</div>
			{/each}
		</div>
	{:else if filteredDatasets.length === 0}
		<EmptyStateGuide isFiltered={!!(searchQuery || formatFilter !== 'all')} on:register={() => (showRegisterModal = true)} />
	{:else}
		<div class="table-wrapper">
			<table class="data-table">
				<thead>
					<tr>
						<th class="col-check">
							<input type="checkbox" checked={allSelected} on:change={toggleSelectAll} />
						</th>
						<th class="col-name">{$t('dataset.name')}</th>
						<th class="col-format">{$t('dataset.format')}</th>
						<th class="col-size">{$t('dataset.size')}</th>
						<th class="col-rows">{$t('dataset.rows')}</th>
						<th class="col-cols">{$t('dataset.columns')}</th>
						<th class="col-status">{$t('dataset.status')}</th>
						<th class="col-quality">{$t('dataset.quality')}</th>
						<th class="col-date">{$t('dataset.updated')}</th>
						<th class="col-actions">{$t('dataset.actions')}</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredDatasets as ds (ds.id)}
						<tr class="data-row" class:selected={selectedIds.has(ds.id)} on:click={() => viewDetail(ds.id)} on:keydown={(e) => e.key === 'Enter' && viewDetail(ds.id)} tabindex="0" role="button" aria-label="{$t('dataset.view')} {ds.name}">
							<td class="col-check" on:click|stopPropagation>
								<input type="checkbox" checked={selectedIds.has(ds.id)} on:change={() => toggleSelect(ds.id)} />
							</td>
							<td class="col-name">
								<div class="name-cell">
									<span class="ds-icon">{formatIcon(ds.format)}</span>
									<div class="name-info">
										<span class="ds-name">{ds.name}</span>
										<span class="ds-id">{ds.id.slice(0, 8)}</span>
									</div>
								</div>
							</td>
							<td class="col-format">
								<span class="format-badge">{ds.format.toUpperCase()}</span>
							</td>
							<td class="col-size">{formatSize(ds.memory_size_mb)}</td>
							<td class="col-rows">{ds.rows.toLocaleString()}</td>
							<td class="col-cols">{ds.columns}</td>
							<td class="col-status">
								<span class="status-badge {ds.status}">
									{ds.status === 'active' ? $t('dataset.active') : $t('dataset.archived')}
								</span>
							</td>
							<td class="col-quality">
								<span
									class="quality-dot"
									class:good={!ds.has_missing_values && ds.rows > 0}
									class:warn={ds.has_missing_values}
									title={ds.has_missing_values ? $t('dataset.hasMissingValues') : $t('dataset.dataComplete')}
								></span>
							</td>
							<td class="col-date">{formatDate(ds.updated_at)}</td>
							<td class="col-actions" on:click|stopPropagation>
								<div class="action-btns">
									{#if ds.status === 'archived'}
										<button
											class="action-btn action-restore"
											title={$t('dataset.restoreDataset')}
											on:click={() => showSingleConfirm('restore', ds)}
										>🔄</button>
									{:else}
										<button
											class="action-btn action-archive"
											title={$t('dataset.archiveDataset')}
											on:click={() => showSingleConfirm('archive', ds)}
										>📦</button>
									{/if}
									<button
										class="action-btn action-delete"
										title={$t('dataset.deleteDataset')}
										on:click={() => showSingleConfirm('delete', ds)}
									>🗑</button>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

{#if showRegisterModal}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="modal-overlay" role="presentation" on:click={() => (showRegisterModal = false)} on:keydown={(e) => { if (e.key === 'Escape') showRegisterModal = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
			<h3>{$t('dataset.register.title')}</h3>

			<div class="form-group">
				<label for="reg-name">{$t('dataset.register.name')} *</label>
				<input
					id="reg-name"
					type="text"
					bind:value={regName}
					placeholder={$t('dataset.register.namePlaceholder')}
					class="input"
					class:input-error={!!regNameError}
					class:input-valid={regNameTouched && !regNameError && regName.trim()}
					on:blur={() => (regNameTouched = true)}
				/>
				{#if regNameError}
					<span class="field-error">{regNameError}</span>
				{:else if regNameTouched && regName.trim()}
					<span class="field-success">✓ {$t('dataset.register.nameAvailable')}</span>
				{/if}
			</div>

			<div class="form-group">
				<label for="reg-format">{$t('dataset.register.format')}</label>
				<select id="reg-format" bind:value={regFormat} class="input">
					<option value="csv">CSV</option>
					<option value="json">JSON</option>
					<option value="parquet">Parquet</option>
					<option value="text">Text</option>
				</select>
			</div>

			<div class="form-group">
				<label for="reg-path">{$t('dataset.register.path')} *</label>
				<div class="path-input-group">
					<input
						id="reg-path"
						type="text"
						bind:value={regPath}
						placeholder={$t('dataset.register.pathPlaceholder')}
						class="input"
						class:input-error={!!regPathError}
						class:input-valid={regPathTouched && !regPathError && regPath.trim()}
						on:blur={() => (regPathTouched = true)}
					/>
					<button class="btn-browse" on:click={selectFile}>{$t('dataset.register.selectFile')}</button>
				</div>
				{#if regPathError}
					<span class="field-error">{regPathError}</span>
				{:else if regPathTouched && regPath.trim()}
					<span class="field-success">✓ {$t('dataset.register.pathValid')}</span>
				{/if}
			</div>

			<div class="form-group">
				<label for="reg-desc">{$t('dataset.register.description')}</label>
				<textarea
					id="reg-desc"
					bind:value={regDescription}
					placeholder={$t('dataset.register.descriptionPlaceholder')}
					class="input textarea"
					rows="2"
				></textarea>
			</div>

			{#if regError}
				<div class="form-error">{regError}</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" on:click={() => (showRegisterModal = false)}>{$t('dataset.register.cancel')}</button>
				<button
					class="btn-primary"
					on:click={registerDataset}
					disabled={registering || !regFormValid}
				>
					{registering ? $t('dataset.register.registering') : $t('dataset.register.submit')}
				</button>
			</div>
		</div>
	</div>
{/if}

<ConfirmDialog
	show={confirmDialog.show}
	title={confirmDialog.title}
	message={confirmDialog.message}
	confirmLabel={confirmDialog.confirmLabel}
	danger={confirmDialog.danger}
	loading={batchArchiving || batchDeleting || batchRestoring}
	onConfirm={confirmDialog.onConfirm}
	onCancel={() => (confirmDialog.show = false)}
/>

<NotificationStack />
<TaskProgressPanel />
<KeyboardShortcuts onRefresh={loadDatasets} onNavigateBack={() => goto('/lab')} />

<style>
	.list-page { padding: 0; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.25rem;
	}
	.page-header h2 { margin: 0 0 0.25rem 0; font-size: 1.3rem; }
	.desc { color: #9ca3af; font-size: 0.85rem; margin: 0; }
	.header-actions { display: flex; gap: 0.5rem; }

	.error-banner {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.65rem 0.85rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: 6px;
		color: #fca5a5;
		font-size: 0.82rem;
		margin-bottom: 1rem;
	}
	.error-close { background: none; border: none; color: #fca5a5; cursor: pointer; font-size: 1rem; padding: 0 0.25rem; }

	.toolbar {
		display: flex;
		gap: 0.75rem;
		margin-bottom: 1rem;
		align-items: center;
	}
	.search-box {
		flex: 1;
		position: relative;
		display: flex;
		align-items: center;
	}
	.search-icon {
		position: absolute;
		left: 0.65rem;
		font-size: 0.85rem;
		pointer-events: none;
	}
	.search-input {
		width: 100%;
		padding: 0.45rem 2rem 0.45rem 2rem;
		background: rgba(255,255,255,0.05);
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 6px;
		color: #e5e7eb;
		font-size: 0.85rem;
		outline: none;
		transition: border-color 0.15s;
	}
	.search-input:focus { border-color: #3b82f6; }
	.search-input::placeholder { color: #6b7280; }
	.search-clear {
		position: absolute;
		right: 0.5rem;
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		font-size: 0.85rem;
		padding: 0.15rem 0.3rem;
	}
	.search-clear:hover { color: #d1d5db; }

	.filter-select {
		padding: 0.45rem 0.65rem;
		background: rgba(255,255,255,0.05);
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 0.82rem;
		outline: none;
	}

	.btn-filter-toggle {
		display: flex; align-items: center; gap: 0.3rem;
		padding: 0.4rem 0.6rem; border-radius: 6px;
		background: rgba(255,255,255,0.05); border: 1px solid rgba(107,114,128,0.25);
		color: #d1d5db; font-size: 0.78rem; cursor: pointer;
		position: relative;
	}

	.btn-filter-toggle:hover { background: rgba(255,255,255,0.08); }
	.btn-filter-toggle.active { background: rgba(59,130,246,0.1); border-color: rgba(59,130,246,0.3); color: #93c5fd; }

	.filter-badge {
		background: #3b82f6; color: white; font-size: 0.55rem; font-weight: 700;
		width: 14px; height: 14px; border-radius: 50%;
		display: flex; align-items: center; justify-content: center;
	}

	.advanced-filters {
		padding: 0.6rem 0.75rem; margin-bottom: 0.5rem;
		background: rgba(255,255,255,0.02);
		border: 1px solid rgba(107,114,128,0.12);
		border-radius: 8px;
		animation: slideDown 0.15s ease;
	}

	@keyframes slideDown {
		from { opacity: 0; transform: translateY(-5px); }
		to { opacity: 1; transform: translateY(0); }
	}

	.filter-row {
		display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap;
	}

	.filter-label { font-size: 0.72rem; color: #94a3b8; font-weight: 500; }

	.filter-select-sm {
		padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.04);
		border: 1px solid rgba(107,114,128,0.2); border-radius: 4px;
		color: #d1d5db; font-size: 0.75rem; outline: none;
	}

	.btn-sort-dir {
		width: 28px; height: 28px; border-radius: 4px;
		background: rgba(255,255,255,0.04); border: 1px solid rgba(107,114,128,0.2);
		color: #d1d5db; font-size: 0.85rem; cursor: pointer;
		display: flex; align-items: center; justify-content: center;
	}

	.btn-sort-dir:hover { background: rgba(255,255,255,0.08); }

	.btn-clear-filters {
		padding: 0.25rem 0.5rem; border-radius: 4px;
		background: none; border: 1px solid rgba(239,68,68,0.2);
		color: #fca5a5; font-size: 0.7rem; cursor: pointer;
	}

	.btn-clear-filters:hover { background: rgba(239,68,68,0.06); }

	.status-tabs {
		display: flex;
		gap: 0;
		border-bottom: 1px solid rgba(107,114,128,0.2);
		margin-bottom: 1rem;
	}
	.status-tab {
		padding: 0.5rem 1rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: #9ca3af;
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.15s;
		margin-bottom: -1px;
	}
	.status-tab:hover { color: #d1d5db; }
	.status-tab.active { color: #e5e7eb; border-bottom-color: #3b82f6; }
	.tab-count {
		font-size: 0.7rem;
		color: #6b7280;
		background: rgba(255,255,255,0.06);
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		margin-left: 0.35rem;
	}

	.batch-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		background: rgba(59,130,246,0.08);
		border: 1px solid rgba(59,130,246,0.2);
		border-radius: 6px;
		margin-bottom: 0.75rem;
		font-size: 0.82rem;
	}
	.batch-info { color: #93c5fd; flex: 1; }
	.batch-progress { width: 100%; height: 3px; background: rgba(59,130,246,0.15); border-radius: 2px; margin-top: 0.3rem; overflow: hidden; }
	.batch-progress-bar { width: 30%; height: 100%; background: #3b82f6; border-radius: 2px; animation: batchProgress 1.5s ease-in-out infinite; }
	@keyframes batchProgress {
		0% { transform: translateX(-100%); }
		100% { transform: translateX(400%); }
	}
	.refresh-time { font-size: 0.72rem; color: #6b7280; }

	.skeleton-list { display: flex; flex-direction: column; gap: 0.5rem; }
	.skeleton-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.65rem 0.75rem;
		background: rgba(255,255,255,0.02);
		border-radius: 6px;
	}
	.skeleton-info { display: flex; flex-direction: column; gap: 0; flex: 1; }

	.empty-state {
		text-align: center;
		padding: 3rem 1rem;
		color: #9ca3af;
	}
	.empty-icon { font-size: 2.5rem; margin-bottom: 0.75rem; }
	.empty-state h3 { color: #d1d5db; margin: 0 0 0.5rem 0; font-size: 1rem; }
	.empty-state p { margin: 0 0 1.25rem 0; font-size: 0.85rem; }

	.table-wrapper {
		border: 1px solid rgba(107,114,128,0.18);
		border-radius: 8px;
		overflow: hidden;
	}
	.data-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.84rem;
	}
	.data-table th {
		text-align: left;
		padding: 0.55rem 0.65rem;
		color: #9ca3af;
		font-weight: 500;
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		border-bottom: 1px solid rgba(107,114,128,0.18);
		background: rgba(255,255,255,0.015);
		white-space: nowrap;
	}
	.data-table td {
		padding: 0.55rem 0.65rem;
		border-bottom: 1px solid rgba(107,114,128,0.08);
		color: #d1d5db;
		vertical-align: middle;
	}
	.data-row { cursor: pointer; transition: background 0.1s; }
	.data-row:hover { background: rgba(59,130,246,0.04); }
	.data-row.selected { background: rgba(59,130,246,0.06); }
	.data-table tbody tr:last-child td { border-bottom: none; }

	.col-check { width: 36px; text-align: center; }
	.col-check input[type="checkbox"] { cursor: pointer; accent-color: #3b82f6; }
	.col-name { min-width: 180px; }
	.col-format { width: 80px; }
	.col-size { width: 80px; text-align: right; font-variant-numeric: tabular-nums; }
	.col-rows { width: 80px; text-align: right; font-variant-numeric: tabular-nums; }
	.col-cols { width: 60px; text-align: center; }
	.col-status { width: 80px; }
	.col-quality { width: 50px; text-align: center; }
	.col-date { width: 120px; color: #9ca3af; font-size: 0.78rem; white-space: nowrap; }

	.quality-dot {
		display: inline-block;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: #6b7280;
		transition: background 0.2s;
	}
	.quality-dot.good { background: #10b981; box-shadow: 0 0 6px rgba(16, 185, 129, 0.4); }
	.quality-dot.warn { background: #f59e0b; box-shadow: 0 0 6px rgba(245, 158, 11, 0.4); }

	.name-cell { display: flex; align-items: center; gap: 0.5rem; }
	.ds-icon { font-size: 1.1rem; flex-shrink: 0; }
	.name-info { display: flex; flex-direction: column; gap: 0.1rem; min-width: 0; }
	.ds-name { font-weight: 500; color: #e5e7eb; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.ds-id { font-size: 0.68rem; color: #6b7280; font-family: monospace; }

	.format-badge {
		display: inline-block;
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		font-size: 0.7rem;
		font-weight: 500;
		background: rgba(107,114,128,0.15);
		color: #9ca3af;
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

	.btn-sm {
		padding: 0.25rem 0.55rem;
		font-size: 0.75rem;
		border-radius: 4px;
		border: 1px solid rgba(107,114,128,0.25);
		background: rgba(255,255,255,0.04);
		color: #d1d5db;
		cursor: pointer;
		transition: all 0.15s;
	}
	.btn-sm:hover { background: rgba(255,255,255,0.08); }
	.btn-warn { color: #fcd34d; border-color: rgba(245,158,11,0.3); }
	.btn-warn:hover { background: rgba(245,158,11,0.1); }
	.btn-danger { color: #fca5a5; border-color: rgba(239,68,68,0.3); }
	.btn-danger:hover { background: rgba(239,68,68,0.12); }
	.btn-restore { color: #93c5fd; border-color: rgba(59,130,246,0.3); }
	.btn-restore:hover { background: rgba(59,130,246,0.1); }

	.col-actions { width: 80px; text-align: center; }
	.action-btns { display: flex; gap: 0.3rem; justify-content: center; }
	.action-btn {
		width: 28px;
		height: 28px;
		padding: 0;
		border: 1px solid rgba(107,114,128,0.15);
		border-radius: 4px;
		background: rgba(255,255,255,0.03);
		cursor: pointer;
		font-size: 0.8rem;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
		opacity: 0.5;
	}
	.data-row:hover .action-btn { opacity: 1; }
	.action-btn:hover { background: rgba(255,255,255,0.08); transform: scale(1.1); }
	.action-archive:hover { border-color: rgba(245,158,11,0.4); background: rgba(245,158,11,0.1); }
	.action-restore:hover { border-color: rgba(59,130,246,0.4); background: rgba(59,130,246,0.1); }
	.action-delete:hover { border-color: rgba(239,68,68,0.4); background: rgba(239,68,68,0.1); }

	/* Modal styles */
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
		max-width: 480px;
		width: 90%;
	}
	.modal h3 { margin: 0 0 1rem 0; color: #e5e7eb; font-size: 1.05rem; }

	.form-group { margin-bottom: 0.85rem; }
	.form-group label {
		display: block;
		font-size: 0.8rem;
		color: #9ca3af;
		margin-bottom: 0.3rem;
	}
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
	.input-error { border-color: rgba(239,68,68,0.5) !important; }
	.field-error { color: #fca5a5; font-size: 0.72rem; margin-top: 0.2rem; display: block; }
	.field-success { color: #6ee7b7; font-size: 0.72rem; margin-top: 0.2rem; display: block; }
	.input-valid { border-color: rgba(16,185,129,0.4) !important; }
	.textarea { resize: vertical; min-height: 48px; font-family: inherit; }

	.path-input-group { display: flex; gap: 0.4rem; }
	.path-input-group .input { flex: 1; }
	.btn-browse {
		padding: 0.45rem 0.75rem;
		background: rgba(255,255,255,0.06);
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 5px;
		color: #d1d5db;
		font-size: 0.82rem;
		cursor: pointer;
		white-space: nowrap;
	}
	.btn-browse:hover { background: rgba(255,255,255,0.1); }

	.form-error {
		padding: 0.5rem 0.65rem;
		background: rgba(239,68,68,0.08);
		border: 1px solid rgba(239,68,68,0.2);
		border-radius: 5px;
		color: #fca5a5;
		font-size: 0.8rem;
		margin-bottom: 0.75rem;
	}

	.modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.25rem; }
</style>
