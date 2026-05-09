<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from '$lib/i18n';

  export let healthCheckRunning = false;
  export let healthCheckResults: Record<string, any> = {};
  export let healthCheckError: string | null = null;

  const dispatch = createEventDispatcher();

  type CheckStatus = 'pending' | 'running' | 'passed' | 'warning' | 'failed';

  interface CheckItem {
    key: string;
    label: string;
    icon: string;
    getStatus: (results: Record<string, any>) => CheckStatus;
    getSummary: (results: Record<string, any>) => string;
    getDetails: (results: Record<string, any>) => DetailLine[];
  }

  interface DetailLine {
    label: string;
    value: string;
    status: 'good' | 'warn' | 'bad' | 'neutral';
  }

  const checkItems: CheckItem[] = [
    {
      key: 'integrity',
      label: $t('healthCheck.integrity'),
      icon: '🔒',
      getStatus: (r) => {
        if (!r.integrity) return 'pending';
        return r.integrity.integrity_ok ? 'passed' : 'failed';
      },
      getSummary: (r) => {
        if (!r.integrity) return '';
        return r.integrity.integrity_ok ? $t('healthCheck.integrityPass') : $t('healthCheck.integrityFail');
      },
      getDetails: (r) => {
        if (!r.integrity) return [];
        const d: DetailLine[] = [];
        d.push({ label: $t('healthCheck.checksum'), value: r.integrity.checksum_match ? $t('healthCheck.match') : $t('healthCheck.mismatch'), status: r.integrity.checksum_match ? 'good' : 'bad' });
        d.push({ label: $t('healthCheck.rowCountConsistent'), value: r.integrity.row_count_consistent ? $t('healthCheck.match') : $t('healthCheck.mismatch'), status: r.integrity.row_count_consistent ? 'good' : 'bad' });
        d.push({ label: $t('healthCheck.schemaConsistent'), value: r.integrity.schema_consistent ? $t('healthCheck.match') : $t('healthCheck.mismatch'), status: r.integrity.schema_consistent ? 'good' : 'bad' });
        return d;
      },
    },
    {
      key: 'validation',
      label: $t('healthCheck.validation'),
      icon: '✅',
      getStatus: (r) => {
        if (!r.validation) return 'pending';
        if (!r.validation.is_valid) return 'failed';
        if (r.validation.warnings?.length > 0) return 'warning';
        return 'passed';
      },
      getSummary: (r) => {
        if (!r.validation) return '';
        if (!r.validation.is_valid) return $t('healthCheck.validationFailed', { count: r.validation.errors?.length || 0 });
        if (r.validation.warnings?.length > 0) return $t('healthCheck.validationWarn', { count: r.validation.warnings.length });
        return $t('healthCheck.validationPass');
      },
      getDetails: (r) => {
        if (!r.validation) return [];
        const d: DetailLine[] = [];
        d.push({ label: $t('healthCheck.formatValidation'), value: r.validation.is_valid ? $t('healthCheck.pass') : $t('healthCheck.fail'), status: r.validation.is_valid ? 'good' : 'bad' });
        if (r.validation.warnings?.length > 0) {
          d.push({ label: $t('healthCheck.warnings'), value: `${r.validation.warnings.length}`, status: 'warn' });
        }
        if (r.validation.errors?.length > 0) {
          d.push({ label: $t('healthCheck.errors'), value: `${r.validation.errors.length}`, status: 'bad' });
        }
        return d;
      },
    },
    {
      key: 'leakage',
      label: $t('healthCheck.leakage'),
      icon: '💧',
      getStatus: (r) => {
        if (!r.leakage) return 'pending';
        if (r.leakage.leakage_detected) return 'failed';
        if (r.leakage.risk_level === 'medium') return 'warning';
        return 'passed';
      },
      getSummary: (r) => {
        if (!r.leakage) return '';
        if (r.leakage.leakage_detected) return $t('healthCheck.leakageRiskDetected');
        return $t('healthCheck.riskLevel', { level: r.leakage.risk_level === 'low' ? $t('healthCheck.low') : r.leakage.risk_level === 'medium' ? $t('healthCheck.medium') : $t('healthCheck.high') });
      },
      getDetails: (r) => {
        if (!r.leakage) return [];
        return [{ label: $t('healthCheck.leakageDetection'), value: r.leakage.leakage_detected ? $t('healthCheck.leakageFound') : $t('healthCheck.noLeakageFound'), status: r.leakage.leakage_detected ? 'bad' : 'good' }];
      },
    },
    {
      key: 'sufficiency',
      label: $t('healthCheck.sufficiency'),
      icon: '📏',
      getStatus: (r) => {
        if (!r.sufficiency) return 'pending';
        return r.sufficiency.is_sufficient ? 'passed' : 'failed';
      },
      getSummary: (r) => {
        if (!r.sufficiency) return '';
        return r.sufficiency.is_sufficient
          ? $t('healthCheck.dataSufficient', { rows: r.sufficiency.current_rows?.toLocaleString() })
          : $t('healthCheck.dataInsufficient', { rows: r.sufficiency.estimated_required?.toLocaleString() });
      },
      getDetails: (r) => {
        if (!r.sufficiency) return [];
        const d: DetailLine[] = [];
        d.push({ label: $t('healthCheck.currentRows'), value: r.sufficiency.current_rows?.toLocaleString() || '-', status: 'neutral' });
        d.push({ label: $t('healthCheck.requiredRows'), value: r.sufficiency.estimated_required?.toLocaleString() || '-', status: 'neutral' });
        d.push({ label: $t('healthCheck.margin'), value: r.sufficiency.margin ? `${(r.sufficiency.margin * 100).toFixed(0)}%` : '-', status: r.sufficiency.margin >= 0.2 ? 'good' : 'warn' });
        return d;
      },
    },
    {
      key: 'readiness',
      label: $t('healthCheck.readiness'),
      icon: '🎯',
      getStatus: (r) => {
        if (!r.readiness) return 'pending';
        const s = r.readiness.overall_score;
        if (s >= 70) return 'passed';
        if (s >= 40) return 'warning';
        return 'failed';
      },
      getSummary: (r) => {
        if (!r.readiness) return '';
        return $t('healthCheck.readinessScore', { score: r.readiness.overall_score, level: r.readiness.readiness_level === 'ready' ? $t('healthCheck.ready') : r.readiness.readiness_level === 'almost_ready' ? $t('healthCheck.almostReady') : $t('healthCheck.notReady') });
      },
      getDetails: (r) => {
        if (!r.readiness) return [];
        const d: DetailLine[] = [];
        if (r.readiness.dimensions) {
          for (const [key, val] of Object.entries(r.readiness.dimensions)) {
            const v = val as any;
            d.push({ label: v.message || key, value: `${v.score}`, status: v.status === 'pass' ? 'good' : v.status === 'warn' ? 'warn' : 'bad' });
          }
        }
        return d;
      },
    },
    {
      key: 'splits',
      label: $t('healthCheck.splits'),
      icon: '📐',
      getStatus: (r) => {
        if (!r.splits) return 'pending';
        return r.splits.length > 0 ? 'passed' : 'warning';
      },
      getSummary: (r) => {
        if (!r.splits) return '';
        return r.splits.length > 0 ? $t('healthCheck.splitsCount', { count: r.splits.length }) : $t('healthCheck.noSplits');
      },
      getDetails: (r) => {
        if (!r.splits || r.splits.length === 0) return [];
        return r.splits.map((s: any) => ({
          label: s.name,
          value: `train:${s.splits.train.rows} val:${s.splits.val.rows} test:${s.splits.test.rows}`,
          status: 'neutral' as const,
        }));
      },
    },
  ];

  $: checkOrder = getCheckOrder();

  function getCheckOrder(): string[] {
    return ['integrity', 'validation', 'leakage', 'sufficiency', 'readiness', 'splits'];
  }

  function getItemStatus(key: string): CheckStatus {
    const item = checkItems.find(c => c.key === key);
    if (!item) return 'pending';
    if (healthCheckRunning && !healthCheckResults[key]) {
      const idx = checkOrder.indexOf(key);
      const prevKey = idx > 0 ? checkOrder[idx - 1] : null;
      if (idx === 0 || (prevKey && healthCheckResults[prevKey])) {
        return 'running';
      }
      return 'pending';
    }
    return item.getStatus(healthCheckResults);
  }

  function statusIcon(status: CheckStatus): string {
    switch (status) {
      case 'pending': return '⏳';
      case 'running': return '⏳';
      case 'passed': return '✅';
      case 'warning': return '⚠️';
      case 'failed': return '❌';
    }
  }

  function statusClass(status: CheckStatus): string {
    return `check-${status}`;
  }

  $: completedCount = checkOrder.filter(k => healthCheckResults[k]).length;
  $: totalCount = checkOrder.length;
  $: overallProgress = totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0;
</script>

<div class="health-check-panel">
  {#if healthCheckRunning}
    <div class="check-progress-bar">
      <div class="check-progress-fill" style="width: {overallProgress}%"></div>
      <span class="check-progress-text">{$t('healthCheck.checkProgress', { completed: completedCount, total: totalCount })}</span>
    </div>
  {/if}

  {#if healthCheckError}
    <div class="check-error">
      <span>❌</span>
      <span>{healthCheckError}</span>
    </div>
  {/if}

  <div class="check-list">
    {#each checkItems as item}
      {@const status = getItemStatus(item.key)}
      <div class="check-item {statusClass(status)}">
        <div class="check-header">
          <span class="check-icon">{item.icon}</span>
          <span class="check-label">{item.label}</span>
          <span class="check-status">
            {#if status === 'running'}
              <span class="spinner"></span>
            {:else}
              {statusIcon(status)}
            {/if}
          </span>
        </div>

        {#if status !== 'pending'}
          <div class="check-summary">{item.getSummary(healthCheckResults)}</div>
        {/if}

        {#if status !== 'pending' && status !== 'running' && item.getDetails(healthCheckResults).length > 0}
          <div class="check-details">
            {#each item.getDetails(healthCheckResults) as detail}
              <div class="detail-row">
                <span class="detail-label">{detail.label}</span>
                <span class="detail-value" class:good={detail.status === 'good'} class:warn={detail.status === 'warn'} class:bad={detail.status === 'bad'}>{detail.value}</span>
              </div>
            {/each}
          </div>
        {/if}

        {#if status === 'running'}
          <div class="check-running-hint">{$t('healthCheck.checking')}</div>
        {/if}
      </div>
    {/each}
  </div>

  {#if !healthCheckRunning && Object.keys(healthCheckResults).length === 0}
    <div class="check-empty">
      <p>{$t('healthCheck.clickToCheck')}</p>
    </div>
  {/if}
</div>

<style>
  .health-check-panel { display: flex; flex-direction: column; gap: 0.5rem; }

  .check-progress-bar {
    position: relative; width: 100%; height: 24px;
    background: rgba(255, 255, 255, 0.04); border-radius: 6px;
    overflow: hidden; margin-bottom: 0.25rem;
  }

  .check-progress-fill {
    height: 100%; background: linear-gradient(90deg, #3b82f6, #8b5cf6);
    border-radius: 6px; transition: width 0.4s ease;
  }

  .check-progress-text {
    position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%);
    font-size: 0.68rem; font-weight: 600; color: #e2e8f0;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .check-error {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.5rem; border-radius: 6px;
    background: rgba(239, 68, 68, 0.06); border: 1px solid rgba(239, 68, 68, 0.15);
    font-size: 0.75rem; color: #fca5a5;
  }

  .check-list { display: flex; flex-direction: column; gap: 0.35rem; }

  .check-item {
    padding: 0.5rem 0.6rem; border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.06);
    background: rgba(255, 255, 255, 0.015);
    transition: border-color 0.2s, background 0.2s;
  }

  .check-item.check-running {
    border-color: rgba(59, 130, 246, 0.2);
    background: rgba(59, 130, 246, 0.04);
  }

  .check-item.check-passed {
    border-color: rgba(16, 185, 129, 0.12);
  }

  .check-item.check-warning {
    border-color: rgba(245, 158, 11, 0.12);
    background: rgba(245, 158, 11, 0.02);
  }

  .check-item.check-failed {
    border-color: rgba(239, 68, 68, 0.12);
    background: rgba(239, 68, 68, 0.02);
  }

  .check-header {
    display: flex; align-items: center; gap: 0.4rem;
  }

  .check-icon { font-size: 0.85rem; }
  .check-label { flex: 1; font-size: 0.78rem; font-weight: 500; color: #e2e8f0; }
  .check-status { font-size: 0.8rem; }

  .spinner {
    display: inline-block; width: 14px; height: 14px;
    border: 2px solid rgba(59, 130, 246, 0.2);
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .check-summary {
    font-size: 0.7rem; color: #94a3b8; margin-top: 0.2rem;
    padding-left: 1.5rem;
  }

  .check-details {
    margin-top: 0.3rem; padding-left: 1.5rem;
    display: flex; flex-direction: column; gap: 0.15rem;
  }

  .detail-row {
    display: flex; justify-content: space-between; align-items: center;
    font-size: 0.68rem;
  }

  .detail-label { color: #64748b; }
  .detail-value { font-weight: 500; color: #cbd5e1; }
  .detail-value.good { color: #10b981; }
  .detail-value.warn { color: #f59e0b; }
  .detail-value.bad { color: #ef4444; }

  .check-running-hint {
    font-size: 0.65rem; color: #60a5fa; padding-left: 1.5rem;
    margin-top: 0.15rem;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }

  .check-empty {
    text-align: center; padding: 1.5rem; color: #64748b; font-size: 0.8rem;
  }
</style>
