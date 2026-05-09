<script lang="ts">
	import { onMount } from 'svelte';
	import { getLabClient, pluginStore } from '$lib/lab/stores/plugins';
	import { datasetRegistryStore } from '$lib/lab/stores/dataset';
	import { t } from '$lib/i18n';
	import type { TrainingConfig, HparamSpace, HparamRange, HparamValue, TuneStrategy, TuneResult, TrialResult, PluginInfo, DatasetSummary, ComputeBackend, DataFormat } from '$lib/lab/adapter/types';

	let tuneStrategy: 'grid' | 'random' = 'grid';
	let nTrials = 10;
	let metricToOptimize = 'val_loss';
	let maximize = false;
	let maxConcurrent = 1;

	let engineId = 'burn';
	let availableEngines: PluginInfo[] = [];

	let dataPath = '';
	let dataFormat: DataFormat = 'csv';
	let modelId = 'mlp';
	let epochs = 10;
	let batchSize = 32;
	let learningRate = 0.001;
	let optimizerType = 'Adam';
	let computeBackend: ComputeBackend = 'cpu';
	let lossFunction = 'cross_entropy';
	let validationSplit = 0.2;
	let targetColumn = 'label';
	let featureColumns = 'x';

	let dataSourceMode: 'file' | 'registered' = 'file';
	let selectedDatasetId = '';
	let registeredDatasets: DatasetSummary[] = [];

	let hparamEntries: { name: string; type: 'float' | 'int' | 'choice'; min: number; max: number; choices: string }[] = [
		{ name: 'learning_rate', type: 'float', min: 0.0001, max: 0.1, choices: '' },
		{ name: 'batch_size', type: 'int', min: 16, max: 128, choices: '' },
	];

	let previewCombinations: Record<string, HparamValue>[] = [];
	let showPreview = false;
	let tuning = false;
	let tuneResult: TuneResult | null = null;
	let error: string | null = null;

	onMount(async () => {
		await loadAvailableEngines();
		await loadRegisteredDatasets();
	});

	async function loadAvailableEngines() {
		try {
			await pluginStore.refresh();
			const unsub = pluginStore.subscribe((s) => {
				availableEngines = s.engine || [];
				if (availableEngines.length > 0 && !availableEngines.find((e) => e.id === engineId)) {
					engineId = availableEngines[0].id;
				}
			});
			unsub();
		} catch (e) {
			console.error('Failed to load engines:', e);
		}
	}

	async function loadRegisteredDatasets() {
		try {
			await datasetRegistryStore.fetchDatasets('active');
			const unsub = datasetRegistryStore.subscribe((s) => {
				registeredDatasets = s.datasets;
			});
			unsub();
		} catch (e) {
			console.error('Failed to load datasets:', e);
		}
	}

	function addHparamEntry() {
		hparamEntries = [...hparamEntries, { name: '', type: 'float', min: 0, max: 1, choices: '' }];
	}

	function removeHparamEntry(index: number) {
		hparamEntries = hparamEntries.filter((_, i) => i !== index);
	}

	function buildHparamSpace(): HparamSpace {
		const params: Record<string, HparamRange> = {};
		for (const entry of hparamEntries) {
			if (!entry.name.trim()) continue;
			if (entry.type === 'float') {
				params[entry.name] = { FloatRange: { min: entry.min, max: entry.max } };
			} else if (entry.type === 'int') {
				params[entry.name] = { IntRange: { min: entry.min, max: entry.max } };
			} else {
				const choices: HparamValue[] = entry.choices
					.split(',')
					.map((s) => s.trim())
					.filter((s) => s.length > 0)
					.map((s) => {
						const num = Number(s);
						if (!isNaN(num) && Number.isInteger(num)) return { Int: num };
						if (!isNaN(num)) return { Float: num };
						return { String: s };
					});
				params[entry.name] = { Choice: choices };
			}
		}
		return { params };
	}

	function buildStrategy(): TuneStrategy {
		if (tuneStrategy === 'grid') return 'Grid';
		return { Random: { n_trials: nTrials } };
	}

	function buildBaseConfig(): TrainingConfig {
		return {
			session_name: 'hparam_tuning',
			task_type: 'classification',
			engine_id: engineId,
			model_id: modelId,
			data_source_id: '',
			data_path: dataPath,
			epochs,
			batch_size: batchSize,
			learning_rate: learningRate,
			optimizer: optimizerType === 'Adam'
				? { Adam: { beta1: 0.9, beta2: 0.999, weight_decay: null } }
				: optimizerType === 'AdamW'
					? { AdamW: { beta1: 0.9, beta2: 0.999, weight_decay: 0.01 } }
					: optimizerType === 'Sgd'
						? { Sgd: { momentum: null, weight_decay: null } }
						: { Rmsprop: { alpha: 0.99, weight_decay: null } },
			loss_function: lossFunction,
			compute_backend: computeBackend,
			data_format: dataFormat,
			validation_split: validationSplit,
			test_split: 0.1,
			shuffle: true,
			seed: 42,
			checkpoint_interval: null,
			early_stopping: null,
			lr_scheduler: 'Constant',
			target_columns: [targetColumn],
			feature_columns: featureColumns.split(',').map((s) => s.trim()),
			custom_params: {},
		};
	}

	async function previewCombinationsAction() {
		try {
			const space = buildHparamSpace();
			const strategy = buildStrategy();
			const client = getLabClient();
			previewCombinations = await client.generateHparamCombinations(space, strategy);
			showPreview = true;
			error = null;
		} catch (e: any) {
			error = e?.toString() || $t('tune.previewFailed');
		}
	}

	async function startTuning() {
		tuning = true;
		error = null;
		tuneResult = null;
		try {
			const client = getLabClient();
			const tuneConfig = {
				base_config: buildBaseConfig(),
				hparam_space: buildHparamSpace(),
				strategy: buildStrategy(),
				metric_to_optimize: metricToOptimize,
				maximize,
				max_concurrent: maxConcurrent,
			};
			tuneResult = await client.startHyperparameterTuning(tuneConfig);
		} catch (e: any) {
			error = e?.toString() || $t('tune.tuningFailed');
		} finally {
			tuning = false;
		}
	}

	function formatHparamValue(v: HparamValue): string {
		if ('Float' in v) return v.Float.toFixed(6);
		if ('Int' in v) return v.Int.toString();
		if ('String' in v) return v.String;
		return '?';
	}

	function trialStatusColor(status: string): string {
		switch (status) {
			case 'Completed': return '#22c55e';
			case 'Running': return '#3b82f6';
			case 'Failed': return '#ef4444';
			default: return '#9ca3af';
		}
	}
</script>

<div class="tune-page">
	<h1>{$t('tune.title')}</h1>
	<p class="subtitle">{$t('tune.subtitle')}</p>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<div class="tune-layout">
		<div class="tune-config">
			<section class="config-section">
				<h2>{$t('tune.baseTrainingConfig')}</h2>
				<div class="form-grid">
					<div class="form-group">
						<label for="auto-f51">{$t('tune.computeEngine')}</label>
						<select id="auto-f51" bind:value={engineId}>
							{#each availableEngines as engine}
								<option value={engine.id}>{engine.name}</option>
							{/each}
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f52">{$t('tune.model')}</label>
						<select id="auto-f52" bind:value={modelId}>
							<option value="mlp">MLP</option>
							<option value="cnn">CNN</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f53">{$t('tune.dataPath')}</label>
						<input id="auto-f53" type="text" bind:value={dataPath} placeholder="/path/to/data.csv" />
					</div>
					<div class="form-group">
						<label for="auto-f54">{$t('tune.dataFormat')}</label>
						<select id="auto-f54" bind:value={dataFormat}>
							<option value="csv">CSV</option>
							<option value="json">JSON</option>
							<option value="parquet">Parquet</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f55">Epochs</label>
						<input id="auto-f55" type="number" bind:value={epochs} min="1" />
					</div>
					<div class="form-group">
						<label for="auto-f56">{$t('tune.optimizer')}</label>
						<select id="auto-f56" bind:value={optimizerType}>
							<option value="Adam">Adam</option>
							<option value="AdamW">AdamW</option>
							<option value="Sgd">SGD</option>
							<option value="Rmsprop">RMSprop</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f57">{$t('tune.lossFunction')}</label>
						<input id="auto-f57" type="text" bind:value={lossFunction} />
					</div>
					<div class="form-group">
						<label for="auto-f58">{$t('tune.computeBackend')}</label>
						<select id="auto-f58" bind:value={computeBackend}>
							<option value="cpu">CPU</option>
							<option value="cuda">CUDA</option>
							<option value="wgpu">WGPU</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f59">{$t('tune.validationSplit')}</label>
						<input id="auto-f59" type="number" bind:value={validationSplit} min="0" max="1" step="0.05" />
					</div>
					<div class="form-group">
						<label for="auto-f60">{$t('tune.targetColumn')}</label>
						<input id="auto-f60" type="text" bind:value={targetColumn} />
					</div>
					<div class="form-group">
						<label for="auto-f61">{$t('tune.featureColumns')}</label>
						<input id="auto-f61" type="text" bind:value={featureColumns} />
					</div>
				</div>
			</section>

			<section class="config-section">
				<h2>{$t('tune.hparamSearchSpace')}</h2>
				<div class="hparam-entries">
					{#each hparamEntries as entry, i}
						<div class="hparam-row">
							<input type="text" bind:value={entry.name} placeholder={$t('tune.paramName')} class="param-name" />
							<select bind:value={entry.type} class="param-type">
								<option value="float">{$t('tune.floatRange')}</option>
								<option value="int">{$t('tune.intRange')}</option>
								<option value="choice">{$t('tune.choiceEnum')}</option>
							</select>
							{#if entry.type === 'choice'}
								<input type="text" bind:value={entry.choices} placeholder={$t('tune.choicePlaceholder')} class="param-choices" />
							{:else}
								<input type="number" bind:value={entry.min} step="any" class="param-min" placeholder={$t('tune.min')} />
								<input type="number" bind:value={entry.max} step="any" class="param-max" placeholder={$t('tune.max')} />
							{/if}
							<button class="btn-remove" on:click={() => removeHparamEntry(i)}>✕</button>
						</div>
					{/each}
				</div>
				<button class="btn-add" on:click={addHparamEntry}>+ {$t('tune.addParam')}</button>
			</section>

			<section class="config-section">
				<h2>{$t('tune.tuningStrategy')}</h2>
				<div class="strategy-options">
					<label class="radio-label">
						<input type="radio" name="strategy" bind:group={tuneStrategy} value="grid" />
						<span>Grid Search ({$t('tune.exhaustiveSearch')})</span>
					</label>
					<label class="radio-label">
						<input type="radio" name="strategy" bind:group={tuneStrategy} value="random" />
						<span>Random Search ({$t('tune.randomSearch')})</span>
					</label>
				</div>
				{#if tuneStrategy === 'random'}
					<div class="form-group" style="margin-top: 12px;">
						<label for="auto-f62">{$t('tune.nTrials')}</label>
						<input id="auto-f62" type="number" bind:value={nTrials} min="1" max="1000" />
					</div>
				{/if}
				<div class="form-grid" style="margin-top: 12px;">
					<div class="form-group">
						<label for="auto-f63">{$t('tune.optimizeMetric')}</label>
						<select id="auto-f63" bind:value={metricToOptimize}>
							<option value="val_loss">{$t('tune.valLoss')} (val_loss)</option>
							<option value="train_loss">{$t('tune.trainLoss')} (train_loss)</option>
							<option value="accuracy">{$t('tune.accuracy')} (accuracy)</option>
							<option value="val_accuracy">{$t('tune.valAccuracy')} (val_accuracy)</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f64">{$t('tune.optimizeDirection')}</label>
						<select id="auto-f64" bind:value={maximize}>
							<option value={false}>{$t('tune.minimize')}</option>
							<option value={true}>{$t('tune.maximize')}</option>
						</select>
					</div>
					<div class="form-group">
						<label for="auto-f65">{$t('tune.maxConcurrency')}</label>
						<input id="auto-f65" type="number" bind:value={maxConcurrent} min="1" max="10" />
					</div>
				</div>
			</section>

			<div class="action-bar">
				<button class="btn-preview" on:click={previewCombinationsAction} disabled={tuning}>
					{$t('tune.previewCombinations')}
				</button>
				<button class="btn-start" on:click={startTuning} disabled={tuning}>
					{tuning ? $t('tune.tuning') : $t('tune.startTuning')}
				</button>
			</div>
		</div>

		<div class="tune-results">
			{#if showPreview && previewCombinations.length > 0}
				<section class="preview-section">
					<h2>{$t('tune.comboPreview', { count: previewCombinations.length })}</h2>
					<div class="preview-table-wrapper">
						<table class="preview-table">
							<thead>
								<tr>
									<th>#</th>
									{#each Object.keys(previewCombinations[0] || {}) as key}
										<th>{key}</th>
									{/each}
								</tr>
							</thead>
							<tbody>
								{#each previewCombinations.slice(0, 50) as combo, i}
									<tr>
										<td>{i + 1}</td>
										{#each Object.values(combo) as val}
											<td>{formatHparamValue(val as HparamValue)}</td>
										{/each}
									</tr>
								{/each}
							</tbody>
						</table>
						{#if previewCombinations.length > 50}
							<p class="more-hint">... {$t('tune.moreCombos', { count: previewCombinations.length - 50 })}</p>
						{/if}
					</div>
				</section>
			{/if}

			{#if tuneResult}
				<section class="result-section">
					<h2>{$t('tune.tuningResult')}</h2>
					<div class="result-summary">
						<div class="summary-card">
							<div class="summary-label">{$t('tune.tuneId')}</div>
							<div class="summary-value">{tuneResult.tune_id.slice(0, 8)}...</div>
						</div>
						<div class="summary-card">
							<div class="summary-label">{$t('tune.totalTrials')}</div>
							<div class="summary-value">{tuneResult.trials.length}</div>
						</div>
						<div class="summary-card">
							<div class="summary-label">{$t('tune.completed')}</div>
							<div class="summary-value">{tuneResult.trials.filter(t => t.status === 'Completed').length}</div>
						</div>
						<div class="summary-card">
							<div class="summary-label">{$t('tune.failed')}</div>
							<div class="summary-value">{tuneResult.trials.filter(t => t.status === 'Failed').length}</div>
						</div>
					</div>

					{#if tuneResult.best_params}
						<div class="best-params">
							<h3>{$t('tune.bestParams')}</h3>
							<div class="params-grid">
								{#each Object.entries(tuneResult.best_params) as [key, value]}
									<div class="param-item">
										<span class="param-key">{key}</span>
										<span class="param-val">{formatHparamValue(value as HparamValue)}</span>
									</div>
								{/each}
							</div>
							{#if tuneResult.best_trial?.metric_value !== null}
								<p class="best-metric">
									{$t('tune.best')} {metricToOptimize}: <strong>{tuneResult.best_trial?.metric_value?.toFixed(6)}</strong>
								</p>
							{/if}
						</div>
					{/if}

					<h3 style="margin-top: 20px;">{$t('tune.allTrials')}</h3>
					<div class="trials-table-wrapper">
						<table class="trials-table">
							<thead>
								<tr>
									<th>#</th>
									<th>{$t('tune.experimentId')}</th>
									<th>{$t('tune.params')}</th>
									<th>{$t('tune.metricValue')}</th>
									<th>{$t('tune.status')}</th>
								</tr>
							</thead>
							<tbody>
								{#each tuneResult.trials as trial, i}
									<tr>
										<td>{i + 1}</td>
										<td class="mono">{trial.experiment_id.slice(0, 8)}...</td>
										<td>
											{#each Object.entries(trial.params) as [key, value]}
												<span class="trial-param">{key}={formatHparamValue(value as HparamValue)}</span>
											{/each}
										</td>
										<td>{trial.metric_value !== null ? trial.metric_value.toFixed(6) : '—'}</td>
										<td>
											<span class="status-badge" style="background: {trialStatusColor(trial.status)}20; color: {trialStatusColor(trial.status)};">
												{trial.status}
											</span>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</section>
			{/if}
		</div>
	</div>
</div>

<style>
	.tune-page {
		padding: 24px;
		max-width: 1400px;
		margin: 0 auto;
	}

	h1 {
		font-size: 1.75rem;
		font-weight: 700;
		margin-bottom: 4px;
	}

	.subtitle {
		color: #6b7280;
		margin-bottom: 24px;
	}

	.error-banner {
		background: #fef2f2;
		border: 1px solid #fecaca;
		color: #dc2626;
		padding: 12px 16px;
		border-radius: 8px;
		margin-bottom: 16px;
	}

	.tune-layout {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 24px;
	}

	.config-section {
		background: #fff;
		border: 1px solid #e5e7eb;
		border-radius: 12px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.config-section h2 {
		font-size: 1.1rem;
		font-weight: 600;
		margin-bottom: 16px;
		padding-bottom: 8px;
		border-bottom: 1px solid #f3f4f6;
	}

	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.form-group label {
		font-size: 0.8rem;
		color: #6b7280;
		font-weight: 500;
	}

	.form-group input,
	.form-group select {
		padding: 8px 12px;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.9rem;
		background: #fff;
	}

	.hparam-entries {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-bottom: 12px;
	}

	.hparam-row {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.param-name {
		width: 140px;
	}

	.param-type {
		width: 110px;
	}

	.param-min, .param-max {
		width: 80px;
	}

	.param-choices {
		flex: 1;
	}

	.btn-remove {
		background: none;
		border: none;
		color: #ef4444;
		cursor: pointer;
		font-size: 1rem;
		padding: 4px 8px;
	}

	.btn-add {
		background: none;
		border: 1px dashed #d1d5db;
		border-radius: 6px;
		padding: 8px 16px;
		color: #6b7280;
		cursor: pointer;
		width: 100%;
	}

	.btn-add:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.strategy-options {
		display: flex;
		gap: 24px;
	}

	.radio-label {
		display: flex;
		align-items: center;
		gap: 8px;
		cursor: pointer;
		font-size: 0.9rem;
	}

	.action-bar {
		display: flex;
		gap: 12px;
		margin-top: 16px;
	}

	.btn-preview {
		padding: 10px 20px;
		background: #f3f4f6;
		border: 1px solid #d1d5db;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.9rem;
	}

	.btn-preview:hover {
		background: #e5e7eb;
	}

	.btn-start {
		padding: 10px 24px;
		background: #3b82f6;
		color: white;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 600;
	}

	.btn-start:hover {
		background: #2563eb;
	}

	.btn-start:disabled {
		background: #93c5fd;
		cursor: not-allowed;
	}

	.preview-section, .result-section {
		background: #fff;
		border: 1px solid #e5e7eb;
		border-radius: 12px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.preview-section h2, .result-section h2 {
		font-size: 1.1rem;
		font-weight: 600;
		margin-bottom: 16px;
	}

	.preview-table-wrapper, .trials-table-wrapper {
		overflow-x: auto;
	}

	.preview-table, .trials-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.preview-table th, .preview-table td,
	.trials-table th, .trials-table td {
		padding: 8px 12px;
		border-bottom: 1px solid #f3f4f6;
		text-align: left;
	}

	.preview-table th, .trials-table th {
		background: #f9fafb;
		font-weight: 600;
		color: #374151;
	}

	.more-hint {
		color: #9ca3af;
		font-size: 0.8rem;
		margin-top: 8px;
		text-align: center;
	}

	.result-summary {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
		margin-bottom: 20px;
	}

	.summary-card {
		background: #f9fafb;
		border-radius: 8px;
		padding: 12px;
		text-align: center;
	}

	.summary-label {
		font-size: 0.75rem;
		color: #6b7280;
		margin-bottom: 4px;
	}

	.summary-value {
		font-size: 1.25rem;
		font-weight: 700;
		color: #111827;
	}

	.best-params {
		background: #f0fdf4;
		border: 1px solid #bbf7d0;
		border-radius: 8px;
		padding: 16px;
	}

	.best-params h3 {
		font-size: 0.95rem;
		font-weight: 600;
		margin-bottom: 12px;
		color: #166534;
	}

	.params-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.param-item {
		display: flex;
		justify-content: space-between;
		padding: 6px 12px;
		background: #fff;
		border-radius: 6px;
	}

	.param-key {
		color: #374151;
		font-weight: 500;
	}

	.param-val {
		color: #166534;
		font-family: monospace;
	}

	.best-metric {
		margin-top: 12px;
		font-size: 0.9rem;
		color: #166534;
	}

	.trial-param {
		display: inline-block;
		background: #f3f4f6;
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 0.8rem;
		margin: 1px;
		font-family: monospace;
	}

	.mono {
		font-family: monospace;
		font-size: 0.85rem;
	}

	.status-badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 10px;
		font-size: 0.8rem;
		font-weight: 500;
	}

	@media (max-width: 1024px) {
		.tune-layout {
			grid-template-columns: 1fr;
		}
	}
</style>
