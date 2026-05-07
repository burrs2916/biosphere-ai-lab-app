<script lang="ts">
  import { getLabClient } from '$lib/lab/stores/plugins';
  import { taskManagerStore } from '$lib/lab/stores/taskManager';

  export let datasetPath: string = '';
  export let datasetId: string = '';

  $: versionLabel = datasetId ? `数据集 ${datasetId.slice(0, 8)}` : '数据集';

  let commits: any[] = [];
  let branches: any[] = [];
  let currentBranch = 'main';
  let loading = false;
  let error: string | null = null;
  let commitMessage = '';
  let newBranchName = '';
  let showBranchDialog = false;
  let selectedFromVersion = '';
  let selectedToVersion = '';
  let diffResult: any = null;
  let diffLoading = false;
  let initialized = false;

  async function initVersionControl() {
    if (!datasetPath) return;
    loading = true;
    error = null;
    try {
      const client = getLabClient();
      await client.dataVersionInit(datasetPath);
      initialized = true;
      await refreshLog();
    } catch (e: any) {
      error = e?.toString() || '初始化版本控制失败';
    } finally {
      loading = false;
    }
  }

  async function refreshLog() {
    if (!datasetPath) return;
    loading = true;
    error = null;
    try {
      const client = getLabClient();
      const [logResult, branchResult] = await Promise.all([
        client.dataVersionLog(datasetPath),
        client.dataVersionBranches(datasetPath),
      ]);
      commits = logResult.commits || [];
      branches = branchResult.branches || [];
      currentBranch = branchResult.current_branch || 'main';
      initialized = true;
    } catch (e: any) {
      error = e?.toString() || '加载版本日志失败';
    } finally {
      loading = false;
    }
  }

  async function commit() {
    if (!datasetPath || !commitMessage.trim()) return;
    const taskId = taskManagerStore.createTask('版本提交', `正在提交: ${commitMessage}`, false);
    try {
      const client = getLabClient();
      await client.dataVersionCommit(datasetPath, commitMessage);
      commitMessage = '';
      taskManagerStore.completeTask(taskId, '提交成功');
      await refreshLog();
    } catch (e: any) {
      taskManagerStore.failTask(taskId, e?.toString() || '提交失败');
    }
  }

  async function checkout(version: string) {
    if (!datasetPath) return;
    const taskId = taskManagerStore.createTask('版本切换', `正在切换到 ${version}...`, false);
    try {
      const client = getLabClient();
      await client.dataVersionCheckout(datasetPath, version);
      taskManagerStore.completeTask(taskId, `已切换到 ${version}`);
      await refreshLog();
    } catch (e: any) {
      taskManagerStore.failTask(taskId, e?.toString() || '切换失败');
    }
  }

  async function createBranch() {
    if (!datasetPath || !newBranchName.trim()) return;
    const taskId = taskManagerStore.createTask('创建分支', `正在创建分支 ${newBranchName}...`, false);
    try {
      const client = getLabClient();
      await client.dataVersionCreateBranch(datasetPath, newBranchName);
      taskManagerStore.completeTask(taskId, `分支 ${newBranchName} 创建成功`);
      newBranchName = '';
      showBranchDialog = false;
      await refreshLog();
    } catch (e: any) {
      taskManagerStore.failTask(taskId, e?.toString() || '创建分支失败');
    }
  }

  async function showDiff() {
    if (!datasetPath || !selectedFromVersion || !selectedToVersion) return;
    diffLoading = true;
    diffResult = null;
    try {
      const client = getLabClient();
      diffResult = await client.dataVersionDiff(datasetPath, selectedFromVersion, selectedToVersion);
    } catch (e: any) {
      error = e?.toString() || '获取差异失败';
    } finally {
      diffLoading = false;
    }
  }

  function formatTime(ts: string): string {
    if (!ts) return '';
    const d = new Date(ts);
    return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`;
  }
</script>

<div class="version-control">
  <div class="vc-header">
    <h3>📦 {versionLabel} - 版本控制</h3>
    {#if initialized}
      <button class="btn-sm" on:click={refreshLog}>🔄</button>
    {/if}
  </div>

  {#if !initialized}
    <div class="vc-init">
      <p class="init-desc">启用版本控制以追踪数据变更历史</p>
      <button class="btn-primary" on:click={initVersionControl} disabled={loading}>
        {loading ? '初始化中...' : '🚀 启用版本控制'}
      </button>
    </div>
  {:else}
    <div class="branch-bar">
      <span class="branch-label">🌿 {currentBranch}</span>
      <div class="branch-chips">
        {#each branches as branch}
          <span class="branch-chip" class:active={branch.name === currentBranch}>
            {branch.name}
            <span class="branch-head">{branch.head}</span>
          </span>
        {/each}
      </div>
      <button class="btn-sm" on:click={() => (showBranchDialog = true)}>+ 分支</button>
    </div>

    {#if showBranchDialog}
      <div class="branch-dialog">
        <input class="input" type="text" bind:value={newBranchName} placeholder="新分支名称" />
        <button class="btn-primary-sm" on:click={createBranch} disabled={!newBranchName.trim()}>创建</button>
        <button class="btn-sm" on:click={() => (showBranchDialog = false)}>取消</button>
      </div>
    {/if}

    <div class="commit-form">
      <input class="input" type="text" bind:value={commitMessage} placeholder="提交信息..." />
      <button class="btn-primary-sm" on:click={commit} disabled={!commitMessage.trim()}>📝 提交</button>
    </div>

    <div class="diff-form">
      <select class="input" bind:value={selectedFromVersion}>
        <option value="">从版本</option>
        {#each commits as c}
          <option value={c.version}>{c.version} - {c.message}</option>
        {/each}
      </select>
      <span class="diff-arrow">→</span>
      <select class="input" bind:value={selectedToVersion}>
        <option value="">到版本</option>
        {#each commits as c}
          <option value={c.version}>{c.version} - {c.message}</option>
        {/each}
      </select>
      <button class="btn-sm" on:click={showDiff} disabled={!selectedFromVersion || !selectedToVersion || diffLoading}>
        {diffLoading ? '...' : '🔍 对比'}
      </button>
    </div>

    {#if diffResult}
      <div class="diff-result">
        <div class="diff-summary">
          <span class="diff-added">+{diffResult.files_added?.length || 0} 文件</span>
          <span class="diff-removed">-{diffResult.files_removed?.length || 0} 文件</span>
          <span class="diff-modified">~{diffResult.files_modified?.length || 0} 文件</span>
          <span class="diff-rows">±{diffResult.row_change || 0} 行</span>
        </div>
        {#if diffResult.files_added?.length}
          <div class="diff-section">
            <div class="diff-section-title" style="color: #10b981">新增文件</div>
            {#each diffResult.files_added as f}
              <div class="diff-file">+ {f}</div>
            {/each}
          </div>
        {/if}
        {#if diffResult.files_modified?.length}
          <div class="diff-section">
            <div class="diff-section-title" style="color: #f59e0b">修改文件</div>
            {#each diffResult.files_modified as f}
              <div class="diff-file">~ {f}</div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <div class="commit-list">
      {#if commits.length === 0}
        <div class="empty-commits">暂无提交记录</div>
      {:else}
        {#each commits as commit, i}
          <div class="commit-item">
            <div class="commit-dot" class:first={i === 0}></div>
            {#if i < commits.length - 1}
              <div class="commit-line"></div>
            {/if}
            <div class="commit-info">
              <div class="commit-top">
                <span class="commit-version">{commit.version}</span>
                <span class="commit-hash">{commit.hash?.substring(0, 7)}</span>
                <span class="commit-time">{formatTime(commit.timestamp)}</span>
              </div>
              <div class="commit-message">{commit.message}</div>
              <button class="checkout-btn" on:click={() => checkout(commit.version)}>切换</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  {#if error}
    <div class="error-box">{error}</div>
  {/if}
</div>

<style>
  .version-control { padding: 0; }
  .vc-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem; }
  .vc-header h3 { margin: 0; font-size: 1rem; }

  .vc-init { text-align: center; padding: 1.5rem 0; }
  .init-desc { font-size: 0.8rem; color: #9ca3af; margin-bottom: 0.6rem; }

  .branch-bar { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.5rem; flex-wrap: wrap; }
  .branch-label { font-size: 0.82rem; font-weight: 600; color: #10b981; }
  .branch-chips { display: flex; gap: 0.3rem; flex-wrap: wrap; flex: 1; }
  .branch-chip { font-size: 0.7rem; padding: 0.15rem 0.4rem; border-radius: 3px; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.15); color: #9ca3af; display: flex; align-items: center; gap: 0.3rem; }
  .branch-chip.active { background: rgba(16,185,129,0.1); border-color: rgba(16,185,129,0.3); color: #10b981; }
  .branch-head { font-family: monospace; font-size: 0.65rem; color: #6b7280; }

  .branch-dialog { display: flex; gap: 0.4rem; align-items: center; margin-bottom: 0.5rem; }

  .commit-form { display: flex; gap: 0.4rem; margin-bottom: 0.5rem; }
  .diff-form { display: flex; gap: 0.4rem; align-items: center; margin-bottom: 0.6rem; }
  .diff-arrow { color: #6b7280; font-size: 0.8rem; }

  .input { padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; color: #e5e7eb; font-size: 0.78rem; }
  .input:focus { outline: none; border-color: rgba(59,130,246,0.5); }
  select.input { appearance: auto; }

  .diff-result { padding: 0.5rem; background: rgba(15,23,42,0.5); border: 1px solid rgba(148,163,184,0.1); border-radius: 6px; margin-bottom: 0.5rem; }
  .diff-summary { display: flex; gap: 0.8rem; margin-bottom: 0.4rem; font-size: 0.75rem; font-weight: 600; }
  .diff-added { color: #10b981; }
  .diff-removed { color: #ef4444; }
  .diff-modified { color: #f59e0b; }
  .diff-rows { color: #8b5cf6; }
  .diff-section { margin-bottom: 0.3rem; }
  .diff-section-title { font-size: 0.72rem; font-weight: 600; margin-bottom: 0.2rem; }
  .diff-file { font-size: 0.72rem; color: #9ca3af; font-family: monospace; padding-left: 0.5rem; }

  .commit-list { position: relative; }
  .empty-commits { text-align: center; padding: 1rem; color: #6b7280; font-size: 0.8rem; }
  .commit-item { display: flex; position: relative; padding-left: 20px; margin-bottom: 0.3rem; }
  .commit-dot { position: absolute; left: 4px; top: 6px; width: 10px; height: 10px; border-radius: 50%; background: #4b5563; border: 2px solid #1e293b; z-index: 1; }
  .commit-dot.first { background: #3b82f6; }
  .commit-line { position: absolute; left: 8px; top: 16px; width: 2px; height: calc(100% + 4px); background: rgba(75,85,99,0.3); }
  .commit-info { flex: 1; padding: 0.3rem 0.5rem; background: rgba(255,255,255,0.03); border-radius: 4px; }
  .commit-top { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.15rem; }
  .commit-version { font-size: 0.75rem; font-weight: 600; color: #93c5fd; }
  .commit-hash { font-size: 0.65rem; font-family: monospace; color: #6b7280; }
  .commit-time { font-size: 0.65rem; color: #6b7280; margin-left: auto; }
  .commit-message { font-size: 0.78rem; color: #d1d5db; }
  .checkout-btn { font-size: 0.65rem; padding: 0.1rem 0.4rem; border: 1px solid rgba(59,130,246,0.3); border-radius: 3px; background: rgba(59,130,246,0.1); color: #93c5fd; cursor: pointer; margin-top: 0.2rem; }
  .checkout-btn:hover { background: rgba(59,130,246,0.2); }

  .btn-sm { padding: 0.2rem 0.5rem; border: 1px solid rgba(148,163,184,0.2); border-radius: 4px; background: rgba(255,255,255,0.05); color: #d1d5db; font-size: 0.72rem; cursor: pointer; }
  .btn-sm:hover { background: rgba(255,255,255,0.1); }
  .btn-primary { padding: 0.4rem 0.9rem; border: none; border-radius: 6px; background: #3b82f6; color: #fff; font-size: 0.8rem; font-weight: 600; cursor: pointer; }
  .btn-primary:hover { background: #2563eb; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary-sm { padding: 0.25rem 0.6rem; border: none; border-radius: 4px; background: #3b82f6; color: #fff; font-size: 0.72rem; font-weight: 600; cursor: pointer; }
  .btn-primary-sm:hover { background: #2563eb; }

  .error-box { padding: 0.5rem; background: rgba(239,68,68,0.08); border: 1px solid rgba(239,68,68,0.2); border-radius: 5px; color: #fca5a5; font-size: 0.78rem; margin-top: 0.5rem; }
</style>
