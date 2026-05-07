<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';

  export let datasetId: string = '';

  let labelColumn = '';
  let qualityResult: any = null;
  let summaryResult: any = null;
  let confidentResult: any = null;
  let loading = false;
  let error: string | null = null;
  let activeTab = 'quality';

  async function runLabelQuality() {
    if (!datasetId || !labelColumn) return;
    loading = true;
    error = null;
    qualityResult = null;
    const taskId = taskManagerStore.createTask('标签质量分析', `正在分析 "${labelColumn}" 列...`, false);
    try {
      const client = getLabClient();
      qualityResult = await client.datasetLabelQuality(datasetId, labelColumn);
      taskManagerStore.completeTask(taskId, `分析完成，误标率 ${(qualityResult.mislabel_rate * 100).toFixed(2)}%`);
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally {
      loading = false;
    }
  }

  async function runSummary() {
    if (!datasetId) return;
    loading = true;
    error = null;
    summaryResult = null;
    const taskId = taskManagerStore.createTask('标签质量汇总', '正在汇总所有标签列...', false);
    try {
      const client = getLabClient();
      summaryResult = await client.datasetLabelQualitySummary(datasetId);
      taskManagerStore.completeTask(taskId, '汇总完成');
    } catch (e: any) {
      error = e?.toString() || '汇总失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally {
      loading = false;
    }
  }

  async function runConfidentLearning() {
    if (!datasetId || !labelColumn) return;
    loading = true;
    error = null;
    confidentResult = null;
    const taskId = taskManagerStore.createTask('Confident Learning', `正在运行 CL 分析...`, false);
    try {
      const client = getLabClient();
      confidentResult = await client.datasetConfidentLearning(datasetId, labelColumn);
      taskManagerStore.completeTask(taskId, `发现 ${confidentResult.identified_label_issues} 个标签问题`);
    } catch (e: any) {
      error = e?.toString() || '分析失败';
      taskManagerStore.failTask(taskId, error || '未知错误');
    } finally {
      loading = false;
    }
  }

  function qualityColor(rate: number): string {
    if (rate >= 0.99) return '#10b981';
    if (rate >= 0.95) return '#f59e0b';
    return '#ef4444';
  }
</script>

<div class="label-quality">
  <h3>🏷️ 标签质量评估</h3>

  <div class="form-row">
    <div class="form-group flex-1">
      <label for="label-column">标签列名</label>
      <input id="label-column" class="input" type="text" bind:value={labelColumn} placeholder="label / category / sentiment" />
    </div>
    <div class="btn-group">
      <button class="btn-primary-sm" on:click={runLabelQuality} disabled={loading || !labelColumn}>
        🔍 质量分析
      </button>
      <button class="btn-secondary-sm" on:click={runConfidentLearning} disabled={loading || !labelColumn}>
        🧠 CL 分析
      </button>
      <button class="btn-secondary-sm" on:click={runSummary} disabled={loading}>
        📊 汇总
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-box">{error}</div>
  {/if}

  {#if qualityResult}
    <div class="result-card">
      <div class="result-header">
        <span>📋 标签质量报告 — {qualityResult.label_column}</span>
        <span class="quality-badge" style="background: {qualityColor(qualityResult.label_consistency)}20; color: {qualityColor(qualityResult.label_consistency)}">
          {(qualityResult.label_consistency * 100).toFixed(1)}%
        </span>
      </div>
      <div class="metrics-row">
        <div class="metric">
          <div class="metric-value">{qualityResult.total_labels.toLocaleString()}</div>
          <div class="metric-label">总标签数</div>
        </div>
        <div class="metric">
          <div class="metric-value">{qualityResult.unique_labels}</div>
          <div class="metric-label">唯一类别</div>
        </div>
        <div class="metric">
          <div class="metric-value" style="color: {qualityColor(qualityResult.label_consistency)}">{(qualityResult.mislabel_rate * 100).toFixed(2)}%</div>
          <div class="metric-label">误标率</div>
        </div>
        <div class="metric">
          <div class="metric-value" style="color: {qualityColor(qualityResult.label_consistency)}">{qualityResult.suspected_mislabels}</div>
          <div class="metric-label">疑似误标</div>
        </div>
      </div>

      {#if qualityResult.per_class_quality}
        <div class="per-class-section">
          <h4>各类别质量</h4>
          <div class="class-bars">
            {#each Object.entries(qualityResult.per_class_quality) as [cls, infoRaw]}
              {@const info = infoRaw as { count: number; suspected_mislabels: number; consistency: number }}
              <div class="class-bar-row">
                <span class="class-name">{cls}</span>
                <div class="bar-track">
                  <div class="bar-fill" style="width: {info.consistency * 100}%; background: {qualityColor(info.consistency)}"></div>
                </div>
                <span class="bar-label" style="color: {qualityColor(info.consistency)}">{(info.consistency * 100).toFixed(1)}%</span>
                <span class="bar-detail">{info.count} · {info.suspected_mislabels} 疑似</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if summaryResult}
    <div class="result-card">
      <div class="result-header">
        <span>📊 标签质量汇总</span>
        <span class="quality-badge" style="background: {summaryResult.overall_quality === 'good' ? 'rgba(16,185,129,0.15)' : 'rgba(245,158,11,0.15)'}; color: {summaryResult.overall_quality === 'good' ? '#10b981' : '#f59e0b'}">
          {summaryResult.overall_quality}
        </span>
      </div>
      <div class="summary-list">
        {#each summaryResult.summaries as s}
          <div class="summary-item">
            <span class="summary-col">{s.column}</span>
            <div class="bar-track">
              <div class="bar-fill" style="width: {s.quality * 100}%; background: {qualityColor(s.quality)}"></div>
            </div>
            <span class="bar-label" style="color: {qualityColor(s.quality)}">{(s.quality * 100).toFixed(1)}%</span>
            <span class="bar-detail">误标率 {(s.mislabel_rate * 100).toFixed(2)}%</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if confidentResult}
    <div class="result-card">
      <div class="result-header">
        <span>🧠 Confident Learning 分析</span>
        <span class="quality-badge" style="background: rgba(139,92,246,0.15); color: #a78bfa">
          {confidentResult.identified_label_issues} 个问题
        </span>
      </div>
      <div class="metrics-row">
        <div class="metric">
          <div class="metric-value">{confidentResult.identified_label_issues}</div>
          <div class="metric-label">识别的标签问题</div>
        </div>
      </div>
      {#if confidentResult.suggested_corrections?.length > 0}
        <div class="corrections-section">
          <h4>建议修正</h4>
          <div class="corrections-list">
            {#each confidentResult.suggested_corrections.slice(0, 10) as corr}
              <div class="correction-item">
                <span class="corr-index">#{corr.index}</span>
                <span class="corr-from">{corr.original_label}</span>
                <span class="corr-arrow">→</span>
                <span class="corr-to">{corr.suggested_label}</span>
                <span class="corr-confidence">{(corr.confidence * 100).toFixed(0)}%</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .label-quality { padding: 0; }
  .label-quality h3 { font-size: 1rem; margin: 0 0 0.6rem; }

  .form-row { display: flex; gap: 0.5rem; align-items: flex-end; margin-bottom: 0.6rem; flex-wrap: wrap; }
  .form-group { margin-bottom: 0; }
  .form-group.flex-1 { flex: 1; min-width: 150px; }
  .form-group label { display: block; font-size: 0.75rem; color: #9ca3af; margin-bottom: 0.2rem; }
  .input { width: 100%; padding: 0.35rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.8rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }

  .btn-group { display: flex; gap: 0.3rem; }
  .btn-primary-sm { padding: 0.3rem 0.6rem; border: none; border-radius: 4px; background: #3b82f6; color: #fff; font-size: 0.72rem; font-weight: 600; cursor: pointer; white-space: nowrap; }
  .btn-primary-sm:hover { background: #2563eb; }
  .btn-primary-sm:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-secondary-sm { padding: 0.3rem 0.6rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.72rem; cursor: pointer; white-space: nowrap; }
  .btn-secondary-sm:hover { background: rgba(255,255,255,0.1); }
  .btn-secondary-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin-bottom: 0.5rem; }

  .result-card { padding: 0.6rem; background: rgba(15,23,42,0.5); border: 1px solid rgba(148,163,184,0.1); border-radius: 8px; margin-bottom: 0.5rem; }
  .result-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
  .quality-badge { font-size: 0.72rem; padding: 0.15rem 0.5rem; border-radius: 10px; font-weight: 600; }

  .metrics-row { display: flex; gap: 1rem; margin-bottom: 0.5rem; flex-wrap: wrap; }
  .metric { text-align: center; min-width: 70px; }
  .metric-value { font-size: 1.1rem; font-weight: 700; color: #e5e7eb; }
  .metric-label { font-size: 0.65rem; color: #6b7280; margin-top: 0.1rem; }

  .per-class-section h4, .corrections-section h4 { font-size: 0.8rem; color: #9ca3af; margin: 0.4rem 0 0.3rem; }
  .class-bars, .summary-list { display: flex; flex-direction: column; gap: 0.3rem; }
  .class-bar-row, .summary-item { display: flex; align-items: center; gap: 0.4rem; }
  .class-name, .summary-col { width: 70px; font-size: 0.72rem; color: #d1d5db; text-align: right; }
  .bar-track { flex: 1; height: 8px; background: rgba(255,255,255,0.05); border-radius: 4px; overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 4px; transition: width 0.3s; }
  .bar-label { width: 45px; font-size: 0.72rem; font-weight: 600; }
  .bar-detail { font-size: 0.65rem; color: #6b7280; }

  .corrections-list { display: flex; flex-direction: column; gap: 0.2rem; }
  .correction-item { display: flex; align-items: center; gap: 0.4rem; padding: 0.2rem 0.3rem; background: rgba(255,255,255,0.02); border-radius: 3px; }
  .corr-index { font-size: 0.7rem; font-family: monospace; color: #6b7280; width: 50px; }
  .corr-from { font-size: 0.72rem; color: #fca5a5; background: rgba(239,68,68,0.1); padding: 0.1rem 0.3rem; border-radius: 3px; }
  .corr-arrow { color: #6b7280; font-size: 0.7rem; }
  .corr-to { font-size: 0.72rem; color: #6ee7b7; background: rgba(16,185,129,0.1); padding: 0.1rem 0.3rem; border-radius: 3px; }
  .corr-confidence { font-size: 0.65rem; color: #a78bfa; margin-left: auto; }
</style>
