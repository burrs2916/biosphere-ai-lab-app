<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let versions: any[] = [];
  export let currentVersion: string = '';
  export let diffData: any = null;
  export let loading = false;

  const dispatch = createEventDispatcher();

  let selectedFrom: string = '';
  let selectedTo: string = '';
  let showDiff = false;

  $: sortedVersions = [...versions].sort((a, b) =>
    new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
  );

  function requestDiff() {
    if (selectedFrom && selectedTo) {
      showDiff = true;
      dispatch('diff', { fromVersion: selectedFrom, toVersion: selectedTo });
    }
  }

  function rollback(version: string) {
    dispatch('rollback', { version });
  }

  function formatTime(dateStr: string): string {
    const d = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;
    return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }

  function changeType(change: any): string {
    if (!change) return 'unknown';
    if (change.added > 0 && change.removed === 0) return 'added';
    if (change.removed > 0 && change.added === 0) return 'removed';
    if (change.modified > 0) return 'modified';
    return 'unchanged';
  }

  function changeIcon(type: string): string {
    switch (type) {
      case 'added': return '➕';
      case 'removed': return '➖';
      case 'modified': return '✏️';
      default: return '—';
    }
  }

  function changeColor(type: string): string {
    switch (type) {
      case 'added': return '#6ee7b7';
      case 'removed': return '#fca5a5';
      case 'modified': return '#93c5fd';
      default: return '#94a3b8';
    }
  }
</script>

<div class="version-panel">
  <div class="version-header">
    <h4>版本历史</h4>
    <span class="version-count">{versions.length} 个版本</span>
  </div>

  {#if versions.length === 0}
    <div class="empty-state">
      <span>暂无版本记录</span>
    </div>
  {:else}
    <div class="version-timeline">
      {#each sortedVersions as ver, i}
        <div class="version-item" class:current={ver.version === currentVersion}>
          <div class="version-dot" class:current={ver.version === currentVersion}></div>
          {#if i < sortedVersions.length - 1}
            <div class="version-line"></div>
          {/if}
          <div class="version-content">
            <div class="version-main">
              <span class="version-tag">{ver.version}</span>
              {#if ver.version === currentVersion}
                <span class="current-badge">当前</span>
              {/if}
              <span class="version-time">{formatTime(ver.created_at)}</span>
            </div>
            <div class="version-meta">
              <span>{ver.rows?.toLocaleString() || '?'} 行</span>
              <span>·</span>
              <span>{ver.columns || '?'} 列</span>
              <span>·</span>
              <span>{ver.memory_size_mb?.toFixed(1) || '?'} MB</span>
            </div>
            {#if ver.change_note}
              <div class="version-note">{ver.change_note}</div>
            {/if}
            {#if ver.version !== currentVersion}
              <button class="btn-rollback" on:click={() => rollback(ver.version)}>
                ↩ 回滚到此版本
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <div class="diff-section">
      <h5>版本对比</h5>
      <div class="diff-controls">
        <select bind:value={selectedFrom} class="diff-select">
          <option value="">选择旧版本</option>
          {#each sortedVersions as ver}
            <option value={ver.version}>{ver.version}</option>
          {/each}
        </select>
        <span class="diff-arrow">→</span>
        <select bind:value={selectedTo} class="diff-select">
          <option value="">选择新版本</option>
          {#each sortedVersions as ver}
            <option value={ver.version}>{ver.version}</option>
          {/each}
        </select>
        <button class="btn-diff" on:click={requestDiff} disabled={!selectedFrom || !selectedTo || loading}>
          {loading ? '对比中...' : '对比'}
        </button>
      </div>

      {#if showDiff && diffData}
        <div class="diff-result">
          {#if diffData.summary}
            <div class="diff-summary">
              {#each Object.entries(diffData.summary) as [key, val]}
                <div class="summary-item">
                  <span class="summary-label">{key}</span>
                  <span class="summary-value">{String(val)}</span>
                </div>
              {/each}
            </div>
          {/if}

          {#if diffData.column_changes?.length > 0}
            <div class="column-changes">
              <h6>列变更</h6>
              {#each diffData.column_changes as change}
                {@const ct = changeType(change)}
                <div class="column-change" style="border-left: 3px solid {changeColor(ct)}">
                  <span class="change-icon">{changeIcon(ct)}</span>
                  <span class="change-name">{change.column || change.name}</span>
                  <span class="change-type" style="color: {changeColor(ct)}">{ct}</span>
                  {#if change.details}
                    <span class="change-details">{change.details}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          {#if diffData.row_changes}
            <div class="row-changes">
              <h6>行变更</h6>
              <div class="row-stats">
                {#if diffData.row_changes.added > 0}
                  <span class="stat-added">+{diffData.row_changes.added} 行新增</span>
                {/if}
                {#if diffData.row_changes.removed > 0}
                  <span class="stat-removed">-{diffData.row_changes.removed} 行删除</span>
                {/if}
                {#if diffData.row_changes.modified > 0}
                  <span class="stat-modified">~{diffData.row_changes.modified} 行修改</span>
                {/if}
              </div>
            </div>
          {/if}

          {#if diffData.sample_rows?.length > 0}
            <div class="sample-diff">
              <h6>变更示例</h6>
              <div class="diff-table-wrapper">
                <table class="diff-table">
                  <thead>
                    <tr>
                      <th>类型</th>
                      {#each diffData.sample_columns || ['列1', '列2'] as col}
                        <th>{col}</th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each diffData.sample_rows as row}
                      <tr class="diff-row-{row.type || 'unchanged'}">
                        <td class="diff-type-cell">{changeIcon(row.type)}</td>
                        {#each row.values || [] as val}
                          <td>{val}</td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>
          {/if}
        </div>
      {:else if showDiff && loading}
        <div class="diff-loading">
          <span class="spinner"></span>
          <span>正在对比版本差异...</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .version-panel {
    padding: 0.75rem;
  }

  .version-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 1rem;
  }

  .version-header h4 { margin: 0; font-size: 0.9rem; color: #e2e8f0; }

  .version-count { font-size: 0.72rem; color: #94a3b8; }

  .empty-state {
    text-align: center; padding: 2rem; color: #64748b; font-size: 0.85rem;
  }

  .version-timeline {
    position: relative;
  }

  .version-item {
    display: flex; position: relative; padding-bottom: 1rem;
  }

  .version-item:last-child { padding-bottom: 0; }

  .version-dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: #475569; border: 2px solid #334155;
    position: relative; z-index: 1; flex-shrink: 0; margin-top: 4px;
  }

  .version-dot.current {
    background: #3b82f6; border-color: #2563eb;
    box-shadow: 0 0 6px rgba(59,130,246,0.4);
  }

  .version-line {
    position: absolute; left: 4px; top: 14px; bottom: 0;
    width: 2px; background: #334155;
  }

  .version-content {
    margin-left: 0.75rem; flex: 1;
  }

  .version-main {
    display: flex; align-items: center; gap: 0.4rem;
  }

  .version-tag {
    font-size: 0.8rem; font-weight: 600; color: #e2e8f0;
    font-family: 'SF Mono', monospace;
  }

  .current-badge {
    font-size: 0.6rem; padding: 0.1rem 0.35rem; border-radius: 3px;
    background: rgba(59,130,246,0.15); color: #93c5fd; font-weight: 500;
  }

  .version-time { font-size: 0.7rem; color: #64748b; }

  .version-meta {
    font-size: 0.7rem; color: #94a3b8; margin-top: 0.15rem;
    display: flex; gap: 0.25rem;
  }

  .version-note {
    font-size: 0.72rem; color: #cbd5e1; margin-top: 0.2rem;
    padding: 0.2rem 0.4rem; background: rgba(255,255,255,0.03);
    border-radius: 3px;
  }

  .btn-rollback {
    margin-top: 0.3rem; padding: 0.2rem 0.5rem; border-radius: 4px;
    background: none; border: 1px solid rgba(234,179,8,0.25);
    color: #fbbf24; font-size: 0.68rem; cursor: pointer;
  }

  .btn-rollback:hover { background: rgba(234,179,8,0.06); }

  .diff-section {
    margin-top: 1.25rem; padding-top: 1rem;
    border-top: 1px solid rgba(107,114,128,0.15);
  }

  .diff-section h5 { margin: 0 0 0.75rem; font-size: 0.85rem; color: #e2e8f0; }

  .diff-controls {
    display: flex; align-items: center; gap: 0.5rem;
  }

  .diff-select {
    padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.04);
    border: 1px solid rgba(107,114,128,0.2); border-radius: 4px;
    color: #d1d5db; font-size: 0.75rem;
  }

  .diff-arrow { color: #64748b; font-size: 0.85rem; }

  .btn-diff {
    padding: 0.3rem 0.65rem; border-radius: 4px;
    background: rgba(59,130,246,0.12); border: 1px solid rgba(59,130,246,0.25);
    color: #93c5fd; font-size: 0.75rem; cursor: pointer;
  }

  .btn-diff:hover:not(:disabled) { background: rgba(59,130,246,0.18); }
  .btn-diff:disabled { opacity: 0.4; cursor: not-allowed; }

  .diff-result { margin-top: 0.75rem; }

  .diff-summary {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.4rem; margin-bottom: 0.75rem;
  }

  .summary-item {
    padding: 0.35rem 0.5rem; background: rgba(255,255,255,0.03);
    border-radius: 4px;
  }

  .summary-label { font-size: 0.68rem; color: #94a3b8; display: block; }
  .summary-value { font-size: 0.8rem; color: #e2e8f0; font-weight: 500; }

  .column-changes { margin-bottom: 0.75rem; }
  .column-changes h6 { margin: 0 0 0.4rem; font-size: 0.78rem; color: #cbd5e1; }

  .column-change {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.25rem 0.5rem; margin-bottom: 0.2rem;
    background: rgba(255,255,255,0.02); border-radius: 3px;
  }

  .change-icon { font-size: 0.7rem; }
  .change-name { font-size: 0.75rem; color: #e2e8f0; font-family: 'SF Mono', monospace; }
  .change-type { font-size: 0.68rem; text-transform: uppercase; }
  .change-details { font-size: 0.68rem; color: #94a3b8; margin-left: auto; }

  .row-changes { margin-bottom: 0.75rem; }
  .row-changes h6 { margin: 0 0 0.4rem; font-size: 0.78rem; color: #cbd5e1; }

  .row-stats { display: flex; gap: 0.75rem; }

  .stat-added { color: #6ee7b7; font-size: 0.78rem; }
  .stat-removed { color: #fca5a5; font-size: 0.78rem; }
  .stat-modified { color: #93c5fd; font-size: 0.78rem; }

  .sample-diff h6 { margin: 0 0 0.4rem; font-size: 0.78rem; color: #cbd5e1; }

  .diff-table-wrapper { overflow-x: auto; }

  .diff-table {
    width: 100%; border-collapse: collapse; font-size: 0.72rem;
  }

  .diff-table th {
    text-align: left; padding: 0.3rem 0.5rem;
    background: rgba(255,255,255,0.03); color: #94a3b8;
    border-bottom: 1px solid rgba(107,114,128,0.15);
  }

  .diff-table td {
    padding: 0.25rem 0.5rem; border-bottom: 1px solid rgba(107,114,128,0.08);
    color: #d1d5db;
  }

  .diff-type-cell { width: 28px; text-align: center; }

  .diff-row-added td { background: rgba(16,185,129,0.04); }
  .diff-row-removed td { background: rgba(239,68,68,0.04); }
  .diff-row-modified td { background: rgba(59,130,246,0.04); }

  .diff-loading {
    display: flex; align-items: center; gap: 0.5rem;
    padding: 1rem; color: #94a3b8; font-size: 0.82rem;
  }

  .spinner {
    width: 16px; height: 16px; border: 2px solid rgba(107,114,128,0.2);
    border-top-color: #3b82f6; border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
