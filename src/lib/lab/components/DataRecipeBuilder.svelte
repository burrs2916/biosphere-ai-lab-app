<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';

  export let availableDatasets: any[] = [];

  let recipeName = '';
  let recipeDescription = '';
  let recipeEntries: { dataset_id: string; name: string; weight: number; max_samples: number | null }[] = [];
  let mixingStrategy = 'Proportional';
  let curriculumEnabled = false;
  let curriculumPacing = 'Linear';
  let curriculumWarmup = 1000;
  let curriculumTotalSteps = 100000;
  let dynamicRatioEnabled = false;
  let ratioSchedule = 'Cosine';
  let totalSamplesTarget: number | null = null;
  let seed = 42;
  let qualityMinScore = 0;
  let qualityMaxToxicity = 1.0;
  let qualityMaxRepetition = 1.0;

  let executing = false;
  let executeResult: any = null;
  let executeError: string | null = null;
  let validationError: string | null = null;

  let showPresets = false;

  $: datasetOptions = availableDatasets.map((d: any) => ({ id: d.id, name: d.name }));

  const presets = [
    { name: 'LLM 预训练', desc: '70%代码 + 20%数学 + 10%通用', entries: [
      { weight: 0.7, tag: 'code' }, { weight: 0.2, tag: 'math' }, { weight: 0.1, tag: 'general' }
    ], strategy: 'Proportional', curriculum: true },
    { name: 'SFT 微调', desc: '指令数据 + 对话数据', entries: [
      { weight: 0.6, tag: 'instruction' }, { weight: 0.4, tag: 'dialogue' }
    ], strategy: 'Interleaved', curriculum: false },
    { name: 'RLHF 偏好', desc: '提示 + 选择 + 拒绝', entries: [
      { weight: 0.34, tag: 'prompt' }, { weight: 0.33, tag: 'chosen' }, { weight: 0.33, tag: 'rejected' }
    ], strategy: 'Staged', curriculum: false },
  ];

  function addEntry() {
    const firstAvailable = datasetOptions.length > 0 ? datasetOptions[0] : null;
    recipeEntries.push({
      dataset_id: firstAvailable?.id || '',
      name: firstAvailable?.name || '',
      weight: 1.0,
      max_samples: null,
    });
    recipeEntries = recipeEntries;
  }

  function removeEntry(index: number) {
    recipeEntries.splice(index, 1);
    recipeEntries = recipeEntries;
  }

  function normalizeWeights() {
    const total = recipeEntries.reduce((s, e) => s + e.weight, 0);
    if (total > 0) {
      recipeEntries = recipeEntries.map(e => ({ ...e, weight: e.weight / total }));
    }
  }

  $: totalWeight = recipeEntries.reduce((s, e) => s + e.weight, 0);
  $: weightWarning = totalWeight > 0 && Math.abs(totalWeight - 1.0) > 0.01;

  function applyPreset(preset: typeof presets[0]) {
    recipeName = preset.name;
    recipeEntries = preset.entries.map(e => ({
      dataset_id: '', name: e.tag, weight: e.weight, max_samples: null
    }));
    mixingStrategy = preset.strategy;
    curriculumEnabled = preset.curriculum;
  }

  function buildRecipeJson(): string {
    return JSON.stringify({
      name: recipeName || 'unnamed_recipe',
      version: '1.0',
      description: recipeDescription || null,
      datasets: recipeEntries.map(e => ({
        name: e.name || e.dataset_id,
        weight: e.weight,
        max_samples: e.max_samples,
        tags: [],
      })),
      mixing_strategy: mixingStrategy === 'Interleaved'
        ? { Interleaved: { samples_per_dataset: 5 } }
        : mixingStrategy === 'Staged'
        ? { Staged: { stages: [{ name: 'main', start_step: 0, end_step: curriculumTotalSteps, dataset_weights: Object.fromEntries(recipeEntries.map(e => [e.name, e.weight])) }] } }
        : mixingStrategy,
      curriculum: curriculumEnabled ? {
        enabled: true,
        difficulty_metric: 'Perplexity',
        pacing: curriculumPacing,
        initial_difficulty: 0.0,
        final_difficulty: 1.0,
        warmup_steps: curriculumWarmup,
        total_steps: curriculumTotalSteps,
      } : null,
      dynamic_ratio: dynamicRatioEnabled ? {
        enabled: true,
        schedule: ratioSchedule,
      } : null,
      quality_thresholds: {
        min_overall_score: qualityMinScore,
        max_toxicity: qualityMaxToxicity,
        min_language_confidence: 0.5,
        max_repetition_ratio: qualityMaxRepetition,
        require_no_pii: false,
      },
      total_samples_target: totalSamplesTarget,
      seed,
    });
  }

  async function validateRecipe() {
    validationError = null;
    try {
      const client = getLabClient();
      const result = await client.dataRecipeValidate(buildRecipeJson());
      validationError = null;
    } catch (e: any) {
      validationError = e?.toString() || '验证失败';
    }
  }

  async function executeRecipe() {
    executing = true;
    executeError = null;
    executeResult = null;

    const taskId = taskManagerStore.createTask(
      '数据配方执行',
      `正在执行配方 "${recipeName}"...`,
      false
    );

    try {
      const client = getLabClient();
      const result = await client.dataRecipeExecute(buildRecipeJson(), totalSamplesTarget || undefined);
      executeResult = JSON.parse(result);
      taskManagerStore.completeTask(taskId, `配方执行完成，生成 ${executeResult.samples_yielded || 0} 个样本`);
    } catch (e: any) {
      executeError = e?.toString() || '执行失败';
      taskManagerStore.failTask(taskId, executeError || '未知错误');
    } finally {
      executing = false;
    }
  }
</script>

<div class="recipe-builder">
  <div class="recipe-header">
    <h3>🧪 数据配方构建器</h3>
    <button class="btn-link" on:click={() => (showPresets = !showPresets)}>
      {showPresets ? '收起预设' : '📋 加载预设'}
    </button>
  </div>

  {#if showPresets}
    <div class="presets-row">
      {#each presets as preset}
        <button class="preset-card" on:click={() => applyPreset(preset)}>
          <div class="preset-name">{preset.name}</div>
          <div class="preset-desc">{preset.desc}</div>
        </button>
      {/each}
    </div>
  {/if}

  <div class="form-row">
    <div class="form-group flex-1">
      <label for="recipe-name">配方名称</label>
      <input id="recipe-name" class="input" type="text" bind:value={recipeName} placeholder="my_recipe" />
    </div>
    <div class="form-group" style="width:100px">
      <label for="recipe-seed">随机种子</label>
      <input id="recipe-seed" class="input" type="number" bind:value={seed} />
    </div>
  </div>

  <div class="form-group">
    <label for="recipe-desc">描述</label>
    <input id="recipe-desc" class="input" type="text" bind:value={recipeDescription} placeholder="配方描述..." />
  </div>

  <div class="section-header">
    <h4>数据集混合</h4>
    <button class="btn-sm btn-primary-sm" on:click={addEntry}>+ 添加数据集</button>
  </div>

  {#if recipeEntries.length > 0}
    <div class="entries-list">
      {#each recipeEntries as entry, i}
        <div class="entry-row">
          {#if datasetOptions.length > 0}
            <select class="input entry-select" on:change={(e) => { const opt = datasetOptions.find((d: any) => d.id === (e.target as HTMLSelectElement).value); if (opt) { entry.dataset_id = opt.id; entry.name = opt.name; } }}>
              <option value="">选择数据集</option>
              {#each datasetOptions as opt}
                <option value={opt.id} selected={entry.dataset_id === opt.id}>{opt.name}</option>
              {/each}
            </select>
          {:else}
            <input class="input entry-name" type="text" bind:value={entry.name} placeholder="数据集名称/标签" />
          {/if}
          <div class="weight-group">
            <input type="range" min="0" max="1" step="0.01" bind:value={entry.weight} />
            <span class="weight-val">{(entry.weight * 100).toFixed(0)}%</span>
          </div>
          <input class="input input-sm" type="number" bind:value={entry.max_samples} placeholder="最大样本" />
          <button class="btn-remove" on:click={() => removeEntry(i)}>✕</button>
        </div>
      {/each}
    </div>

    <div class="weight-bar">
      {#each recipeEntries as entry, i}
        <div class="weight-seg" style="width: {totalWeight > 0 ? (entry.weight / totalWeight * 100) : 0}%; background: hsl({i * 137}, 70%, 55%)"></div>
      {/each}
    </div>
    {#if weightWarning}
      <div class="weight-warning">⚠️ 权重总和 = {totalWeight.toFixed(2)}，建议归一化到 1.0</div>
    {/if}
    <button class="btn-sm" on:click={normalizeWeights}>归一化权重</button>
  {:else}
    <div class="empty-entries">点击"添加数据集"开始构建配方</div>
  {/if}

  <div class="section-header"><h4>混合策略</h4></div>
  <div class="strategy-options">
    {#each ['Proportional', 'Interleaved', 'Staged'] as strategy}
      <label class="radio-label">
        <input type="radio" name="strategy" bind:group={mixingStrategy} value={strategy} />
        <span>{strategy === 'Proportional' ? '比例混合' : strategy === 'Interleaved' ? '交错混合' : '分阶段混合'}</span>
      </label>
    {/each}
  </div>

  <details class="advanced-section">
    <summary>高级配置</summary>
    <div class="advanced-content">
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={curriculumEnabled} />
        课程学习
      </label>
      {#if curriculumEnabled}
        <div class="form-row">
          <div class="form-group">
            <label for="curriculum-pacing">节奏</label>
            <select id="curriculum-pacing" class="input" bind:value={curriculumPacing}>
              <option value="Linear">线性</option>
              <option value="Cosine">余弦</option>
              <option value="Root">平方根</option>
            </select>
          </div>
          <div class="form-group">
            <label for="curriculum-warmup">预热步数</label>
            <input id="curriculum-warmup" class="input" type="number" bind:value={curriculumWarmup} />
          </div>
          <div class="form-group">
            <label for="curriculum-total">总步数</label>
            <input id="curriculum-total" class="input" type="number" bind:value={curriculumTotalSteps} />
          </div>
        </div>
      {/if}

      <label class="checkbox-label">
        <input type="checkbox" bind:checked={dynamicRatioEnabled} />
        动态比例调度
      </label>
      {#if dynamicRatioEnabled}
        <div class="form-group">
          <label for="ratio-schedule">调度策略</label>
          <select id="ratio-schedule" class="input" bind:value={ratioSchedule}>
            <option value="Linear">线性</option>
            <option value="Cosine">余弦</option>
          </select>
        </div>
      {/if}

      <div class="form-group">
        <label for="total-samples">目标样本数</label>
        <input id="total-samples" class="input" type="number" bind:value={totalSamplesTarget} placeholder="留空为无限" />
      </div>

      <div class="section-header"><h4>质量阈值</h4></div>
      <div class="form-row">
        <div class="form-group">
          <label for="quality-min">最低质量分</label>
          <input id="quality-min" class="input" type="number" bind:value={qualityMinScore} min="0" max="100" />
        </div>
        <div class="form-group">
          <label for="quality-toxicity">最大毒性</label>
          <input id="quality-toxicity" class="input" type="number" bind:value={qualityMaxToxicity} min="0" max="1" step="0.1" />
        </div>
        <div class="form-group">
          <label for="quality-repetition">最大重复率</label>
          <input id="quality-repetition" class="input" type="number" bind:value={qualityMaxRepetition} min="0" max="1" step="0.1" />
        </div>
      </div>
    </div>
  </details>

  {#if validationError}
    <div class="error-box">{validationError}</div>
  {/if}

  {#if executeError}
    <div class="error-box">{executeError}</div>
  {/if}

  {#if executeResult}
    <div class="result-box">
      <div class="result-header">✅ 配方执行完成</div>
      <div class="result-details">
        <span>生成样本: {executeResult.samples_yielded || 0}</span>
        <span>总步数: {executeResult.total_steps || 0}</span>
      </div>
    </div>
  {/if}

  <div class="actions">
    <button class="btn-secondary" on:click={validateRecipe} disabled={!recipeName || recipeEntries.length === 0}>
      ✅ 验证配方
    </button>
    <button class="btn-primary" on:click={executeRecipe} disabled={executing || !recipeName || recipeEntries.length === 0}>
      {executing ? '执行中...' : '🚀 执行配方'}
    </button>
  </div>
</div>

<style>
  .recipe-builder { padding: 0; }
  .recipe-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
  .recipe-header h3 { margin: 0; font-size: 1rem; }
  .btn-link { background: none; border: none; color: #93c5fd; cursor: pointer; font-size: 0.8rem; }
  .btn-link:hover { color: #60a5fa; }

  .presets-row { display: flex; gap: 0.5rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
  .preset-card { padding: 0.5rem 0.75rem; border: 1px solid rgba(59,130,246,0.25); border-radius: 6px; background: rgba(59,130,246,0.06); cursor: pointer; text-align: left; }
  .preset-card:hover { background: rgba(59,130,246,0.12); border-color: rgba(59,130,246,0.4); }
  .preset-name { font-size: 0.82rem; font-weight: 600; color: #93c5fd; }
  .preset-desc { font-size: 0.7rem; color: #9ca3af; }

  .form-row { display: flex; gap: 0.5rem; align-items: flex-end; }
  .form-group { margin-bottom: 0.5rem; }
  .form-group.flex-1 { flex: 1; }
  .form-group label { display: block; font-size: 0.75rem; color: #9ca3af; margin-bottom: 0.2rem; }
  .input { width: 100%; padding: 0.35rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.8rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }
  .input-sm { width: 90px; }
  select.input { appearance: auto; }

  .section-header { display: flex; justify-content: space-between; align-items: center; margin: 0.75rem 0 0.4rem; }
  .section-header h4 { margin: 0; font-size: 0.85rem; color: #d1d5db; }

  .entries-list { display: flex; flex-direction: column; gap: 0.4rem; margin-bottom: 0.5rem; }
  .entry-row { display: flex; align-items: center; gap: 0.4rem; }
  .entry-name { flex: 1; }
  .weight-group { display: flex; align-items: center; gap: 0.3rem; flex: 0 0 160px; }
  .weight-group input[type="range"] { flex: 1; accent-color: #3b82f6; }
  .weight-val { width: 36px; text-align: right; font-size: 0.75rem; color: #e5e7eb; font-weight: 600; }
  .btn-remove { background: none; border: 1px solid rgba(239,68,68,0.3); border-radius: 3px; color: #fca5a5; cursor: pointer; font-size: 0.7rem; padding: 0.15rem 0.4rem; }
  .btn-remove:hover { background: rgba(239,68,68,0.15); }

  .weight-bar { display: flex; height: 8px; border-radius: 4px; overflow: hidden; margin-bottom: 0.3rem; }
  .weight-seg { transition: width 0.2s; }
  .weight-warning { font-size: 0.72rem; color: #fbbf24; margin-bottom: 0.3rem; }

  .btn-sm { padding: 0.2rem 0.5rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.72rem; cursor: pointer; }
  .btn-sm:hover { background: rgba(255,255,255,0.1); }
  .btn-primary-sm { background: rgba(59,130,246,0.15); border-color: rgba(59,130,246,0.3); color: #93c5fd; }

  .empty-entries { text-align: center; padding: 1rem; color: #6b7280; font-size: 0.8rem; border: 1px dashed rgba(148,163,184,0.15); border-radius: 6px; }

  .strategy-options { display: flex; gap: 1rem; margin-bottom: 0.5rem; }
  .radio-label { display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #d1d5db; cursor: pointer; }
  .radio-label input { accent-color: #3b82f6; }
  .checkbox-label { display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #d1d5db; cursor: pointer; margin-bottom: 0.4rem; }
  .checkbox-label input { accent-color: #3b82f6; }

  .advanced-section { margin-top: 0.5rem; }
  .advanced-section summary { font-size: 0.8rem; color: #9ca3af; cursor: pointer; padding: 0.3rem 0; }
  .advanced-section summary:hover { color: #d1d5db; }
  .advanced-content { padding: 0.5rem 0; }

  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin: 0.5rem 0; }
  .result-box { padding: 0.5rem; background: rgba(16,185,129,0.06); border: 1px solid rgba(16,185,129,0.2); border-radius: 6px; margin: 0.5rem 0; }
  .result-header { font-size: 0.85rem; color: #10b981; font-weight: 600; margin-bottom: 0.3rem; }
  .result-details { display: flex; gap: 1rem; font-size: 0.78rem; color: #9ca3af; }

  .actions { display: flex; gap: 0.5rem; margin-top: 0.75rem; }
  .btn-secondary { padding: 0.4rem 0.9rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 6px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.8rem; cursor: pointer; }
  .btn-secondary:hover { background: rgba(255,255,255,0.1); }
  .btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary { padding: 0.4rem 0.9rem; border: none; border-radius: 6px; background: #3b82f6; color: #fff; font-size: 0.8rem; font-weight: 600; cursor: pointer; }
  .btn-primary:hover { background: #2563eb; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
