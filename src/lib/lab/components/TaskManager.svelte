<script lang="ts">
  import { taskManagerStore, activeTaskCount } from '$lib/lab/stores/taskManager';
  import type { BackgroundTask } from '$lib/lab/stores/taskManager';

  let expanded = false;

  function statusIcon(status: string): string {
    switch (status) {
      case 'running': return '⏳';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'cancelled': return '🚫';
      default: return '⏸️';
    }
  }

  function statusClass(status: string): string {
    return `task-${status}`;
  }

  function formatTimeRemaining(seconds: number | null): string {
    if (seconds === null || seconds <= 0) return '计算中...';
    if (seconds < 60) return `${Math.round(seconds)}秒`;
    if (seconds < 3600) return `${Math.round(seconds / 60)}分钟`;
    return `${(seconds / 3600).toFixed(1)}小时`;
  }

  function formatElapsed(ms: number): string {
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}秒`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}分${seconds % 60}秒`;
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${hours}时${mins}分`;
  }

  function handleCancel(task: BackgroundTask) {
    taskManagerStore.cancelTask(task.id);
  }

  function handleDismiss(task: BackgroundTask) {
    taskManagerStore.removeTask(task.id);
  }

  function toggleExpand() {
    expanded = !expanded;
  }
</script>

{#if $activeTaskCount > 0 || expanded}
  <div class="task-manager" class:expanded>
    <button class="task-toggle" on:click={toggleExpand}>
      <span class="task-toggle-icon">📋</span>
      <span class="task-toggle-text">
        {#if $activeTaskCount > 0}
          后台任务 ({$activeTaskCount})
        {:else}
          任务历史
        {/if}
      </span>
      <span class="task-toggle-arrow">{expanded ? '▼' : '▲'}</span>
    </button>

    {#if expanded}
      <div class="task-list">
        {#if $taskManagerStore.length === 0}
          <div class="task-empty">暂无任务</div>
        {:else}
          {#each $taskManagerStore as task (task.id)}
            <div class="task-item {statusClass(task.status)}">
              <div class="task-header">
                <span class="task-status-icon">{statusIcon(task.status)}</span>
                <span class="task-name">{task.name}</span>
                <span class="task-elapsed">{formatElapsed(Date.now() - task.startedAt)}</span>
              </div>

              {#if task.status === 'running'}
                <div class="task-progress-bar">
                  <div
                    class="task-progress-fill"
                    style="width: {task.progress}%"
                  ></div>
                </div>
                <div class="task-progress-info">
                  <span class="task-progress-text">{task.progressMessage}</span>
                  <span class="task-progress-pct">{task.progress.toFixed(0)}%</span>
                </div>
                {#if task.estimatedTimeRemaining !== null}
                  <div class="task-eta">
                    预计剩余: {formatTimeRemaining(task.estimatedTimeRemaining)}
                  </div>
                {/if}
                {#if task.cancellable}
                  <button class="task-cancel-btn" on:click={() => handleCancel(task)}>
                    取消
                  </button>
                {/if}
              {:else if task.status === 'failed'}
                <div class="task-error">{task.error || '未知错误'}</div>
                <button class="task-dismiss-btn" on:click={() => handleDismiss(task)}>
                  关闭
                </button>
              {:else if task.status === 'completed'}
                <div class="task-result">{task.result || '操作完成'}</div>
              {:else if task.status === 'cancelled'}
                <div class="task-cancelled">任务已取消</div>
              {/if}
            </div>
          {/each}

          {#if $taskManagerStore.some(t => t.status === 'completed' || t.status === 'failed' || t.status === 'cancelled')}
            <button class="task-clear-btn" on:click={() => taskManagerStore.clearCompleted()}>
              清除已完成任务
            </button>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .task-manager {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 9999;
    max-width: 380px;
    width: 100%;
  }

  .task-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.6rem 1rem;
    background: rgba(30, 41, 59, 0.95);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 10px;
    color: #e2e8f0;
    font-size: 0.85rem;
    cursor: pointer;
    backdrop-filter: blur(12px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    transition: all 0.2s;
  }

  .task-toggle:hover {
    background: rgba(51, 65, 85, 0.95);
    border-color: rgba(148, 163, 184, 0.4);
  }

  .task-toggle-icon {
    font-size: 1rem;
  }

  .task-toggle-text {
    flex: 1;
    text-align: left;
    font-weight: 500;
  }

  .task-toggle-arrow {
    font-size: 0.7rem;
    opacity: 0.6;
  }

  .task-list {
    margin-top: 0.5rem;
    background: rgba(30, 41, 59, 0.95);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 10px;
    padding: 0.5rem;
    backdrop-filter: blur(12px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    max-height: 400px;
    overflow-y: auto;
  }

  .task-empty {
    text-align: center;
    padding: 1rem;
    color: #64748b;
    font-size: 0.8rem;
  }

  .task-item {
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    margin-bottom: 0.4rem;
    border: 1px solid rgba(148, 163, 184, 0.1);
    transition: all 0.2s;
  }

  .task-item:last-child {
    margin-bottom: 0;
  }

  .task-running {
    background: rgba(59, 130, 246, 0.08);
    border-color: rgba(59, 130, 246, 0.2);
  }

  .task-completed {
    background: rgba(16, 185, 129, 0.08);
    border-color: rgba(16, 185, 129, 0.2);
  }

  .task-failed {
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.2);
  }

  .task-cancelled {
    background: rgba(100, 116, 139, 0.08);
    border-color: rgba(100, 116, 139, 0.2);
  }

  .task-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.3rem;
  }

  .task-status-icon {
    font-size: 0.8rem;
  }

  .task-name {
    flex: 1;
    font-size: 0.8rem;
    font-weight: 500;
    color: #e2e8f0;
  }

  .task-elapsed {
    font-size: 0.7rem;
    color: #64748b;
  }

  .task-progress-bar {
    height: 4px;
    background: rgba(148, 163, 184, 0.15);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.3rem;
  }

  .task-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b82f6, #60a5fa);
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .task-progress-info {
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
    color: #94a3b8;
    margin-bottom: 0.2rem;
  }

  .task-eta {
    font-size: 0.7rem;
    color: #64748b;
    margin-bottom: 0.3rem;
  }

  .task-error {
    font-size: 0.75rem;
    color: #fca5a5;
    margin-bottom: 0.3rem;
    line-height: 1.4;
  }

  .task-result {
    font-size: 0.75rem;
    color: #6ee7b7;
  }

  .task-cancelled {
    font-size: 0.75rem;
    color: #94a3b8;
  }

  .task-cancel-btn,
  .task-dismiss-btn {
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    border: 1px solid rgba(148, 163, 184, 0.2);
    background: rgba(148, 163, 184, 0.1);
    color: #94a3b8;
    font-size: 0.7rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .task-cancel-btn:hover {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.4);
    color: #fca5a5;
  }

  .task-dismiss-btn:hover {
    background: rgba(148, 163, 184, 0.2);
    color: #e2e8f0;
  }

  .task-clear-btn {
    display: block;
    width: 100%;
    padding: 0.4rem;
    margin-top: 0.4rem;
    background: none;
    border: 1px dashed rgba(148, 163, 184, 0.2);
    border-radius: 6px;
    color: #64748b;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .task-clear-btn:hover {
    border-color: rgba(148, 163, 184, 0.4);
    color: #94a3b8;
  }
</style>
