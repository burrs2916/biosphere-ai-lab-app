<script lang="ts">
  export let qualityScore: any = null;

  let expandedDimension: string | null = null;

  function gradeLabel(grade: string): string {
    const map: Record<string, string> = { excellent: '优秀', good: '良好', fair: '一般', poor: '较差', critical: '严重' };
    return map[grade] || grade;
  }

  function gradeColor(grade: string): string {
    const map: Record<string, string> = { excellent: '#10b981', good: '#3b82f6', fair: '#f59e0b', poor: '#f97316', critical: '#ef4444' };
    return map[grade] || '#6b7280';
  }

  function scoreColor(score: number): string {
    if (score >= 85) return '#10b981';
    if (score >= 70) return '#3b82f6';
    if (score >= 55) return '#f59e0b';
    if (score >= 40) return '#f97316';
    return '#ef4444';
  }

  function toggleDimension(name: string) {
    expandedDimension = expandedDimension === name ? null : name;
  }

  function severityIcon(severity: string): string {
    if (severity === 'error') return '🔴';
    if (severity === 'warning') return '⚠️';
    return 'ℹ️';
  }
</script>

{#if qualityScore}
  <div class="quality-panel">
    <div class="quality-overview">
      <div class="score-circle" style="--score-color: {gradeColor(qualityScore.grade)}">
        <span class="score-number">{qualityScore.overall_score.toFixed(0)}</span>
        <span class="score-grade">{gradeLabel(qualityScore.grade)}</span>
      </div>
      <div class="score-summary">
        <div class="score-title">综合质量评分</div>
        <div class="score-meta">
          {#if qualityScore.dimensions?.length}
            {qualityScore.dimensions.length} 个维度评估
          {/if}
          {#if qualityScore.issues?.length}
            · {qualityScore.issues.length} 个问题
          {/if}
        </div>
      </div>
    </div>

    <div class="dimensions-list">
      {#each qualityScore.dimensions || [] as dim}
        <div class="dimension-item" class:expanded={expandedDimension === dim.name}>
          <div class="dimension-header" role="button" tabindex="0" on:click={() => toggleDimension(dim.name)} on:keydown={(e) => e.key === 'Enter' && toggleDimension(dim.name)}>
            <div class="dim-main">
              <span class="dim-label">{dim.label}</span>
              <span class="dim-details">{dim.details}</span>
            </div>
            <div class="dim-score-area">
              <span class="dim-score" style="color: {scoreColor(dim.score)}">{dim.score.toFixed(0)}</span>
              <span class="dim-weight">权重 {(dim.weight * 100).toFixed(0)}%</span>
            </div>
          </div>
          <div class="dim-bar">
            <div class="dim-bar-fill" style="width: {dim.score}%; background: {scoreColor(dim.score)}"></div>
          </div>

          {#if expandedDimension === dim.name}
            <div class="dim-expand">
              {#if dim.sub_metrics?.length > 0}
                <div class="sub-metrics">
                  {#each dim.sub_metrics as sm}
                    <div class="sub-metric">
                      <div class="sm-header">
                        <span class="sm-label">{sm.label}</span>
                        <span class="sm-score" style="color: {scoreColor(sm.score)}">{sm.score.toFixed(0)}</span>
                      </div>
                      <div class="sm-bar">
                        <div class="sm-bar-fill" style="width: {sm.score}%; background: {scoreColor(sm.score)}"></div>
                      </div>
                      <div class="sm-desc">{sm.description}</div>
                    </div>
                  {/each}
                </div>
              {/if}
              {#if dim.improvement}
                <div class="dim-improvement">
                  <span class="improvement-icon">💡</span>
                  <span class="improvement-text">{dim.improvement}</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    {#if qualityScore.issues?.length > 0}
      <div class="issues-section">
        <h5 class="section-title">发现的问题</h5>
        {#each qualityScore.issues as issue}
          <div class="issue-item" class:error={issue.severity === 'error'} class:warning={issue.severity === 'warning'} class:info={issue.severity === 'info'}>
            <span class="issue-icon">{severityIcon(issue.severity)}</span>
            <div class="issue-content">
              <div class="issue-desc">{issue.description}</div>
              {#if issue.suggestion}
                <div class="issue-suggestion">💡 {issue.suggestion}</div>
              {/if}
            </div>
            {#if issue.dimension}
              <span class="issue-dim-tag">{issue.dimension}</span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if qualityScore.recommendations?.length > 0}
      <div class="recs-section">
        <h5 class="section-title">改进建议</h5>
        <div class="recs-list">
          {#each qualityScore.recommendations as rec}
            <div class="rec-item">{rec}</div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .quality-panel { display: flex; flex-direction: column; gap: 0.75rem; }

  .quality-overview {
    display: flex; align-items: center; gap: 1rem;
    padding: 0.75rem; border-radius: 8px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(148, 163, 184, 0.08);
  }

  .score-circle {
    width: 64px; height: 64px; border-radius: 50%;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    border: 3px solid var(--score-color);
    background: rgba(0, 0, 0, 0.2);
    flex-shrink: 0;
  }

  .score-number { font-size: 1.2rem; font-weight: 700; color: var(--score-color); line-height: 1; }
  .score-grade { font-size: 0.6rem; color: var(--score-color); margin-top: 2px; }

  .score-summary { flex: 1; }
  .score-title { font-size: 0.85rem; font-weight: 600; color: #e2e8f0; }
  .score-meta { font-size: 0.7rem; color: #94a3b8; margin-top: 0.15rem; }

  .dimensions-list { display: flex; flex-direction: column; gap: 0.4rem; }

  .dimension-item {
    border-radius: 6px; padding: 0.5rem;
    background: rgba(255, 255, 255, 0.015);
    border: 1px solid rgba(148, 163, 184, 0.06);
    transition: border-color 0.15s;
  }

  .dimension-item.expanded { border-color: rgba(59, 130, 246, 0.2); background: rgba(59, 130, 246, 0.03); }

  .dimension-header {
    display: flex; justify-content: space-between; align-items: center;
    cursor: pointer; user-select: none;
  }

  .dim-main { flex: 1; min-width: 0; }
  .dim-label { font-size: 0.78rem; font-weight: 500; color: #e2e8f0; }
  .dim-details { display: block; font-size: 0.65rem; color: #94a3b8; margin-top: 0.1rem; }

  .dim-score-area { display: flex; flex-direction: column; align-items: flex-end; flex-shrink: 0; }
  .dim-score { font-size: 0.9rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  .dim-weight { font-size: 0.6rem; color: #64748b; }

  .dim-bar {
    width: 100%; height: 3px; background: rgba(255, 255, 255, 0.06);
    border-radius: 2px; overflow: hidden; margin-top: 0.35rem;
  }

  .dim-bar-fill { height: 100%; border-radius: 2px; transition: width 0.4s ease; }

  .dim-expand {
    margin-top: 0.5rem; padding-top: 0.4rem;
    border-top: 1px solid rgba(148, 163, 184, 0.06);
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

  .sub-metrics { display: flex; flex-direction: column; gap: 0.35rem; }

  .sub-metric { padding: 0.25rem 0; }
  .sm-header { display: flex; justify-content: space-between; align-items: center; }
  .sm-label { font-size: 0.7rem; color: #cbd5e1; }
  .sm-score { font-size: 0.72rem; font-weight: 600; font-variant-numeric: tabular-nums; }
  .sm-bar { width: 100%; height: 2px; background: rgba(255, 255, 255, 0.04); border-radius: 1px; overflow: hidden; margin-top: 0.15rem; }
  .sm-bar-fill { height: 100%; border-radius: 1px; transition: width 0.3s ease; }
  .sm-desc { font-size: 0.62rem; color: #64748b; margin-top: 0.1rem; }

  .dim-improvement {
    display: flex; align-items: flex-start; gap: 0.3rem;
    margin-top: 0.4rem; padding: 0.35rem 0.5rem;
    background: rgba(245, 158, 11, 0.06);
    border: 1px solid rgba(245, 158, 11, 0.12);
    border-radius: 4px;
  }

  .improvement-icon { font-size: 0.75rem; flex-shrink: 0; margin-top: 1px; }
  .improvement-text { font-size: 0.68rem; color: #fbbf24; line-height: 1.4; }

  .section-title { font-size: 0.78rem; font-weight: 600; color: #e2e8f0; margin: 0 0 0.4rem 0; }

  .issues-section { display: flex; flex-direction: column; gap: 0.3rem; }

  .issue-item {
    display: flex; align-items: flex-start; gap: 0.4rem;
    padding: 0.4rem 0.5rem; border-radius: 4px;
    border: 1px solid rgba(148, 163, 184, 0.06);
  }

  .issue-item.error { background: rgba(239, 68, 68, 0.04); border-color: rgba(239, 68, 68, 0.12); }
  .issue-item.warning { background: rgba(245, 158, 11, 0.04); border-color: rgba(245, 158, 11, 0.12); }
  .issue-item.info { background: rgba(59, 130, 246, 0.04); border-color: rgba(59, 130, 246, 0.12); }

  .issue-icon { font-size: 0.75rem; flex-shrink: 0; margin-top: 1px; }
  .issue-content { flex: 1; min-width: 0; }
  .issue-desc { font-size: 0.72rem; color: #e2e8f0; }
  .issue-suggestion { font-size: 0.65rem; color: #94a3b8; margin-top: 0.15rem; }
  .issue-dim-tag {
    font-size: 0.58rem; color: #64748b; background: rgba(255, 255, 255, 0.04);
    padding: 0.1rem 0.3rem; border-radius: 3px; flex-shrink: 0; align-self: center;
  }

  .recs-section { display: flex; flex-direction: column; gap: 0.25rem; }
  .recs-list { display: flex; flex-direction: column; gap: 0.2rem; }
  .rec-item {
    font-size: 0.7rem; color: #cbd5e1; padding: 0.3rem 0.5rem;
    background: rgba(16, 185, 129, 0.03); border-radius: 4px;
    border: 1px solid rgba(16, 185, 129, 0.08);
    line-height: 1.4;
  }
</style>
