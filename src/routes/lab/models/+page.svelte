<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { modelStore } from '$lib/lab/stores/model';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import type { ModelRegistration, ModelRegistrationStatus } from '$lib/lab/adapter/types';
	import { t } from '$lib/i18n';

	let loading = true;
	let statusFilter: ModelRegistrationStatus | 'all' = 'all';
	let searchQuery = '';
	let showRegisterModal = false;
	let showDetailModal = false;
	let selectedModel: ModelRegistration | null = null;
	let confirmDeleteId: string | null = null;

	let regName = '';
	let regVersion = '';
	let regFramework = 'burn';
	let regPath = '';
	let registering = false;
	let regError: string | null = null;

	let newTag = '';
	let newAlias = '';
	let newDescription = '';
	let showEditDesc = false;
	let showAddTag = false;
	let showAddAlias = false;

	let allModels: ModelRegistration[] = [];
	let unsubModels: (() => void) | null = null;
	let modelVersions: any[] = [];
	let versionsLoading = false;
	let compareVersionA: string | null = null;
	let compareVersionB: string | null = null;

	let deployedModelIds: Set<string> = new Set();
	let deployLoading = false;
	let serveInput = '';
	let serveResult: any | null = null;
	let serveRunning = false;
	let serveError: string | null = null;

	$: filteredModels = allModels
		.filter((m) => {
			if (statusFilter !== 'all' && m.status !== statusFilter) return false;
			if (searchQuery && !m.name.toLowerCase().includes(searchQuery.toLowerCase()) && !m.framework.toLowerCase().includes(searchQuery.toLowerCase())) return false;
			return true;
		})
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());

	onMount(async () => {
		unsubModels = modelStore.subscribe((m) => {
			allModels = [...m];
		});
		try {
			await modelStore.refresh();
			await loadDeployedEndpoints();
		} catch (e) {
			console.error('Failed to load models:', e);
		} finally {
			loading = false;
		}
	});

	onDestroy(() => {
		if (unsubModels) {
			unsubModels();
			unsubModels = null;
		}
	});

	function statusLabel(status: ModelRegistrationStatus): string {
		switch (status) {
			case 'none': return $t('models.statusUnpublished');
			case 'staging': return $t('models.statusStaging');
			case 'production': return $t('models.statusProduction');
			case 'archived': return $t('models.statusArchived');
			default: return status;
		}
	}

	function statusColor(status: ModelRegistrationStatus): string {
		switch (status) {
			case 'none': return '#8b5cf6';
			case 'staging': return '#f59e0b';
			case 'production': return '#10b981';
			case 'archived': return '#6b7280';
			default: return '#6b7280';
		}
	}

	function statusIcon(status: ModelRegistrationStatus): string {
		switch (status) {
			case 'none': return '○';
			case 'staging': return '◐';
			case 'production': return '●';
			case 'archived': return '⊘';
			default: return '?';
		}
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		if (diff < 60000) return $t('time.justNow');
		if (diff < 3600000) return `${Math.floor(diff / 60000)} ${$t('time.minutesAgo')}`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)} ${$t('time.hoursAgo')}`;
		return `${Math.floor(diff / 86400000)} ${$t('time.daysAgo')}`;
	}

	function formatSize(bytes: number): string {
		if (bytes === 0) return '0 B';
		const units = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(1024));
		return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
	}

	async function openDetail(model: ModelRegistration) {
		selectedModel = model;
		showDetailModal = true;
		compareVersionA = null;
		compareVersionB = null;
		loadModelVersions(model.id);
	}

	async function loadModelVersions(modelId: string) {
		versionsLoading = true;
		try {
			const client = getLabClient();
			modelVersions = await client.listModelVersions(modelId);
		} catch (e) {
			console.warn('Failed to load model versions:', e);
			modelVersions = [];
		} finally {
			versionsLoading = false;
		}
	}

	async function selectModelPath() {
		const client = getLabClient();
		const path = await client.selectDirectory();
		if (path) {
			regPath = path;
		}
	}

	async function handleRegister() {
		if (!regName.trim() || !regVersion.trim()) {
			regError = $t('models.nameVersionRequired');
			return;
		}
		registering = true;
		regError = null;
		try {
			await modelStore.register(regName.trim(), regVersion.trim(), regFramework);
			if (regPath && selectedModel) {
				await modelStore.setPath(selectedModel.id, regPath);
			}
			showRegisterModal = false;
			regName = '';
			regVersion = '';
			regFramework = 'burn';
			regPath = '';
		} catch (e: any) {
			regError = e?.message || $t('models.registerFailed');
		} finally {
			registering = false;
		}
	}

	async function handlePromoteStaging(modelId: string) {
		try {
			await modelStore.promoteStaging(modelId);
		} catch (e) {
			console.error('Failed to promote to staging:', e);
		}
	}

	async function handlePromoteProduction(modelId: string) {
		try {
			await modelStore.promoteProduction(modelId);
		} catch (e) {
			console.error('Failed to promote to production:', e);
		}
	}

	async function handleArchive(modelId: string) {
		try {
			await modelStore.archive(modelId);
		} catch (e) {
			console.error('Failed to archive model:', e);
		}
	}

	async function handleDemoteStaging(modelId: string) {
		try {
			const client = getLabClient();
			await client.demoteModelStaging(modelId);
			await modelStore.refresh();
		} catch (e) {
			console.error('Failed to demote to staging:', e);
		}
	}

	async function handleAddAlias() {
		if (!selectedModel || !newAlias.trim()) return;
		try {
			const client = getLabClient();
			await client.addModelAlias(selectedModel.id, newAlias.trim());
			selectedModel = { ...selectedModel, aliases: [...selectedModel.aliases, newAlias.trim()] };
			newAlias = '';
		} catch (e) {
			console.error('Failed to add alias:', e);
		}
	}

	async function handleRemoveAlias(alias: string) {
		if (!selectedModel) return;
		try {
			const client = getLabClient();
			await client.removeModelAlias(selectedModel.id, alias);
			selectedModel = { ...selectedModel, aliases: selectedModel.aliases.filter(a => a !== alias) };
		} catch (e) {
			console.error('Failed to remove alias:', e);
		}
	}

	async function handleDelete(modelId: string) {
		try {
			await modelStore.delete(modelId);
			confirmDeleteId = null;
		} catch (e: any) {
			console.error('Failed to delete model:', e);
			alert(e?.message || $t('models.deleteFailed'));
		}
	}

	async function handleSetPath(model: ModelRegistration) {
		const client = getLabClient();
		const path = await client.selectDirectory();
		if (path) {
			try {
				await modelStore.setPath(model.id, path);
				if (selectedModel && selectedModel.id === model.id) {
					selectedModel = { ...selectedModel, path };
				}
			} catch (e) {
				console.error('Failed to set model path:', e);
			}
		}
	}

	function nextAction(status: ModelRegistrationStatus): { label: string; action: 'staging' | 'production' | 'archive' | 'demote' } | null {
		switch (status) {
			case 'none': return { label: $t('models.promoteToStaging'), action: 'staging' };
			case 'staging': return { label: $t('models.promoteToProduction'), action: 'production' };
			case 'production': return { label: $t('models.demoteToStaging'), action: 'demote' };
			case 'archived': return null;
			default: return null;
		}
	}

	function getPerformanceMetrics(model: ModelRegistration): { name: string; value: number; isBest: boolean }[] {
		const metrics: { name: string; value: number; isBest: boolean }[] = [];
		for (const [key, val] of Object.entries(model.metadata)) {
			if (key.startsWith('best_') && typeof val === 'number') {
				metrics.push({ name: key.replace('best_', ''), value: val, isBest: true });
			} else if (key.startsWith('last_') && typeof val === 'number') {
				const metricName = key.replace('last_', '');
				if (!metrics.some(m => m.name === metricName && m.isBest)) {
					metrics.push({ name: metricName, value: val, isBest: false });
				}
			}
		}
		return metrics;
	}

	async function handleAddTag() {
		if (!selectedModel || !newTag.trim()) return;
		try {
			const client = getLabClient();
			await client.addModelTag(selectedModel.id, newTag.trim());
			selectedModel = { ...selectedModel, tags: [...selectedModel.tags, newTag.trim()] };
			newTag = '';
			showAddTag = false;
		} catch (e) {
			console.error('Failed to add tag:', e);
		}
	}

	async function handleRemoveTag(tag: string) {
		if (!selectedModel) return;
		try {
			const client = getLabClient();
			await client.removeModelTag(selectedModel.id, tag);
			selectedModel = { ...selectedModel, tags: selectedModel.tags.filter(t => t !== tag) };
		} catch (e) {
			console.error('Failed to remove tag:', e);
		}
	}

	async function handleSaveDescription() {
		if (!selectedModel) return;
		try {
			const client = getLabClient();
			await client.setModelDescription(selectedModel.id, newDescription);
			selectedModel = { ...selectedModel, description: newDescription };
			showEditDesc = false;
		} catch (e) {
			console.error('Failed to save description:', e);
		}
	}

	async function loadDeployedEndpoints() {
		try {
			const client = getLabClient();
			const endpoints = await client.modelListEndpoints();
			deployedModelIds = new Set(endpoints.map(e => e.model_id));
		} catch (e) {
			deployedModelIds = new Set();
		}
	}

	async function handleDeploy(modelId: string) {
		deployLoading = true;
		try {
			const client = getLabClient();
			await client.modelDeploy(modelId);
			deployedModelIds = new Set([...deployedModelIds, modelId]);
		} catch (e: any) {
			console.error('Failed to deploy model:', e);
		} finally {
			deployLoading = false;
		}
	}

	async function handleUndeploy(modelId: string) {
		deployLoading = true;
		try {
			const client = getLabClient();
			await client.modelUndeploy(modelId);
			const newSet = new Set(deployedModelIds);
			newSet.delete(modelId);
			deployedModelIds = newSet;
		} catch (e: any) {
			console.error('Failed to undeploy model:', e);
		} finally {
			deployLoading = false;
		}
	}

	async function handleServePredict() {
		if (!selectedModel) return;
		serveRunning = true;
		serveResult = null;
		serveError = null;
		try {
			const client = getLabClient();
			const inputs = serveInput.trim().split('\n')
				.filter(line => line.trim())
				.map(line => line.split(',').map(v => parseFloat(v.trim())).filter(v => !isNaN(v)));
			if (inputs.length === 0 || inputs[0].length === 0) {
				serveError = $t('models.invalidServeData');
				return;
			}
			serveResult = await client.modelPredict(selectedModel.id, inputs);
		} catch (e: any) {
			serveError = e?.toString() || $t('models.serveFailed');
		} finally {
			serveRunning = false;
		}
	}
</script>

<div class="models-page">
	<h2>{$t('models.title')}</h2>
	<p class="desc">{$t('models.desc')}</p>

	<div class="toolbar">
		<input
			type="text"
			class="search-input"
			placeholder={$t('models.searchPlaceholder')}
			bind:value={searchQuery}
		/>
		<div class="filter-group">
			<button class="filter-btn" class:active={statusFilter === 'all'} on:click={() => statusFilter = 'all'}>{$t('experiments.all')}</button>
			<button class="filter-btn" class:active={statusFilter === 'none'} on:click={() => statusFilter = 'none'}>{$t('models.statusUnpublished')}</button>
			<button class="filter-btn" class:active={statusFilter === 'staging'} on:click={() => statusFilter = 'staging'}>{$t('models.statusStaging')}</button>
			<button class="filter-btn" class:active={statusFilter === 'production'} on:click={() => statusFilter = 'production'}>{$t('models.statusProduction')}</button>
			<button class="filter-btn" class:active={statusFilter === 'archived'} on:click={() => statusFilter = 'archived'}>{$t('models.statusArchived')}</button>
		</div>
		<button class="register-btn" on:click={() => showRegisterModal = true}>+ {$t('models.registerModel')}</button>
	</div>

	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>{$t('models.loadingModels')}</p>
		</div>
	{:else if filteredModels.length === 0}
		<div class="empty-state">
			<span class="empty-icon">📦</span>
			<p class="empty-text">{$t('models.noModels')}</p>
			<p class="empty-hint">{$t('models.noModelsHint')}</p>
		</div>
	{:else}
		<div class="model-grid">
			{#each filteredModels as model (model.id)}
				<div class="model-card">
					<div class="card-header">
						<div class="card-title-row">
							<h3 class="model-name">{model.name}</h3>
							<span
								class="status-badge"
								style="color: {statusColor(model.status)}; border-color: {statusColor(model.status)}30; background: {statusColor(model.status)}10"
							>
								{statusIcon(model.status)} {statusLabel(model.status)}
							</span>
						</div>
						<div class="model-meta">
							<span class="meta-chip">v{model.version}</span>
							<span class="meta-chip framework">{model.framework}</span>
						</div>
					</div>

					<div class="card-body">
						{#if model.path}
							<div class="path-row">
								<span class="path-label">{$t('models.path')}</span>
								<span class="path-value" title={model.path}>{model.path}</span>
							</div>
						{/if}

						{#if Object.keys(model.metadata).length > 0}
							<div class="metadata-row">
								{#each Object.entries(model.metadata).slice(0, 3) as [key, val]}
									<span class="meta-tag">{key}: {typeof val === 'number' ? val.toFixed(4) : String(val)}</span>
								{/each}
							</div>
						{/if}

						<div class="time-row">
							<span>{$t('models.created')}: {formatTime(model.created_at)}</span>
							<span>{$t('models.updated')}: {formatTime(model.updated_at)}</span>
						</div>
					</div>

					<div class="card-actions">
						<button class="action-btn detail" on:click={() => openDetail(model)}>{$t('models.detail')}</button>
						{#if !model.path}
							<button class="action-btn path" on:click={() => handleSetPath(model)}>{$t('models.setPath')}</button>
						{/if}
						{#if nextAction(model.status)}
							{@const na = nextAction(model.status)}
							{#if na}
								<button
									class="action-btn promote"
									on:click={() => {
										if (na.action === 'staging') handlePromoteStaging(model.id);
										else if (na.action === 'production') handlePromoteProduction(model.id);
										else if (na.action === 'archive') handleArchive(model.id);
										else if (na.action === 'demote') handleDemoteStaging(model.id);
									}}
								>
									{na.label}
								</button>
							{/if}
						{/if}
						{#if model.status === 'production' || model.status === 'staging'}
							<button class="action-btn archive-btn" on:click={() => handleArchive(model.id)}>{$t('models.archive')}</button>
						{/if}
						<button class="action-btn delete" on:click={() => confirmDeleteId = model.id}>{$t('models.delete')}</button>
					</div>

					{#if confirmDeleteId === model.id}
						<div class="confirm-overlay">
							<p>{$t('models.confirmDeleteMsg')}</p>
							<div class="confirm-btns">
								<button class="confirm-yes" on:click={() => handleDelete(model.id)}>{$t('confirm.ok')}</button>
								<button class="confirm-no" on:click={() => confirmDeleteId = null}>{$t('confirm.cancel')}</button>
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if showRegisterModal}
	<div class="modal-overlay" role="presentation" on:click|self={() => showRegisterModal = false} on:keydown={(e) => { if (e.key === 'Escape') showRegisterModal = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1">

			<div class="form-group">
				<label for="auto-f84">{$t('models.modelName')} *</label>
				<input id="auto-f84" type="text" bind:value={regName} placeholder={$t('models.modelNamePlaceholder')} class="form-input" />
			</div>

			<div class="form-group">
				<label for="auto-f85">{$t('models.version')} *</label>
				<input id="auto-f85" type="text" bind:value={regVersion} placeholder={$t('models.versionPlaceholder')} class="form-input" />
			</div>

			<div class="form-group">
				<label for="auto-f86">{$t('models.framework')}</label>
				<select id="auto-f86" bind:value={regFramework} class="form-input">
					<option value="burn">Burn</option>
					<option value="pytorch">PyTorch</option>
					<option value="tensorflow">TensorFlow</option>
					<option value="onnx">ONNX</option>
					<option value="other">{$t('models.other')}</option>
				</select>
			</div>

			<div class="form-group">
				<label for="reg-model-path">{$t('models.modelPath')}</label>
				<div class="input-with-button">
					<input id="reg-model-path" type="text" bind:value={regPath} placeholder={$t('models.selectModelDir')} class="form-input" readonly />
					<button class="btn-browse" on:click={selectModelPath}>{$t('models.selectDir')}</button>
				</div>
			</div>

			{#if regError}
				<div class="form-error">{regError}</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-cancel" on:click={() => showRegisterModal = false}>{$t('confirm.cancel')}</button>
				<button class="btn-submit" on:click={handleRegister} disabled={registering}>
					{registering ? $t('models.registering') : $t('models.registerBtn')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if showDetailModal && selectedModel}
	<div class="modal-overlay" role="presentation" on:click|self={() => showDetailModal = false} on:keydown={(e) => { if (e.key === 'Escape') showDetailModal = false; }}>
		<div class="modal modal-lg" role="dialog" aria-modal="true" tabindex="-1">
			<div class="detail-header">
				<h3>{selectedModel.name}</h3>
				<span
					class="status-badge"
					style="color: {statusColor(selectedModel.status)}; border-color: {statusColor(selectedModel.status)}30; background: {statusColor(selectedModel.status)}10"
				>
					{statusIcon(selectedModel.status)} {statusLabel(selectedModel.status)}
				</span>
			</div>

			{#if selectedModel.description || showEditDesc}
				<div class="description-section">
					{#if showEditDesc}
						<textarea bind:value={newDescription} class="desc-textarea" placeholder={$t('models.enterDesc')}></textarea>
						<div class="desc-actions">
							<button class="btn-sm btn-save" on:click={handleSaveDescription}>{$t('models.save')}</button>
							<button class="btn-sm btn-cancel-sm" on:click={() => showEditDesc = false}>{$t('confirm.cancel')}</button>
						</div>
					{:else}
						<p class="description-text">{selectedModel.description}</p>
						<button class="btn-sm btn-edit" on:click={() => { newDescription = selectedModel?.description || ''; showEditDesc = true; }}>{$t('models.edit')}</button>
					{/if}
				</div>
			{:else}
				<button class="btn-sm btn-edit" on:click={() => { newDescription = ''; showEditDesc = true; }}>+ {$t('models.addDesc')}</button>
			{/if}

			<div class="model-card-summary">
				<div class="card-stat">
					<span class="card-stat-label">{$t('models.version')}</span>
					<span class="card-stat-value">v{selectedModel.version}</span>
				</div>
				<div class="card-stat">
					<span class="card-stat-label">{$t('models.framework')}</span>
					<span class="card-stat-value">{selectedModel.framework}</span>
				</div>
				<div class="card-stat">
					<span class="card-stat-label">{$t('models.status')}</span>
					<span class="card-stat-value" style="color: {statusColor(selectedModel.status)}">{statusLabel(selectedModel.status)}</span>
				</div>
				{#if selectedModel.lineage?.experiment_name}
					<div class="card-stat">
						<span class="card-stat-label">{$t('models.sourceExperiment')}</span>
						<a href="/lab/experiments/{selectedModel.lineage.experiment_id}" class="card-stat-link">{selectedModel.lineage.experiment_name}</a>
					</div>
				{/if}
				{#if selectedModel.structured_signature}
					<div class="card-stat">
						<span class="card-stat-label">{$t('models.inputDim')}</span>
						<span class="card-stat-value">{selectedModel.structured_signature.inputs.map(i => `[${i.shape.join('×')}]`).join(', ')}</span>
					</div>
					<div class="card-stat">
						<span class="card-stat-label">{$t('models.outputDim')}</span>
						<span class="card-stat-value">{selectedModel.structured_signature.outputs.map(o => `[${o.shape.join('×')}]`).join(', ')}</span>
					</div>
				{/if}
				<div class="card-stat">
					<span class="card-stat-label">{$t('models.created')}</span>
					<span class="card-stat-value">{new Date(selectedModel.created_at).toLocaleDateString('zh-CN')}</span>
				</div>
			</div>

			{#if selectedModel.tags.length > 0 || showAddTag}
				<div class="tags-section">
					{#each selectedModel.tags as tag}
						<span class="tag-chip">
							{tag}
							<button class="tag-remove" on:click={() => handleRemoveTag(tag)}>×</button>
						</span>
					{/each}
					{#if showAddTag}
						<div class="tag-add-form">
							<input type="text" bind:value={newTag} class="tag-input" placeholder={$t('models.tagName')} />
							<button class="btn-sm btn-save" on:click={handleAddTag}>{$t('models.add')}</button>
							<button class="btn-sm btn-cancel-sm" on:click={() => showAddTag = false}>{$t('confirm.cancel')}</button>
						</div>
					{:else}
						<button class="btn-sm btn-edit" on:click={() => showAddTag = true}>+ {$t('models.tag')}</button>
					{/if}
				</div>

				<h4 class="section-title">{$t('models.aliases')}</h4>
				<div class="tags-section">
					{#each selectedModel.aliases as alias}
						<span class="tag-chip alias-chip">
							{alias}
							<button class="tag-remove" on:click={() => handleRemoveAlias(alias)}>×</button>
						</span>
					{/each}
					{#if showAddAlias}
						<div class="tag-add-form">
							<input type="text" bind:value={newAlias} class="tag-input" placeholder={$t('models.aliasPlaceholder')} />
							<button class="btn-sm btn-save" on:click={handleAddAlias}>{$t('models.add')}</button>
							<button class="btn-sm btn-cancel-sm" on:click={() => showAddAlias = false}>{$t('confirm.cancel')}</button>
						</div>
					{:else}
						<button class="btn-sm btn-edit" on:click={() => showAddAlias = true}>+ {$t('models.alias')}</button>
					{/if}
				</div>
			{:else}
				<button class="btn-sm btn-edit" on:click={() => showAddTag = true}>+ {$t('models.addTag')}</button>
			{/if}

			{#if getPerformanceMetrics(selectedModel).length > 0}
				<h4 class="section-title">{$t('models.performanceMetrics')}</h4>
				<div class="metrics-grid">
					{#each getPerformanceMetrics(selectedModel) as metric}
						<div class="metric-card" class:best={metric.isBest}>
							<span class="metric-name">{metric.name}</span>
							<span class="metric-value">{metric.value.toFixed(4)}</span>
							{#if metric.isBest}
								<span class="metric-badge">{$t('models.best')}</span>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			<div class="detail-grid">
				<div class="detail-item">
					<span class="detail-label">{$t('models.modelId')}</span>
					<span class="detail-value mono">{selectedModel.id}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">{$t('models.version')}</span>
					<span class="detail-value">{selectedModel.version}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">{$t('models.framework')}</span>
					<span class="detail-value">{selectedModel.framework}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">{$t('models.path')}</span>
					<span class="detail-value mono">{selectedModel.path || '-'}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">{$t('models.createdAt')}</span>
					<span class="detail-value">{new Date(selectedModel.created_at).toLocaleString('zh-CN')}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">{$t('models.updatedAt')}</span>
					<span class="detail-value">{new Date(selectedModel.updated_at).toLocaleString('zh-CN')}</span>
				</div>
			</div>

			{#if selectedModel.structured_signature}
				<h4 class="section-title">{$t('models.modelSignature')}</h4>
				<div class="signature-grid">
					<div class="signature-section">
						<h5 class="sig-subtitle">{$t('models.input')}</h5>
						{#each selectedModel.structured_signature.inputs as input}
							<div class="tensor-spec">
								<span class="tensor-name">{input.name}</span>
								<span class="tensor-dtype">{input.dtype}</span>
								<span class="tensor-shape">[{input.shape.join(', ')}]</span>
							</div>
						{/each}
					</div>
					<div class="signature-section">
						<h5 class="sig-subtitle">{$t('models.output')}</h5>
						{#each selectedModel.structured_signature.outputs as output}
							<div class="tensor-spec">
								<span class="tensor-name">{output.name}</span>
								<span class="tensor-dtype">{output.dtype}</span>
								<span class="tensor-shape">[{output.shape.join(', ')}]</span>
							</div>
						{/each}
					</div>
				</div>
			{:else if selectedModel.signature}
				<h4 class="section-title">{$t('models.modelSignature')}</h4>
				<pre class="signature-block">{JSON.stringify(selectedModel.signature, null, 2)}</pre>
			{/if}

			{#if selectedModel.lineage}
				<h4 class="section-title">{$t('models.modelLineage')}</h4>
				<div class="lineage-grid">
					{#if selectedModel.lineage.experiment_name}
						<div class="detail-item">
							<span class="detail-label">{$t('models.sourceExperiment')}</span>
							<a href="/lab/experiments/{selectedModel.lineage.experiment_id}" class="detail-value lineage-link">{selectedModel.lineage.experiment_name}</a>
						</div>
					{/if}
					{#if selectedModel.lineage.parent_model_id}
						<div class="detail-item">
							<span class="detail-label">{$t('models.parentModel')}</span>
							<span class="detail-value mono">{selectedModel.lineage.parent_model_id}</span>
						</div>
					{/if}
					{#if selectedModel.lineage.dataset}
						<div class="detail-item">
							<span class="detail-label">{$t('models.dataset')}</span>
							<span class="detail-value">{selectedModel.lineage.dataset}</span>
						</div>
					{/if}
					{#if selectedModel.lineage.training_config}
						<div class="detail-item full-width">
							<span class="detail-label">{$t('models.trainingConfig')}</span>
							<pre class="config-block">{JSON.stringify(selectedModel.lineage.training_config, null, 2)}</pre>
						</div>
					{/if}
				</div>
			{/if}

			{#if Object.keys(selectedModel.metadata).length > 0}
				<h4 class="section-title">{$t('models.allMetadata')}</h4>
				<div class="metadata-table">
					{#each Object.entries(selectedModel.metadata) as [key, val]}
						<div class="meta-row">
							<span class="meta-key">{key}</span>
							<span class="meta-val">{typeof val === 'object' ? JSON.stringify(val) : String(val)}</span>
						</div>
					{/each}
				</div>
			{/if}

			<h4 class="section-title">{$t('models.versionHistory')}</h4>
			{#if versionsLoading}
				<p class="loading-hint">{$t('models.loadingVersions')}</p>
			{:else if modelVersions.length > 0}
				<div class="version-list">
					{#each modelVersions as ver}
						<div class="version-item" class:version-selected={compareVersionA === ver.version || compareVersionB === ver.version}>
							<div class="version-info">
								<span class="version-tag">v{ver.version}</span>
								<span class="version-size">{formatSize(ver.size_bytes)}</span>
								<span class="version-date">{new Date(ver.created_at).toLocaleString('zh-CN')}</span>
							</div>
							{#if ver.description}
								<p class="version-desc">{ver.description}</p>
							{/if}
							<div class="version-actions">
								<button
									class="btn-sm btn-compare-v"
									class:active-compare={compareVersionA === ver.version}
									on:click={() => { compareVersionA = compareVersionA === ver.version ? null : ver.version; }}
								>
									A{compareVersionA === ver.version ? ' ✓' : ''}
								</button>
								<button
									class="btn-sm btn-compare-v"
									class:active-compare={compareVersionB === ver.version}
									on:click={() => { compareVersionB = compareVersionB === ver.version ? null : ver.version; }}
								>
									B{compareVersionB === ver.version ? ' ✓' : ''}
								</button>
							</div>
						</div>
					{/each}
				</div>
				{#if compareVersionA && compareVersionB && compareVersionA !== compareVersionB}
					<div class="version-compare">
						<h5>{$t('models.versionCompare')}: v{compareVersionA} vs v{compareVersionB}</h5>
						<div class="compare-table">
							{#each modelVersions.filter(v => v.version === compareVersionA || v.version === compareVersionB) as ver}
								<div class="compare-row">
									<span class="compare-label">v{ver.version}</span>
									<span class="compare-size">{formatSize(ver.size_bytes)}</span>
									<span class="compare-date">{new Date(ver.created_at).toLocaleString('zh-CN')}</span>
									<span class="compare-desc">{ver.description || '-'}</span>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			{:else}
				<p class="loading-hint">{$t('models.noVersionHistory')}</p>
			{/if}

			<div class="modal-actions">
				{#if !selectedModel.path}
					<button class="btn-browse" on:click={() => handleSetPath(selectedModel!)}>{$t('models.setPath')}</button>
				{/if}
				{#if deployedModelIds.has(selectedModel.id)}
					<button class="btn-undeploy" on:click={() => handleUndeploy(selectedModel!.id)} disabled={deployLoading}>
						{deployLoading ? $t('models.processing') : $t('models.undeploy')}
					</button>
				{:else if selectedModel.status === 'production' || selectedModel.status === 'staging'}
					<button class="btn-deploy" on:click={() => handleDeploy(selectedModel!.id)} disabled={deployLoading}>
						{deployLoading ? $t('models.deploying') : $t('models.deployService')}
					</button>
				{/if}
				<button class="btn-cancel" on:click={() => showDetailModal = false}>{$t('models.close')}</button>
			</div>

			{#if deployedModelIds.has(selectedModel.id)}
				<div class="serve-section" style="margin-top: 16px; padding: 16px; border: 1px solid var(--border); border-radius: 8px;">
					<h4 style="margin: 0 0 12px 0; font-size: 14px; font-weight: 600;">{$t('models.onlineInference')}</h4>
					<p style="font-size: 12px; color: var(--text-secondary); margin: 0 0 8px 0;">{$t('models.inferenceHint')}</p>
					<textarea
						bind:value={serveInput}
						placeholder="0.1, 0.2, 0.3, ..."
						rows="3"
						style="width: 100%; padding: 8px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-primary); color: var(--text-primary); font-family: monospace; font-size: 13px; resize: vertical;"
					></textarea>
					<button class="btn-deploy" on:click={handleServePredict} disabled={serveRunning} style="margin-top: 8px;">
						{serveRunning ? $t('models.inferring') : $t('models.runInference')}
					</button>
					{#if serveError}
						<div class="error-banner" style="margin-top: 8px;">{serveError}</div>
					{/if}
					{#if serveResult}
						<div style="margin-top: 12px; padding: 12px; background: var(--bg-secondary); border-radius: 6px;">
							<div style="font-weight: 600; margin-bottom: 8px;">{$t('models.inferenceResult')} ({$t('models.latency')}: {serveResult.latency_ms.toFixed(1)}ms)</div>
							{#if serveResult.predicted_classes}
								<div style="font-size: 13px;">
									{$t('models.predictedClass')}: {serveResult.predicted_classes.join(', ')}
								</div>
							{/if}
							{#if serveResult.predictions}
								<div style="font-size: 13px; margin-top: 4px;">
									{$t('models.predictedValue')}: {serveResult.predictions.map((v: number) => v.toFixed(4)).join(', ')}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.models-page { max-width: 1400px; }
	h2 { font-size: 1.5rem; font-weight: 600; color: var(--text-primary, #e5e7eb); margin-bottom: 0.5rem; }
	.desc { color: var(--text-secondary, #9ca3af); margin-bottom: 2rem; }

	.toolbar {
		display: flex;
		gap: 1rem;
		align-items: center;
		margin-bottom: 1.5rem;
		flex-wrap: wrap;
	}

	.search-input {
		flex: 1;
		min-width: 200px;
		max-width: 400px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		padding: 0.6rem 1rem;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		transition: border-color 0.2s;
	}
	.search-input:focus { border-color: #10b981; }
	.search-input::placeholder { color: var(--text-secondary, #6b7280); }

	.filter-group { display: flex; gap: 0.5rem; }

	.filter-btn {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		padding: 0.4rem 0.8rem;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s;
	}
	.filter-btn:hover { background: rgba(16, 185, 129, 0.1); border-color: rgba(16, 185, 129, 0.3); }
	.filter-btn.active { background: rgba(16, 185, 129, 0.15); border-color: #10b981; color: #10b981; }

	.register-btn {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #10b981, #059669);
		border: none;
		border-radius: 8px;
		color: white;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		margin-left: auto;
	}
	.register-btn:hover { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3); }

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
	@keyframes spin { to { transform: rotate(360deg); } }

	.empty-state {
		text-align: center;
		padding: 4rem 2rem;
	}
	.empty-icon { font-size: 3rem; margin-bottom: 1rem; display: block; }
	.empty-text { font-size: 1.2rem; color: var(--text-primary, #e5e7eb); margin-bottom: 0.5rem; }
	.empty-hint { color: var(--text-secondary, #6b7280); font-size: 0.9rem; }

	.model-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
		gap: 1rem;
	}

	.model-card {
		background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		transition: all 0.2s;
		position: relative;
		overflow: hidden;
	}
	.model-card:hover { border-color: rgba(16, 185, 129, 0.3); box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3); }

	.card-header { display: flex; flex-direction: column; gap: 0.5rem; }
	.card-title-row { display: flex; justify-content: space-between; align-items: center; gap: 0.75rem; }
	.model-name { font-size: 1.05rem; font-weight: 600; color: var(--text-primary, #e5e7eb); margin: 0; }

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

	.model-meta { display: flex; gap: 0.5rem; }
	.meta-chip {
		background: rgba(139, 92, 246, 0.1);
		color: #a78bfa;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 500;
	}
	.meta-chip.framework {
		background: rgba(59, 130, 246, 0.1);
		color: #93c5fd;
	}

	.card-body { display: flex; flex-direction: column; gap: 0.5rem; }

	.path-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8rem;
	}
	.path-label { color: var(--text-muted, #6b7280); white-space: nowrap; }
	.path-value {
		color: var(--text-secondary, #9ca3af);
		font-family: monospace;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.metadata-row { display: flex; flex-wrap: wrap; gap: 0.3rem; }
	.meta-tag {
		background: rgba(16, 185, 129, 0.08);
		color: #6ee7b7;
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		font-size: 0.7rem;
		font-family: monospace;
	}

	.time-row {
		display: flex;
		justify-content: space-between;
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
	}

	.card-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 0.75rem;
	}

	.action-btn {
		padding: 0.35rem 0.75rem;
		border-radius: 6px;
		font-size: 0.8rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		border: 1px solid;
		background: transparent;
	}
	.action-btn.detail {
		color: #93c5fd;
		border-color: rgba(59, 130, 246, 0.3);
	}
	.action-btn.detail:hover { background: rgba(59, 130, 246, 0.1); }
	.action-btn.path {
		color: #a78bfa;
		border-color: rgba(139, 92, 246, 0.3);
	}
	.action-btn.path:hover { background: rgba(139, 92, 246, 0.1); }
	.action-btn.promote {
		color: #10b981;
		border-color: rgba(16, 185, 129, 0.3);
	}
	.action-btn.promote:hover { background: rgba(16, 185, 129, 0.1); }
	.action-btn.delete {
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.3);
		margin-left: auto;
	}
	.action-btn.delete:hover { background: rgba(239, 68, 68, 0.1); }

	.confirm-overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.85);
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		border-radius: 12px;
		z-index: 10;
	}
	.confirm-overlay p { color: var(--text-primary, #e5e7eb); font-size: 0.95rem; }
	.confirm-btns { display: flex; gap: 0.75rem; }
	.confirm-yes {
		padding: 0.5rem 1.25rem;
		background: rgba(239, 68, 68, 0.2);
		border: 1px solid rgba(239, 68, 68, 0.5);
		border-radius: 6px;
		color: #ef4444;
		cursor: pointer;
		font-weight: 500;
	}
	.confirm-no {
		padding: 0.5rem 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 6px;
		color: var(--text-secondary, #9ca3af);
		cursor: pointer;
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		backdrop-filter: blur(4px);
	}

	.modal {
		background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
		border: 1px solid rgba(16, 185, 129, 0.2);
		border-radius: 16px;
		padding: 2rem;
		width: 90%;
		max-width: 500px;
		max-height: 85vh;
		overflow-y: auto;
	}
	.modal-lg { max-width: 650px; }
	.modal h3 { font-size: 1.25rem; font-weight: 600; color: var(--text-primary, #e5e7eb); margin-bottom: 1rem; }

	.form-group { margin-bottom: 1rem; }
	.form-group label {
		display: block;
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.4rem;
	}
	.form-input {
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		transition: border-color 0.2s;
	}
	.form-input:focus { border-color: #10b981; }
	select.form-input { cursor: pointer; }

	.input-with-button { display: flex; gap: 0.5rem; }
	.input-with-button .form-input { flex: 1; cursor: pointer; }
	.btn-browse {
		padding: 0.6rem 1rem;
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 8px;
		color: #10b981;
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}
	.btn-browse:hover { background: rgba(16, 185, 129, 0.25); border-color: #10b981; }

	.btn-deploy {
		padding: 0.6rem 1rem;
		background: rgba(59, 130, 246, 0.15);
		border: 1px solid rgba(59, 130, 246, 0.3);
		border-radius: 8px;
		color: #3b82f6;
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}
	.btn-deploy:hover { background: rgba(59, 130, 246, 0.25); border-color: #3b82f6; }
	.btn-deploy:disabled { opacity: 0.5; cursor: not-allowed; }

	.btn-undeploy {
		padding: 0.6rem 1rem;
		background: rgba(245, 158, 11, 0.15);
		border: 1px solid rgba(245, 158, 11, 0.3);
		border-radius: 8px;
		color: #f59e0b;
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}
	.btn-undeploy:hover { background: rgba(245, 158, 11, 0.25); border-color: #f59e0b; }
	.btn-undeploy:disabled { opacity: 0.5; cursor: not-allowed; }

	.form-error {
		padding: 0.5rem 0.75rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 6px;
		color: #ef4444;
		font-size: 0.85rem;
		margin-bottom: 1rem;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		margin-top: 1.5rem;
	}

	.version-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.version-item {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 8px;
		padding: 0.6rem 0.8rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.version-selected {
		border-color: rgba(96, 165, 250, 0.3);
		background: rgba(96, 165, 250, 0.05);
	}

	.version-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.version-tag {
		font-size: 0.85rem;
		font-weight: 600;
		color: #60a5fa;
		font-family: monospace;
	}

	.version-size {
		font-size: 0.78rem;
		color: var(--text-secondary, #9ca3af);
		font-family: monospace;
	}

	.version-date {
		font-size: 0.75rem;
		color: var(--text-secondary, #9ca3af);
	}

	.version-desc {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		margin: 0.3rem 0 0;
	}

	.version-actions {
		display: flex;
		gap: 0.3rem;
	}

	.btn-compare-v {
		padding: 0.2rem 0.5rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 4px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-compare-v:hover {
		background: rgba(96, 165, 250, 0.1);
		border-color: rgba(96, 165, 250, 0.3);
		color: #60a5fa;
	}

	.active-compare {
		background: rgba(96, 165, 250, 0.15) !important;
		border-color: #60a5fa !important;
		color: #60a5fa !important;
	}

	.version-compare {
		margin-top: 0.75rem;
		padding: 0.75rem;
		background: rgba(96, 165, 250, 0.05);
		border: 1px solid rgba(96, 165, 250, 0.15);
		border-radius: 8px;
	}

	.version-compare h5 {
		color: #60a5fa;
		font-size: 0.85rem;
		margin-bottom: 0.5rem;
	}

	.compare-table {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.compare-row {
		display: grid;
		grid-template-columns: 60px 80px 140px 1fr;
		gap: 0.5rem;
		padding: 0.3rem 0.5rem;
		border-radius: 4px;
		font-size: 0.8rem;
	}

	.compare-row:nth-child(odd) {
		background: rgba(0, 0, 0, 0.15);
	}

	.compare-label {
		font-weight: 600;
		color: #60a5fa;
		font-family: monospace;
	}

	.compare-size, .compare-date {
		color: var(--text-secondary, #9ca3af);
		font-family: monospace;
	}

	.compare-desc {
		color: var(--text-primary, #e5e7eb);
	}

	.loading-hint {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		font-style: italic;
	}
	.btn-cancel {
		padding: 0.5rem 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		cursor: pointer;
		font-size: 0.9rem;
	}
	.btn-submit {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #10b981, #059669);
		border: none;
		border-radius: 8px;
		color: white;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
	}
	.btn-submit:disabled { opacity: 0.6; cursor: not-allowed; }

	.detail-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
		margin: 1rem 0;
	}
	.detail-item {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.5rem 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 6px;
		border: 1px solid rgba(107, 114, 128, 0.15);
	}
	.detail-label { font-size: 0.75rem; color: var(--text-muted, #6b7280); }
	.detail-value { font-size: 0.85rem; color: var(--text-primary, #e5e7eb); }
	.detail-value.mono { font-family: monospace; font-size: 0.8rem; word-break: break-all; }

	.section-title {
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin: 1.25rem 0 0.5rem;
	}

	.metadata-table {
		display: flex;
		flex-direction: column;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		overflow: hidden;
	}
	.meta-row {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.meta-row:last-child { border-bottom: none; }
	.meta-key { color: var(--text-secondary, #9ca3af); font-size: 0.85rem; font-family: monospace; }
	.meta-val { color: var(--text-primary, #e5e7eb); font-size: 0.85rem; font-family: monospace; text-align: right; word-break: break-all; }

	.signature-block {
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		padding: 1rem;
		color: #6ee7b7;
		font-size: 0.8rem;
		overflow-x: auto;
		white-space: pre-wrap;
		margin: 0;
	}

	.signature-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	.signature-section {
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		padding: 0.75rem;
	}

	.sig-subtitle {
		margin: 0 0 0.5rem;
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		font-weight: 500;
	}

	.tensor-spec {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.3rem 0;
		font-size: 0.8rem;
	}

	.tensor-name {
		color: #93c5fd;
		font-family: 'SF Mono', 'Fira Code', monospace;
		min-width: 60px;
	}

	.tensor-dtype {
		color: #a78bfa;
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: 0.75rem;
	}

	.tensor-shape {
		color: #6ee7b7;
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: 0.75rem;
	}

	.lineage-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.lineage-link {
		color: #10b981;
		text-decoration: none;
		font-size: 0.85rem;
	}

	.lineage-link:hover {
		text-decoration: underline;
	}

	.detail-item.full-width {
		grid-column: 1 / -1;
	}

	.config-block {
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		padding: 0.75rem;
		color: #6ee7b7;
		font-size: 0.75rem;
		overflow-x: auto;
		white-space: pre-wrap;
		margin: 0.25rem 0 0;
		max-height: 200px;
		overflow-y: auto;
	}

	.detail-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.description-section {
		margin-bottom: 0.75rem;
	}

	.model-card-summary {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin: 0.75rem 0;
		padding: 0.75rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 10px;
	}

	.card-stat {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		padding: 0.4rem 0.7rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 6px;
		min-width: 80px;
	}

	.card-stat-label {
		font-size: 0.7rem;
		color: var(--text-secondary, #9ca3af);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.card-stat-value {
		font-size: 0.85rem;
		color: var(--text-primary, #e5e7eb);
		font-weight: 500;
		font-family: monospace;
	}

	.card-stat-link {
		font-size: 0.85rem;
		color: #60a5fa;
		text-decoration: none;
		font-weight: 500;
	}
	.card-stat-link:hover {
		text-decoration: underline;
	}
	.description-text {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		margin: 0 0 0.5rem;
		line-height: 1.5;
	}
	.desc-textarea {
		width: 100%;
		min-height: 80px;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		padding: 0.75rem;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.85rem;
		resize: vertical;
		outline: none;
		margin-bottom: 0.5rem;
	}
	.desc-textarea:focus { border-color: #10b981; }
	.desc-actions { display: flex; gap: 0.5rem; }

	.tags-section {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: center;
		margin-bottom: 0.75rem;
	}
	.tag-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		background: rgba(139, 92, 246, 0.1);
		color: #a78bfa;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		font-size: 0.8rem;
	}
	.alias-chip {
		background: rgba(59, 130, 246, 0.1);
		color: #60a5fa;
	}
	.archive-btn {
		background: rgba(107, 114, 128, 0.15) !important;
		color: #9ca3af !important;
	}
	.archive-btn:hover {
		background: rgba(107, 114, 128, 0.25) !important;
	}
	.tag-remove {
		background: none;
		border: none;
		color: #a78bfa;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		padding: 0;
		margin-left: 0.15rem;
	}
	.tag-remove:hover { color: #ef4444; }
	.tag-add-form {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.tag-input {
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 4px;
		padding: 0.25rem 0.5rem;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.8rem;
		outline: none;
		width: 100px;
	}
	.tag-input:focus { border-color: #10b981; }

	.btn-sm {
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		cursor: pointer;
		border: none;
		transition: all 0.2s;
	}
	.btn-save {
		background: rgba(16, 185, 129, 0.2);
		color: #10b981;
	}
	.btn-save:hover { background: rgba(16, 185, 129, 0.3); }
	.btn-cancel-sm {
		background: rgba(107, 114, 128, 0.2);
		color: #9ca3af;
	}
	.btn-cancel-sm:hover { background: rgba(107, 114, 128, 0.3); }
	.btn-edit {
		background: rgba(59, 130, 246, 0.1);
		color: #93c5fd;
	}
	.btn-edit:hover { background: rgba(59, 130, 246, 0.2); }

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	.metric-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.2rem;
		padding: 0.6rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.15);
		border-radius: 8px;
		position: relative;
	}
	.metric-card.best {
		border-color: rgba(16, 185, 129, 0.3);
		background: rgba(16, 185, 129, 0.05);
	}
	.metric-name {
		font-size: 0.7rem;
		color: var(--text-muted, #6b7280);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}
	.metric-value {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		font-family: 'SF Mono', 'Fira Code', monospace;
	}
	.metric-badge {
		position: absolute;
		top: 0.25rem;
		right: 0.35rem;
		font-size: 0.6rem;
		color: #10b981;
		font-weight: 600;
	}
</style>
