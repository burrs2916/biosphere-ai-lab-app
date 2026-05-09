<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n';

  export let isFiltered = false;

  const dispatch = createEventDispatcher();

  const quickStartSteps = [
    { icon: '📁', title: $t('emptyGuide.stepRegister'), desc: $t('emptyGuide.stepRegisterDesc'), action: 'register' },
    { icon: '🔍', title: $t('emptyGuide.stepDiscover'), desc: $t('emptyGuide.stepDiscoverDesc'), action: 'discover' },
    { icon: '📊', title: $t('emptyGuide.stepQuality'), desc: $t('emptyGuide.stepQualityDesc'), action: 'quality' },
    { icon: '🧪', title: $t('emptyGuide.stepTrain'), desc: $t('emptyGuide.stepTrainDesc'), action: 'train' },
  ];

  function handleAction(action: string) {
    switch (action) {
      case 'register':
        dispatch('register');
        break;
      case 'discover':
        goto('/lab/data/discover');
        break;
      case 'quality':
        dispatch('register');
        break;
      case 'train':
        goto('/lab/experiments/new');
        break;
    }
  }
</script>

{#if isFiltered}
  <div class="empty-state">
    <div class="empty-icon">🔍</div>
    <h3>{$t('emptyGuide.noMatch')}</h3>
    <p>{$t('emptyGuide.adjustFilter')}</p>
  </div>
{:else}
  <div class="onboarding-state">
    <div class="onboarding-hero">
      <div class="hero-icon">🚀</div>
      <h2>{$t('emptyGuide.startJourney')}</h2>
      <p class="hero-desc">{$t('emptyGuide.startDesc')}</p>
    </div>

    <div class="quick-start-grid">
      {#each quickStartSteps as step, i}
        <button class="step-card" on:click={() => handleAction(step.action)}>
          <div class="step-number">{i + 1}</div>
          <div class="step-icon">{step.icon}</div>
          <div class="step-title">{step.title}</div>
          <div class="step-desc">{step.desc}</div>
        </button>
      {/each}
    </div>

    <div class="onboarding-tips">
      <div class="tip-item">
        <span class="tip-icon">💡</span>
        <span>{$t('emptyGuide.tip1')}</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">💡</span>
        <span>{$t('emptyGuide.tip2')}</span>
      </div>
      <div class="tip-item">
        <span class="tip-icon">💡</span>
        <span>{$t('emptyGuide.tip3')}</span>
      </div>
    </div>

    <button class="btn-primary" on:click={() => dispatch('register')}>
      📁 {$t('emptyGuide.registerFirst')}
    </button>
  </div>
{/if}

<style>
  .empty-state {
    text-align: center; padding: 3rem 1rem;
  }

  .empty-icon { font-size: 2.5rem; margin-bottom: 0.75rem; }
  .empty-state h3 { color: #d1d5db; margin: 0 0 0.5rem 0; font-size: 1rem; }
  .empty-state p { margin: 0; font-size: 0.85rem; color: #9ca3af; }

  .onboarding-state {
    display: flex; flex-direction: column; align-items: center;
    padding: 2.5rem 1.5rem; gap: 1.5rem;
  }

  .onboarding-hero { text-align: center; }
  .hero-icon { font-size: 3rem; margin-bottom: 0.5rem; }
  .onboarding-hero h2 { color: #e2e8f0; margin: 0 0 0.4rem 0; font-size: 1.2rem; font-weight: 600; }
  .hero-desc { color: #94a3b8; font-size: 0.85rem; margin: 0; }

  .quick-start-grid {
    display: grid; grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem; width: 100%; max-width: 480px;
  }

  .step-card {
    display: flex; flex-direction: column; align-items: center;
    padding: 1rem 0.75rem; border-radius: 10px;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(148, 163, 184, 0.08);
    cursor: pointer; transition: all 0.2s;
    position: relative; text-align: center;
  }

  .step-card:hover {
    background: rgba(59, 130, 246, 0.06);
    border-color: rgba(59, 130, 246, 0.2);
    transform: translateY(-2px);
  }

  .step-number {
    position: absolute; top: 0.35rem; left: 0.5rem;
    font-size: 0.6rem; font-weight: 700; color: #475569;
    width: 18px; height: 18px; border-radius: 50%;
    background: rgba(255, 255, 255, 0.05);
    display: flex; align-items: center; justify-content: center;
  }

  .step-icon { font-size: 1.5rem; margin-bottom: 0.4rem; }
  .step-title { font-size: 0.8rem; font-weight: 600; color: #e2e8f0; margin-bottom: 0.2rem; }
  .step-desc { font-size: 0.68rem; color: #94a3b8; }

  .onboarding-tips {
    display: flex; flex-direction: column; gap: 0.3rem;
    width: 100%; max-width: 400px;
  }

  .tip-item {
    display: flex; align-items: center; gap: 0.4rem;
    font-size: 0.72rem; color: #94a3b8;
  }

  .tip-icon { font-size: 0.75rem; flex-shrink: 0; }

  .btn-primary {
    padding: 0.6rem 1.5rem; border-radius: 8px;
    background: linear-gradient(135deg, #3b82f6, #6366f1);
    color: white; border: none; font-size: 0.85rem;
    font-weight: 600; cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-primary:hover { opacity: 0.9; }
</style>
