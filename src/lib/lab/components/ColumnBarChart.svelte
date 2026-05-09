<script lang="ts">
  import { onMount, afterUpdate, createEventDispatcher } from 'svelte';
  import { t } from '$lib/i18n';

  export let topValues: [string, number][] = [];
  export let columnName: string = '';
  export let width: number = 400;
  export let height: number = 200;
  export let maxBars: number = 10;
  export let selectedValue: string | null = null;

  const dispatch = createEventDispatcher();

  let canvas: HTMLCanvasElement;
  let tooltip: { x: number; y: number; text: string } | null = null;
  let hoveredBar: number = -1;
  let localSelectedValue: string | null = null;

  $: localSelectedValue = selectedValue;

  const colors = [
    '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
  ];

  function draw() {
    if (!canvas || topValues.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    const data = topValues.slice(0, maxBars);
    const maxVal = Math.max(...data.map(([, c]) => c), 1);

    const padding = { top: 10, right: 10, bottom: 50, left: 50 };
    const chartW = width - padding.left - padding.right;
    const chartH = height - padding.top - padding.bottom;
    const barWidth = chartW / data.length - 4;

    ctx.fillStyle = '#64748b';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    const ySteps = 4;
    for (let i = 0; i <= ySteps; i++) {
      const y = padding.top + chartH - (i / ySteps) * chartH;
      const val = Math.round((i / ySteps) * maxVal);
      ctx.fillText(String(val), padding.left - 6, y);
      ctx.strokeStyle = 'rgba(148, 163, 184, 0.1)';
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(width - padding.right, y);
      ctx.stroke();
    }

    ctx.textAlign = 'right';
    ctx.textBaseline = 'top';

    for (let i = 0; i < data.length; i++) {
      const [label, count] = data[i];
      const barH = (count / maxVal) * chartH;
      const x = padding.left + i * (chartW / data.length) + 2;
      const y = padding.top + chartH - barH;

      const isSelected = localSelectedValue === label;
      const isHovered = i === hoveredBar;

      let color = colors[i % colors.length];
      if (isSelected) {
        color = '#f59e0b';
      } else if (isHovered) {
        color = color + 'cc';
      }

      const gradient = ctx.createLinearGradient(x, y, x, padding.top + chartH);
      gradient.addColorStop(0, color);
      gradient.addColorStop(1, color + '44');
      ctx.fillStyle = gradient;
      ctx.fillRect(x, y, barWidth, barH);

      ctx.strokeStyle = color + '88';
      ctx.lineWidth = 0.5;
      ctx.strokeRect(x, y, barWidth, barH);

      ctx.save();
      ctx.translate(x + barWidth / 2, padding.top + chartH + 4);
      ctx.rotate(-0.5);
      ctx.fillStyle = isSelected ? '#f59e0b' : '#94a3b8';
      ctx.font = '9px sans-serif';
      ctx.textAlign = 'right';
      ctx.fillText(label.length > 8 ? label.substring(0, 7) + '…' : label, 0, 0);
      ctx.restore();
    }

    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(columnName || $t('chart.categoryDist'), width / 2, height - 2);

    if (localSelectedValue) {
      ctx.fillStyle = '#f59e0b';
      ctx.font = '10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText($t('chart.selected', { value: localSelectedValue }), width / 2, padding.top - 2);
    }
  }

  function getBarIndex(mx: number): number {
    if (topValues.length === 0) return -1;
    const data = topValues.slice(0, maxBars);
    const padding = { top: 10, right: 10, bottom: 50, left: 50 };
    const chartW = width - padding.left - padding.right;
    const barStep = chartW / data.length;
    const idx = Math.floor((mx - padding.left) / barStep);
    return idx >= 0 && idx < data.length ? idx : -1;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || topValues.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    const idx = getBarIndex(mx);
    hoveredBar = idx;

    const data = topValues.slice(0, maxBars);
    if (idx >= 0 && idx < data.length) {
      const [label, count] = data[idx];
      tooltip = {
        x: mx + 10,
        y: my - 10,
        text: `${label}: ${count}${$t('chart.items')}`,
      };
    } else {
      tooltip = null;
    }
    draw();
  }

  function handleMouseLeave() {
    tooltip = null;
    hoveredBar = -1;
    draw();
  }

  function handleClick(e: MouseEvent) {
    if (!canvas || topValues.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const idx = getBarIndex(mx);
    if (idx < 0) return;

    const data = topValues.slice(0, maxBars);
    const [label] = data[idx];

    if (localSelectedValue === label) {
      localSelectedValue = null;
      dispatch('valueselect', null);
    } else {
      localSelectedValue = label;
      dispatch('valueselect', label);
    }
    draw();
  }

  function handleDblClick() {
    localSelectedValue = null;
    dispatch('valueselect', null);
    draw();
  }

  onMount(draw);
  afterUpdate(draw);
</script>

<div class="barchart-container" style="width:{width}px;height:{height}px">
  <canvas
    bind:this={canvas}
    width={width}
    height={height}
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
    on:click={handleClick}
    on:dblclick={handleDblClick}
    role="img"
    aria-label="{columnName} {$t('chart.categoryChart')}"
  ></canvas>
  {#if tooltip}
    <div
      class="tooltip"
      style="left:{tooltip.x}px;top:{tooltip.y}px"
    >
      {tooltip.text}
    </div>
  {/if}
  {#if localSelectedValue}
    <button class="clear-selection" on:click={handleDblClick} title={$t('chart.clearSelection')}>✕</button>
  {/if}
</div>

<style>
  .barchart-container {
    position: relative;
    display: inline-block;
  }

  canvas {
    border-radius: 6px;
    background: rgba(15, 23, 42, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.1);
    cursor: pointer;
  }

  .tooltip {
    position: absolute;
    background: rgba(30, 41, 59, 0.95);
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-size: 0.7rem;
    color: #e2e8f0;
    pointer-events: none;
    white-space: nowrap;
    z-index: 10;
  }

  .clear-selection {
    position: absolute;
    top: 2px;
    right: 2px;
    background: rgba(30, 41, 59, 0.9);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 3px;
    color: #94a3b8;
    font-size: 0.6rem;
    padding: 0.1rem 0.3rem;
    cursor: pointer;
    z-index: 5;
  }

  .clear-selection:hover { color: #e2e8f0; background: rgba(30, 41, 59, 1); }
</style>
