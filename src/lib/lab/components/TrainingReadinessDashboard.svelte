<script lang="ts">
  import { t } from '$lib/i18n';
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
        label: $t('readiness.quality'),
        icon: '📊',
        score: qualityScore?.overall_score ?? null,
        status: getQualityStatus(qualityScore?.overall_score),
        message: getQualityMessage(qualityScore?.overall_score),
      },
      {
        key: 'completeness',
        label: $t('readiness.completeness'),
        icon: '🔒',
        score: healthCheckResults.integrity ? (healthCheckResults.integrity.integrity_ok ? 95 : 30) : null,
        status: getIntegrityStatus(),
        message: getIntegrityMessage(),
      },
      {
        key: 'validation',
        label: $t('readiness.formatValidation'),
        icon: '✅',
        score: healthCheckResults.validation?.valid ? 90 : null,
        status: getValidationStatus(),
        message: getValidationMessage(),
      },
      {
        key: 'leakage',
        label: $t('readiness.leakageCheck'),
        icon: '🛡️',
        score: healthCheckResults.leakage?.leakage_detected === false ? 100 : (healthCheckResults.leakage?.leakage_detected === true ? 20 : null),
        status: getLeakageStatus(),
        message: getLeakageMessage(),
      },
      {
        key: 'sufficiency',
        label: $t('readiness.sufficiency'),
        icon: '📈',
        score: healthCheckResults.sufficiency?.sufficient ? 85 : null,
        status: getSufficiencyStatus(),
        message: getSufficiencyMessage(),
      },
      {
        key: 'balance',
        label: $t('readiness.balance'),
        icon: '⚖️',
        score: readinessScore?.dimensions?.balance?.score ?? null,
        status: getBalanceStatus(readinessScore?.dimensions?.balance?.score),
        message: readinessScore?.dimensions?.balance?.message || $t('readiness.pending'),
      },
    ];

    const scoredDimensions = dimensions.filter(d => d.score != null);
    const totalWeight = scoredDimensions.length || 1;
    overallScore = Math.round(
      scoredDimensions.reduce((sum, d) => sum + (d.score || 0), 0) / totalWeight
    );

    issues = [];

    if (healthCheckResults.leakage?.leakage_detected) {
      issues.push({ severity: 'error', text: $t('readiness.leakageRisk'), dimension: 'leakage' });
    }
    if (!healthCheckResults.integrity?.integrity_ok && healthCheckResults.integrity) {
      issues.push({ severity: 'error', text: $t('readiness.integrityFailed'), dimension: 'integrity' });
    }
    if (readinessScore?.issues?.length > 0) {
      for (const issue of readinessScore.issues) {
        issues.push({ severity: issue.severity, text: issue.message, dimension: issue.dimension });
      }
    }
    if (overallScore < 50) {
      issues.push({ severity: 'error', text: $t('readiness.overallTooLow'), dimension: 'overall' });
    } else if (overallScore < 70) {
      issues.push({ severity: 'warning', text: $t('readiness.overallNeedsOpt'), dimension: 'overall' });
    }

    recommendations = readinessScore?.recommendations || [];
    if (overallScore >= 70 && overallScore < 85 && !recommendations.includes($t('readiness.runQualityAssess'))) {
      recommendations.push($t('readiness.runQualityAssessDetail'));
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
    if (score == null) return $t('readiness.pending');
    if (score >= 80) return $t('readiness.qualityGood', { score: score.toFixed(0) });
    if (score >= 60) return $t('readiness.qualityFair', { score: score.toFixed(0) });
    return $t('readiness.qualityPoor', { score: score.toFixed(0) });
  }

  function getIntegrityStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.integrity) return 'pending';
    return healthCheckResults.integrity.integrity_ok ? 'pass' : 'fail';
  }

  function getIntegrityMessage(): string {
    if (!healthCheckResults.integrity) return $t('readiness.pendingCheck');
    return healthCheckResults.integrity.integrity_ok ? $t('readiness.dataIntact') : $t('readiness.integrityIssue');
  }

  function getValidationStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.validation) return 'pending';
    return healthCheckResults.validation.valid ? 'pass' : 'fail';
  }

  function getValidationMessage(): string {
    if (!healthCheckResults.validation) return $t('readiness.pendingValidation');
    return healthCheckResults.validation.valid ? $t('readiness.formatCorrect') : $t('readiness.formatIssue');
  }

  function getLeakageStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.leakage) return 'pending';
    return healthCheckResults.leakage.leakage_detected === false ? 'pass'
      : healthCheckResults.leakage.leakage_detected === true ? 'fail' : 'pending';
  }

  function getLeakageMessage(): string {
    if (!healthCheckResults.leakage) return $t('readiness.pendingCheck');
    if (healthCheckResults.leakage.leakage_detected === false) return $t('readiness.noLeakage');
    if (healthCheckResults.leakage.leakage_detected === true) return $t('readiness.leakageDetected');
    return $t('readiness.unknown');
  }

  function getSufficiencyStatus(): ReadinessDimension['status'] {
    if (!healthCheckResults.sufficiency) return 'pending';
    return healthCheckResults.sufficiency.sufficient ? 'pass' : 'warn';
  }

  function getSufficiencyMessage(): string {
    if (!healthCheckResults.sufficiency) return $t('readiness.pending');
    if (healthCheckResults.sufficiency.sufficient) return $t('readiness.dataSufficient');
    return $t('readiness.dataInsufficient', { rows: healthCheckResults.sufficiency.current_rows?.toLocaleString() });
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
      case 'pass': return $t('readiness.pass');
      case 'warn': return $t('readiness.warn');
      case 'fail': return $t('readiness.fail');
      default: return $t('readiness.pendingLabel');
    }
  }

  function overallLabel(status: string): string {
    switch (status) {
      case 'ready': return $t('readiness.ready');
      case 'warning': return $t('readiness.warning');
      case 'not_ready': return $t('readiness.notReady');
      default: return $t('readiness.evaluating');
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
    <h4>{$t('readiness.title')}</h4>
    {#if overallStatus !== 'loading'}
      <div class="overall-status" style="color: {overallColor(overallStatus)}">
        {overallLabel(overallStatus)}
      </div>
    {:else}
      <div class="overall-status">{$t('readiness.evaluating')}</div>
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
        <span class="label-item"><span style="color:#10b981">●</span> ≥85 {$t('readiness.readyShort')}</span>
        <span class="label-item"><span style="color:#3b82f6">●</span> ≥70 {$t('readiness.usable')}</span>
        <span class="label-item"><span style="color:#f59e0b">●</span> ≥55 {$t('readiness.needsOpt')}</span>
        <span class="label-item"><span style="color:#ef4444">●</span> &lt;55 {$t('readiness.notRecommended')}</span>
      </div>
    </div>

    <div class="dimensions-section">
      <h5>{$t('readiness.dimensionDetails')}</h5>
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
      <h5>{$t('readiness.issuesFound', { count: issues.length })}</h5>
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
      <h5>💡 {$t('readiness.recommendations')}</h5>
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
