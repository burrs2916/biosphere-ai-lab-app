<script lang="ts">
  import { uxStore } from '$lib/lab/stores/uxStore';

  $: notifications = $uxStore.notifications;

  function typeIcon(type: string): string {
    if (type === 'success') return '✅';
    if (type === 'error') return '❌';
    if (type === 'warning') return '⚠️';
    return 'ℹ️';
  }

  function typeClass(type: string): string {
    return `notif-${type}`;
  }
</script>

{#if notifications.length > 0}
  <div class="notification-stack">
    {#each notifications as notif (notif.id)}
      <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
      <div class="notification {typeClass(notif.type)}" role="alert" on:click={() => uxStore.dismiss(notif.id)} on:keydown={(e) => { if (e.key === 'Enter' || e.key === 'Escape') uxStore.dismiss(notif.id); }}>
        <span class="notif-icon">{typeIcon(notif.type)}</span>
        <div class="notif-content">
          <div class="notif-title">{notif.title}</div>
          <div class="notif-message">{notif.message}</div>
        </div>
        <span class="notif-close" role="button" tabindex="0" on:click|stopPropagation={() => uxStore.dismiss(notif.id)} on:keydown={(e) => { e.stopPropagation(); if (e.key === 'Enter') uxStore.dismiss(notif.id); }}>✕</span>
        {#if notif.autoDismiss}
          <div class="notif-progress" style="animation-duration: {notif.duration}ms"></div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .notification-stack {
    position: fixed; top: 1rem; right: 1rem; z-index: 9999;
    display: flex; flex-direction: column; gap: 0.4rem;
    max-width: 380px; pointer-events: none;
  }
  .notification {
    display: flex; align-items: flex-start; gap: 0.5rem;
    padding: 0.6rem 0.8rem; border-radius: 8px;
    backdrop-filter: blur(12px); pointer-events: auto;
    animation: slideIn 0.25s ease-out; position: relative; overflow: hidden;
    border: 1px solid rgba(148,163,184,0.15);
  }
  .notif-success { background: rgba(16,185,129,0.12); border-color: rgba(16,185,129,0.25); }
  .notif-error { background: rgba(239,68,68,0.12); border-color: rgba(239,68,68,0.25); }
  .notif-warning { background: rgba(245,158,11,0.12); border-color: rgba(245,158,11,0.25); }
  .notif-info { background: rgba(59,130,246,0.12); border-color: rgba(59,130,246,0.25); }

  .notif-icon { font-size: 1rem; margin-top: 1px; }
  .notif-content { flex: 1; min-width: 0; }
  .notif-title { font-size: 0.82rem; font-weight: 600; color: #e5e7eb; }
  .notif-message { font-size: 0.72rem; color: #9ca3af; margin-top: 0.1rem; }
  .notif-close {
    background: none; border: none; color: #6b7280; cursor: pointer;
    font-size: 0.7rem; padding: 0.1rem; line-height: 1;
  }
  .notif-close:hover { color: #e5e7eb; }

  .notif-progress {
    position: absolute; bottom: 0; left: 0; height: 2px;
    background: rgba(255,255,255,0.2);
    animation: shrink linear forwards;
  }
  .notif-success .notif-progress { background: rgba(16,185,129,0.5); }
  .notif-error .notif-progress { background: rgba(239,68,68,0.5); }

  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
  @keyframes shrink {
    from { width: 100%; }
    to { width: 0%; }
  }
</style>
