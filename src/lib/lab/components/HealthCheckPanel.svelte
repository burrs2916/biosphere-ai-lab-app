<script lang="ts">
  import { createEventDispatcher } from 'svelte';

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
      label: '数据完整性',
      icon: '🔒',
      getStatus: (r) => {
        if (!r.integrity) return 'pending';
        return r.integrity.integrity_ok ? 'passed' : 'failed';
      },
      getSummary: (r) => {
        if (!r.integrity) return '';
        return r.integrity.integrity_ok ? '数据完整性验证通过' : '数据完整性存在问题';
      },
      getDetails: (r) => {
        if (!r.integrity) return [];
        const d: DetailLine[] = [];
        d.push({ label: '校验和', value: r.integrity.checksum_match ? '匹配' : '不匹配', status: r.integrity.checksum_match ? 'good' : 'bad' });
        d.push({ label: '行数一致', value: r.integrity.row_count_consistent ? '一致' : '不一致', status: r.integrity.row_count_consistent ? 'good' : 'bad' });
        d.push({ label: '结构一致', value: r.integrity.schema_consistent ? '一致' : '不一致', status: r.integrity.schema_consistent ? 'good' : 'bad' });
        return d;
      },
    },
    {
      key: 'validation',
      label: '数据格式验证',
      icon: '✅',
      getStatus: (r) => {
        if (!r.validation) return 'pending';
        if (!r.validation.is_valid) return 'failed';
        if (r.validation.warnings?.length > 0) return 'warning';
        return 'passed';
      },
      getSummary: (r) => {
        if (!r.validation) return '';
        if (!r.validation.is_valid) return `验证失败，${r.validation.errors?.length || 0} 个错误`;
        if (r.validation.warnings?.length > 0) return `验证通过，${r.validation.warnings.length} 个警告`;
        return '数据格式验证通过';
      },
      getDetails: (r) => {
        if (!r.validation) return [];
        const d: DetailLine[] = [];
        d.push({ label: '格式验证', value: r.validation.is_valid ? '通过' : '失败', status: r.validation.is_valid ? 'good' : 'bad' });
        if (r.validation.warnings?.length > 0) {
          d.push({ label: '警告', value: `${r.validation.warnings.length} 项`, status: 'warn' });
        }
        if (r.validation.errors?.length > 0) {
          d.push({ label: '错误', value: `${r.validation.errors.length} 项`, status: 'bad' });
        }
        return d;
      },
    },
    {
      key: 'leakage',
      label: '数据泄露检测',
      icon: '💧',
      getStatus: (r) => {
        if (!r.leakage) return 'pending';
        if (r.leakage.leakage_detected) return 'failed';
        if (r.leakage.risk_level === 'medium') return 'warning';
        return 'passed';
      },
      getSummary: (r) => {
        if (!r.leakage) return '';
        if (r.leakage.leakage_detected) return '检测到数据泄露风险';
        return `风险等级: ${r.leakage.risk_level === 'low' ? '低' : r.leakage.risk_level === 'medium' ? '中' : '高'}`;
      },
      getDetails: (r) => {
        if (!r.leakage) return [];
        return [{ label: '泄露检测', value: r.leakage.leakage_detected ? '存在泄露' : '未检测到泄露', status: r.leakage.leakage_detected ? 'bad' : 'good' }];
      },
    },
    {
      key: 'sufficiency',
      label: '数据充分性',
      icon: '📏',
      getStatus: (r) => {
        if (!r.sufficiency) return 'pending';
        return r.sufficiency.is_sufficient ? 'passed' : 'failed';
      },
      getSummary: (r) => {
        if (!r.sufficiency) return '';
        return r.sufficiency.is_sufficient
          ? `数据充足 (${r.sufficiency.current_rows?.toLocaleString()} 行)`
          : `数据不足 (需 ${r.sufficiency.estimated_required?.toLocaleString()} 行)`;
      },
      getDetails: (r) => {
        if (!r.sufficiency) return [];
        const d: DetailLine[] = [];
        d.push({ label: '当前行数', value: r.sufficiency.current_rows?.toLocaleString() || '-', status: 'neutral' });
        d.push({ label: '所需行数', value: r.sufficiency.estimated_required?.toLocaleString() || '-', status: 'neutral' });
        d.push({ label: '裕度', value: r.sufficiency.margin ? `${(r.sufficiency.margin * 100).toFixed(0)}%` : '-', status: r.sufficiency.margin >= 0.2 ? 'good' : 'warn' });
        return d;
      },
    },
    {
      key: 'readiness',
      label: '训练就绪评分',
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
        return `就绪评分 ${r.readiness.overall_score} 分 — ${r.readiness.readiness_level === 'ready' ? '就绪' : r.readiness.readiness_level === 'almost_ready' ? '基本就绪' : '未就绪'}`;
      },
      getDetails: (r) => {
        if (!r.readiness) return [];
        const d: DetailLine[] = [];
        if (r.readiness.dimensions) {
          for (const [key, val] of Object.entries(r.readiness.dimensions)) {
            const v = val as any;
            d.push({ label: v.message || key, value: `${v.score}分`, status: v.status === 'pass' ? 'good' : v.status === 'warn' ? 'warn' : 'bad' });
          }
        }
        return d;
      },
    },
    {
      key: 'splits',
      label: '数据划分',
      icon: '📐',
      getStatus: (r) => {
        if (!r.splits) return 'pending';
        return r.splits.length > 0 ? 'passed' : 'warning';
      },
      getSummary: (r) => {
        if (!r.splits) return '';
        return r.splits.length > 0 ? `${r.splits.length} 个数据划分` : '尚未创建数据划分';
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
      <span class="check-progress-text">{completedCount}/{totalCount} 项检查完成</span>
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
          <div class="check-running-hint">正在检查...</div>
        {/if}
      </div>
    {/each}
  </div>

  {#if !healthCheckRunning && Object.keys(healthCheckResults).length === 0}
    <div class="check-empty">
      <p>点击"全面检查"评估数据集是否准备好用于训练</p>
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
