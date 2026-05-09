<script lang="ts">
	import { onMount } from 'svelte';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { t } from '$lib/i18n';
	import type { AppSettings, HardwareInfo, PluginInfo } from '$lib/lab/adapter/types';

	let settings: AppSettings | null = null;
	let hardware: HardwareInfo | null = null;
	let engines: PluginInfo[] = [];
	let loading = true;
	let saving = false;
	let saved = false;
	let error: string | null = null;
	let activeSection: 'general' | 'training' | 'storage' | 'system' = 'general';

	onMount(async () => {
		const client = getLabClient();
		try {
			const [s, h, e] = await Promise.all([
				client.getSettings(),
				client.getHardwareInfo(),
				client.listEngines(),
			]);
			settings = s;
			hardware = h;
			engines = e;
		} catch (e: any) {
			error = e?.message || $t('settings.loadFailed');
		} finally {
			loading = false;
		}
	});

	async function save() {
		if (!settings) return;
		saving = true;
		saved = false;
		error = null;
		try {
			const client = getLabClient();
			await client.saveSettings(settings);
			saved = true;
			setTimeout(() => { saved = false; }, 2000);
		} catch (e: any) {
			error = e?.message || $t('settings.saveFailed');
		} finally {
			saving = false;
		}
	}

	async function selectDirectory(field: 'data_directory' | 'model_directory' | 'checkpoint_directory') {
		if (!settings) return;
		const client = getLabClient();
		const path = await client.selectDirectory();
		if (path) {
			settings.storage[field] = path;
			settings = settings;
		}
	}

	function formatMemory(mb: number): string {
		if (mb >= 1024) return (mb / 1024).toFixed(1) + ' GB';
		return mb + ' MB';
	}
</script>

<div class="settings-page">
	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>{$t('settings.loadingConfig')}</p>
		</div>
	{:else if settings}
		<div class="settings-header">
			<h2>{$t('settings.title')}</h2>
			<div class="header-actions">
				{#if saved}
					<span class="saved-badge">✓ {$t('settings.saved')}</span>
				{/if}
				<button class="save-btn" on:click={save} disabled={saving}>
					{saving ? $t('settings.saving') : $t('settings.saveConfig')}
				</button>
			</div>
		</div>

		{#if error}
			<div class="error-banner">{error}</div>
		{/if}

		<div class="settings-layout">
			<nav class="settings-nav">
				<button class="nav-item" class:active={activeSection === 'general'} on:click={() => activeSection = 'general'}>
					<span class="nav-icon">🌐</span> {$t('settings.general')}
				</button>
				<button class="nav-item" class:active={activeSection === 'training'} on:click={() => activeSection = 'training'}>
					<span class="nav-icon">🚀</span> {$t('settings.training')}
				</button>
				<button class="nav-item" class:active={activeSection === 'storage'} on:click={() => activeSection = 'storage'}>
					<span class="nav-icon">💾</span> {$t('settings.storage')}
				</button>
				<button class="nav-item" class:active={activeSection === 'system'} on:click={() => activeSection = 'system'}>
					<span class="nav-icon">🖥️</span> {$t('settings.system')}
				</button>
			</nav>

			<div class="settings-content">
				{#if activeSection === 'general'}
					<div class="section">
						<h3>{$t('settings.generalSettings')}</h3>
						<div class="form-group">
							<label for="auto-f66">{$t('settings.language')}</label>
							<select id="auto-f66" bind:value={settings.general.language} class="input">
								<option value="zh-CN">简体中文</option>
								<option value="en">English</option>
							</select>
						</div>
						<div class="form-group">
							<label for="auto-f67">{$t('settings.theme')}</label>
							<select id="auto-f67" bind:value={settings.general.theme} class="input">
								<option value="dark">{$t('settings.darkTheme')}</option>
								<option value="light">{$t('settings.lightTheme')}</option>
							</select>
						</div>
						<div class="form-group">
							<label for="auto-f68">{$t('settings.logLevel')}</label>
							<select id="auto-f68" bind:value={settings.general.log_level} class="input">
								<option value="trace">Trace</option>
								<option value="debug">Debug</option>
								<option value="info">Info</option>
								<option value="warn">Warn</option>
								<option value="error">Error</option>
							</select>
						</div>
						<div class="form-group">
							<label for="auto-f69">{$t('settings.autoRefreshInterval')}</label>
							<input id="auto-f69" type="number" bind:value={settings.general.auto_refresh_interval} min="1" max="60" class="input" />
							<span class="hint">{$t('settings.autoRefreshHint')}</span>
						</div>
					</div>
				{:else if activeSection === 'training'}
					<div class="section">
						<h3>{$t('settings.trainingSettings')}</h3>
						<div class="form-group">
							<label for="auto-f70">{$t('settings.defaultComputeBackend')}</label>
							<select id="auto-f70" bind:value={settings.training.default_compute_backend} class="input">
								<option value="cpu">CPU</option>
								<option value="cuda">CUDA (NVIDIA GPU)</option>
								<option value="wgpu">WGPU</option>
								<option value="metal">Metal (Apple GPU)</option>
								<option value="rocm">ROCm (AMD GPU)</option>
							</select>
							<span class="hint">{$t('settings.defaultBackendHint')}</span>
						</div>
						<div class="form-group">
							<label for="auto-f71">{$t('settings.defaultEngine')}</label>
							<select id="auto-f71" bind:value={settings.training.default_engine} class="input">
								{#each engines as engine}
									<option value={engine.id}>{engine.name}</option>
								{/each}
							</select>
						</div>
						<div class="form-group">
							<label for="auto-f72">{$t('settings.maxConcurrentExperiments')}</label>
							<input id="auto-f72" type="number" bind:value={settings.training.max_concurrent_experiments} min="1" max="8" class="input" />
							<span class="hint">{$t('settings.maxConcurrentHint')}</span>
						</div>
						<div class="form-group">
							<label class="checkbox-label">
								<input type="checkbox" bind:checked={settings.training.auto_checkpoint} />
								{$t('settings.autoCheckpoint')}
							</label>
							<span class="hint">{$t('settings.autoCheckpointHint')}</span>
						</div>
						{#if settings.training.auto_checkpoint}
							<div class="form-group">
								<label for="auto-f73">{$t('settings.checkpointInterval')}</label>
								<input id="auto-f73" type="number" bind:value={settings.training.checkpoint_interval} min="1" class="input" />
							</div>
						{/if}
					</div>
				{:else if activeSection === 'storage'}
					<div class="section">
						<h3>{$t('settings.storageSettings')}</h3>
						<div class="form-group">
							<label for="data-dir">{$t('settings.dataDirectory')}</label>
							<div class="path-input">
								<input id="data-dir" type="text" bind:value={settings.storage.data_directory} class="input" />
								<button class="browse-btn" on:click={() => selectDirectory('data_directory')}>{$t('settings.browse')}</button>
							</div>
							<span class="hint">{$t('settings.dataDirHint')}</span>
						</div>
						<div class="form-group">
							<label for="model-dir">{$t('settings.modelDirectory')}</label>
							<div class="path-input">
								<input id="model-dir" type="text" bind:value={settings.storage.model_directory} class="input" />
								<button class="browse-btn" on:click={() => selectDirectory('model_directory')}>{$t('settings.browse')}</button>
							</div>
							<span class="hint">{$t('settings.modelDirHint')}</span>
						</div>
						<div class="form-group">
							<label for="ckpt-dir">{$t('settings.checkpointDirectory')}</label>
							<div class="path-input">
								<input id="ckpt-dir" type="text" bind:value={settings.storage.checkpoint_directory} class="input" />
								<button class="browse-btn" on:click={() => selectDirectory('checkpoint_directory')}>{$t('settings.browse')}</button>
							</div>
							<span class="hint">{$t('settings.checkpointDirHint')}</span>
						</div>
						<div class="form-group">
							<label for="auto-f74">{$t('settings.maxStorage')}</label>
							<input id="auto-f74" type="number" bind:value={settings.storage.max_storage_gb} min="1" step="1" class="input" />
							<span class="hint">{$t('settings.maxStorageHint')}</span>
						</div>
					</div>
				{:else if activeSection === 'system'}
					<div class="section">
						<h3>{$t('settings.systemInfo')}</h3>
						{#if hardware}
							<div class="info-grid">
								<div class="info-card">
									<div class="info-icon">🖥️</div>
									<div class="info-body">
										<h4>{$t('settings.processor')}</h4>
										<p class="info-value">{hardware.cpu_model}</p>
										<p class="info-detail">{hardware.cpu_cores} {$t('settings.cores')}</p>
									</div>
								</div>
								<div class="info-card">
									<div class="info-icon">💾</div>
									<div class="info-body">
										<h4>{$t('settings.memory')}</h4>
										<p class="info-value">{formatMemory(hardware.total_memory_mb)}</p>
										<p class="info-detail">{$t('settings.available')} {formatMemory(hardware.available_memory_mb)}</p>
									</div>
								</div>
								<div class="info-card">
									<div class="info-icon">💻</div>
									<div class="info-body">
										<h4>{$t('settings.os')}</h4>
										<p class="info-value">{hardware.os_name}</p>
										<p class="info-detail">{hardware.os_version}</p>
									</div>
								</div>
								{#if hardware.gpu_devices.length > 0}
									{#each hardware.gpu_devices as gpu, i}
										<div class="info-card gpu">
											<div class="info-icon">🎮</div>
											<div class="info-body">
												<h4>GPU {i + 1}</h4>
												<p class="info-value">{gpu.name}</p>
												<p class="info-detail">{$t('settings.vram')} {formatMemory(gpu.vram_mb)} | {gpu.compute_backend}</p>
											</div>
										</div>
									{/each}
								{:else}
									<div class="info-card">
										<div class="info-icon">🎮</div>
										<div class="info-body">
											<h4>GPU</h4>
											<p class="info-value">{$t('settings.notDetected')}</p>
											<p class="info-detail">{$t('settings.willUseCpu')}</p>
										</div>
									</div>
								{/if}
							</div>

							<div class="plugin-section">
								<h4>{$t('settings.registeredPlugins')}</h4>
								<div class="plugin-list">
									{#each engines as engine}
										<div class="plugin-item">
											<span class="plugin-name">{engine.name}</span>
											<span class="plugin-version">v{engine.version}</span>
											<span class="plugin-desc">{engine.description}</span>
										</div>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.settings-page {
		max-width: 1200px;
		margin: 0 auto;
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

	.settings-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.saved-badge {
		color: #10b981;
		font-size: 0.9rem;
		font-weight: 500;
	}

	.save-btn {
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.5rem 1.5rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.save-btn:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.save-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
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

	.settings-layout {
		display: flex;
		gap: 2rem;
	}

	.settings-nav {
		width: 200px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: transparent;
		border: none;
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.nav-item:hover {
		background: rgba(255, 255, 255, 0.05);
		color: var(--text-primary, #e5e7eb);
	}

	.nav-item.active {
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
		font-weight: 500;
	}

	.nav-icon {
		font-size: 1.1rem;
	}

	.settings-content {
		flex: 1;
		min-width: 0;
	}

	.section {
		background: linear-gradient(135deg, rgba(26, 26, 46, 0.5), rgba(22, 33, 62, 0.5));
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.5rem;
	}

	h3 {
		font-size: 1.15rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1.5rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	.form-group label {
		display: block;
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.4rem;
		font-weight: 500;
	}

	.checkbox-label {
		display: flex !important;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.checkbox-label input[type="checkbox"] {
		width: 16px;
		height: 16px;
		accent-color: #10b981;
	}

	.input {
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		transition: border-color 0.2s;
		box-sizing: border-box;
	}

	.input:focus {
		border-color: #10b981;
	}

	select.input {
		cursor: pointer;
	}

	.hint {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		margin-top: 0.3rem;
	}

	.path-input {
		display: flex;
		gap: 0.5rem;
	}

	.path-input .input {
		flex: 1;
	}

	.browse-btn {
		padding: 0.6rem 1rem;
		background: rgba(16, 185, 129, 0.1);
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 8px;
		color: #10b981;
		cursor: pointer;
		font-size: 0.85rem;
		white-space: nowrap;
		transition: all 0.2s;
	}

	.browse-btn:hover {
		background: rgba(16, 185, 129, 0.2);
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.info-card {
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 1rem;
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
	}

	.info-card.gpu {
		border-color: rgba(16, 185, 129, 0.2);
	}

	.info-icon {
		font-size: 1.5rem;
		flex-shrink: 0;
	}

	.info-body h4 {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		font-weight: 500;
		margin-bottom: 0.25rem;
	}

	.info-value {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.95rem;
		font-weight: 500;
	}

	.info-detail {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
		margin-top: 0.15rem;
	}

	.plugin-section {
		margin-top: 1.5rem;
	}

	.plugin-section h4 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1rem;
	}

	.plugin-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.plugin-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 8px;
	}

	.plugin-name {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		font-weight: 500;
	}

	.plugin-version {
		color: #10b981;
		font-size: 0.8rem;
		background: rgba(16, 185, 129, 0.1);
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
	}

	.plugin-desc {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
		flex: 1;
		text-align: right;
	}
</style>
