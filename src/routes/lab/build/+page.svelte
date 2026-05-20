<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getLabClient, pluginStore } from '$lib/lab/stores/plugins';
	import { t } from '$lib/i18n';
	import type { PluginInfo, ModelArchDef, ArchType } from '$lib/lab/adapter/types';

	let loading = true;
	let error: string | null = null;

	let availableModels: PluginInfo[] = [];
	let availableEngines: PluginInfo[] = [];
	let selectedModelId = 'mlp';
	let selectedEngineId = 'burn';

	let modelArch: ModelArchDef | null = null;
	let archLoading = false;

	let taskType: string = 'classification';
	let inputSize = 784;
	let outputSize = 10;
	let hiddenLayers = '128,64';
	let activation = 'relu';
	let dropout = 0.0;

	let cnnInputChannels = 1;
	let cnnInputHeight = 28;
	let cnnInputWidth = 28;

	let customCode = `// Custom model definition
// Implement your model architecture here
fn build_model(input_size: usize, output_size: usize) -> Model {
    Model::new()
        .linear(input_size, 128)
        .relu()
        .linear(128, output_size)
}`;

	onMount(async () => {
		await loadAvailablePlugins();
		loading = false;
	});

	async function loadAvailablePlugins() {
		try {
			await pluginStore.refresh();
			const unsub = pluginStore.subscribe((s) => {
				availableEngines = s.engine || [];
				availableModels = s.model || [];
				if (availableEngines.length > 0 && !availableEngines.find((e) => e.id === selectedEngineId)) {
					selectedEngineId = availableEngines[0].id;
				}
				if (availableModels.length > 0 && !availableModels.find((m) => m.id === selectedModelId)) {
					selectedModelId = availableModels[0].id;
				}
			});
			unsub();
		} catch (e) {
			console.error('Failed to load plugins:', e);
		}
	}

	async function loadModelArch() {
		archLoading = true;
		try {
			const client = getLabClient();
			modelArch = await client.getModelArch(selectedModelId);
		} catch (e: any) {
			modelArch = null;
		} finally {
			archLoading = false;
		}
	}

	function goToTraining() {
		const params = new URLSearchParams({
			engine: selectedEngineId,
			model: selectedModelId,
			task: taskType,
		});
		goto(`/lab/train/new?${params.toString()}`);
	}

	$: archType = selectedModelId.includes('cnn') ? 'cnn' : 'mlp';
	$: estimatedParams = calculateParams();

	function calculateParams(): number {
		if (archType === 'mlp') {
			const layers = hiddenLayers.split(',').map((s) => parseInt(s.trim())).filter((n) => !isNaN(n));
			let total = 0;
			let prevSize = inputSize;
			for (const size of layers) {
				total += prevSize * size + size;
				prevSize = size;
			}
			total += prevSize * outputSize + outputSize;
			return total;
		} else if (archType === 'cnn') {
			const conv1 = cnnInputChannels * 32 * 3 * 3 + 32;
			const conv2 = 32 * 64 * 3 * 3 + 64;
			const fcInput = 64 * Math.floor(cnnInputHeight / 4) * Math.floor(cnnInputWidth / 4);
			const fc = fcInput * 128 + 128;
			const out = 128 * outputSize + outputSize;
			return conv1 + conv2 + fc + out;
		}
		return 0;
	}

	function formatParams(n: number): string {
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
		return n.toString();
	}
</script>

<div class="build-page">
	<h1>{$t('build.title')}</h1>
	<p class="subtitle">{$t('build.subtitle')}</p>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="loading">{$t('build.loading')}</div>
	{:else}
		<div class="build-layout">
			<div class="build-config">
				<section class="config-section">
					<h2>{$t('build.engineAndTask')}</h2>
					<div class="form-grid">
						<div class="form-group">
							<label for="compute-engine">{$t('build.computeEngine')}</label>
							<select id="compute-engine" bind:value={selectedEngineId}>
								{#each availableEngines as engine}
									<option value={engine.id}>{engine.name} v{engine.version}</option>
								{/each}
								{#if availableEngines.length === 0}
									<option value="burn">Burn (Rust)</option>
								{/if}
							</select>
						</div>
						<div class="form-group">
							<label for="task-type">{$t('build.taskType')}</label>
							<select id="task-type" bind:value={taskType}>
								<option value="classification">{$t('build.classification')}</option>
								<option value="regression">{$t('build.regression')}</option>
								<option value="clustering">{$t('build.clustering')}</option>
								<option value="detection">{$t('build.detection')}</option>
								<option value="segmentation">{$t('build.segmentation')}</option>
							</select>
						</div>
					</div>
				</section>

				<section class="config-section">
					<h2>{$t('build.architecture')}</h2>
					<div class="arch-tabs">
						<button class="arch-tab" class:active={archType === 'mlp'} on:click={() => { selectedModelId = 'mlp'; }}>
							<span class="tab-icon">🧠</span> MLP
						</button>
						<button class="arch-tab" class:active={archType === 'cnn'} on:click={() => { selectedModelId = 'cnn'; }}>
							<span class="tab-icon">🖼️</span> CNN
						</button>
						<button class="arch-tab" class:active={archType === 'custom'} on:click={() => { selectedModelId = 'custom'; }}>
							<span class="tab-icon">✨</span> {$t('build.custom')}
						</button>
					</div>

					{#if archType === 'mlp'}
						<div class="form-grid">
							<div class="form-group">
								<label for="input-size">{$t('build.inputSize')}</label>
								<input id="input-size" type="number" bind:value={inputSize} min="1" />
							</div>
							<div class="form-group">
								<label for="output-size">{$t('build.outputSize')}</label>
								<input id="output-size" type="number" bind:value={outputSize} min="1" />
							</div>
							<div class="form-group full-width">
								<label for="hidden-layers">{$t('build.hiddenLayers')}</label>
								<input id="hidden-layers" type="text" bind:value={hiddenLayers} placeholder="128,64,32" />
								<span class="hint">{$t('build.hiddenLayersHint')}</span>
							</div>
							<div class="form-group">
								<label for="activation">{$t('build.activation')}</label>
								<select id="activation" bind:value={activation}>
									<option value="relu">ReLU</option>
									<option value="sigmoid">Sigmoid</option>
									<option value="tanh">Tanh</option>
									<option value="gelu">GELU</option>
									<option value="leaky_relu">Leaky ReLU</option>
									<option value="swish">Swish</option>
								</select>
							</div>
							<div class="form-group">
								<label for="dropout">{$t('build.dropout')}</label>
								<input id="dropout" type="number" bind:value={dropout} min="0" max="0.9" step="0.05" />
							</div>
						</div>
					{:else if archType === 'cnn'}
						<div class="form-grid">
							<div class="form-group">
								<label for="cnn-input-channels">{$t('build.inputChannels')}</label>
								<input id="cnn-input-channels" type="number" bind:value={cnnInputChannels} min="1" />
							</div>
							<div class="form-group">
								<label for="cnn-input-height">{$t('build.inputHeight')}</label>
								<input id="cnn-input-height" type="number" bind:value={cnnInputHeight} min="1" />
							</div>
							<div class="form-group">
								<label for="cnn-input-width">{$t('build.inputWidth')}</label>
								<input id="cnn-input-width" type="number" bind:value={cnnInputWidth} min="1" />
							</div>
							<div class="form-group">
								<label for="cnn-output-size">{$t('build.outputSize')}</label>
								<input id="cnn-output-size" type="number" bind:value={outputSize} min="1" />
							</div>
							<div class="form-group">
								<label for="cnn-activation">{$t('build.activation')}</label>
								<select id="cnn-activation" bind:value={activation}>
									<option value="relu">ReLU</option>
									<option value="sigmoid">Sigmoid</option>
									<option value="tanh">Tanh</option>
									<option value="gelu">GELU</option>
								</select>
							</div>
							<div class="form-group">
								<label for="cnn-dropout">{$t('build.dropout')}</label>
								<input id="cnn-dropout" type="number" bind:value={dropout} min="0" max="0.9" step="0.05" />
							</div>
						</div>
					{:else}
						<div class="custom-editor">
							<textarea bind:value={customCode} rows="16" class="code-editor"></textarea>
						</div>
					{/if}
				</section>

				<div class="action-bar">
					<button class="btn-secondary" on:click={loadModelArch} disabled={archLoading}>
						{archLoading ? $t('build.loadingArch') : $t('build.inspectArch')}
					</button>
					<button class="btn-primary" on:click={goToTraining}>
						{$t('build.goToTraining')} →
					</button>
				</div>
			</div>

			<div class="build-preview">
				<section class="preview-section">
					<h2>{$t('build.modelSummary')}</h2>
					<div class="summary-cards">
						<div class="summary-card">
							<div class="summary-icon">🏗️</div>
							<div class="summary-info">
								<div class="summary-label">{$t('build.archTypeLabel')}</div>
								<div class="summary-value">{archType.toUpperCase()}</div>
							</div>
						</div>
						<div class="summary-card">
							<div class="summary-icon">📊</div>
							<div class="summary-info">
								<div class="summary-label">{$t('build.estimatedParams')}</div>
								<div class="summary-value">{formatParams(estimatedParams)}</div>
							</div>
						</div>
						<div class="summary-card">
							<div class="summary-icon">⚡</div>
							<div class="summary-info">
								<div class="summary-label">{$t('build.engineLabel')}</div>
								<div class="summary-value">{selectedEngineId.toUpperCase()}</div>
							</div>
						</div>
						<div class="summary-card">
							<div class="summary-icon">🎯</div>
							<div class="summary-info">
								<div class="summary-label">{$t('build.taskLabel')}</div>
								<div class="summary-value">{taskType}</div>
							</div>
						</div>
					</div>
				</section>

				{#if archType === 'mlp'}
					<section class="preview-section">
						<h2>{$t('build.layerVisualization')}</h2>
						<div class="layer-viz">
							<div class="layer-node input">
								<div class="layer-label">{$t('build.input')}</div>
								<div class="layer-size">{inputSize}</div>
							</div>
							{#each hiddenLayers.split(',').map((s) => parseInt(s.trim())).filter((n) => !isNaN(n)) as size, i}
								<div class="layer-arrow">→</div>
								<div class="layer-node hidden">
									<div class="layer-label">{activation.toUpperCase()}</div>
									<div class="layer-size">{size}</div>
									{#if dropout > 0}
										<div class="layer-detail">Dropout {dropout}</div>
									{/if}
								</div>
							{/each}
							<div class="layer-arrow">→</div>
							<div class="layer-node output">
								<div class="layer-label">{$t('build.output')}</div>
								<div class="layer-size">{outputSize}</div>
							</div>
						</div>
					</section>
				{:else if archType === 'cnn'}
					<section class="preview-section">
						<h2>{$t('build.layerVisualization')}</h2>
						<div class="layer-viz">
							<div class="layer-node input">
								<div class="layer-label">{$t('build.input')}</div>
								<div class="layer-size">{cnnInputChannels}x{cnnInputHeight}x{cnnInputWidth}</div>
							</div>
							<div class="layer-arrow">→</div>
							<div class="layer-node conv">
								<div class="layer-label">Conv2D</div>
								<div class="layer-size">32 @ 3x3</div>
							</div>
							<div class="layer-arrow">→</div>
							<div class="layer-node conv">
								<div class="layer-label">Conv2D</div>
								<div class="layer-size">64 @ 3x3</div>
							</div>
							<div class="layer-arrow">→</div>
							<div class="layer-node hidden">
								<div class="layer-label">{$t('build.flatten')}</div>
								<div class="layer-size">FC</div>
							</div>
							<div class="layer-arrow">→</div>
							<div class="layer-node output">
								<div class="layer-label">{$t('build.output')}</div>
								<div class="layer-size">{outputSize}</div>
							</div>
						</div>
					</section>
				{/if}

				{#if modelArch}
					<section class="preview-section">
						<h2>{$t('build.archDetail')}</h2>
						<div class="arch-detail">
							<div class="arch-meta">
								<span class="meta-item"><strong>ID:</strong> {modelArch.id}</span>
								<span class="meta-item"><strong>{$t('build.archTypeLabel')}:</strong> {modelArch.arch_type}</span>
								<span class="meta-item"><strong>{$t('build.totalParams')}:</strong> {formatParams(modelArch.total_params)}</span>
							</div>
							<table class="layers-table">
								<thead>
									<tr>
										<th>#</th>
										<th>{$t('build.layerType')}</th>
										<th>{$t('build.layerName')}</th>
										<th>{$t('build.inputShape')}</th>
										<th>{$t('build.outputShape')}</th>
										<th>{$t('build.params')}</th>
									</tr>
								</thead>
								<tbody>
									{#each modelArch.layers as layer, i}
										<tr>
											<td>{i + 1}</td>
											<td>{layer.layer_type}</td>
											<td>{layer.name}</td>
											<td>[{layer.input_shape.dims.join(', ')}]</td>
											<td>[{layer.output_shape.dims.join(', ')}]</td>
											<td>{formatParams(layer.params)}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</section>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.build-page {
		padding: 24px;
		max-width: 1400px;
		margin: 0 auto;
	}

	h1 {
		font-size: 1.75rem;
		font-weight: 700;
		margin-bottom: 4px;
		color: #e5e7eb;
	}

	.subtitle {
		color: #6b7280;
		margin-bottom: 24px;
	}

	.loading {
		text-align: center;
		padding: 60px;
		color: #6b7280;
	}

	.error-banner {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		padding: 12px 16px;
		border-radius: 8px;
		margin-bottom: 16px;
	}

	.build-layout {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 24px;
	}

	.config-section,
	.preview-section {
		background: rgba(30, 41, 59, 0.6);
		border: 1px solid rgba(16, 185, 129, 0.12);
		border-radius: 12px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.config-section h2,
	.preview-section h2 {
		font-size: 1.1rem;
		font-weight: 600;
		margin-bottom: 16px;
		padding-bottom: 8px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		color: #e5e7eb;
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

	.form-group.full-width {
		grid-column: 1 / -1;
	}

	.form-group label {
		font-size: 0.8rem;
		color: #9ca3af;
		font-weight: 500;
	}

	.form-group input,
	.form-group select {
		padding: 8px 12px;
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 6px;
		font-size: 0.9rem;
		background: rgba(15, 23, 42, 0.6);
		color: #e5e7eb;
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: #10b981;
	}

	.hint {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.arch-tabs {
		display: flex;
		gap: 8px;
		margin-bottom: 16px;
	}

	.arch-tab {
		flex: 1;
		padding: 10px 16px;
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 8px;
		background: rgba(15, 23, 42, 0.4);
		color: #9ca3af;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
		transition: all 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
	}

	.arch-tab:hover {
		border-color: rgba(16, 185, 129, 0.3);
		color: #e5e7eb;
	}

	.arch-tab.active {
		background: rgba(16, 185, 129, 0.1);
		border-color: #10b981;
		color: #10b981;
	}

	.tab-icon {
		font-size: 1rem;
	}

	.custom-editor {
		margin-top: 8px;
	}

	.code-editor {
		width: 100%;
		padding: 12px;
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 8px;
		background: rgba(15, 23, 42, 0.8);
		color: #10b981;
		font-family: 'Fira Code', 'JetBrains Mono', monospace;
		font-size: 0.85rem;
		line-height: 1.6;
		resize: vertical;
	}

	.code-editor:focus {
		outline: none;
		border-color: #10b981;
	}

	.action-bar {
		display: flex;
		gap: 12px;
		margin-top: 8px;
	}

	.btn-primary {
		flex: 1;
		padding: 12px 24px;
		background: #10b981;
		color: #fff;
		border: none;
		border-radius: 8px;
		font-size: 0.95rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}

	.btn-primary:hover {
		background: #059669;
	}

	.btn-secondary {
		padding: 12px 24px;
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 8px;
		font-size: 0.95rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-secondary:hover {
		background: rgba(16, 185, 129, 0.15);
	}

	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.summary-cards {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
	}

	.summary-card {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 14px;
		background: rgba(15, 23, 42, 0.5);
		border: 1px solid rgba(75, 85, 99, 0.3);
		border-radius: 8px;
	}

	.summary-icon {
		font-size: 1.5rem;
	}

	.summary-label {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.summary-value {
		font-size: 1.1rem;
		font-weight: 600;
		color: #10b981;
	}

	.layer-viz {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
		padding: 16px 0;
	}

	.layer-node {
		padding: 10px 14px;
		border-radius: 8px;
		text-align: center;
		min-width: 70px;
	}

	.layer-node.input {
		background: rgba(59, 130, 246, 0.15);
		border: 1px solid rgba(59, 130, 246, 0.3);
	}

	.layer-node.hidden,
	.layer-node.conv {
		background: rgba(139, 92, 246, 0.15);
		border: 1px solid rgba(139, 92, 246, 0.3);
	}

	.layer-node.output {
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.3);
	}

	.layer-label {
		font-size: 0.7rem;
		color: #9ca3af;
		margin-bottom: 2px;
	}

	.layer-size {
		font-size: 0.9rem;
		font-weight: 600;
		color: #e5e7eb;
	}

	.layer-detail {
		font-size: 0.65rem;
		color: #6b7280;
		margin-top: 2px;
	}

	.layer-arrow {
		color: #4b5563;
		font-size: 1.2rem;
	}

	.arch-detail {
		overflow-x: auto;
	}

	.arch-meta {
		display: flex;
		gap: 16px;
		margin-bottom: 12px;
		flex-wrap: wrap;
	}

	.meta-item {
		font-size: 0.85rem;
		color: #9ca3af;
	}

	.meta-item strong {
		color: #e5e7eb;
	}

	.layers-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.layers-table th {
		text-align: left;
		padding: 8px 10px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		color: #9ca3af;
		font-weight: 500;
	}

	.layers-table td {
		padding: 6px 10px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: #e5e7eb;
	}

	.layers-table tbody tr:hover {
		background: rgba(16, 185, 129, 0.03);
	}
</style>
