<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';
  import { t } from '$lib/i18n';

  export let datasetId: string = '';

  let activeTab = 'curation';

  let curationSteps: any[] = [];
  let selectedSteps: string[] = [];
  let curationRunning = false;
  let curationResult: any = null;
  let piiText = '';
  let piiResult: any = null;

  let kfoldK = 5;
  let kfoldShuffle = true;
  let kfoldSeed = 42;
  let kfoldResult: any = null;
  let kfoldRunning = false;

  let augmentationPresets: any[] = [];
  let augFormat = 'text';
  let augLoading = false;

  let previewOffset = 0;
  let previewLimit = 20;
  let previewResult: any = null;
  let previewLoading = false;

  let sampleN = 100;
  let sampleSeed = 42;
  let sampleResult: any = null;
  let sampleLoading = false;

  let splitName = 'train';
  let splitOffset = 0;
  let splitLimit = 50;
  let splitResult: any = null;
  let splitLoading = false;

  let columnStatsName = '';
  let columnStatsResult: any = null;
  let columnStatsLoading = false;

  let error: string | null = null;

  async function loadCurationConfig() {
    try {
      const client = getLabClient();
      const config = await client.curationConfig();
      curationSteps = config.available_steps || [];
    } catch (e: any) {
      error = e?.toString() || $t('tools.loadConfigFailed');
    }
  }

  function toggleStep(stepId: string) {
    if (selectedSteps.includes(stepId)) {
      selectedSteps = selectedSteps.filter(s => s !== stepId);
    } else {
      selectedSteps = [...selectedSteps, stepId];
    }
  }

  async function runCuration() {
    if (!datasetId || selectedSteps.length === 0) return;
    curationRunning = true; error = null;
    const taskId = taskManagerStore.createTask($t('tools.dataCuration'), $t('tools.executingSteps', { count: selectedSteps.length }), false);
    try {
      const client = getLabClient();
      curationResult = await client.datasetCuration(datasetId, { steps: selectedSteps });
      taskManagerStore.completeTask(taskId, $t('tools.curationComplete', { rows: curationResult.removed_rows }));
    } catch (e: any) {
      error = e?.toString() || $t('tools.curationFailed');
      taskManagerStore.failTask(taskId, error || $t('tools.unknownError'));
    } finally { curationRunning = false; }
  }

  async function maskPii() {
    if (!piiText) return;
    try {
      const client = getLabClient();
      piiResult = await client.curationMaskPii(piiText);
    } catch (e: any) {
      error = e?.toString() || $t('tools.piiMaskFailed');
    }
  }

  async function createKfold() {
    if (!datasetId) return;
    kfoldRunning = true; error = null;
    const taskId = taskManagerStore.createTask($t('tools.kfoldCreate'), $t('tools.creatingKfold', { k: kfoldK }), false);
    try {
      const client = getLabClient();
      kfoldResult = await client.datasetCreateKfold(datasetId, kfoldK, kfoldShuffle, kfoldSeed);
      taskManagerStore.completeTask(taskId, $t('tools.kfoldComplete', { k: kfoldK }));
    } catch (e: any) {
      error = e?.toString() || $t('tools.createFailed');
      taskManagerStore.failTask(taskId, error || $t('tools.unknownError'));
    } finally { kfoldRunning = false; }
  }

  async function loadAugmentationPresets() {
    augLoading = true;
    try {
      const client = getLabClient();
      const result = await client.datasetListAugmentationPresets(augFormat);
      augmentationPresets = result.presets || [];
    } catch (e: any) {
      error = e?.toString() || $t('tools.loadPresetsFailed');
    } finally { augLoading = false; }
  }

  async function loadPreview() {
    if (!datasetId) return;
    previewLoading = true; error = null;
    try {
      const client = getLabClient();
      previewResult = await client.datasetPreview(datasetId, previewOffset, previewLimit);
    } catch (e: any) {
      error = e?.toString() || $t('tools.previewFailed');
    } finally { previewLoading = false; }
  }

  async function loadSample() {
    if (!datasetId) return;
    sampleLoading = true; error = null;
    try {
      const client = getLabClient();
      sampleResult = await client.datasetSample(datasetId, sampleN, sampleSeed);
    } catch (e: any) {
      error = e?.toString() || $t('tools.sampleFailed');
    } finally { sampleLoading = false; }
  }

  async function loadSplit() {
    if (!datasetId || !splitName) return;
    splitLoading = true; error = null;
    try {
      const client = getLabClient();
      splitResult = await client.datasetReadSplit(datasetId, splitName, splitOffset, splitLimit);
    } catch (e: any) {
      error = e?.toString() || $t('tools.readSplitFailed');
    } finally { splitLoading = false; }
  }

  async function loadColumnStats() {
    if (!datasetId || !columnStatsName) return;
    columnStatsLoading = true; error = null;
    try {
      const client = getLabClient();
      columnStatsResult = await client.datasetColumnStats(datasetId, columnStatsName);
    } catch (e: any) {
      error = e?.toString() || $t('tools.statsFailed');
    } finally { columnStatsLoading = false; }
  }

  $: if (activeTab === 'curation' && curationSteps.length === 0) loadCurationConfig();
  $: if (activeTab === 'augmentation' && augmentationPresets.length === 0) loadAugmentationPresets();
</script>

<div class="advanced-panel">
  <h3>🛠️ {$t('tools.title')}</h3>

  <div class="tab-bar">
    {#each [
      { id: 'curation', label: $t('tools.tabCuration') },
      { id: 'preview', label: $t('tools.tabPreview') },
      { id: 'sample', label: $t('tools.tabSample') },
      { id: 'split', label: $t('tools.tabSplit') },
      { id: 'kfold', label: $t('tools.tabKfold') },
      { id: 'stats', label: $t('tools.tabStats') },
      { id: 'augmentation', label: $t('tools.tabAugmentation') },
    ] as tab}
      <button class="tab-btn" class:active={activeTab === tab.id} on:click={() => (activeTab = tab.id)}>
        {tab.label}
      </button>
    {/each}
  </div>

  {#if activeTab === 'curation'}
    <div class="section">
      <div class="step-list">
        {#each curationSteps as step}
          <label class="step-item" class:selected={selectedSteps.includes(step.id)}>
            <input type="checkbox" checked={selectedSteps.includes(step.id)} on:change={() => toggleStep(step.id)} />
            <div class="step-info">
              <span class="step-name">{step.name}</span>
              <span class="step-desc">{step.description}</span>
            </div>
          </label>
        {/each}
      </div>
      <button class="btn-primary-sm" on:click={runCuration} disabled={curationRunning || selectedSteps.length === 0}>
        {curationRunning ? $t('tools.executing') : $t('tools.executeCuration', { count: selectedSteps.length })}
      </button>

      <details class="pii-section">
        <summary>{$t('tools.piiMaskTest')}</summary>
        <div class="pii-form">
          <textarea class="input textarea" bind:value={piiText} placeholder={$t('tools.piiPlaceholder')}></textarea>
          <button class="btn-sm" on:click={maskPii}>{$t('tools.mask')}</button>
          {#if piiResult}
            <div class="pii-result">
              <div class="pii-original">{piiResult.original}</div>
              <div class="pii-masked">{piiResult.masked}</div>
              <span class="pii-count">{$t('tools.piiFound', { count: piiResult.pii_found })}</span>
            </div>
          {/if}
        </div>
      </details>

      {#if curationResult}
        <div class="result-card">
          <div class="curation-summary">
            <span>{$t('tools.original')}: {curationResult.original_rows.toLocaleString()}</span>
            <span>→ {$t('tools.curated')}: {curationResult.curated_rows.toLocaleString()}</span>
            <span class="removed">{$t('tools.removed')}: {curationResult.removed_rows}</span>
          </div>
          <div class="curation-steps-applied">
            {#each curationResult.steps_applied as step}
              <span class="applied-step">✅ {step}</span>
            {/each}
          </div>
          {#if curationResult.curation_report}
            <div class="curation-report">
              {#each Object.entries(curationResult.curation_report) as [key, val]}
                <span class="report-item">{key}: {val}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'preview'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="preview-offset">{$t('tools.offset')}</label><input id="preview-offset" class="input input-sm" type="number" bind:value={previewOffset} /></div>
        <div class="form-group"><label for="preview-limit">{$t('tools.limit')}</label><input id="preview-limit" class="input input-sm" type="number" bind:value={previewLimit} /></div>
        <button class="btn-primary-sm" on:click={loadPreview} disabled={previewLoading}>{$t('tools.load')}</button>
      </div>
      {#if previewResult}
        <div class="data-table-wrap">
          <table class="data-table">
            <thead><tr>{#each (previewResult.columns || []) as col}<th>{col}</th>{/each}</tr></thead>
            <tbody>
              {#each (previewResult.rows || []) as row}
                <tr>{#each row as cell}<td>{cell}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="table-info">{$t('tools.offset')}: {previewResult.offset} / {$t('tools.totalRows')}: {previewResult.total_rows?.toLocaleString()}</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'sample'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="sample-n">{$t('tools.sampleCount')}</label><input id="sample-n" class="input input-sm" type="number" bind:value={sampleN} /></div>
        <div class="form-group"><label for="sample-seed">{$t('tools.seed')}</label><input id="sample-seed" class="input input-sm" type="number" bind:value={sampleSeed} /></div>
        <button class="btn-primary-sm" on:click={loadSample} disabled={sampleLoading}>🎲 {$t('tools.sample')}</button>
      </div>
      {#if sampleResult}
        <div class="data-table-wrap">
          <table class="data-table">
            <thead><tr>{#each (sampleResult.columns || []) as col}<th>{col}</th>{/each}</tr></thead>
            <tbody>
              {#each (sampleResult.rows || []) as row}
                <tr>{#each row as cell}<td>{cell}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="table-info">{$t('tools.sampleSize')}: {sampleResult.sample_size}</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'split'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="split-name">{$t('tools.splitName')}</label>
          <select id="split-name" class="input input-sm" bind:value={splitName}>
            <option value="train">train</option>
            <option value="val">val</option>
            <option value="test">test</option>
          </select>
        </div>
        <div class="form-group"><label for="split-offset">{$t('tools.offset')}</label><input id="split-offset" class="input input-sm" type="number" bind:value={splitOffset} /></div>
        <div class="form-group"><label for="split-limit">{$t('tools.limit')}</label><input id="split-limit" class="input input-sm" type="number" bind:value={splitLimit} /></div>
        <button class="btn-primary-sm" on:click={loadSplit} disabled={splitLoading}>{$t('tools.read')}</button>
      </div>
      {#if splitResult}
        <div class="data-table-wrap">
          <table class="data-table">
            <thead><tr>{#each (splitResult.columns || []) as col}<th>{col}</th>{/each}</tr></thead>
            <tbody>
              {#each (splitResult.rows || []) as row}
                <tr>{#each row as cell}<td>{cell}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="table-info">{splitResult.split_name}: {splitResult.total_rows?.toLocaleString()} {$t('tools.rows')}</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'kfold'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="kfold-k">{$t('tools.kValue')}</label><input id="kfold-k" class="input input-sm" type="number" bind:value={kfoldK} min="2" max="20" /></div>
        <div class="form-group"><label for="kfold-seed">{$t('tools.seed')}</label><input id="kfold-seed" class="input input-sm" type="number" bind:value={kfoldSeed} /></div>
        <label class="checkbox-label"><input type="checkbox" bind:checked={kfoldShuffle} /> {$t('tools.shuffle')}</label>
        <button class="btn-primary-sm" on:click={createKfold} disabled={kfoldRunning}>🔄 {$t('tools.create')}</button>
      </div>
      {#if kfoldResult}
        <div class="kfold-result">
          <div class="kfold-header">{$t('tools.kfoldCv', { k: kfoldResult.k })}</div>
          <div class="kfold-folds">
            {#each (kfoldResult.folds || []) as fold}
              <div class="kfold-item">
                <span class="fold-id">Fold {fold.fold_id}</span>
                <span class="fold-train">{$t('tools.train')}: {fold.train_indices?.length.toLocaleString()}</span>
                <span class="fold-val">{$t('tools.validation')}: {fold.val_indices?.length.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'stats'}
    <div class="section">
      <div class="form-row">
        <div class="form-group flex-1"><label for="col-stats-name">{$t('tools.columnName')}</label><input id="col-stats-name" class="input" type="text" bind:value={columnStatsName} placeholder="column_name" /></div>
        <button class="btn-primary-sm" on:click={loadColumnStats} disabled={columnStatsLoading || !columnStatsName}>📊 {$t('tools.statistics')}</button>
      </div>
      {#if columnStatsResult}
        <div class="result-card">
          <div class="stats-grid">
            <div class="stat-item"><span class="stat-label">{$t('tools.mean')}</span><span class="stat-val">{columnStatsResult.mean?.toFixed(2)}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.std')}</span><span class="stat-val">{columnStatsResult.std?.toFixed(2)}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.min')}</span><span class="stat-val">{columnStatsResult.min}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.max')}</span><span class="stat-val">{columnStatsResult.max}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.median')}</span><span class="stat-val">{columnStatsResult.median}</span></div>
            <div class="stat-item"><span class="stat-label">Q1</span><span class="stat-val">{columnStatsResult.q1}</span></div>
            <div class="stat-item"><span class="stat-label">Q3</span><span class="stat-val">{columnStatsResult.q3}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.nullValues')}</span><span class="stat-val">{columnStatsResult.null_count}</span></div>
            <div class="stat-item"><span class="stat-label">{$t('tools.uniqueValues')}</span><span class="stat-val">{columnStatsResult.unique_count}</span></div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'augmentation'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="aug-format">{$t('tools.format')}</label>
          <select id="aug-format" class="input input-sm" bind:value={augFormat} on:change={loadAugmentationPresets}>
            <option value="text">{$t('tools.text')}</option>
            <option value="image">{$t('tools.image')}</option>
          </select>
        </div>
        <button class="btn-sm" on:click={loadAugmentationPresets}>🔄</button>
      </div>
      {#if augmentationPresets.length > 0}
        <div class="preset-list">
          {#each augmentationPresets as preset}
            <div class="aug-preset">
              <div class="preset-name">{preset.name}</div>
              <div class="preset-desc">{preset.description}</div>
              <div class="preset-ops">
                {#each preset.operations as op}
                  <span class="op-tag">{op}</span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  {#if error}
    <div class="error-box">{error}</div>
  {/if}
</div>

<style>
  .advanced-panel { padding: 0; }
  .advanced-panel h3 { font-size: 1rem; margin: 0 0 0.5rem; }

  .tab-bar { display: flex; gap: 0.25rem; margin-bottom: 0.6rem; flex-wrap: wrap; }
  .tab-btn { padding: 0.3rem 0.55rem; border: 1px solid rgba(148,163,184,0.15); border-radius: 5px; background: rgba(255,255,255,0.03); color: #9ca3af; font-size: 0.7rem; cursor: pointer; }
  .tab-btn:hover { background: rgba(255,255,255,0.06); color: #d1d5db; }
  .tab-btn.active { background: rgba(59,130,246,0.12); border-color: rgba(59,130,246,0.3); color: #93c5fd; }

  .form-row { display: flex; gap: 0.4rem; align-items: flex-end; margin-bottom: 0.5rem; flex-wrap: wrap; }
  .form-group { margin-bottom: 0; }
  .form-group.flex-1 { flex: 1; min-width: 100px; }
  .form-group label { display: block; font-size: 0.72rem; color: #9ca3af; margin-bottom: 0.15rem; }
  .input { width: 100%; padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.78rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }
  .input-sm { width: 80px; }
  .textarea { min-height: 50px; resize: vertical; font-family: inherit; }
  select.input { appearance: auto; }
  .checkbox-label { display: flex; align-items: center; gap: 0.3rem; font-size: 0.78rem; color: #d1d5db; cursor: pointer; padding-bottom: 0.3rem; }

  .btn-sm { padding: 0.2rem 0.5rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.72rem; cursor: pointer; }
  .btn-sm:hover { background: rgba(255,255,255,0.1); }
  .btn-primary-sm { padding: 0.3rem 0.6rem; border: none; border-radius: 4px; background: #3b82f6; color: #fff; font-size: 0.72rem; font-weight: 600; cursor: pointer; white-space: nowrap; }
  .btn-primary-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .step-list { display: flex; flex-direction: column; gap: 0.3rem; margin-bottom: 0.5rem; }
  .step-item { display: flex; align-items: center; gap: 0.4rem; padding: 0.4rem; border: 1px solid rgba(148,163,184,0.1); border-radius: 6px; cursor: pointer; transition: all 0.15s; }
  .step-item:hover { background: rgba(255,255,255,0.03); }
  .step-item.selected { background: rgba(59,130,246,0.08); border-color: rgba(59,130,246,0.25); }
  .step-item input { accent-color: #3b82f6; }
  .step-info { display: flex; flex-direction: column; }
  .step-name { font-size: 0.8rem; font-weight: 600; color: #d1d5db; }
  .step-desc { font-size: 0.68rem; color: #6b7280; }

  .pii-section { margin-top: 0.5rem; }
  .pii-section summary { font-size: 0.78rem; color: #9ca3af; cursor: pointer; }
  .pii-form { display: flex; flex-direction: column; gap: 0.3rem; margin-top: 0.3rem; }
  .pii-result { padding: 0.3rem; background: rgba(15,23,42,0.5); border-radius: 4px; }
  .pii-original { font-size: 0.72rem; color: #fca5a5; }
  .pii-masked { font-size: 0.72rem; color: #6ee7b7; margin-top: 0.2rem; }
  .pii-count { font-size: 0.65rem; color: #9ca3af; }

  .result-card { padding: 0.5rem; background: rgba(15,23,42,0.5); border: 1px solid rgba(148,163,184,0.1); border-radius: 8px; margin-top: 0.5rem; }
  .curation-summary { display: flex; gap: 0.6rem; font-size: 0.78rem; color: #d1d5db; margin-bottom: 0.3rem; }
  .removed { color: #f59e0b; font-weight: 600; }
  .curation-steps-applied { display: flex; gap: 0.3rem; flex-wrap: wrap; margin-bottom: 0.3rem; }
  .applied-step { font-size: 0.68rem; padding: 0.1rem 0.4rem; border-radius: 3px; background: rgba(16,185,129,0.1); color: #6ee7b7; }
  .curation-report { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .report-item { font-size: 0.68rem; color: #9ca3af; }

  .data-table-wrap { overflow-x: auto; margin-top: 0.3rem; }
  .data-table { width: 100%; border-collapse: collapse; font-size: 0.72rem; }
  .data-table th { padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.05); color: #9ca3af; text-align: left; border-bottom: 1px solid rgba(148,163,184,0.1); }
  .data-table td { padding: 0.25rem 0.5rem; color: #d1d5db; border-bottom: 1px solid rgba(148,163,184,0.05); max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .table-info { font-size: 0.68rem; color: #6b7280; margin-top: 0.3rem; }

  .kfold-result { margin-top: 0.4rem; }
  .kfold-header { font-size: 0.85rem; font-weight: 600; color: #93c5fd; margin-bottom: 0.3rem; }
  .kfold-folds { display: flex; flex-direction: column; gap: 0.2rem; }
  .kfold-item { display: flex; gap: 0.6rem; padding: 0.25rem 0.4rem; background: rgba(255,255,255,0.03); border-radius: 4px; font-size: 0.72rem; }
  .fold-id { font-weight: 600; color: #93c5fd; min-width: 60px; }
  .fold-train { color: #6ee7b7; }
  .fold-val { color: #fbbf24; }

  .stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.3rem; }
  .stat-item { display: flex; flex-direction: column; padding: 0.3rem; background: rgba(255,255,255,0.03); border-radius: 4px; }
  .stat-label { font-size: 0.62rem; color: #6b7280; }
  .stat-val { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }

  .preset-list { display: flex; flex-direction: column; gap: 0.4rem; }
  .aug-preset { padding: 0.4rem; border: 1px solid rgba(148,163,184,0.1); border-radius: 6px; }
  .preset-name { font-size: 0.8rem; font-weight: 600; color: #d1d5db; }
  .preset-desc { font-size: 0.68rem; color: #6b7280; margin-bottom: 0.2rem; }
  .preset-ops { display: flex; gap: 0.2rem; flex-wrap: wrap; }
  .op-tag { font-size: 0.62rem; padding: 0.1rem 0.3rem; border-radius: 3px; background: rgba(139,92,246,0.1); color: #a78bfa; }

  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin-top: 0.5rem; }
</style>
