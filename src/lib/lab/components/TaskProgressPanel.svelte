<script lang="ts">
  import { taskManagerStore, activeTaskCount, formatETA, formatElapsed } from '$lib/lab/stores/taskManager';
  import { uxStore } from '$lib/lab/stores/uxStore';

  let expanded = false;

  $: tasks = $taskManagerStore;
  $: runningTasks = tasks.filter(t => t.status === 'running' || t.status === 'pending');
  $: completedTasks = tasks.filter(t => t.status === 'completed' || t.status === 'failed');
  $: hasActive = $activeTaskCount > 0;

  function statusIcon(status: string): string {
    switch (status) {
      case 'running': case 'pending': return '⏳';
      case 'completed': return '✅';
      case 'failed': return '❌';
      case 'cancelled': return '🚫';
      default: return '❓';
    }
  }

  function statusClass(status: string): string {
    return `task-${status}`;
  }

  function progressColor(progress: number, status: string): string {
    if (status === 'failed') return '#ef4444';
    if (status === 'completed') return '#10b981';
    if (progress < 30) return '#3b82f6';
    if (progress < 70) return '#8b5cf6';
    return '#10b981';
  }

  function stepDisplay(task: any): string {
    if (task.totalSteps > 0) {
      return `${task.currentStep}/${task.totalSteps}`;
    }
    return '';
  }
</script>

{#if tasks.length > 0}
  <div class="task-panel" class:expanded>
    <button class="task-panel-toggle" on:click={() => (expanded = !expanded)} aria-label="任务进度面板">
      <span class="toggle-icon">{hasActive ? '⏳' : '📋'}</span>
      {#if hasActive}
        <span class="toggle-badge">{$activeTaskCount}</span>
      {/if}
    </button>

    {#if expanded}
      <div class="task-panel-content">
        <div class="panel-header">
          <h4>后台任务</h4>
          <div class="panel-actions">
            {#if completedTasks.length > 0}
              <button class="btn-clear" on:click={() => taskManagerStore.clearCompleted()}>清除已完成</button>
            {/if}
            <button class="btn-collapse" on:click={() => (expanded = false)}>收起</button>
          </div>
        </div>

        <div class="task-list">
          {#each runningTasks as task (task.id)}
            <div class="task-item {statusClass(task.status)}">
              <div class="task-header">
                <span class="task-icon">{statusIcon(task.status)}</span>
                <div class="task-info">
                  <span class="task-name">{task.name}</span>
                  <span class="task-step">
                    {#if stepDisplay(task)}
                      步骤 {stepDisplay(task)}
                      {#if task.stepLabel}· {task.stepLabel}{/if}
                    {:else}
                      {task.progressMessage}
                    {/if}
                  </span>
                </div>
                <div class="task-meta">
                  <span class="task-progress-pct">{task.progress}%</span>
                  {#if task.estimatedTimeRemaining}
                    <span class="task-eta">{formatETA(task.estimatedTimeRemaining)}</span>
                  {/if}
                </div>
              </div>
              <div class="task-progress-bar">
                <div
                  class="task-progress-fill"
                  style="width: {task.progress}%; background: {progressColor(task.progress, task.status)}"
                ></div>
              </div>
              <div class="task-footer">
                <span class="task-elapsed">已用 {formatElapsed(task.startedAt, null)}</span>
                {#if task.cancellable}
                  <button class="btn-cancel" on:click={() => taskManagerStore.cancelTask(task.id)}>取消</button>
                {/if}
              </div>
            </div>
          {/each}

          {#each completedTasks as task (task.id)}
            <div class="task-item {statusClass(task.status)}">
              <div class="task-header">
                <span class="task-icon">{statusIcon(task.status)}</span>
                <div class="task-info">
                  <span class="task-name">{task.name}</span>
                  <span class="task-step">
                    {#if task.status === 'completed'}
                      {task.result || '完成'}
                    {:else}
                      {task.error || '失败'}
                    {/if}
                  </span>
                </div>
                <span class="task-elapsed">耗时 {formatElapsed(task.startedAt, task.completedAt)}</span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .task-panel {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 9998;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .task-panel-toggle {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: rgba(30, 41, 59, 0.95);
    border: 1px solid rgba(148, 163, 184, 0.2);
    color: #e2e8f0;
    font-size: 1.1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    transition: transform 0.15s;
  }

  .task-panel-toggle:hover { transform: scale(1.05); }

  .toggle-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    background: #3b82f6;
    color: white;
    font-size: 0.6rem;
    font-weight: 700;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .task-panel-content {
    position: absolute;
    bottom: 52px;
    right: 0;
    width: 360px;
    max-height: 400px;
    background: rgba(15, 23, 42, 0.98);
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: slideUp 0.2s ease-out;
  }

  @keyframes slideUp {
    from { transform: translateY(10px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid rgba(148, 163, 184, 0.1);
  }

  .panel-header h4 { margin: 0; font-size: 0.85rem; color: #e2e8f0; }

  .panel-actions { display: flex; gap: 0.4rem; }

  .btn-clear,
  .btn-collapse {
    background: none;
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 4px;
    color: #94a3b8;
    font-size: 0.68rem;
    padding: 0.15rem 0.4rem;
    cursor: pointer;
  }

  .btn-clear:hover,
  .btn-collapse:hover { color: #e2e8f0; background: rgba(255, 255, 255, 0.05); }

  .task-list {
    overflow-y: auto;
    max-height: 340px;
    padding: 0.4rem;
  }

  .task-item {
    padding: 0.5rem;
    border-radius: 6px;
    margin-bottom: 0.3rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(148, 163, 184, 0.06);
  }

  .task-item.task-failed { border-color: rgba(239, 68, 68, 0.15); background: rgba(239, 68, 68, 0.03); }
  .task-item.task-completed { border-color: rgba(16, 185, 129, 0.15); background: rgba(16, 185, 129, 0.03); }

  .task-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.3rem;
  }

  .task-icon { font-size: 0.85rem; flex-shrink: 0; }

  .task-info { flex: 1; min-width: 0; }
  .task-name { display: block; font-size: 0.78rem; color: #e2e8f0; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .task-step { display: block; font-size: 0.68rem; color: #94a3b8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .task-meta { display: flex; flex-direction: column; align-items: flex-end; gap: 0.1rem; flex-shrink: 0; }
  .task-progress-pct { font-size: 0.78rem; font-weight: 600; color: #e2e8f0; font-variant-numeric: tabular-nums; }
  .task-eta { font-size: 0.65rem; color: #93c5fd; }

  .task-progress-bar {
    width: 100%;
    height: 3px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.25rem;
  }

  .task-progress-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .task-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .task-elapsed { font-size: 0.65rem; color: #64748b; }

  .btn-cancel {
    background: none;
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 3px;
    color: #fca5a5;
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    cursor: pointer;
  }

  .btn-cancel:hover { background: rgba(239, 68, 68, 0.1); }
</style>
