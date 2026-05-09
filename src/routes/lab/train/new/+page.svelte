<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { experimentStore } from '$lib/lab/stores/experiment';
	import { datasetRegistryStore, activeDatasets } from '$lib/lab/stores/dataset';
	import { getLabClient, pluginStore } from '$lib/lab/stores/plugins';
	import { t } from '$lib/i18n';
	import type { TrainingConfig, OptimizerConfig, TaskType, ComputeBackend, DataFormat, EarlyStoppingConfig, LrSchedulerConfig, DatasetInfo, DatasetSummary, PluginInfo } from '$lib/lab/adapter/types';

	let currentStep = 0;
	const steps = [$t('trainNew.stepBasicInfo'), $t('trainNew.stepDataConfig'), $t('trainNew.stepModelHparams'), $t('trainNew.stepConfirm')];
	let submitting = false;
	let error: string | null = null;

	let name = '';
	let taskType: TaskType = 'classification';
	let engineId = 'burn';
	let availableEngines: PluginInfo[] = [];

	let dataPath = '';
	let dataFormat: DataFormat = 'csv';
	let validationSplit = 0.2;
	let testSplit = 0.1;
	let shuffle = true;
	let seed: number | null = 42;

	let datasetInfo: DatasetInfo | null = null;
	let datasetLoading = false;
	let targetColumn = '';
	let featureColumns: string[] = [];

	let dataSourceMode: 'file' | 'registered' = 'file';
	let selectedDatasetId: string = '';
	let registeredDatasets: DatasetSummary[] = [];
	let datasetsLoaded = false;

	let modelId = 'mlp';
	let epochs = 10;
	let batchSize = 32;
	let learningRate = 0.001;
	let optimizerType = 'Adam';
	let lossFunction = 'cross_entropy';
	let computeBackend: ComputeBackend = 'cpu';

	let checkpointInterval: number | null = null;

	let enableEarlyStopping = false;
	let earlyStoppingMetric: 'loss' | 'accuracy' = 'loss';
	let earlyStoppingPatience = 5;
	let earlyStoppingMinDelta = 0.001;
	let earlyStoppingMode: 'min' | 'max' = 'min';

	let cnnInputChannels = 1;
	let cnnInputHeight = 28;
	let cnnInputWidth = 28;

	let lrSchedulerType: 'Constant' | 'Step' | 'Exponential' | 'CosineAnnealing' | 'Linear' = 'Constant';
	let stepSize = 10;
	let stepGamma = 0.1;
	let expGamma = 0.99;
	let cosineMinLr = 0.0;
	let cosineNumIters = 100;
	let linearFinalLr = 0.0;
	let linearNumIters = 100;

	const modelOptions = [
		{ value: 'mlp', label: `MLP (${$t('trainNew.mlp')})`, desc: $t('trainNew.mlpDesc') },
		{ value: 'cnn', label: `CNN (${$t('trainNew.cnn')})`, desc: $t('trainNew.cnnDesc') },
	];

	onMount(async () => {
		await loadDatasetColumns();
		await loadRegisteredDatasets();
		await loadAvailableEngines();
	});

	async function loadAvailableEngines() {
		try {
			await pluginStore.refresh();
			const unsub = pluginStore.subscribe((s) => {
				availableEngines = s.engine || [];
				if (availableEngines.length > 0 && !availableEngines.find(e => e.id === engineId)) {
					engineId = availableEngines[0].id;
				}
			});
			unsub();
		} catch (e) {
			console.error('Failed to load engines:', e);
			availableEngines = [{ id: 'burn', name: 'Burn (Rust)', version: '0.1.0', description: 'Burn deep learning engine', plugin_kind: 'engine' }];
		}
	}

	async function loadRegisteredDatasets() {
		if (datasetsLoaded) return;
		try {
			await datasetRegistryStore.fetchDatasets('active');
			const unsub = datasetRegistryStore.subscribe((s) => {
				registeredDatasets = s.datasets;
			});
			unsub();
			datasetsLoaded = true;
		} catch (e) {
			console.error('Failed to load registered datasets:', e);
		}
	}

	async function selectRegisteredDataset(datasetId: string) {
		selectedDatasetId = datasetId;
		datasetLoading = true;
		try {
			const client = getLabClient();
			const dataset = await client.getDataset(datasetId);
			dataPath = dataset.path;
			dataFormat = dataset.format;
			datasetInfo = {
				name: dataset.name,
				format: dataset.format,
				rows: dataset.rows,
				columns: dataset.columns,
				column_names: dataset.column_profiles.map((c) => c.name),
				column_types: dataset.column_profiles.map((c) => c.column_type),
				has_missing_values: dataset.column_profiles.some((c) => c.null_count > 0),
				memory_size_mb: dataset.memory_size_mb,
			};
			if (datasetInfo.column_names.length > 0) {
				featureColumns = datasetInfo.column_names.slice(0, -1);
				targetColumn = datasetInfo.column_names[datasetInfo.column_names.length - 1];
			}
		} catch (e) {
			datasetInfo = null;
		} finally {
			datasetLoading = false;
		}
	}

	async function loadDatasetColumns() {
		if (!dataPath) return;
		datasetLoading = true;
		try {
			const client = getLabClient();
			datasetInfo = await client.loadData({
				path: dataPath,
				format: dataFormat,
				has_header: true,
				delimiter: null,
				encoding: null,
				max_rows: null,
				custom_params: {},
			});
			if (datasetInfo && datasetInfo.column_names.length > 0) {
				featureColumns = datasetInfo.column_names.slice(0, -1);
				targetColumn = datasetInfo.column_names[datasetInfo.column_names.length - 1];
			}
		} catch (e) {
			datasetInfo = null;
		} finally {
			datasetLoading = false;
		}
	}

	function toggleFeatureColumn(col: string) {
		if (col === targetColumn) return;
		if (featureColumns.includes(col)) {
			featureColumns = featureColumns.filter(c => c !== col);
		} else {
			featureColumns = [...featureColumns, col];
		}
	}

	function selectTarget(col: string) {
		if (featureColumns.includes(col)) {
			featureColumns = featureColumns.filter(c => c !== col);
		}
		targetColumn = col;
	}

	async function selectDataFile() {
		const client = getLabClient();
		const path = await client.selectFile([
			{ name: 'CSV Files', extensions: ['csv'] },
			{ name: 'JSON Files', extensions: ['json'] },
			{ name: 'All Files', extensions: ['*'] },
		]);
		if (path) {
			dataPath = path;
			const ext = path.split('.').pop()?.toLowerCase();
			if (ext === 'csv') dataFormat = 'csv';
			else if (ext === 'json') dataFormat = 'json';
			await loadDatasetColumns();
		}
	}

	async function selectDataDirectory() {
		const client = getLabClient();
		const path = await client.selectDirectory();
		if (path) {
			dataPath = path;
			await loadDatasetColumns();
		}
	}

	function getOptimizerConfig(): OptimizerConfig {
		switch (optimizerType) {
			case 'Adam':
				return { Adam: { beta1: 0.9, beta2: 0.999, weight_decay: null } };
			case 'AdamW':
				return { AdamW: { beta1: 0.9, beta2: 0.999, weight_decay: 0.01 } };
			case 'Sgd':
				return { Sgd: { momentum: null, weight_decay: null } };
			default:
				return { Custom: { name: optimizerType, params: {} } };
		}
	}

	function getLrSchedulerConfig(): LrSchedulerConfig {
		switch (lrSchedulerType) {
			case 'Step':
				return { Step: { step_size: stepSize, gamma: stepGamma } };
			case 'Exponential':
				return { Exponential: { gamma: expGamma } };
			case 'CosineAnnealing':
				return { CosineAnnealing: { min_lr: cosineMinLr, num_iters: cosineNumIters } };
			case 'Linear':
				return { Linear: { final_lr: linearFinalLr, num_iters: linearNumIters } };
			default:
				return 'Constant';
		}
	}

	function buildConfig(): TrainingConfig {
		const customParams: Record<string, unknown> = {};

		if (modelId === 'cnn') {
			customParams['input_channels'] = cnnInputChannels;
			customParams['input_height'] = cnnInputHeight;
			customParams['input_width'] = cnnInputWidth;
		}

		if (datasetInfo) {
			customParams['num_features'] = featureColumns.length;
		}

		let earlyStopping: EarlyStoppingConfig | null = null;
		if (enableEarlyStopping) {
			earlyStopping = {
				metric: earlyStoppingMetric,
				patience: earlyStoppingPatience,
				min_delta: earlyStoppingMinDelta,
				mode: earlyStoppingMode,
			};
		}

		return {
			session_name: name,
			task_type: taskType,
			engine_id: engineId,
			model_id: modelId,
			data_source_id: dataSourceMode === 'registered' ? selectedDatasetId : '',
			data_path: dataPath,
			epochs,
			batch_size: batchSize,
			learning_rate: learningRate,
			optimizer: getOptimizerConfig(),
			loss_function: lossFunction,
			compute_backend: computeBackend,
			data_format: dataFormat,
			validation_split: validationSplit,
			test_split: testSplit,
			shuffle,
			seed,
			checkpoint_interval: checkpointInterval,
			early_stopping: earlyStopping,
			lr_scheduler: getLrSchedulerConfig(),
			target_columns: targetColumn ? [targetColumn] : [],
			feature_columns: featureColumns,
			custom_params: customParams,
		};
	}

	function canProceed(): boolean {
		switch (currentStep) {
			case 0:
				if (name.trim().length === 0) return false;
				if (name.trim().length > 128) return false;
				return true;
			case 1:
				if (dataSourceMode === 'registered' && !selectedDatasetId) return false;
				if (dataSourceMode === 'file' && !dataPath.trim()) return false;
				if (validationSplit + testSplit >= 1.0) return false;
				return true;
			case 2:
				if (epochs <= 0 || epochs > 10000) return false;
				if (batchSize <= 0 || batchSize > 4096) return false;
				if (learningRate <= 0 || learningRate > 100) return false;
				if (taskType === 'classification' && (!targetColumn || targetColumn.trim() === '')) return false;
				return true;
			default:
				return true;
		}
	}

	function getValidationMessage(): string | null {
		switch (currentStep) {
			case 0:
				if (name.trim().length === 0) return $t('trainNew.valEnterName');
				if (name.trim().length > 128) return $t('trainNew.valNameTooLong');
				return null;
			case 1:
				if (dataSourceMode === 'registered' && !selectedDatasetId) return $t('trainNew.valSelectDataset');
				if (dataSourceMode === 'file' && !dataPath.trim()) return $t('trainNew.valEnterDataPath');
				if (validationSplit + testSplit >= 1.0) return $t('trainNew.valSplitSum');
				return null;
			case 2:
				if (epochs <= 0) return $t('trainNew.valEpochsPositive');
				if (epochs > 10000) return $t('trainNew.valEpochsMax');
				if (batchSize <= 0) return $t('trainNew.valBatchPositive');
				if (batchSize > 4096) return $t('trainNew.valBatchMax');
				if (learningRate <= 0) return $t('trainNew.valLrPositive');
				if (learningRate > 100) return $t('trainNew.valLrTooLarge');
				if (taskType === 'classification' && (!targetColumn || targetColumn.trim() === '')) return $t('trainNew.valClassificationTarget');
				return null;
			default:
				return null;
		}
	}

	function nextStep() {
		if (canProceed() && currentStep < steps.length - 1) {
			currentStep++;
		}
	}

	function prevStep() {
		if (currentStep > 0) {
			currentStep--;
		}
	}

	async function submit() {
		submitting = true;
		error = null;
		try {
			const config = buildConfig();
			const experimentId = await experimentStore.startTraining(name, taskType, config);
			if (experimentId) {
				goto(`/lab/experiments/${experimentId}`);
			} else {
				error = $t('trainNew.createFailed');
			}
		} catch (e: any) {
			error = e?.message || $t('trainNew.startError');
		} finally {
			submitting = false;
		}
	}

	const taskTypes = [
		{ value: 'classification', label: $t('trainNew.taskClassification'), icon: '🏷️' },
		{ value: 'regression', label: $t('trainNew.taskRegression'), icon: '📊' },
		{ value: 'clustering', label: $t('trainNew.taskClustering'), icon: '🔮' },
		{ value: 'detection', label: $t('trainNew.taskDetection'), icon: '🎯' },
		{ value: 'segmentation', label: $t('trainNew.taskSegmentation'), icon: '✂️' },
		{ value: 'generation', label: $t('trainNew.taskGeneration'), icon: '✨' },
		{ value: 'custom', label: $t('trainNew.taskCustom'), icon: '🔧' },
	];
</script>

<div class="wizard-page">
	<h2>{$t('trainNew.title')}</h2>
	<p class="desc">{$t('trainNew.desc')}</p>

	<div class="stepper">
		{#each steps as step, i}
			<button class="step-btn" class:active={i === currentStep} class:done={i < currentStep} on:click={() => { if (i <= currentStep) currentStep = i; }}>
				<span class="step-num">{i < currentStep ? '✓' : i + 1}</span>
				<span class="step-label">{step}</span>
			</button>
			{#if i < steps.length - 1}
				<div class="step-line" class:filled={i < currentStep}></div>
			{/if}
		{/each}
	</div>

	<div class="step-content">
		{#if currentStep === 0}
			<div class="form-section">
				<h3>{$t('trainNew.experimentName')}</h3>
				<input type="text" bind:value={name} placeholder={$t('trainNew.experimentNamePlaceholder')} class="input" />

				<h3>{$t('trainNew.taskType')}</h3>
				<div class="task-grid">
					{#each taskTypes as t}
						<button class="task-card" class:selected={taskType === t.value} on:click={() => taskType = t.value as TaskType}>
							<span class="task-icon">{t.icon}</span>
							<span class="task-label">{t.label}</span>
						</button>
					{/each}
				</div>

				<h3>{$t('trainNew.computeEngine')}</h3>
				<select bind:value={engineId} class="input">
					{#each availableEngines as engine}
						<option value={engine.id}>{engine.name}</option>
					{/each}
					{#if availableEngines.length === 0}
						<option value="burn">Burn (Rust)</option>
					{/if}
				</select>
				{#if availableEngines.find(e => e.id === engineId)?.description}
					<p class="hint-text">{availableEngines.find(e => e.id === engineId)?.description}</p>
				{/if}
			</div>

		{:else if currentStep === 1}
			<div class="form-section">
				<h3>{$t('trainNew.dataSource')}</h3>
				<div class="source-mode-tabs">
					<button class="source-tab" class:active={dataSourceMode === 'file'} on:click={() => dataSourceMode = 'file'}>
						📁 {$t('trainNew.selectFile')}
					</button>
					<button class="source-tab" class:active={dataSourceMode === 'registered'} on:click={() => { dataSourceMode = 'registered'; loadRegisteredDatasets(); }}>
						📦 {$t('trainNew.registeredDatasets')}
					</button>
				</div>

				{#if dataSourceMode === 'file'}
					<h3>{$t('trainNew.dataPath')}</h3>
					<div class="input-with-button">
						<input type="text" bind:value={dataPath} placeholder={$t('trainNew.clickToSelect')} class="input" readonly />
						<button class="btn-browse" on:click={selectDataFile}>{$t('trainNew.selectFileBtn')}</button>
						<button class="btn-browse" on:click={selectDataDirectory}>{$t('trainNew.selectDirBtn')}</button>
					</div>

					<h3>{$t('trainNew.dataFormat')}</h3>
					<select bind:value={dataFormat} class="input">
						<option value="csv">CSV</option>
						<option value="json">JSON</option>
						<option value="parquet">Parquet</option>
					</select>
				{:else}
					<h3>{$t('trainNew.selectRegisteredDataset')}</h3>
					{#if registeredDatasets.length === 0}
						<div class="empty-datasets">
							<p>{$t('trainNew.noRegisteredDatasets')}</p>
							<a href="/lab/data" class="link-btn">{$t('trainNew.goRegister')} →</a>
						</div>
					{:else}
						<div class="dataset-selector">
							{#each registeredDatasets as ds}
								<button
									class="dataset-option"
									class:selected={selectedDatasetId === ds.id}
									on:click={() => selectRegisteredDataset(ds.id)}
								>
									<div class="dataset-option-header">
										<span class="dataset-option-name">{ds.name}</span>
										<span class="dataset-option-format">{ds.format.toUpperCase()}</span>
									</div>
									<div class="dataset-option-meta">
										<span>{ds.rows.toLocaleString()} {$t('trainNew.rows')}</span>
										<span>{ds.columns} {$t('trainNew.cols')}</span>
										<span>{ds.memory_size_mb < 1 ? (ds.memory_size_mb * 1024).toFixed(1) + ' KB' : ds.memory_size_mb.toFixed(1) + ' MB'}</span>
										{#if ds.has_missing_values}
											<span class="missing-badge">{$t('trainNew.hasMissingValues')}</span>
										{/if}
									</div>
									{#if ds.tags.length > 0}
										<div class="dataset-option-tags">
											{#each ds.tags.slice(0, 3) as tag}
												<span class="mini-tag">{tag}</span>
											{/each}
										</div>
									{/if}
								</button>
							{/each}
						</div>
					{/if}
				{/if}

				<div class="form-row">
					<div class="form-field">
						<label for="auto-f26">{$t('trainNew.valSplit')}</label>
						<input id="auto-f26" type="number" bind:value={validationSplit} min="0" max="1" step="0.05" class="input" />
					</div>
					<div class="form-field">
						<label for="auto-f27">{$t('trainNew.testSplit')}</label>
						<input id="auto-f27" type="number" bind:value={testSplit} min="0" max="1" step="0.05" class="input" />
					</div>
				</div>

				<div class="form-row">
					<div class="form-field">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={shuffle} />
							{$t('trainNew.shuffleData')}
						</label>
					</div>
					<div class="form-field">
						<label for="auto-f28">{$t('trainNew.randomSeed')}</label>
						<input id="auto-f28" type="number" bind:value={seed} class="input" placeholder={$t('trainNew.randomSeedPlaceholder')} />
					</div>
				</div>

				{#if datasetInfo}
					<h3>{$t('trainNew.columnConfig')}</h3>
					<p class="hint-text">{$t('trainNew.columnConfigHint')}</p>
					<div class="column-grid">
						{#each datasetInfo.column_names as col, i}
							<button
								class="column-chip"
								class:is-target={col === targetColumn}
								class:is-feature={featureColumns.includes(col)}
								on:click={() => {
									if (col === targetColumn) {
										targetColumn = '';
										featureColumns = [...featureColumns, col];
									} else if (featureColumns.includes(col)) {
										selectTarget(col);
									} else {
										featureColumns = [...featureColumns, col];
									}
								}}
							>
								{#if col === targetColumn}
									🎯
								{:else if featureColumns.includes(col)}
									📊
								{:else}
									○
								{/if}
								{col}
							</button>
						{/each}
					</div>
					<div class="column-summary">
						<span class="summary-tag target-tag">🎯 {$t('trainNew.targetCol')}: {targetColumn || $t('trainNew.notSelected')}</span>
						<span class="summary-tag feature-tag">📊 {$t('trainNew.featureCols')}: {featureColumns.length} {$t('trainNew.countUnit')}</span>
					</div>
				{/if}

				<h3>{$t('trainNew.lrScheduler')}</h3>
				<div class="form-field">
					<label for="auto-f29">{$t('trainNew.schedulerStrategy')}</label>
					<select id="auto-f29" bind:value={lrSchedulerType} class="input">
						<option value="Constant">Constant ({$t('trainNew.constantLr')})</option>
						<option value="Step">Step ({$t('trainNew.stepDecay')})</option>
						<option value="Exponential">Exponential ({$t('trainNew.expDecay')})</option>
						<option value="CosineAnnealing">Cosine Annealing ({$t('trainNew.cosineAnnealing')})</option>
						<option value="Linear">Linear ({$t('trainNew.linearDecay')})</option>
					</select>
				</div>

				{#if lrSchedulerType === 'Step'}
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f30">{$t('trainNew.stepSize')}</label>
							<input id="auto-f30" type="number" bind:value={stepSize} min="1" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f31">{$t('trainNew.gamma')}</label>
							<input id="auto-f31" type="number" bind:value={stepGamma} min="0" max="1" step="0.01" class="input" />
						</div>
					</div>
				{:else if lrSchedulerType === 'Exponential'}
					<div class="form-field">
						<label for="auto-f32">{$t('trainNew.gamma')}</label>
						<input id="auto-f32" type="number" bind:value={expGamma} min="0" max="1" step="0.001" class="input" />
					</div>
				{:else if lrSchedulerType === 'CosineAnnealing'}
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f33">{$t('trainNew.minLr')}</label>
							<input id="auto-f33" type="number" bind:value={cosineMinLr} min="0" step="0.00001" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f34">{$t('trainNew.numIterations')}</label>
							<input id="auto-f34" type="number" bind:value={cosineNumIters} min="1" class="input" />
						</div>
					</div>
				{:else if lrSchedulerType === 'Linear'}
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f35">{$t('trainNew.finalLr')}</label>
							<input id="auto-f35" type="number" bind:value={linearFinalLr} min="0" step="0.00001" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f36">{$t('trainNew.numIterations')}</label>
							<input id="auto-f36" type="number" bind:value={linearNumIters} min="1" class="input" />
						</div>
					</div>
				{/if}
			</div>

		{:else if currentStep === 2}
			<div class="form-section">
				<h3>{$t('trainNew.modelArchitecture')}</h3>
				<div class="model-grid">
					{#each modelOptions as opt}
						<button class="model-card" class:selected={modelId === opt.value} on:click={() => modelId = opt.value}>
							<span class="model-name">{opt.label}</span>
							<span class="model-desc">{opt.desc}</span>
						</button>
					{/each}
				</div>

				{#if modelId === 'cnn'}
					<h3>{$t('trainNew.cnnInputConfig')}</h3>
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f37">{$t('trainNew.inputChannels')}</label>
							<input id="auto-f37" type="number" bind:value={cnnInputChannels} min="1" max="4" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f38">{$t('trainNew.imageHeight')}</label>
							<input id="auto-f38" type="number" bind:value={cnnInputHeight} min="1" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f39">{$t('trainNew.imageWidth')}</label>
							<input id="auto-f39" type="number" bind:value={cnnInputWidth} min="1" class="input" />
						</div>
					</div>
				{/if}

				<h3>{$t('trainNew.trainingHparams')}</h3>
				<div class="form-row">
					<div class="form-field">
						<label for="auto-f40">{$t('trainNew.epochs')}</label>
						<input id="auto-f40" type="number" bind:value={epochs} min="1" class="input" />
					</div>
					<div class="form-field">
						<label for="auto-f41">{$t('trainNew.batchSize')}</label>
						<input id="auto-f41" type="number" bind:value={batchSize} min="1" class="input" />
					</div>
				</div>

				<div class="form-row">
					<div class="form-field">
						<label for="auto-f42">{$t('trainNew.learningRate')}</label>
						<input id="auto-f42" type="number" bind:value={learningRate} min="0" step="0.0001" class="input" />
					</div>
					<div class="form-field">
						<label for="auto-f43">{$t('trainNew.optimizer')}</label>
						<select id="auto-f43" bind:value={optimizerType} class="input">
							<option value="Adam">Adam</option>
							<option value="AdamW">AdamW</option>
							<option value="Sgd">SGD</option>
						</select>
					</div>
				</div>

				<div class="form-row">
					<div class="form-field">
						<label for="auto-f44">{$t('trainNew.lossFunction')}</label>
						<select id="auto-f44" bind:value={lossFunction} class="input">
							<option value="cross_entropy">Cross Entropy</option>
							<option value="mse">MSE</option>
							<option value="mae">MAE</option>
						</select>
					</div>
					<div class="form-field">
						<label for="auto-f45">{$t('trainNew.computeBackend')}</label>
						<select id="auto-f45" bind:value={computeBackend} class="input">
							<option value="cpu">CPU</option>
							<option value="wgpu">WGPU ({$t('trainNew.gpuGeneral')})</option>
							<option value="metal">Metal (Apple GPU)</option>
							<option value="cuda">CUDA (NVIDIA GPU)</option>
							<option value="rocm">ROCm (AMD GPU)</option>
						</select>
					</div>
				</div>

				<div class="form-field">
					<label for="auto-f46">{$t('trainNew.checkpointInterval')}</label>
					<input id="auto-f46" type="number" bind:value={checkpointInterval} min="1" class="input" placeholder={$t('trainNew.checkpointIntervalPlaceholder')} />
				</div>

				<h3>Early Stopping</h3>
				<div class="form-field">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={enableEarlyStopping} />
						{$t('trainNew.enableEarlyStopping')}
					</label>
				</div>

				{#if enableEarlyStopping}
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f47">{$t('trainNew.monitorMetric')}</label>
							<select id="auto-f47" bind:value={earlyStoppingMetric} class="input">
								<option value="loss">Loss</option>
								<option value="accuracy">Accuracy</option>
							</select>
						</div>
						<div class="form-field">
							<label for="auto-f48">{$t('trainNew.mode')}</label>
							<select id="auto-f48" bind:value={earlyStoppingMode} class="input">
								<option value="min">Min ({$t('trainNew.lowerBetter')})</option>
								<option value="max">Max ({$t('trainNew.higherBetter')})</option>
							</select>
						</div>
					</div>
					<div class="form-row">
						<div class="form-field">
							<label for="auto-f49">{$t('trainNew.patience')}</label>
							<input id="auto-f49" type="number" bind:value={earlyStoppingPatience} min="1" class="input" />
						</div>
						<div class="form-field">
							<label for="auto-f50">{$t('trainNew.minDelta')}</label>
							<input id="auto-f50" type="number" bind:value={earlyStoppingMinDelta} min="0" step="0.0001" class="input" />
						</div>
					</div>
				{/if}
			</div>

		{:else if currentStep === 3}
			<div class="form-section">
				<h3>{$t('trainNew.configConfirm')}</h3>
				<div class="summary-grid">
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.experimentName')}</span>
						<span class="summary-value">{name}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.taskType')}</span>
						<span class="summary-value">{taskTypes.find(t => t.value === taskType)?.label || taskType}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.model')}</span>
						<span class="summary-value">{modelOptions.find(m => m.value === modelId)?.label || modelId}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.dataPath')}</span>
						<span class="summary-value">{dataPath || $t('trainNew.notSpecified')}</span>
					</div>
					{#if targetColumn}
						<div class="summary-item">
							<span class="summary-label">{$t('trainNew.targetCol')}</span>
							<span class="summary-value">{targetColumn}</span>
						</div>
						<div class="summary-item">
							<span class="summary-label">{$t('trainNew.featureColsCount')}</span>
							<span class="summary-value">{featureColumns.length}</span>
						</div>
					{/if}
					{#if modelId === 'cnn'}
						<div class="summary-item">
							<span class="summary-label">{$t('trainNew.cnnInput')}</span>
							<span class="summary-value">{cnnInputChannels}x{cnnInputHeight}x{cnnInputWidth}</span>
						</div>
					{/if}
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.epochs')}</span>
						<span class="summary-value">{epochs}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.batchSize')}</span>
						<span class="summary-value">{batchSize}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.learningRate')}</span>
						<span class="summary-value">{learningRate}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.optimizer')}</span>
						<span class="summary-value">{optimizerType}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.lossFunction')}</span>
						<span class="summary-value">{lossFunction}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">{$t('trainNew.computeBackend')}</span>
						<span class="summary-value">{computeBackend.toUpperCase()}</span>
					</div>
					{#if enableEarlyStopping}
						<div class="summary-item">
							<span class="summary-label">Early Stopping</span>
							<span class="summary-value">{earlyStoppingMetric} {earlyStoppingMode} patience={earlyStoppingPatience}</span>
						</div>
					{/if}
				</div>

				{#if error}
					<div class="error-msg">{error}</div>
				{/if}
			</div>
		{/if}
	</div>

	<div class="actions">
		{#if currentStep > 0}
			<button class="btn btn-secondary" on:click={prevStep}>{$t('trainNew.prevStep')}</button>
		{:else}
			<a href="/lab" class="btn btn-secondary">{$t('trainNew.cancel')}</a>
		{/if}
		<div class="spacer"></div>
		{#if getValidationMessage()}
			<span class="validation-error">{getValidationMessage()}</span>
		{/if}
		{#if currentStep < steps.length - 1}
			<button class="btn btn-primary" disabled={!canProceed()} on:click={nextStep}>{$t('trainNew.nextStep')}</button>
		{:else}
			<button class="btn btn-launch" disabled={submitting} on:click={submit}>
				{submitting ? $t('trainNew.launching') : `🚀 ${$t('trainNew.launchTraining')}`}
			</button>
		{/if}
	</div>
</div>

<style>
	.wizard-page { max-width: 900px; }
	h2 { font-size: 1.5rem; font-weight: 600; color: var(--text-primary, #e5e7eb); margin-bottom: 0.5rem; }
	h3 { font-size: 1rem; font-weight: 600; color: var(--text-primary, #e5e7eb); margin: 1.5rem 0 0.75rem; }
	.desc { color: var(--text-secondary, #9ca3af); margin-bottom: 2rem; }

	.stepper {
		display: flex;
		align-items: center;
		gap: 0;
		margin-bottom: 2rem;
		padding: 1rem 0;
	}
	.step-btn {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0.5rem 0.75rem;
		border-radius: 8px;
		transition: all 0.2s;
	}
	.step-num {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.8rem;
		font-weight: 600;
		background: rgba(107, 114, 128, 0.2);
		color: var(--text-muted, #6b7280);
		transition: all 0.2s;
	}
	.step-label { font-size: 0.85rem; color: var(--text-muted, #6b7280); transition: color 0.2s; }
	.step-btn.active .step-num { background: #10b981; color: #fff; }
	.step-btn.active .step-label { color: var(--text-primary, #e5e7eb); font-weight: 500; }
	.step-btn.done .step-num { background: rgba(16, 185, 129, 0.2); color: #10b981; }
	.step-btn.done .step-label { color: #10b981; }
	.step-line { flex: 1; height: 2px; background: rgba(107, 114, 128, 0.2); margin: 0 0.25rem; }
	.step-line.filled { background: #10b981; }

	.step-content {
		background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
		border: 1px solid rgba(16, 185, 129, 0.1);
		border-radius: 12px;
		padding: 1.5rem 2rem;
		min-height: 300px;
	}

	.input {
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		transition: border-color 0.2s;
	}
	.input:focus { outline: none; border-color: #10b981; }
	select.input { cursor: pointer; }

	.input-with-button {
		display: flex;
		gap: 0.5rem;
	}
	.input-with-button .input {
		flex: 1;
		background: rgba(0, 0, 0, 0.3);
		cursor: pointer;
	}
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
	.btn-browse:hover {
		background: rgba(16, 185, 129, 0.25);
		border-color: #10b981;
	}

	.task-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 0.75rem;
	}
	.task-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 10px;
		cursor: pointer;
		transition: all 0.2s;
		color: var(--text-secondary, #9ca3af);
	}
	.task-card:hover { border-color: rgba(16, 185, 129, 0.3); background: rgba(16, 185, 129, 0.05); }
	.task-card.selected { border-color: #10b981; background: rgba(16, 185, 129, 0.1); color: #10b981; }
	.task-icon { font-size: 1.5rem; }
	.task-label { font-size: 0.85rem; font-weight: 500; }

	.form-row { display: flex; gap: 1rem; margin-top: 0.5rem; }
	.form-field { flex: 1; }
	.form-field label { display: block; font-size: 0.85rem; color: var(--text-secondary, #9ca3af); margin-bottom: 0.4rem; }
	.checkbox-label { display: flex !important; align-items: center; gap: 0.5rem; cursor: pointer; }
	.checkbox-label input { accent-color: #10b981; }

	.summary-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}
	.summary-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.75rem 1rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 8px;
		border: 1px solid rgba(107, 114, 128, 0.15);
	}
	.summary-label { font-size: 0.8rem; color: var(--text-muted, #6b7280); }
	.summary-value { font-size: 0.95rem; color: var(--text-primary, #e5e7eb); font-weight: 500; }

	.error-msg {
		margin-top: 1rem;
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.9rem;
	}

	.actions {
		display: flex;
		align-items: center;
		margin-top: 2rem;
		gap: 1rem;
	}
	.spacer { flex: 1; }
	.btn {
		padding: 0.6rem 1.5rem;
		border-radius: 8px;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		border: none;
		transition: all 0.2s;
		text-decoration: none;
		display: inline-flex;
		align-items: center;
	}
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-secondary {
		background: rgba(107, 114, 128, 0.2);
		color: var(--text-secondary, #9ca3af);
		border: 1px solid rgba(107, 114, 128, 0.3);
	}
	.btn-secondary:hover:not(:disabled) { background: rgba(107, 114, 128, 0.3); }
	.btn-primary {
		background: #10b981;
		color: #fff;
	}
	.btn-primary:hover:not(:disabled) { background: #059669; }
	.btn-launch {
		background: linear-gradient(135deg, #10b981, #059669);
		color: #fff;
		font-weight: 600;
		padding: 0.7rem 2rem;
	}
	.btn-launch:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3); }

	.validation-error {
		color: #ef4444;
		font-size: 0.85rem;
		font-weight: 500;
		padding: 0.4rem 0.8rem;
		background: rgba(239, 68, 68, 0.1);
		border-radius: 6px;
		border: 1px solid rgba(239, 68, 68, 0.2);
	}

	.hint-text { font-size: 0.85rem; color: var(--text-muted, #6b7280); margin-bottom: 0.75rem; }

	.column-grid {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	.column-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.35rem 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 6px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.2s;
	}
	.column-chip:hover { border-color: rgba(16, 185, 129, 0.3); }
	.column-chip.is-target {
		border-color: #f59e0b;
		background: rgba(245, 158, 11, 0.1);
		color: #f59e0b;
	}
	.column-chip.is-feature {
		border-color: #10b981;
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
	}

	.column-summary {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.summary-tag {
		font-size: 0.8rem;
		padding: 0.25rem 0.6rem;
		border-radius: 4px;
	}
	.target-tag {
		background: rgba(245, 158, 11, 0.1);
		color: #f59e0b;
	}
	.feature-tag {
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
	}

	.model-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}
	.model-card {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 10px;
		cursor: pointer;
		transition: all 0.2s;
		color: var(--text-secondary, #9ca3af);
	}
	.model-card:hover { border-color: rgba(16, 185, 129, 0.3); background: rgba(16, 185, 129, 0.05); }
	.model-card.selected { border-color: #10b981; background: rgba(16, 185, 129, 0.1); color: #10b981; }
	.model-name { font-size: 0.95rem; font-weight: 600; }
	.model-desc { font-size: 0.8rem; opacity: 0.7; }

	.source-mode-tabs {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}
	.source-tab {
		flex: 1;
		padding: 0.75rem 1rem;
		background: rgba(107, 114, 128, 0.1);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: center;
	}
	.source-tab:hover { border-color: rgba(16, 185, 129, 0.3); background: rgba(16, 185, 129, 0.05); }
	.source-tab.active { border-color: #10b981; background: rgba(16, 185, 129, 0.1); color: #10b981; font-weight: 600; }

	.empty-datasets {
		text-align: center;
		padding: 2rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 8px;
		border: 1px dashed rgba(107, 114, 128, 0.3);
	}
	.empty-datasets p { color: var(--text-muted, #6b7280); margin-bottom: 0.75rem; }
	.link-btn {
		color: #10b981;
		text-decoration: none;
		font-size: 0.85rem;
		font-weight: 500;
	}
	.link-btn:hover { text-decoration: underline; }

	.dataset-selector {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: 0.75rem;
		max-height: 400px;
		overflow-y: auto;
		padding: 0.25rem;
	}
	.dataset-option {
		padding: 0.75rem 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		color: var(--text-primary, #e5e7eb);
	}
	.dataset-option:hover { border-color: rgba(16, 185, 129, 0.3); background: rgba(16, 185, 129, 0.05); }
	.dataset-option.selected { border-color: #10b981; background: rgba(16, 185, 129, 0.1); }
	.dataset-option-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.4rem;
	}
	.dataset-option-name { font-weight: 600; font-size: 0.9rem; }
	.dataset-option-format {
		font-size: 0.7rem;
		padding: 0.1rem 0.4rem;
		border-radius: 3px;
		background: rgba(96, 165, 250, 0.15);
		color: #60a5fa;
	}
	.dataset-option-meta {
		display: flex;
		gap: 0.75rem;
		font-size: 0.8rem;
		color: var(--text-muted, #6b7280);
	}
	.missing-badge {
		font-size: 0.7rem;
		padding: 0.05rem 0.3rem;
		border-radius: 3px;
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}
	.dataset-option-tags {
		display: flex;
		gap: 0.25rem;
		margin-top: 0.4rem;
	}
	.mini-tag {
		font-size: 0.65rem;
		padding: 0.1rem 0.3rem;
		border-radius: 3px;
		background: rgba(107, 114, 128, 0.15);
		color: var(--text-muted, #6b7280);
	}
</style>
