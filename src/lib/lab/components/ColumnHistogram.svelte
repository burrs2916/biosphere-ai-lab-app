<script lang="ts">
  import { onMount, afterUpdate, createEventDispatcher } from 'svelte';
  import { t } from '$lib/i18n';

  export let columnName: string = '';
  export let values: number[] = [];
  export let width: number = 400;
  export let height: number = 200;
  export let bins: number = 20;
  export let selectedRange: [number, number] | null = null;

  const dispatch = createEventDispatcher();

  let canvas: HTMLCanvasElement;
  let tooltip: { x: number; y: number; text: string } | null = null;
  let hoveredBin: number = -1;
  let localSelectedRange: [number, number] | null = null;
  let isDragging = false;
  let dragStartBin = -1;

  $: localSelectedRange = selectedRange;

  const colors = {
    bar: '#3b82f6',
    barHover: '#60a5fa',
    barSelected: '#f59e0b',
    barSelectedHover: '#fbbf24',
    grid: 'rgba(148, 163, 184, 0.1)',
    text: '#94a3b8',
    axis: '#64748b',
  };

  let binData: { start: number; end: number; count: number }[] = [];

  function computeBins() {
    if (values.length === 0) { binData = []; return; }
    const min = Math.min(...values);
    const max = Math.max(...values);
    const range = max - min || 1;
    const binWidth = range / bins;
    binData = [];
    for (let i = 0; i < bins; i++) {
      const start = min + i * binWidth;
      const end = start + binWidth;
      const count = values.filter(v => v >= start && (i === bins - 1 ? v <= end : v < end)).length;
      binData.push({ start, end, count });
    }
  }

  function draw() {
    if (!canvas || binData.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    const padding = { top: 15, right: 10, bottom: 35, left: 45 };
    const chartW = width - padding.left - padding.right;
    const chartH = height - padding.top - padding.bottom;
    const maxCount = Math.max(...binData.map(b => b.count), 1);
    const barWidth = chartW / binData.length - 2;

    ctx.fillStyle = colors.axis;
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    const ySteps = 4;
    for (let i = 0; i <= ySteps; i++) {
      const y = padding.top + chartH - (i / ySteps) * chartH;
      const val = Math.round((i / ySteps) * maxCount);
      ctx.fillText(String(val), padding.left - 6, y);
      ctx.strokeStyle = colors.grid;
      ctx.beginPath();
      ctx.moveTo(padding.left, y);
      ctx.lineTo(width - padding.right, y);
      ctx.stroke();
    }

    for (let i = 0; i < binData.length; i++) {
      const { start, end, count } = binData[i];
      const barH = (count / maxCount) * chartH;
      const x = padding.left + i * (chartW / binData.length) + 1;
      const y = padding.top + chartH - barH;

      const isSelected = localSelectedRange && start >= localSelectedRange[0] && end <= localSelectedRange[1];
      const isHovered = i === hoveredBin;

      let fillColor: string;
      if (isSelected) {
        fillColor = isHovered ? colors.barSelectedHover : colors.barSelected;
      } else {
        fillColor = isHovered ? colors.barHover : colors.bar;
      }

      const gradient = ctx.createLinearGradient(x, y, x, padding.top + chartH);
      gradient.addColorStop(0, fillColor);
      gradient.addColorStop(1, fillColor + '44');
      ctx.fillStyle = gradient;
      ctx.fillRect(x, y, barWidth, barH);

      ctx.strokeStyle = fillColor + '88';
      ctx.lineWidth = 0.5;
      ctx.strokeRect(x, y, barWidth, barH);

      if (i % Math.ceil(binData.length / 5) === 0) {
        ctx.fillStyle = colors.text;
        ctx.font = '9px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';
        ctx.fillText(start.toFixed(1), x + barWidth / 2, padding.top + chartH + 4);
      }
    }

    ctx.fillStyle = colors.text;
    ctx.font = '11px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(columnName || $t('chart.numericDist'), width / 2, height - 2);

    if (localSelectedRange) {
      ctx.fillStyle = colors.barSelected;
      ctx.font = '10px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(
        $t('chart.rangeSelected', { from: localSelectedRange[0].toFixed(1), to: localSelectedRange[1].toFixed(1) }),
        width / 2,
        padding.top - 2
      );
    }
  }

  function getBinIndex(mx: number): number {
    if (binData.length === 0) return -1;
    const padding = { top: 15, right: 10, bottom: 35, left: 45 };
    const chartW = width - padding.left - padding.right;
    const binStep = chartW / binData.length;
    const idx = Math.floor((mx - padding.left) / binStep);
    return idx >= 0 && idx < binData.length ? idx : -1;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || binData.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    const idx = getBinIndex(mx);
    hoveredBin = idx;

    if (idx >= 0) {
      const { start, end, count } = binData[idx];
      tooltip = {
        x: mx + 10,
        y: my - 10,
        text: `${start.toFixed(2)} – ${end.toFixed(2)}: ${count}${$t('chart.items')}`,
      };
    } else {
      tooltip = null;
    }

    draw();
  }

  function handleMouseLeave() {
    tooltip = null;
    hoveredBin = -1;
    isDragging = false;
    draw();
  }

  function handleClick(e: MouseEvent) {
    if (!canvas || binData.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const idx = getBinIndex(mx);
    if (idx < 0) return;

    const { start, end } = binData[idx];

    if (localSelectedRange && localSelectedRange[0] === start && localSelectedRange[1] === end) {
      localSelectedRange = null;
      dispatch('rangeselect', null);
    } else {
      localSelectedRange = [start, end];
      dispatch('rangeselect', { start, end });
    }
    draw();
  }

  function handleDblClick() {
    localSelectedRange = null;
    dispatch('rangeselect', null);
    draw();
  }

  $: {
    computeBins();
  }

  onMount(() => {
    computeBins();
    draw();
  });

  afterUpdate(draw);
</script>

<div class="histogram-container" style="width:{width}px;height:{height}px">
  <canvas
    bind:this={canvas}
    width={width}
    height={height}
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
    on:click={handleClick}
    on:dblclick={handleDblClick}
    role="img"
    aria-label="{columnName} {$t('chart.histogram')}"
  ></canvas>
  {#if tooltip}
    <div
      class="tooltip"
      style="left:{tooltip.x}px;top:{tooltip.y}px"
    >
      {tooltip.text}
    </div>
  {/if}
  {#if localSelectedRange}
    <button class="clear-selection" on:click={handleDblClick} title={$t('chart.clearSelection')}>✕</button>
  {/if}
</div>

<style>
  .histogram-container {
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
