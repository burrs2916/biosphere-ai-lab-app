<script lang="ts">
  export let datasetId: string = '';
  export let qualityScore: any = null;
  export let healthCheckResults: Record<string, any> = {};
  export let readinessScore: any = null;
  export let loading = false;

  import { onMount } from 'svelte';

  interface ReadinessDimension {
    key: string;
    label: string;
    icon: string;
    score: number | null;
    status: 'pass' | 'warn' | 'fail' | 'pending';
    message: string;
    details?: string[];
  }

  let dimensions: ReadinessDimension[] = [];
  let overallScore = 0;
  let overallStatus: 'ready' | 'warning' | 'not_ready' | 'loading' = 'loading';
  let issues: { severity: 'error' | 'warning'; text: string; dimension: string }[] = [];
  let recommendations: string[] = [];

  $: computeReadiness();

  function computeReadiness() {
    if (loading) {
      overallStatus = 'loading';
      return;
    }

    dimensions = [
      {
        key: 'quality',
        label: '数据质量',
        icon: '📊',
        score: qualityScore?.overall_score ?? null,
        status: getQualityStatus(qualityScore?.overall_score),
        message: getQualityMessage(qualityScore?.overall_score),
      },
      {
        key: 'completeness',
        label: '完整性',
        icon: '🔒',
        score: healthCheckResults.integrity ? (healthCheckResults.integrity.integrity_ok ? 95 : 30) : null,
        status: getIntegrityStatus(),
        message: getIntegrityMessage(),
      },
      {
        key: 'validation',
        label: '格式验证',
        icon: '✅',
        score: healthCheckResults.validation?.valid ? 90 : null,
        status: getValidationStatus(),
        message: getValidationMessage(),
      },
      {
        key: 'leakage',
        label: '数据泄漏检查',
        icon: '🛡️',
        score: healthCheckResults.leakage?.leakage_detected === false ? 100 : (healthCheckResults.leakage?.leakage_detected === true ? 20 : null),
        status: getLeakageStatus(),
        message: getLeakageMessage(),
      },
      {
        key: 'sufficiency',
        label: '数据量充足性',
        icon: '📈',
        score: healthCheckResults.sufficiency?.sufficient ? 85 : null,
        status: getSufficiencyStatus(),
        message: getSufficiencyMessage(),
      },
      {
        key: 'balance',
        label: '类别均衡性',
        icon: '⚖️',
        score: readinessScore?.dimensions?.balance?.score ?? null,
        status: getBalanceStatus(readinessScore?.dimensions?.balance?.score),
        message: readinessScore?.dimensions?.balance?.message || '待评估',
      },
    ];

    const scoredDimensions = dimensions.filter(d => d.score != null);
    const totalWeight = scoredDimensions.length || 1;
    overallScore = Math.round(
      scoredDimensions.reduce((sum, d) => sum + (d.score || 0), 0) / totalWeight
    );

    issues = [];

    if (healthCheckResults.leakage?.leakage_detected) {
      issues.push({ severity: 'error', text: '检测到数据泄漏风险', dimension: 'leakage' });
    }
    if (!healthCheckResults.integrity?.integrity_ok && healthCheckResults.integrity) {
      issues.push({ severity: 'error', text: '数据完整性校验未通过', dimension: 'integrity' });
    }
    if (readinessScore?.issues?.length > 0) {
      for (const issue of readinessScore.issues) {
        issues.push({ severity: issue.severity, text: issue.message, dimension: issue.dimension });
      }
    }
    if (overallScore < 50) {
      issues.push({ severity: 'error', text: '综合就绪度过低，不建议用于训练', dimension: 'overall' });
    } else if (overallScore < 70) {
      issues.push({ severity: 'warning', text: '综合就绪度一般，建议优化后使用', dimension: 'overall' });
    }

    recommendations = readinessScore?.recommendations || [];
    if (overallScore >= 70 && overallScore < 85 && !recommendations.includes('建议运行质量评估')) {
      recommendations.push('建议运行质量评估获取详细改进方向');
    }

    const hasError = issues.some(i => i.severity === 'error');
    const hasWarning = issues.some(i => i.severity === 'warning');

    if (hasError) {
      overallStatus = 'not_ready';
    } else if (hasWarning || overallScore < 70) {
      overallStatus = 'warning';
    } else {
      overallStatus = 'ready';
    }
  }

  function getQualityStatus(score: number | undefined): ReadinessDimension['status'] {
    if (score == null) return 'pending';
    if (score >= 80) return 'pass';
    if (score >= 60) return 'warn';
    return 'fail';
  }

  function getQualityMessage(score: number | undefined): string {
    if (score == null) return '待评估';
    if (score >= 80) return `质量良好 (${score.toFixed(0)}分)`;
    if (score >= 60) return `质量一般 (${score.toFixed(0)}分)，需要优化`;
    return `质量较差 (${score.toFixed(0)}分)，强烈建议修复`;
  }

  function getIntegrityStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.integrity) return 'pending';
    return healthCheckResults.integrity.integrity_ok ? 'pass' : 'fail';
  }

  function getIntegrityMessage(): string {
    if (!healthCheckResults.integrity) return '待检查';
    return healthCheckResults.integrity.integrity_ok ? '数据完整' : '存在完整性问题';
  }

  function getValidationStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.validation) return 'pending';
    return healthCheckResults.validation.valid ? 'pass' : 'fail';
  }

  function getValidationMessage(): string {
    if (!healthCheckResults.validation) return '待验证';
    return healthCheckResults.validation.valid ? '格式正确' : '格式存在问题';
  }

  function getLeakageStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.leakage) return 'pending';
    return healthCheckResults.leakage.leakage_detected === false ? 'pass'
      : healthCheckResults.leakage.leakage_detected === true ? 'fail' : 'pending';
  }

  function getLeakageMessage(): string {
    if (!healthCheckResults.leakage) return '待检查';
    if (healthCheckResults.leakage.leakage_detected === false) return '无泄漏风险';
    if (healthCheckResults.leakage.leakage_detected === true) return '检测到数据泄漏！';
    return '未知';
  }

  function getSufficiencyStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.sufficiency) return 'pending';
    return healthCheckResults.sufficiency.sufficient ? 'pass' : 'warn';
  }

  function getSufficiencyMessage(): string {
    if (!healthCheckResults.sufficiency) return '待评估';
    if (healthCheckResults.sufficiency.sufficient) return '数据量充足';
    return `数据量可能不足 (当前${healthCheckResults.sufficiency.current_rows?.toLocaleString()}行)`;
  }

  function getBalanceStatus(score: number | undefined): ReadinessDimension['status'] {
    if (score == null) return 'pending';
    if (score >= 75) return 'pass';
    if (score >= 55) return 'warn';
    return 'fail';
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'pass': return '#10b981';
      case 'warn': return '#f59e0b';
      case 'fail': return '#ef4444';
      default: return '#6b7280';
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'pass': return '通过';
      case 'warn': return '警告';
      case 'fail': return '不通过';
      default: return '待定';
    }
  }

  function overallLabel(status: string): string {
    switch (status) {
      case 'ready': return '✅ 数据已准备好训练';
      case 'warning': return '⚠️ 数据基本可用，有轻微问题';
      case 'not_ready': return '❌ 数据尚未准备好训练';
      default: return '⏳ 正在评估...';
    }
  }

  function overallColor(status: string): string {
    switch (status) {
      case 'ready': return '#10b981';
      case 'warning': return '#f59e0b';
      case 'not_ready': return '#ef4444';
      default: return '#6b7280';
    }
  }

  function scoreRingColor(score: number): string {
    if (score >= 85) return '#10b981';
    if (score >= 70) return '#3b82f6';
    if (score >= 55) return '#f59e0b';
    return '#ef4444';
  }

  function circumference(): number {
    return 2 * Math.PI * 54;
  }
</script>

<div class="readiness-dashboard">
  <div class="dashboard-header">
    <h4>🎯 训练就绪度评估</h4>
    {#if overallStatus !== 'loading'}
      <div class="overall-status" style="color: {overallColor(overallStatus)}">
        {overallLabel(overallStatus)}
      </div>
    {:else}
      <div class="overall-status">⏳ 正在评估...</div>
    {/if}
  </div>

  <div class="dashboard-body">
    <div class="score-section">
      <svg class="score-ring" viewBox="0 0 120 120">
        <circle cx="60" cy="60" r="54" fill="none" stroke="rgba(255,255,255,0.06)" stroke-width="8" />
        {#if overallStatus !== 'loading'}
          <circle
            cx="60" cy="60" r="54" fill="none"
            stroke={scoreRingColor(overallScore)}
            stroke-width="8"
            stroke-linecap="round"
            stroke-dasharray={circumference()}
            stroke-dashoffset={circumference() - (circumference() * overallScore / 100)}
            transform="rotate(-90 60 60)"
            style="transition: stroke-dashoffset 0.5s ease"
          />
          <text x="60" y="52" text-anchor="middle" fill={scoreRingColor(overallScore)} font-size="24" font-weight="700">{overallScore}</text>
          <text x="60" y="72" text-anchor="middle" fill="#94a3b8" font-size="11">/ 100</text>
        {:else}
          <text x="60" y="62" text-anchor="middle" fill="#64748b" font-size="14">...</text>
        {/if}
      </svg>

      <div class="score-labels">
        <span class="label-item"><span style="color:#10b981">●</span> ≥85 就绪</span>
        <span class="label-item"><span style="color:#3b82f6">●</span> ≥70 可用</span>
        <span class="label-item"><span style="color:#f59e0b">●</span> ≥55 需优化</span>
        <span class="label-item"><span style="color:#ef4444">●</span> &lt;55 不推荐</span>
      </div>
    </div>

    <div class="dimensions-section">
      <h5>维度详情</h5>
      <div class="dim-list">
        {#each dimensions as dim}
          <div class="dim-row">
            <div class="dim-left">
              <span class="dim-icon">{dim.icon}</span>
              <span class="dim-name">{dim.label}</span>
            </div>
            <div class="dim-right">
              {#if dim.score != null}
                <span class="dim-score" style="color: {statusColor(dim.status)}">{dim.score.toFixed(0)}</span>
              {:else}
                <span class="dim-score dim-pending">—</span>
              {/if}
              <span class="dim-badge" style="background: {statusColor(dim.status)}18; color: {statusColor(dim.status)}; border-color: ${statusColor(dim.status)}33;">
                {statusLabel(dim.status)}
              </span>
            </div>
          </div>
          <div class="dim-message">{dim.message}</div>
          {#if dim.details && dim.details.length > 0}
            <ul class="dim-details">
              {#each dim.details as detail}
                <li>{detail}</li>
              {/each}
            </ul>
          {/if}
        {/each}
      </div>
    </div>
  </div>

  {#if issues.length > 0}
    <div class="issues-section">
      <h5>发现的问题 ({issues.length})</h5>
      {#each issues as issue}
        <div class="issue-item" class:error={issue.severity === 'error'} class:warning={issue.severity === 'warning'}>
          <span class="issue-sev">{issue.severity === 'error' ? '🔴' : '⚠️'}</span>
          <span class="issue-text">{issue.text}</span>
          <span class="issue-dim">{issue.dimension}</span>
        </div>
      {/each}
    </div>
  {/if}

  {#if recommendations.length > 0}
    <div class="recs-section">
      <h5>💡 改进建议</h5>
      {#each recommendations as rec}
        <div class="rec-item">{rec}</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .readiness-dashboard {
    padding: 0.75rem;
  }

  .dashboard-header {
    display: flex; justify-content: space-between; align-items: flex-start;
    margin-bottom: 1rem;
  }

  .dashboard-header h4 { margin: 0; font-size: 0.92rem; color: #e2e8f0; }

  .overall-status {
    font-size: 0.78rem; font-weight: 500;
    padding: 0.25rem 0.6rem; border-radius: 6px;
    background: rgba(255,255,255,0.03);
  }

  .dashboard-body {
    display: grid; grid-template-columns: 160px 1fr; gap: 1.25rem;
  }

  .score-section { display: flex; flex-direction: column; align-items: center; gap: 0.75rem; }

  .score-ring { width: 110px; height: 110px; }

  .score-labels {
    display: flex; flex-direction: column; gap: 0.2rem;
    font-size: 0.62rem; color: #94a3b8;
  }

  .label-item { display: flex; align-items: center; gap: 0.2rem; }

  .dimensions-section h5 { margin: 0 0 0.5rem; font-size: 0.82rem; color: #cbd5e1; }

  .dim-list { display: flex; flex-direction: column; gap: 0.35rem; }

  .dim-row {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.35rem 0.5rem; background: rgba(255,255,255,0.02);
    border-radius: 5px;
  }

  .dim-left { display: flex; align-items: center; gap: 0.4rem; }

  .dim-icon { font-size: 0.9rem; }

  .dim-name { font-size: 0.78rem; color: #d1d5db; }

  .dim-right { display: flex; align-items: center; gap: 0.4rem; }

  .dim-score { font-size: 0.88rem; font-weight: 700; min-width: 22px; text-align: right; }

  .dim-pending { color: #6b7280 !important; }

  .dim-badge {
    font-size: 0.6rem; padding: 0.08rem 0.35rem; border-radius: 3px;
    border: 1px solid; font-weight: 500;
  }

  .dim-message {
    font-size: 0.7rem; color: #94a3b8; padding-left: 0.5rem;
  }

  .dim-details {
    margin: 0.15rem 0 0 0.5rem; padding-left: 1rem;
    list-style: disc; font-size: 0.68rem; color: #94a3b8;
  }

  .issues-section {
    margin-top: 1rem; padding-top: 0.75rem;
    border-top: 1px solid rgba(107,114,128,0.15);
  }

  .issues-section h5 { margin: 0 0 0.5rem; font-size: 0.82rem; color: #cbd5e1; }

  .issue-item {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.3rem 0.5rem; margin-bottom: 0.2rem;
    border-radius: 4px; font-size: 0.76rem;
  }

  .issue-item.error { background: rgba(239,68,68,0.06); color: #fca5a5; }
  .issue-item.warning { background: rgba(245,158,11,0.06); color: #fbbf24; }

  .issue-text { flex: 1; }

  .issue-dim {
    font-size: 0.6rem; padding: 0.05rem 0.3rem; border-radius: 3px;
    background: rgba(107,114,128,0.12); color: #94a3b8;
  }

  .recs-section {
    margin-top: 0.75rem;
  }

  .recs-section h5 { margin: 0 0 0.4rem; font-size: 0.82rem; color: #cbd5e1; }

  .rec-item {
    padding: 0.3rem 0.5rem; margin-bottom: 0.15rem;
    background: rgba(16,185,129,0.04); border-radius: 4px;
    border-left: 3px solid #10b981;
    font-size: 0.74rem; color: #d1d5db;
  }
</style>
