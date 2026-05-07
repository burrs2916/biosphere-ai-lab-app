<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { uxStore } from '$lib/lab/stores/uxStore';

  export let onRefresh: (() => void) | null = null;
  export let onSearch: (() => void) | null = null;
  export let onNavigateBack: (() => void) | null = null;
  export let onToggleHelp: (() => void) | null = null;

  let showHelp = false;

  function handleKeydown(e: KeyboardEvent) {
    if (!$uxStore.keyboardShortcutsEnabled) return;

    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') return;

    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case 'r':
          e.preventDefault();
          onRefresh?.();
          uxStore.info('刷新', '数据已刷新');
          break;
        case 'k':
          e.preventDefault();
          onSearch?.();
          break;
      }
      return;
    }

    switch (e.key) {
      case '?':
        showHelp = !showHelp;
        onToggleHelp?.();
        break;
      case 'Escape':
        showHelp = false;
        break;
      case 'Backspace':
        if (!e.ctrlKey && !e.metaKey) {
          onNavigateBack?.();
        }
        break;
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

{#if showHelp}
  <div class="shortcut-overlay" role="dialog" aria-modal="true" aria-label="快捷键帮助" tabindex="-1" on:click={() => (showHelp = false)} on:keydown={(e) => { if (e.key === 'Escape') showHelp = false; }}>
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <div class="shortcut-panel" role="document" on:click|stopPropagation on:keydown|stopPropagation>
      <div class="shortcut-header">
        <h3>⌨️ 快捷键</h3>
        <button class="close-btn" on:click={() => (showHelp = false)}>✕</button>
      </div>
      <div class="shortcut-list">
        <div class="shortcut-item">
          <kbd>Ctrl</kbd> + <kbd>R</kbd>
          <span class="shortcut-desc">刷新数据</span>
        </div>
        <div class="shortcut-item">
          <kbd>Ctrl</kbd> + <kbd>K</kbd>
          <span class="shortcut-desc">搜索</span>
        </div>
        <div class="shortcut-item">
          <kbd>?</kbd>
          <span class="shortcut-desc">显示快捷键帮助</span>
        </div>
        <div class="shortcut-item">
          <kbd>Esc</kbd>
          <span class="shortcut-desc">关闭弹窗/帮助</span>
        </div>
        <div class="shortcut-item">
          <kbd>Backspace</kbd>
          <span class="shortcut-desc">返回上一页</span>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .shortcut-overlay {
    position: fixed; inset: 0; z-index: 9998;
    background: rgba(0,0,0,0.5); backdrop-filter: blur(4px);
    display: flex; align-items: center; justify-content: center;
  }
  .shortcut-panel {
    background: rgba(30,41,59,0.98); border: 1px solid rgba(148,163,184,0.2);
    border-radius: 12px; padding: 1.2rem; min-width: 320px; max-width: 400px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.4);
  }
  .shortcut-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.8rem; }
  .shortcut-header h3 { margin: 0; font-size: 1rem; color: #e5e7eb; }
  .close-btn { background: none; border: none; color: #9ca3af; cursor: pointer; font-size: 0.9rem; }
  .close-btn:hover { color: #e5e7eb; }

  .shortcut-list { display: flex; flex-direction: column; gap: 0.5rem; }
  .shortcut-item { display: flex; align-items: center; gap: 0.4rem; font-size: 0.82rem; color: #d1d5db; }
  .shortcut-desc { margin-left: auto; color: #9ca3af; font-size: 0.78rem; }

  kbd {
    display: inline-block; padding: 0.15rem 0.4rem; font-size: 0.72rem;
    font-family: monospace; background: rgba(255,255,255,0.08);
    border: 1px solid rgba(148,163,184,0.2); border-radius: 4px;
    color: #e5e7eb; box-shadow: 0 1px 0 rgba(148,163,184,0.15);
  }
</style>
