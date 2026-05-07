<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';

  export let datasetId: string = '';

  let activeSection = 'imbalance';

  let labelColumn = '';
  let sensitiveColumn = '';
  let referenceDatasetId = '';
  let experimentId = '';
  let sliceBy = '';
  let sliceConditions: Record<string, any> = {};

  let imbalanceResult: any = null;
  let biasResult: any = null;
  let driftResult: any = null;
  let correlationResult: any = null;
  let sliceResult: any = null;
  let influenceResult: any = null;
  let loading = false;
  let error: string | null = null;

  async function runImbalance() {
    if (!datasetId || !labelColumn) return;
    loading = true; error = null; imbalanceResult = null;
    const taskId = taskManagerStore.createTask('类别不平衡分析', `分析 ${labelColumn}...`, false);
    try {
      const client = getLabClient();
      imbalanceResult = await client.datasetAnalyzeImbalance(datasetId, labelColumn);
      taskManagerStore.completeTask(taskId, `不平衡比率 ${imbalanceResult.imbalance_ratio.toFixed(2)}`);
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  async function runBias() {
    if (!datasetId || !sensitiveColumn || !labelColumn) return;
    loading = true; error = null; biasResult = null;
    const taskId = taskManagerStore.createTask('偏差检测', `检测 ${sensitiveColumn} 偏差...`, false);
    try {
      const client = getLabClient();
      biasResult = await client.datasetBiasDetection(datasetId, { sensitive_column: sensitiveColumn, label_column: labelColumn });
      taskManagerStore.completeTask(taskId, biasResult.bias_detected ? '检测到偏差' : '未检测到显著偏差');
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  async function runDrift() {
    if (!datasetId || !referenceDatasetId) return;
    loading = true; error = null; driftResult = null;
    const taskId = taskManagerStore.createTask('数据漂移分析', '对比数据分布...', false);
    try {
      const client = getLabClient();
      driftResult = await client.datasetAnalyzeDrift(datasetId, referenceDatasetId);
      taskManagerStore.completeTask(taskId, driftResult.drift_detected ? '检测到漂移' : '未检测到漂移');
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  async function runCorrelation() {
    if (!datasetId) return;
    loading = true; error = null; correlationResult = null;
    const taskId = taskManagerStore.createTask('相关性分析', '计算特征相关性...', false);
    try {
      const client = getLabClient();
      correlationResult = await client.datasetAnalyzeCorrelation(datasetId);
      taskManagerStore.completeTask(taskId, '分析完成');
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  async function runSlice() {
    if (!datasetId || !sliceBy) return;
    loading = true; error = null; sliceResult = null;
    const taskId = taskManagerStore.createTask('切片分析', `按 ${sliceBy} 切片...`, false);
    try {
      const client = getLabClient();
      sliceResult = await client.datasetSliceAnalysis(datasetId, { slice_by: sliceBy, conditions: sliceConditions });
      taskManagerStore.completeTask(taskId, `${sliceResult.slices?.length || 0} 个切片`);
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  async function runInfluence() {
    if (!datasetId || !experimentId) return;
    loading = true; error = null; influenceResult = null;
    const taskId = taskManagerStore.createTask('数据影响分析', '计算样本影响力...', false);
    try {
      const client = getLabClient();
      influenceResult = await client.datasetInfluenceTracin(datasetId, experimentId);
      taskManagerStore.completeTask(taskId, '分析完成');
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally { loading = false; }
  }

  function severityColor(level: string): string {
    if (level === 'low') return '#10b981';
    if (level === 'mild') return '#f59e0b';
    return '#ef4444';
  }
</script>

<div class="analysis-panel">
  <h3>🔬 数据深度分析</h3>

  <div class="tab-bar">
    {#each [
      { id: 'imbalance', label: '⚖️ 不平衡', icon: '' },
      { id: 'bias', label: '🛡️ 偏差', icon: '' },
      { id: 'drift', label: '📈 漂移', icon: '' },
      { id: 'correlation', label: '🔗 相关', icon: '' },
      { id: 'slice', label: '🔪 切片', icon: '' },
      { id: 'influence', label: '💡 影响', icon: '' },
    ] as tab}
      <button class="tab-btn" class:active={activeSection === tab.id} on:click={() => (activeSection = tab.id)}>
        {tab.label}
      </button>
    {/each}
  </div>

  {#if activeSection === 'imbalance'}
    <div class="section">
      <div class="form-row">
        <div class="form-group flex-1">
          <label for="imbalance-label">标签列</label>
          <input id="imbalance-label" class="input" type="text" bind:value={labelColumn} placeholder="label" />
        </div>
        <button class="btn-primary-sm" on:click={runImbalance} disabled={loading || !labelColumn}>分析</button>
      </div>
      {#if imbalanceResult}
        <div class="result-card">
          <div class="metrics-row">
            <div class="metric"><div class="metric-value">{imbalanceResult.total_samples.toLocaleString()}</div><div class="metric-label">总样本</div></div>
            <div class="metric"><div class="metric-value" style="color: {imbalanceResult.is_imbalanced ? '#f59e0b' : '#10b981'}">{imbalanceResult.imbalance_ratio.toFixed(2)}</div><div class="metric-label">不平衡比率</div></div>
            <div class="metric"><div class="metric-value">{imbalanceResult.entropy.toFixed(2)}</div><div class="metric-label">熵</div></div>
          </div>
          <div class="class-bars">
            {#each Object.entries(imbalanceResult.class_counts) as [cls, count]}
              {@const ratio = (count as number) / imbalanceResult.total_samples}
              <div class="bar-row">
                <span class="bar-name">{cls}</span>
                <div class="bar-track"><div class="bar-fill" style="width: {ratio * 100}%; background: #3b82f6"></div></div>
                <span class="bar-val">{(count as number).toLocaleString()} ({(ratio * 100).toFixed(1)}%)</span>
              </div>
            {/each}
          </div>
          {#if imbalanceResult.recommendations?.length}
            <div class="recs">{#each imbalanceResult.recommendations as r}<div class="rec">💡 {r}</div>{/each}</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeSection === 'bias'}
    <div class="section">
      <div class="form-row">
        <div class="form-group"><label for="bias-sensitive">敏感列</label><input id="bias-sensitive" class="input" type="text" bind:value={sensitiveColumn} placeholder="gender" /></div>
        <div class="form-group"><label for="bias-label">标签列</label><input id="bias-label" class="input" type="text" bind:value={labelColumn} placeholder="label" /></div>
        <button class="btn-primary-sm" on:click={runBias} disabled={loading || !sensitiveColumn || !labelColumn}>检测</button>
      </div>
      {#if biasResult}
        <div class="result-card">
          <div class="bias-status" style="color: {biasResult.bias_detected ? '#ef4444' : '#10b981'}">
            {biasResult.bias_detected ? '⚠️ 检测到偏差' : '✅ 未检测到显著偏差'}
          </div>
          <div class="metrics-row">
            <div class="metric"><div class="metric-value">{(biasResult.demographic_parity * 100).toFixed(0)}%</div><div class="metric-label">人口统计平等</div></div>
            <div class="metric"><div class="metric-value">{(biasResult.equalized_odds * 100).toFixed(0)}%</div><div class="metric-label">机会均等</div></div>
            <div class="metric"><div class="metric-value">{(biasResult.predictive_parity * 100).toFixed(0)}%</div><div class="metric-label">预测平等</div></div>
          </div>
          {#if biasResult.group_metrics?.length}
            <div class="group-table">
              <div class="group-header"><span>群体</span><span>正例率</span><span>TPR</span><span>FPR</span></div>
              {#each biasResult.group_metrics as g}
                <div class="group-row"><span>{g.group}</span><span>{(g.positive_rate * 100).toFixed(1)}%</span><span>{(g.tpr * 100).toFixed(1)}%</span><span>{(g.fpr * 100).toFixed(1)}%</span></div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeSection === 'drift'}
    <div class="section">
      <div class="form-row">
        <div class="form-group flex-1"><label for="drift-ref">参考数据集ID</label><input id="drift-ref" class="input" type="text" bind:value={referenceDatasetId} placeholder="dataset_ref_id" /></div>
        <button class="btn-primary-sm" on:click={runDrift} disabled={loading || !referenceDatasetId}>对比</button>
      </div>
      {#if driftResult}
        <div class="result-card">
          <div class="drift-status" style="color: {severityColor(driftResult.overall_severity)}">
            {driftResult.drift_detected ? '⚠️ 检测到漂移' : '✅ 未检测到漂移'} (严重度: {driftResult.overall_severity})
          </div>
          <div class="metric"><div class="metric-value">{driftResult.drift_score.toFixed(3)}</div><div class="metric-label">漂移分数</div></div>
          {#if driftResult.feature_drifts?.length}
            <div class="drift-features">
              {#each driftResult.feature_drifts as fd}
                <div class="drift-row">
                  <span class="drift-feature">{fd.feature}</span>
                  <div class="bar-track"><div class="bar-fill" style="width: {fd.drift_score * 100}%; background: {severityColor(fd.drift_type)}"></div></div>
                  <span class="drift-type" style="color: {severityColor(fd.drift_type)}">{fd.drift_type}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeSection === 'correlation'}
    <div class="section">
      <button class="btn-primary-sm" on:click={runCorrelation} disabled={loading}>分析相关性</button>
      {#if correlationResult}
        <div class="result-card">
          {#if correlationResult.highly_correlated_pairs?.length}
            <h4>高度相关特征对</h4>
            {#each correlationResult.highly_correlated_pairs as pair}
              <div class="corr-pair">
                <span>{pair.feature_a}</span>
                <span class="corr-val" style="color: {pair.correlation > 0.8 ? '#ef4444' : '#f59e0b'}">↔ {pair.correlation.toFixed(2)}</span>
                <span>{pair.feature_b}</span>
              </div>
            {/each}
          {/if}
          {#if correlationResult.recommendations?.length}
            <div class="recs">{#each correlationResult.recommendations as r}<div class="rec">💡 {r}</div>{/each}</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeSection === 'slice'}
    <div class="section">
      <div class="form-row">
        <div class="form-group flex-1"><label for="slice-by">切片依据</label><input id="slice-by" class="input" type="text" bind:value={sliceBy} placeholder="age / region / category" /></div>
        <button class="btn-primary-sm" on:click={runSlice} disabled={loading || !sliceBy}>切片</button>
      </div>
      {#if sliceResult}
        <div class="result-card">
          <div class="metrics-row"><div class="metric"><div class="metric-value">{sliceResult.slices?.length || 0}</div><div class="metric-label">切片数</div></div></div>
          {#each (sliceResult.slices || []) as sl}
            <div class="slice-item">
              <div class="slice-header">
                <span class="slice-name">{sl.slice_name}</span>
                <span class="slice-count">{sl.row_count.toLocaleString()} ({(sl.row_ratio * 100).toFixed(1)}%)</span>
                <span class="slice-quality" style="color: {sl.avg_quality > 0.9 ? '#10b981' : '#f59e0b'}">质量 {(sl.avg_quality * 100).toFixed(0)}%</span>
              </div>
            </div>
          {/each}
          {#if sliceResult.recommendations?.length}
            <div class="recs">{#each sliceResult.recommendations as r}<div class="rec">💡 {r}</div>{/each}</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if activeSection === 'influence'}
    <div class="section">
      <div class="form-row">
        <div class="form-group flex-1"><label for="track-exp">实验ID</label><input id="track-exp" class="input" type="text" bind:value={experimentId} placeholder="experiment_id" /></div>
        <button class="btn-primary-sm" on:click={runInfluence} disabled={loading || !experimentId}>分析</button>
      </div>
      {#if influenceResult}
        <div class="result-card">
          <div class="influence-method">方法: {influenceResult.method}</div>
          <div class="influence-cols">
            <div class="influence-col">
              <h4 style="color: #10b981">👍 最有帮助</h4>
              {#each (influenceResult.most_helpful || []).slice(0, 5) as h}
                <div class="influence-item good">#{h.index} — {h.influence_score?.toFixed(3) || h.score_change?.toFixed(4)}</div>
              {/each}
            </div>
            <div class="influence-col">
              <h4 style="color: #ef4444">👎 最有害</h4>
              {#each (influenceResult.most_harmful || []).slice(0, 5) as h}
                <div class="influence-item bad">#{h.index} — {h.influence_score?.toFixed(3) || h.score_change?.toFixed(4)}</div>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if error}
    <div class="error-box">{error}</div>
  {/if}
</div>

<style>
  .analysis-panel { padding: 0; }
  .analysis-panel h3 { font-size: 1rem; margin: 0 0 0.5rem; }

  .tab-bar { display: flex; gap: 0.25rem; margin-bottom: 0.6rem; flex-wrap: wrap; }
  .tab-btn { padding: 0.3rem 0.6rem; border: 1px solid rgba(148,163,184,0.15); border-radius: 5px; background: rgba(255,255,255,0.03); color: #9ca3af; font-size: 0.72rem; cursor: pointer; transition: all 0.15s; }
  .tab-btn:hover { background: rgba(255,255,255,0.06); color: #d1d5db; }
  .tab-btn.active { background: rgba(59,130,246,0.12); border-color: rgba(59,130,246,0.3); color: #93c5fd; }

  .section { min-height: 0; }

  .form-row { display: flex; gap: 0.4rem; align-items: flex-end; margin-bottom: 0.5rem; flex-wrap: wrap; }
  .form-group { margin-bottom: 0; }
  .form-group.flex-1 { flex: 1; min-width: 120px; }
  .form-group label { display: block; font-size: 0.72rem; color: #9ca3af; margin-bottom: 0.15rem; }
  .input { width: 100%; padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.78rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }

  .btn-primary-sm { padding: 0.3rem 0.6rem; border: none; border-radius: 4px; background: #3b82f6; color: #fff; font-size: 0.72rem; font-weight: 600; cursor: pointer; white-space: nowrap; }
  .btn-primary-sm:hover { background: #2563eb; }
  .btn-primary-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .result-card { padding: 0.5rem; background: rgba(15,23,42,0.5); border: 1px solid rgba(148,163,184,0.1); border-radius: 8px; margin-top: 0.5rem; }
  .metrics-row { display: flex; gap: 1rem; margin-bottom: 0.4rem; flex-wrap: wrap; }
  .metric { text-align: center; min-width: 60px; }
  .metric-value { font-size: 1rem; font-weight: 700; color: #e5e7eb; }
  .metric-label { font-size: 0.62rem; color: #6b7280; }

  .class-bars, .drift-features { display: flex; flex-direction: column; gap: 0.25rem; }
  .bar-row, .drift-row { display: flex; align-items: center; gap: 0.4rem; }
  .bar-name, .drift-feature { width: 60px; font-size: 0.72rem; color: #d1d5db; text-align: right; }
  .bar-track { flex: 1; height: 8px; background: rgba(255,255,255,0.05); border-radius: 4px; overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 4px; transition: width 0.3s; }
  .bar-val { font-size: 0.68rem; color: #9ca3af; width: 100px; }
  .drift-type { font-size: 0.68rem; font-weight: 600; width: 40px; }

  .bias-status, .drift-status { font-size: 0.85rem; font-weight: 600; margin-bottom: 0.4rem; }
  .group-table { font-size: 0.72rem; }
  .group-header { display: flex; gap: 1rem; color: #6b7280; font-weight: 600; padding: 0.2rem 0; border-bottom: 1px solid rgba(148,163,184,0.1); }
  .group-header span { min-width: 60px; }
  .group-row { display: flex; gap: 1rem; padding: 0.2rem 0; color: #d1d5db; }
  .group-row span { min-width: 60px; }

  .corr-pair { display: flex; align-items: center; gap: 0.4rem; font-size: 0.78rem; color: #d1d5db; margin-bottom: 0.2rem; }
  .corr-val { font-weight: 700; }

  .slice-item { padding: 0.3rem 0.4rem; border-bottom: 1px solid rgba(148,163,184,0.05); }
  .slice-header { display: flex; gap: 0.6rem; align-items: center; }
  .slice-name { font-size: 0.78rem; font-weight: 600; color: #93c5fd; }
  .slice-count { font-size: 0.7rem; color: #9ca3af; }
  .slice-quality { font-size: 0.7rem; font-weight: 600; }

  .influence-cols { display: flex; gap: 1rem; }
  .influence-col { flex: 1; }
  .influence-col h4 { font-size: 0.78rem; margin: 0.3rem 0; }
  .influence-item { font-size: 0.72rem; padding: 0.15rem 0.3rem; border-radius: 3px; margin-bottom: 0.15rem; font-family: monospace; }
  .influence-item.good { background: rgba(16,185,129,0.06); color: #6ee7b7; }
  .influence-item.bad { background: rgba(239,68,68,0.06); color: #fca5a5; }
  .influence-method { font-size: 0.72rem; color: #6b7280; margin-bottom: 0.3rem; }

  .recs { margin-top: 0.4rem; }
  .rec { font-size: 0.72rem; color: #9ca3af; padding: 0.15rem 0; }

  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin-top: 0.5rem; }
</style>
