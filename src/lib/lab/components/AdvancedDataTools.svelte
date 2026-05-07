<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';

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
      error = e?.toString() || '加载配置失败';
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
    const taskId = taskManagerStore.createTask('数据策展', `正在执行 ${selectedSteps.length} 个步骤...`, false);
    try {
      const client = getLabClient();
      curationResult = await client.datasetCuration(datasetId, { steps: selectedSteps });
      taskManagerStore.completeTask(taskId, `策展完成，移除 ${curationResult.removed_rows} 行`);
    } catch (e: any) {
      error = e?.toString() || '策展失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { curationRunning = false; }
  }

  async function maskPii() {
    if (!piiText) return;
    try {
      const client = getLabClient();
      piiResult = await client.curationMaskPii(piiText);
    } catch (e: any) {
      error = e?.toString() || 'PII脱敏失败';
    }
  }

  async function createKfold() {
    if (!datasetId) return;
    kfoldRunning = true; error = null;
    const taskId = taskManagerStore.createTask('K-Fold 创建', `创建 ${kfoldK}-Fold...`, false);
    try {
      const client = getLabClient();
      kfoldResult = await client.datasetCreateKfold(datasetId, kfoldK, kfoldShuffle, kfoldSeed);
      taskManagerStore.completeTask(taskId, `${kfoldK}-Fold 创建完成`);
    } catch (e: any) {
      error = e?.toString() || '创建失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { kfoldRunning = false; }
  }

  async function loadAugmentationPresets() {
    augLoading = true;
    try {
      const client = getLabClient();
      const result = await client.datasetListAugmentationPresets(augFormat);
      augmentationPresets = result.presets || [];
    } catch (e: any) {
      error = e?.toString() || '加载预设失败';
    } finally { augLoading = false; }
  }

  async function loadPreview() {
    if (!datasetId) return;
    previewLoading = true; error = null;
    try {
      const client = getLabClient();
      previewResult = await client.datasetPreview(datasetId, previewOffset, previewLimit);
    } catch (e: any) {
      error = e?.toString() || '预览失败';
    } finally { previewLoading = false; }
  }

  async function loadSample() {
    if (!datasetId) return;
    sampleLoading = true; error = null;
    try {
      const client = getLabClient();
      sampleResult = await client.datasetSample(datasetId, sampleN, sampleSeed);
    } catch (e: any) {
      error = e?.toString() || '采样失败';
    } finally { sampleLoading = false; }
  }

  async function loadSplit() {
    if (!datasetId || !splitName) return;
    splitLoading = true; error = null;
    try {
      const client = getLabClient();
      splitResult = await client.datasetReadSplit(datasetId, splitName, splitOffset, splitLimit);
    } catch (e: any) {
      error = e?.toString() || '读取划分失败';
    } finally { splitLoading = false; }
  }

  async function loadColumnStats() {
    if (!datasetId || !columnStatsName) return;
    columnStatsLoading = true; error = null;
    try {
      const client = getLabClient();
      columnStatsResult = await client.datasetColumnStats(datasetId, columnStatsName);
    } catch (e: any) {
      error = e?.toString() || '统计失败';
    } finally { columnStatsLoading = false; }
  }

  $: if (activeTab === 'curation' && curationSteps.length === 0) loadCurationConfig();
  $: if (activeTab === 'augmentation' && augmentationPresets.length === 0) loadAugmentationPresets();
</script>

<div class="advanced-panel">
  <h3>🛠️ 高级数据工具</h3>

  <div class="tab-bar">
    {#each [
      { id: 'curation', label: '🧹 策展' },
      { id: 'preview', label: '👁️ 预览' },
      { id: 'sample', label: '🎲 采样' },
      { id: 'split', label: '✂️ 划分' },
      { id: 'kfold', label: '🔄 K-Fold' },
      { id: 'stats', label: '📊 统计' },
      { id: 'augmentation', label: '✨ 增强' },
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
        {curationRunning ? '执行中...' : `🧹 执行策展 (${selectedSteps.length} 步)`}
      </button>

      <details class="pii-section">
        <summary>PII 脱敏测试</summary>
        <div class="pii-form">
          <textarea class="input textarea" bind:value={piiText} placeholder="输入包含敏感信息的文本..."></textarea>
          <button class="btn-sm" on:click={maskPii}>脱敏</button>
          {#if piiResult}
            <div class="pii-result">
              <div class="pii-original">{piiResult.original}</div>
              <div class="pii-masked">{piiResult.masked}</div>
              <span class="pii-count">发现 {piiResult.pii_found} 处</span>
            </div>
          {/if}
        </div>
      </details>

      {#if curationResult}
        <div class="result-card">
          <div class="curation-summary">
            <span>原始: {curationResult.original_rows.toLocaleString()}</span>
            <span>→ 策展后: {curationResult.curated_rows.toLocaleString()}</span>
            <span class="removed">移除: {curationResult.removed_rows}</span>
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
        <div class="form-group"><label for="preview-offset">偏移</label><input id="preview-offset" class="input input-sm" type="number" bind:value={previewOffset} /></div>
        <div class="form-group"><label for="preview-limit">限制</label><input id="preview-limit" class="input input-sm" type="number" bind:value={previewLimit} /></div>
        <button class="btn-primary-sm" on:click={loadPreview} disabled={previewLoading}>加载</button>
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
        <div class="table-info">偏移: {previewResult.offset} / 总行数: {previewResult.total_rows?.toLocaleString()}</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'sample'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="sample-n">样本数</label><input id="sample-n" class="input input-sm" type="number" bind:value={sampleN} /></div>
        <div class="form-group"><label for="sample-seed">种子</label><input id="sample-seed" class="input input-sm" type="number" bind:value={sampleSeed} /></div>
        <button class="btn-primary-sm" on:click={loadSample} disabled={sampleLoading}>🎲 采样</button>
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
        <div class="table-info">样本数: {sampleResult.sample_size}</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'split'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="split-name">划分名</label>
          <select id="split-name" class="input input-sm" bind:value={splitName}>
            <option value="train">train</option>
            <option value="val">val</option>
            <option value="test">test</option>
          </select>
        </div>
        <div class="form-group"><label for="split-offset">偏移</label><input id="split-offset" class="input input-sm" type="number" bind:value={splitOffset} /></div>
        <div class="form-group"><label for="split-limit">限制</label><input id="split-limit" class="input input-sm" type="number" bind:value={splitLimit} /></div>
        <button class="btn-primary-sm" on:click={loadSplit} disabled={splitLoading}>读取</button>
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
        <div class="table-info">{splitResult.split_name}: {splitResult.total_rows?.toLocaleString()} 行</div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'kfold'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="kfold-k">K 值</label><input id="kfold-k" class="input input-sm" type="number" bind:value={kfoldK} min="2" max="20" /></div>
        <div class="form-group"><label for="kfold-seed">种子</label><input id="kfold-seed" class="input input-sm" type="number" bind:value={kfoldSeed} /></div>
        <label class="checkbox-label"><input type="checkbox" bind:checked={kfoldShuffle} /> 打乱</label>
        <button class="btn-primary-sm" on:click={createKfold} disabled={kfoldRunning}>🔄 创建</button>
      </div>
      {#if kfoldResult}
        <div class="kfold-result">
          <div class="kfold-header">{kfoldResult.k}-Fold 交叉验证</div>
          <div class="kfold-folds">
            {#each (kfoldResult.folds || []) as fold}
              <div class="kfold-item">
                <span class="fold-id">Fold {fold.fold_id}</span>
                <span class="fold-train">训练: {fold.train_indices?.length.toLocaleString()}</span>
                <span class="fold-val">验证: {fold.val_indices?.length.toLocaleString()}</span>
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
        <div class="form-group flex-1"><label for="col-stats-name">列名</label><input id="col-stats-name" class="input" type="text" bind:value={columnStatsName} placeholder="column_name" /></div>
        <button class="btn-primary-sm" on:click={loadColumnStats} disabled={columnStatsLoading || !columnStatsName}>📊 统计</button>
      </div>
      {#if columnStatsResult}
        <div class="result-card">
          <div class="stats-grid">
            <div class="stat-item"><span class="stat-label">均值</span><span class="stat-val">{columnStatsResult.mean?.toFixed(2)}</span></div>
            <div class="stat-item"><span class="stat-label">标准差</span><span class="stat-val">{columnStatsResult.std?.toFixed(2)}</span></div>
            <div class="stat-item"><span class="stat-label">最小值</span><span class="stat-val">{columnStatsResult.min}</span></div>
            <div class="stat-item"><span class="stat-label">最大值</span><span class="stat-val">{columnStatsResult.max}</span></div>
            <div class="stat-item"><span class="stat-label">中位数</span><span class="stat-val">{columnStatsResult.median}</span></div>
            <div class="stat-item"><span class="stat-label">Q1</span><span class="stat-val">{columnStatsResult.q1}</span></div>
            <div class="stat-item"><span class="stat-label">Q3</span><span class="stat-val">{columnStatsResult.q3}</span></div>
            <div class="stat-item"><span class="stat-label">空值</span><span class="stat-val">{columnStatsResult.null_count}</span></div>
            <div class="stat-item"><span class="stat-label">唯一值</span><span class="stat-val">{columnStatsResult.unique_count}</span></div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if activeTab === 'augmentation'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="aug-format">格式</label>
          <select id="aug-format" class="input input-sm" bind:value={augFormat} on:change={loadAugmentationPresets}>
            <option value="text">文本</option>
            <option value="image">图像</option>
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
