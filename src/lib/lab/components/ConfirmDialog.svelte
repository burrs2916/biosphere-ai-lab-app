<script lang="ts">
  import { t } from '$lib/i18n';

  export let show: boolean = false;
  export let title: string = '';
  export let message: string = '';
  export let confirmLabel: string = '';
  export let cancelLabel: string = '';
  export let danger: boolean = false;
  export let loading: boolean = false;

  $: resolvedTitle = title || $t('confirm.title');
  $: resolvedMessage = message || $t('confirm.message');
  $: resolvedConfirmLabel = confirmLabel || $t('confirm.ok');
  $: resolvedCancelLabel = cancelLabel || $t('confirm.cancel');

  export let onConfirm: () => void = () => {};
  export let onCancel: () => void = () => {};

  function handleConfirm() {
    if (!loading) onConfirm();
  }

  function handleCancel() {
    if (!loading) onCancel();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') handleCancel();
    if (e.key === 'Enter') handleConfirm();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="confirm-overlay" role="presentation" on:click={handleCancel} on:keydown={handleKeydown}>
    <div class="confirm-dialog" on:click|stopPropagation role="dialog" aria-modal="true" tabindex="-1">
      <div class="confirm-icon" class:danger>
        {danger ? '⚠️' : 'ℹ️'}
      </div>
      <h3 class="confirm-title">{resolvedTitle}</h3>
      <p class="confirm-message">{resolvedMessage}</p>
      <div class="confirm-actions">
        <button
          class="confirm-btn cancel-btn"
          on:click={handleCancel}
          disabled={loading}
        >
          {resolvedCancelLabel}
        </button>
        <button
          class="confirm-btn confirm-btn-main"
          class:danger
          on:click={handleConfirm}
          disabled={loading}
        >
          {#if loading}
            <span class="spinner"></span>
          {/if}
          {resolvedConfirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    animation: fadeIn 0.15s ease;
  }

  .confirm-dialog {
    background: #1e293b;
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 420px;
    width: 90%;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    animation: scaleIn 0.2s ease;
  }

  .confirm-icon {
    font-size: 2rem;
    text-align: center;
    margin-bottom: 0.75rem;
  }

  .confirm-icon.danger {
    font-size: 2.2rem;
  }

  .confirm-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: #e2e8f0;
    text-align: center;
    margin: 0 0 0.5rem 0;
  }

  .confirm-message {
    font-size: 0.85rem;
    color: #94a3b8;
    text-align: center;
    margin: 0 0 1.25rem 0;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .confirm-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: center;
  }

  .confirm-btn {
    padding: 0.5rem 1.25rem;
    font-size: 0.85rem;
    border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.15);
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .cancel-btn {
    background: rgba(255, 255, 255, 0.05);
    color: #94a3b8;
  }

  .cancel-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: #e2e8f0;
  }

  .confirm-btn-main {
    background: rgba(59, 130, 246, 0.15);
    border-color: rgba(59, 130, 246, 0.3);
    color: #93c5fd;
  }

  .confirm-btn-main:hover:not(:disabled) {
    background: rgba(59, 130, 246, 0.25);
  }

  .confirm-btn-main.danger {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.3);
    color: #fca5a5;
  }

  .confirm-btn-main.danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.25);
  }

  .confirm-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes scaleIn {
    from { transform: scale(0.95); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
