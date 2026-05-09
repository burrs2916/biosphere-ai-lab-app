<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { datasetRegistryStore } from '$lib/lab/stores/dataset';
	import { taskManagerStore } from '$lib/lab/stores/taskManager';
	import { t } from '$lib/i18n';
	import type { DataLoadConfig, DataFormat, PipelineStep, PreprocessType, DataPreview } from '$lib/lab/adapter/types';
	import Skeleton from '$lib/lab/components/Skeleton.svelte';

	type PreprocessStepDef = {
		id: string;
		type: string;
		label: string;
		desc: string;
		defaultParams: Record<string, string>;
	};

	const stepTypes: PreprocessStepDef[] = [
		{ id: 'normalize', type: 'Normalize', label: $t('workshop.normalize'), desc: $t('workshop.normalizeDesc'), defaultParams: { column: '' } },
		{ id: 'standardize', type: 'Standardize', label: $t('workshop.standardize'), desc: $t('workshop.standardizeDesc'), defaultParams: { column: '' } },
		{ id: 'one_hot', type: 'OneHotEncode', label: $t('workshop.oneHotEncode'), desc: $t('workshop.oneHotEncodeDesc'), defaultParams: { column: '' } },
		{ id: 'label_encode', type: 'LabelEncode', label: $t('workshop.labelEncode'), desc: $t('workshop.labelEncodeDesc'), defaultParams: { column: '' } },
		{ id: 'fill_missing', type: 'FillMissing', label: $t('workshop.fillMissing'), desc: $t('workshop.fillMissingDesc'), defaultParams: { column: '', fill_value: '0' } },
		{ id: 'drop_missing', type: 'DropMissing', label: $t('workshop.dropMissing'), desc: $t('workshop.dropMissingDesc'), defaultParams: { column: '' } },
	];

	const presetTemplates: { name: string; desc: string; steps: { type: string; params: Record<string, string> }[] }[] = [
		{
			name: $t('workshop.numericCleaning'),
			desc: $t('workshop.numericCleaningDesc'),
			steps: [
				{ type: 'FillMissing', params: { column: '', fill_value: '0' } },
				{ type: 'Standardize', params: { column: '' } },
			],
		},
		{
			name: $t('workshop.categoricalEncoding'),
			desc: $t('workshop.categoricalEncodingDesc'),
			steps: [
				{ type: 'FillMissing', params: { column: '', fill_value: 'unknown' } },
				{ type: 'LabelEncode', params: { column: '' } },
			],
		},
		{
			name: $t('workshop.fullPreprocess'),
			desc: $t('workshop.fullPreprocessDesc'),
			steps: [
				{ type: 'FillMissing', params: { column: '', fill_value: '0' } },
				{ type: 'Normalize', params: { column: '' } },
				{ type: 'OneHotEncode', params: { column: '' } },
			],
		},
	];

	type UIStep = {
		id: string;
		type: string;
		params: Record<string, string>;
	};

	let filePath = '';
	let fileFormat: DataFormat = 'csv';
	let preview: DataPreview | null = null;
	let previewLoading = false;
	let error: string | null = null;
	let previewPage = 0;
	const previewPageSize = 20;

	let steps: UIStep[] = [];
	let stepCounter = 0;
	let executing = false;
	let executeProgress = 0;
	let executeMessage = '';

	let showRegisterAfter = false;
	let regName = '';
	let registering = false;

	function buildLoadConfig(): DataLoadConfig {
		return {
			path: filePath,
			format: fileFormat,
			has_header: true,
			delimiter: null,
			encoding: null,
			max_rows: null,
			custom_params: {},
		};
	}

	async function selectFile() {
		const client = getLabClient();
		const path = await client.selectFile([
			{ name: 'Data Files', extensions: ['csv', 'json', 'parquet'] },
			{ name: 'All Files', extensions: ['*'] },
		]);
		if (path) {
			filePath = path;
			const ext = path.split('.').pop()?.toLowerCase();
			if (ext === 'csv') fileFormat = 'csv';
			else if (ext === 'json') fileFormat = 'json';
			else if (ext === 'parquet') fileFormat = 'parquet';
			await loadPreview();
		}
	}

	async function loadPreview() {
		if (!filePath) return;
		previewLoading = true;
		error = null;
		try {
			const client = getLabClient();
			const config = buildLoadConfig();
			preview = await client.previewData(config, previewPage * previewPageSize, previewPageSize);
		} catch (e: any) {
			error = e?.toString() || $t('workshop.loadPreviewFailed');
		} finally {
			previewLoading = false;
		}
	}

	function prevPage() {
		if (previewPage > 0) {
			previewPage--;
			loadPreview();
		}
	}

	function nextPage() {
		previewPage++;
		loadPreview();
	}

	function addStep(def: PreprocessStepDef) {
		stepCounter++;
		steps = [...steps, { id: `step-${stepCounter}`, type: def.type, params: { ...def.defaultParams } }];
	}

	function removeStep(id: string) {
		steps = steps.filter((s) => s.id !== id);
	}

	function moveStepUp(index: number) {
		if (index <= 0) return;
		const next = [...steps];
		[next[index - 1], next[index]] = [next[index], next[index - 1]];
		steps = next;
	}

	function moveStepDown(index: number) {
		if (index >= steps.length - 1) return;
		const next = [...steps];
		[next[index], next[index + 1]] = [next[index + 1], next[index]];
		steps = next;
	}

	function applyPreset(template: typeof presetTemplates[0]) {
		steps = template.steps.map((s, i) => ({
			id: `preset-${stepCounter + i}`,
			type: s.type,
			params: { ...s.params },
		}));
		stepCounter += template.steps.length;
	}

	function buildPipelineSteps(): PipelineStep[] {
		return steps.map((s) => ({
			step_type: s.type as PreprocessType,
			params: Object.fromEntries(
				Object.entries(s.params).map(([k, v]) => [k, v])
			),
		}));
	}

	async function executePipeline() {
		if (steps.length === 0) return;
		executing = true;
		executeProgress = 0;
		executeMessage = $t('workshop.startingPipeline');
		error = null;

		const taskId = taskManagerStore.createTask(
			$t('workshop.dataPreprocessPipeline'),
			$t('workshop.executeStepsCount', { count: steps.length }),
			true
		);

		try {
			const client = getLabClient();
			const pipelineSteps = buildPipelineSteps();

			taskManagerStore.updateProgress(taskId, 10, $t('workshop.initializingEngine'));
			executeProgress = 10;
			executeMessage = $t('workshop.initializingEngine');

			await new Promise((r) => setTimeout(r, 300));

			for (let i = 0; i < pipelineSteps.length; i++) {
				const step = pipelineSteps[i];
				const stepLabel = getStepLabel(step.step_type);
				const progress = 10 + Math.round(((i + 1) / pipelineSteps.length) * 80);
				const msg = $t('workshop.executingStep', { current: i + 1, total: pipelineSteps.length, label: stepLabel });
				taskManagerStore.updateProgress(taskId, progress, msg);
				executeProgress = progress;
				executeMessage = msg;
			}

			const result = await client.preprocessData(filePath, fileFormat, pipelineSteps);

			executeProgress = 100;
			executeMessage = $t('workshop.preprocessComplete', { rows: result.total_rows.toLocaleString() });
			taskManagerStore.completeTask(taskId, $t('workshop.processedRows', { rows: result.total_rows.toLocaleString() }));
			preview = result;
			showRegisterAfter = true;
		} catch (e: any) {
			error = e?.toString() || $t('workshop.pipelineFailed');
			executeMessage = $t('workshop.executeFailed');
			taskManagerStore.failTask(taskId, e?.toString() || $t('workshop.pipelineFailed'));
		} finally {
			executing = false;
		}
	}

	async function registerAfterProcess() {
		if (!regName.trim()) return;
		registering = true;
		try {
			await datasetRegistryStore.registerDataset(regName.trim(), fileFormat, filePath);
			goto('/lab/data/list');
		} catch (e: any) {
			error = e?.toString() || $t('workshop.registerFailed');
		} finally {
			registering = false;
		}
	}

	function formatValue(val: unknown): string {
		if (val === null || val === undefined) return 'null';
		if (typeof val === 'object') return JSON.stringify(val);
		return String(val);
	}

	function getStepLabel(type: PreprocessType | string): string {
		if (typeof type === 'object' && type !== null && 'Custom' in type) {
			return `${$t('workshop.custom')}: ${(type as { Custom: string }).Custom}`;
		}
		return stepTypes.find((s) => s.type === type)?.label || String(type);
	}

	onMount(() => {});
</script>

<div class="workshop-page">
	<div class="page-header">
		<div>
			<h2>{$t('workshop.title')}</h2>
			<p class="desc">{$t('workshop.desc')}</p>
		</div>
		<div class="header-actions">
			<button class="btn-secondary" on:click={() => goto('/lab/data/list')}>📋 {$t('workshop.datasetList')}</button>
		</div>
	</div>

	<div class="workshop-layout">
		<div class="panel panel-left">
			<div class="panel-section">
				<h4>{$t('workshop.step1SelectFile')}</h4>
				<div class="file-selector">
					<input
						type="text"
						bind:value={filePath}
						placeholder={$t('workshop.selectOrInputPath')}
						class="input"
						readonly
					/>
					<button class="btn-browse" on:click={selectFile}>{$t('workshop.browseFile')}</button>
				</div>
				{#if filePath}
					<div class="file-info">
						<span class="file-format">{fileFormat.toUpperCase()}</span>
						<span class="file-path-text">{filePath}</span>
					</div>
				{/if}
			</div>

			<div class="panel-section">
				<h4>{$t('workshop.step2Pipeline')}</h4>

				<div class="preset-templates">
					<span class="preset-label">{$t('workshop.presetTemplates')}:</span>
					{#each presetTemplates as tmpl}
						<button class="preset-btn" on:click={() => applyPreset(tmpl)} title={tmpl.desc}>
							{tmpl.name}
						</button>
					{/each}
				</div>

				<div class="step-list">
					{#each steps as step, i (step.id)}
						<div class="step-item">
							<div class="step-header">
								<span class="step-number">{i + 1}</span>
								<span class="step-type">{getStepLabel(step.type)}</span>
								<div class="step-actions">
									<button class="step-btn" on:click={() => moveStepUp(i)} disabled={i === 0} title={$t('workshop.moveUp')}>↑</button>
									<button class="step-btn" on:click={() => moveStepDown(i)} disabled={i === steps.length - 1} title={$t('workshop.moveDown')}>↓</button>
									<button class="step-btn step-remove" on:click={() => removeStep(step.id)} title={$t('workshop.delete')}>✕</button>
								</div>
							</div>
							<div class="step-params">
								{#each Object.entries(step.params) as [key, value]}
									<div class="param-row">
										<label for="param-{step.id}-{key}">{key}</label>
										<input
											id="param-{step.id}-{key}"
											type="text"
											value={value}
											on:input={(e) => {
												step.params[key] = e.currentTarget.value;
												steps = steps;
											}}
											class="param-input"
										/>
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>

				<div class="add-step-area">
					<span class="add-label">{$t('workshop.addStep')}:</span>
					<div class="step-buttons">
						{#each stepTypes as st}
							<button class="add-step-btn" on:click={() => addStep(st)} title={st.desc}>
								+ {st.label}
							</button>
						{/each}
					</div>
				</div>

				{#if steps.length > 0}
					<button
						class="btn-execute"
						on:click={executePipeline}
						disabled={executing || !filePath}
					>
						{executing ? `⏳ ${$t('workshop.executing')}` : `▶ ${$t('workshop.executePipeline')}`}
					</button>
				{/if}

				{#if executing}
					<div class="progress-section">
						<div class="progress-bar-track">
							<div class="progress-bar-fill" style="width: {executeProgress}%"></div>
						</div>
						<span class="progress-text">{executeMessage}</span>
					</div>
				{/if}

				{#if error}
					<div class="error-box">{error}</div>
				{/if}
			</div>

			{#if showRegisterAfter}
				<div class="panel-section">
					<h4>{$t('workshop.step3Register')}</h4>
					<div class="register-row">
						<input
							type="text"
							bind:value={regName}
							placeholder={$t('workshop.datasetName')}
							class="input"
						/>
						<button class="btn-primary" on:click={registerAfterProcess} disabled={registering || !regName.trim()}>
							{registering ? $t('workshop.registering') : $t('workshop.register')}
						</button>
					</div>
				</div>
			{/if}
		</div>

		<div class="panel panel-right">
			<h4>{$t('workshop.dataPreview')}</h4>
			{#if !filePath}
				<div class="empty-preview">
					<div class="empty-icon">📂</div>
					<p>{$t('workshop.selectFileToPreview')}</p>
				</div>
			{:else if previewLoading}
				<div class="skeleton-preview">
					{#each Array(5) as _}
						<Skeleton width="100%" height="28px" />
					{/each}
				</div>
			{:else if preview && preview.rows.length > 0}
				<div class="preview-table-wrapper">
					<table class="preview-table">
						<thead>
							<tr>
								<th class="row-num">#</th>
								{#each preview.columns as col}
									<th>{col}</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each preview.rows as row, ri}
								<tr>
									<td class="row-num">{preview.offset + ri + 1}</td>
									{#each row as cell}
										<td class="cell {cell === null ? 'null-cell' : ''}">
											{formatValue(cell)}
										</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				<div class="pagination">
					<button class="btn-sm" on:click={prevPage} disabled={previewPage === 0}>← {$t('workshop.prevPage')}</button>
					<span class="page-info">{$t('workshop.pageInfo', { page: previewPage + 1, rows: preview?.total_rows?.toLocaleString() || '?' })}</span>
					<button class="btn-sm" on:click={nextPage} disabled={preview && preview.rows.length < previewPageSize}>{$t('workshop.nextPage')} →</button>
				</div>
			{:else}
				<div class="empty-preview"><p>{$t('workshop.noDataToPreview')}</p></div>
			{/if}
		</div>
	</div>
</div>

<style>
	.workshop-page { padding: 0; }

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.25rem;
	}
	.page-header h2 { margin: 0 0 0.25rem 0; font-size: 1.3rem; }
	.desc { color: #9ca3af; font-size: 0.85rem; margin: 0; }
	.header-actions { display: flex; gap: 0.5rem; }

	.workshop-layout {
		display: grid;
		grid-template-columns: 420px 1fr;
		gap: 1.25rem;
		align-items: start;
	}

	.panel {
		background: rgba(255,255,255,0.015);
		border: 1px solid rgba(107,114,128,0.15);
		border-radius: 8px;
		padding: 1rem;
	}
	.panel h4 { margin: 0 0 0.65rem 0; font-size: 0.88rem; color: #e5e7eb; }

	.panel-section {
		padding-bottom: 1rem;
		margin-bottom: 1rem;
		border-bottom: 1px solid rgba(107,114,128,0.1);
	}
	.panel-section:last-child { border-bottom: none; margin-bottom: 0; padding-bottom: 0; }

	.file-selector { display: flex; gap: 0.4rem; }
	.input {
		flex: 1;
		padding: 0.45rem 0.6rem;
		background: rgba(255,255,255,0.05);
		border: 1px solid rgba(107,114,128,0.25);
		border-radius: 5px;
		color: #e5e7eb;
		font-size: 0.82rem;
		outline: none;
		box-sizing: border-box;
	}
	.input:focus { border-color: #3b82f6; }
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

	.file-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
		font-size: 0.75rem;
	}
	.file-format {
		padding: 0.08rem 0.35rem;
		background: rgba(59,130,246,0.12);
		color: #93c5fd;
		border-radius: 3px;
		font-weight: 500;
	}
	.file-path-text { color: #6b7280; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.preset-templates {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		flex-wrap: wrap;
		margin-bottom: 0.65rem;
	}
	.preset-label { font-size: 0.72rem; color: #6b7280; }
	.preset-btn {
		padding: 0.2rem 0.5rem;
		font-size: 0.7rem;
		border-radius: 4px;
		border: 1px solid rgba(16,185,129,0.25);
		background: rgba(16,185,129,0.06);
		color: #6ee7b7;
		cursor: pointer;
	}
	.preset-btn:hover { background: rgba(16,185,129,0.12); }

	.step-list { display: flex; flex-direction: column; gap: 0.5rem; margin-bottom: 0.65rem; }
	.step-item {
		background: rgba(255,255,255,0.03);
		border: 1px solid rgba(107,114,128,0.12);
		border-radius: 6px;
		padding: 0.5rem;
	}
	.step-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.step-number {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: rgba(59,130,246,0.15);
		color: #93c5fd;
		font-size: 0.7rem;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}
	.step-type { font-size: 0.8rem; color: #d1d5db; flex: 1; }
	.step-actions { display: flex; gap: 0.15rem; }
	.step-btn {
		background: none;
		border: 1px solid rgba(107,114,128,0.2);
		border-radius: 3px;
		color: #9ca3af;
		font-size: 0.65rem;
		cursor: pointer;
		padding: 0.1rem 0.3rem;
		line-height: 1;
	}
	.step-btn:hover { background: rgba(255,255,255,0.06); color: #d1d5db; }
	.step-btn:disabled { opacity: 0.3; cursor: default; }
	.step-remove:hover { color: #fca5a5; border-color: rgba(239,68,68,0.3); }

	.step-params { margin-top: 0.4rem; display: flex; flex-direction: column; gap: 0.25rem; }
	.param-row { display: flex; align-items: center; gap: 0.4rem; }
	.param-row label {
		font-size: 0.68rem;
		color: #6b7280;
		width: 60px;
		flex-shrink: 0;
		text-align: right;
	}
	.param-input {
		flex: 1;
		padding: 0.2rem 0.4rem;
		background: rgba(255,255,255,0.04);
		border: 1px solid rgba(107,114,128,0.15);
		border-radius: 3px;
		color: #d1d5db;
		font-size: 0.72rem;
		outline: none;
	}
	.param-input:focus { border-color: #3b82f6; }

	.add-step-area { margin-bottom: 0.65rem; }
	.add-label { font-size: 0.72rem; color: #6b7280; display: block; margin-bottom: 0.3rem; }
	.step-buttons { display: flex; flex-wrap: wrap; gap: 0.3rem; }
	.add-step-btn {
		padding: 0.2rem 0.45rem;
		font-size: 0.7rem;
		border-radius: 4px;
		border: 1px dashed rgba(107,114,128,0.25);
		background: transparent;
		color: #9ca3af;
		cursor: pointer;
	}
	.add-step-btn:hover { border-color: #3b82f6; color: #93c5fd; background: rgba(59,130,246,0.05); }

	.btn-execute {
		width: 100%;
		padding: 0.55rem;
		background: #3b82f6;
		border: none;
		border-radius: 6px;
		color: #fff;
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.15s;
	}
	.btn-execute:hover { background: #2563eb; }
	.btn-execute:disabled { background: #1e40af; opacity: 0.6; cursor: not-allowed; }

	.progress-section { margin-top: 0.65rem; }
	.progress-bar-track {
		height: 4px;
		background: rgba(255,255,255,0.08);
		border-radius: 2px;
		overflow: hidden;
		margin-bottom: 0.3rem;
	}
	.progress-bar-fill {
		height: 100%;
		background: #3b82f6;
		border-radius: 2px;
		transition: width 0.3s ease;
	}
	.progress-text { font-size: 0.72rem; color: #9ca3af; }

	.error-box {
		padding: 0.5rem 0.65rem;
		background: rgba(239,68,68,0.08);
		border: 1px solid rgba(239,68,68,0.2);
		border-radius: 5px;
		color: #fca5a5;
		font-size: 0.78rem;
		margin-top: 0.5rem;
	}

	.register-row { display: flex; gap: 0.4rem; }

	.empty-preview {
		text-align: center;
		padding: 2rem 1rem;
		color: #6b7280;
	}
	.empty-icon { font-size: 2rem; margin-bottom: 0.5rem; }
	.empty-preview p { font-size: 0.82rem; margin: 0; }

	.skeleton-preview { display: flex; flex-direction: column; gap: 0.4rem; }

	.preview-table-wrapper { overflow-x: auto; max-height: 400px; overflow-y: auto; }
	.preview-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.75rem;
	}
	.preview-table th {
		position: sticky;
		top: 0;
		background: #1f2937;
		padding: 0.35rem 0.5rem;
		color: #9ca3af;
		font-weight: 500;
		font-size: 0.68rem;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		border-bottom: 1px solid rgba(107,114,128,0.2);
		white-space: nowrap;
		text-align: left;
	}
	.preview-table td {
		padding: 0.25rem 0.5rem;
		border-bottom: 1px solid rgba(107,114,128,0.06);
		color: #d1d5db;
		max-width: 200px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row-num { color: #6b7280; font-size: 0.68rem; text-align: center; width: 40px; }
	.null-cell { color: #6b7280; font-style: italic; }

	.pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.65rem;
		margin-top: 0.5rem;
	}
	.page-info { font-size: 0.75rem; color: #9ca3af; }

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
	.btn-sm:disabled { opacity: 0.4; cursor: default; }
</style>
