<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';
  import { t } from '$lib/i18n';

  export let datasetId: string = '';

  let card: any = null;
  let stats: any = null;
  let searchQuery = '';
  let searchResults: any = null;
  let searchLoading = false;
  let cardLoading = false;
  let statsLoading = false;
  let error: string | null = null;
  let editing = false;
  let editCard: any = {};

  async function loadCard() {
    if (!datasetId) return;
    cardLoading = true; error = null;
    try {
      const client = getLabClient();
      card = await client.datasetGetCard(datasetId);
    } catch (e: any) {
      error = e?.toString() || $t('cardPanel.loadCardFailed');
    } finally { cardLoading = false; }
  }

  async function saveCard() {
    if (!datasetId) return;
    cardLoading = true; error = null;
    const taskId = taskManagerStore.createTask($t('cardPanel.saveCard'), $t('cardPanel.updatingCard'), false);
    try {
      const client = getLabClient();
      await client.datasetSetCard(datasetId, editCard);
      card = { ...editCard };
      editing = false;
      taskManagerStore.completeTask(taskId, $t('cardPanel.cardUpdated'));
    } catch (e: any) {
      error = e?.toString() || $t('cardPanel.saveFailed');
      taskManagerStore.failTask(taskId, error || $t('task.unknownError'));
    } finally { cardLoading = false; }
  }

  async function loadStats() {
    if (!datasetId) return;
    statsLoading = true; error = null;
    try {
      const client = getLabClient();
      stats = await client.datasetUsageStats(datasetId);
    } catch (e: any) {
      error = e?.toString() || $t('cardPanel.loadStatsFailed');
    } finally { statsLoading = false; }
  }

  async function search() {
    if (!searchQuery.trim()) return;
    searchLoading = true; error = null;
    try {
      const client = getLabClient();
      searchResults = await client.datasetDiscoverySearch(searchQuery);
    } catch (e: any) {
      error = e?.toString() || $t('cardPanel.searchFailed');
    } finally { searchLoading = false; }
  }

  function startEdit() {
    editCard = card ? { ...card } : { name: '', description: '', license: '', citation: '', usage_notes: '' };
    editing = true;
  }

  $: if (datasetId && !card && !cardLoading) loadCard();
  $: if (datasetId && !stats && !statsLoading) loadStats();
</script>

<div class="dataset-card-panel">
  <div class="panel-sections">
    <div class="card-section">
      <div class="section-header">
        <h3>{$t('cardPanel.cardTitle')}</h3>
        <button class="btn-sm" on:click={startEdit}>{$t('cardPanel.edit')}</button>
      </div>

      {#if editing}
        <div class="edit-form">
          <div class="form-group"><label for="card-name">{$t('cardPanel.name')}</label><input id="card-name" class="input" type="text" bind:value={editCard.name} /></div>
          <div class="form-group"><label for="card-desc">{$t('cardPanel.description')}</label><textarea id="card-desc" class="input textarea" bind:value={editCard.description}></textarea></div>
          <div class="form-row">
            <div class="form-group flex-1"><label for="card-license">{$t('cardPanel.license')}</label><input id="card-license" class="input" type="text" bind:value={editCard.license} /></div>
              <div class="form-group flex-1"><label for="card-homepage">{$t('cardPanel.homepage')}</label><input id="card-homepage" class="input" type="text" bind:value={editCard.homepage} /></div>
          </div>
          <div class="form-group"><label for="card-citation">{$t('cardPanel.citation')}</label><input id="card-citation" class="input" type="text" bind:value={editCard.citation} /></div>
            <div class="form-group"><label for="card-usage">{$t('cardPanel.usageNotes')}</label><textarea id="card-usage" class="input textarea" bind:value={editCard.usage_notes}></textarea></div>
            <div class="form-group"><label for="card-issues">{$t('cardPanel.knownIssues')}</label><input id="card-issues" class="input" type="text" bind:value={editCard.known_issues} /></div>
          <div class="edit-actions">
            <button class="btn-primary-sm" on:click={saveCard}>💾 {$t('cardPanel.save')}</button>
              <button class="btn-sm" on:click={() => (editing = false)}>{$t('confirm.cancel')}</button>
          </div>
        </div>
      {:else if card}
        <div class="card-display">
          <div class="card-title">{card.name || $t('cardPanel.unnamedDataset')}</div>
          <div class="card-desc">{card.description || $t('cardPanel.noDescription')}</div>

          <div class="card-meta">
            {#if card.license}<span class="meta-tag">📜 {card.license}</span>{/if}
            {#if card.size_categories}<span class="meta-tag">📏 {card.size_categories}</span>{/if}
            {#if card.languages}<span class="meta-tag">🌐 {card.languages?.join(', ')}</span>{/if}
            {#if card.quality_score != null}
              <span class="meta-tag quality" style="background: {card.quality_score >= 80 ? 'rgba(16,185,129,0.12)' : 'rgba(245,158,11,0.12)'}; color: {card.quality_score >= 80 ? '#10b981' : '#f59e0b'}">
                {$t('cardPanel.quality')} {card.quality_score}/100
              </span>
            {/if}
          </div>

          {#if card.task_categories?.length}
            <div class="card-tags">
              {#each card.task_categories as tag}
                <span class="task-tag">{tag}</span>
              {/each}
            </div>
          {/if}

          {#if card.known_issues?.length}
            <div class="card-issues">
              <h4>⚠️ {$t('cardPanel.knownIssuesTitle')}</h4>
              {#each card.known_issues as issue}
                <div class="issue-item">{issue}</div>
              {/each}
            </div>
          {/if}

          {#if card.citation}
            <div class="card-citation">
              <h4>📖 {$t('cardPanel.citation')}</h4>
              <code>{card.citation}</code>
            </div>
          {/if}

          {#if card.usage_notes}
            <div class="card-usage">
              <h4>📝 {$t('cardPanel.usageNotesTitle')}</h4>
              <p>{card.usage_notes}</p>
            </div>
          {/if}
        </div>
      {:else}
        <div class="empty-state">{$t('common.loading')}</div>
      {/if}
    </div>

    <div class="stats-section">
      <div class="section-header">
        <h3>{$t('cardPanel.usageStats')}</h3>
        <button class="btn-sm" on:click={loadStats}>🔄</button>
      </div>
      {#if stats}
        <div class="stats-grid">
          <div class="stat-card">
            <div class="stat-value">{stats.total_experiments}</div>
            <div class="stat-label">{$t('cardPanel.totalExperiments')}</div>
          </div>
          <div class="stat-card">
            <div class="stat-value" style="color: #3b82f6">{stats.active_experiments}</div>
            <div class="stat-label">{$t('cardPanel.inProgress')}</div>
          </div>
          <div class="stat-card">
            <div class="stat-value" style="color: #10b981">{stats.completed_experiments}</div>
            <div class="stat-label">{$t('cardPanel.completed')}</div>
          </div>
          <div class="stat-card">
            <div class="stat-value" style="color: #ef4444">{stats.failed_experiments}</div>
            <div class="stat-label">{$t('cardPanel.failed')}</div>
          </div>
        </div>
        {#if stats.avg_accuracy != null}
          <div class="avg-accuracy">{$t('cardPanel.avgAccuracy')}: <strong>{(stats.avg_accuracy * 100).toFixed(1)}%</strong></div>
        {/if}
        {#if stats.usage_timeline?.length}
          <div class="timeline-chart">
            {#each stats.usage_timeline as item, i}
              {@const maxExp = Math.max(...stats.usage_timeline.map((t: any) => t.experiments))}
              <div class="timeline-bar" style="height: {(item.experiments / maxExp) * 60}px">
                <span class="timeline-val">{item.experiments}</span>
              </div>
              <span class="timeline-label">{$t('cardPanel.month', { month: item.date.split('-')[1] })}</span>
            {/each}
          </div>
        {/if}
      {:else}
        <div class="empty-state">{$t('common.loading')}</div>
      {/if}
    </div>
  </div>

  <div class="discovery-section">
    <div class="section-header">
      <h3>{$t('cardPanel.datasetDiscovery')}</h3>
    </div>
    <div class="search-form">
      <input class="input search-input" type="text" bind:value={searchQuery} placeholder={$t('cardPanel.searchPlaceholder')} on:keydown={(e) => e.key === 'Enter' && search()} />
      <button class="btn-primary-sm" on:click={search} disabled={searchLoading || !searchQuery.trim()}>
        {searchLoading ? '...' : '🔍'}
      </button>
    </div>
    {#if searchResults}
      <div class="search-results">
        <div class="results-count">{$t('cardPanel.foundResults', { count: searchResults.total_results })}</div>
        {#each searchResults.results as result}
          <div class="result-item">
            <div class="result-name">{result.name}</div>
            <div class="result-meta">
              <span>{result.format}</span>
              <span>{result.rows?.toLocaleString()} {$t('versionDiff.rows')}</span>
              <span class="relevance">{$t('cardPanel.relevance')} {(result.relevance * 100).toFixed(0)}%</span>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if error}
    <div class="error-box">{error}</div>
  {/if}
</div>

<style>
  .dataset-card-panel { padding: 0; }
  .panel-sections { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; margin-bottom: 0.75rem; }

  .section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
  .section-header h3 { font-size: 0.9rem; margin: 0; }

  .btn-sm { padding: 0.2rem 0.5rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.72rem; cursor: pointer; }
  .btn-sm:hover { background: rgba(255,255,255,0.1); }
  .btn-primary-sm { padding: 0.3rem 0.6rem; border: none; border-radius: 4px; background: #3b82f6; color: #fff; font-size: 0.72rem; font-weight: 600; cursor: pointer; }
  .btn-primary-sm:disabled { opacity: 0.5; cursor: not-allowed; }

  .form-group { margin-bottom: 0.4rem; }
  .form-group.flex-1 { flex: 1; }
  .form-group label { display: block; font-size: 0.72rem; color: #9ca3af; margin-bottom: 0.15rem; }
  .form-row { display: flex; gap: 0.4rem; }
  .input { width: 100%; padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.78rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }
  .textarea { min-height: 50px; resize: vertical; font-family: inherit; }
  .edit-actions { display: flex; gap: 0.4rem; margin-top: 0.4rem; }

  .card-display { min-height: 0; }
  .card-title { font-size: 1rem; font-weight: 700; color: #e5e7eb; margin-bottom: 0.3rem; }
  .card-desc { font-size: 0.8rem; color: #9ca3af; margin-bottom: 0.5rem; line-height: 1.4; }
  .card-meta { display: flex; gap: 0.3rem; flex-wrap: wrap; margin-bottom: 0.4rem; }
  .meta-tag { font-size: 0.68rem; padding: 0.15rem 0.4rem; border-radius: 3px; background: rgba(255,255,255,0.05); color: #9ca3af; }
  .meta-tag.quality { font-weight: 600; }
  .card-tags { display: flex; gap: 0.3rem; flex-wrap: wrap; margin-bottom: 0.4rem; }
  .task-tag { font-size: 0.68rem; padding: 0.15rem 0.4rem; border-radius: 3px; background: rgba(139,92,246,0.1); color: #a78bfa; border: 1px solid rgba(139,92,246,0.2); }
  .card-issues, .card-citation, .card-usage { margin-top: 0.4rem; }
  .card-issues h4, .card-citation h4, .card-usage h4 { font-size: 0.75rem; color: #9ca3af; margin: 0 0 0.2rem; }
  .issue-item { font-size: 0.72rem; color: #fbbf24; padding-left: 0.5rem; }
  .card-citation code { font-size: 0.68rem; color: #6b7280; display: block; padding: 0.3rem; background: rgba(0,0,0,0.2); border-radius: 3px; overflow-x: auto; }
  .card-usage p { font-size: 0.75rem; color: #d1d5db; margin: 0; line-height: 1.4; }

  .stats-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.4rem; margin-bottom: 0.4rem; }
  .stat-card { text-align: center; padding: 0.4rem; background: rgba(255,255,255,0.03); border-radius: 6px; border: 1px solid rgba(148,163,184,0.08); }
  .stat-value { font-size: 1.1rem; font-weight: 700; color: #e5e7eb; }
  .stat-label { font-size: 0.62rem; color: #6b7280; }
  .avg-accuracy { font-size: 0.78rem; color: #9ca3af; margin-bottom: 0.4rem; }
  .avg-accuracy strong { color: #10b981; }

  .timeline-chart { display: flex; align-items: flex-end; gap: 4px; height: 80px; padding-top: 15px; }
  .timeline-bar { width: 30px; background: rgba(59,130,246,0.3); border-radius: 3px 3px 0 0; position: relative; display: flex; flex-direction: column; justify-content: flex-start; align-items: center; }
  .timeline-val { font-size: 0.6rem; color: #93c5fd; position: relative; top: -14px; }
  .timeline-label { font-size: 0.55rem; color: #6b7280; position: absolute; bottom: -16px; }

  .discovery-section { margin-top: 0.5rem; }
  .search-form { display: flex; gap: 0.3rem; margin-bottom: 0.4rem; }
  .search-input { flex: 1; }
  .results-count { font-size: 0.72rem; color: #6b7280; margin-bottom: 0.3rem; }
  .result-item { padding: 0.4rem; background: rgba(255,255,255,0.03); border-radius: 5px; margin-bottom: 0.3rem; border: 1px solid rgba(148,163,184,0.08); }
  .result-name { font-size: 0.82rem; font-weight: 600; color: #93c5fd; }
  .result-meta { display: flex; gap: 0.5rem; font-size: 0.68rem; color: #9ca3af; margin-top: 0.15rem; }
  .relevance { color: #a78bfa; font-weight: 600; }

  .empty-state { text-align: center; padding: 1rem; color: #6b7280; font-size: 0.8rem; }
  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin-top: 0.5rem; }

  @media (max-width: 700px) { .panel-sections { grid-template-columns: 1fr; } }
</style>
