<script lang="ts">
  import { onMount, afterUpdate } from 'svelte';
  import type { ColumnProfile } from '$lib/lab/adapter/types';

  export let profiles: ColumnProfile[] = [];
  export let width: number = 500;
  export let height: number = 200;

  let canvas: HTMLCanvasElement;
  let tooltip: { x: number; y: number; text: string } | null = null;

  function getColor(ratio: number): string {
    if (ratio === 0) return '#10b981';
    if (ratio < 0.05) return '#84cc16';
    if (ratio < 0.1) return '#f59e0b';
    if (ratio < 0.3) return '#f97316';
    return '#ef4444';
  }

  function draw() {
    if (!canvas || profiles.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    ctx.clearRect(0, 0, width, height);

    const padding = { top: 10, right: 20, bottom: 10, left: 120 };
    const chartW = width - padding.left - padding.right;
    const chartH = height - padding.top - padding.bottom;
    const cellH = Math.min(chartH, 30);
    const cellW = Math.min(chartW / profiles.length, 60);

    const startY = padding.top + (chartH - cellH) / 2;

    for (let i = 0; i < profiles.length; i++) {
      const p = profiles[i];
      const ratio = p.total_count > 0 ? p.null_count / p.total_count : 0;
      const x = padding.left + i * cellW;
      const y = startY;

      ctx.fillStyle = getColor(ratio);
      ctx.fillRect(x, y, cellW - 2, cellH);

      ctx.strokeStyle = 'rgba(148, 163, 184, 0.2)';
      ctx.lineWidth = 0.5;
      ctx.strokeRect(x, y, cellW - 2, cellH);

      ctx.fillStyle = '#e2e8f0';
      ctx.font = '9px sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      const pct = (ratio * 100).toFixed(1);
      ctx.fillText(`${pct}%`, x + (cellW - 2) / 2, y + cellH / 2);
    }

    ctx.fillStyle = '#94a3b8';
    ctx.font = '10px sans-serif';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    for (let i = 0; i < profiles.length; i++) {
      const p = profiles[i];
      const x = padding.left + i * cellW;
      const y = startY;
      const label = p.name.length > 12 ? p.name.substring(0, 11) + '…' : p.name;

      ctx.save();
      ctx.translate(padding.left - 6, y + cellH / 2);
      ctx.fillText(label, 0, 0);
      ctx.restore();
    }

    const legendX = padding.left;
    const legendY = startY + cellH + 16;
    const legendItems = [
      { label: '0%', color: '#10b981' },
      { label: '<5%', color: '#84cc16' },
      { label: '<10%', color: '#f59e0b' },
      { label: '<30%', color: '#f97316' },
      { label: '≥30%', color: '#ef4444' },
    ];

    const legendW = 40;
    for (let i = 0; i < legendItems.length; i++) {
      const lx = legendX + i * (legendW + 20);
      ctx.fillStyle = legendItems[i].color;
      ctx.fillRect(lx, legendY, 10, 10);
      ctx.fillStyle = '#94a3b8';
      ctx.font = '8px sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(legendItems[i].label, lx + 14, legendY + 8);
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas || profiles.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    const padding = { top: 10, right: 20, bottom: 10, left: 120 };
    const chartW = width - padding.left - padding.right;
    const cellW = Math.min(chartW / profiles.length, 60);

    const idx = Math.floor((mx - padding.left) / cellW);
    if (idx >= 0 && idx < profiles.length) {
      const p = profiles[idx];
      const ratio = p.total_count > 0 ? p.null_count / p.total_count : 0;
      tooltip = {
        x: mx + 10,
        y: my - 10,
        text: `${p.name}: ${p.null_count}/${p.total_count} 缺失 (${(ratio * 100).toFixed(1)}%)`,
      };
    } else {
      tooltip = null;
    }
  }

  function handleMouseLeave() {
    tooltip = null;
  }

  onMount(draw);
  afterUpdate(draw);
</script>

<div class="heatmap-container" style="width:{width}px;height:{height}px">
  <canvas
    bind:this={canvas}
    width={width}
    height={height}
    on:mousemove={handleMouseMove}
    on:mouseleave={handleMouseLeave}
  ></canvas>
  {#if tooltip}
    <div
      class="tooltip"
      style="left:{tooltip.x}px;top:{tooltip.y}px"
    >
      {tooltip.text}
    </div>
  {/if}
</div>

<style>
  .heatmap-container {
    position: relative;
    display: inline-block;
  }

  canvas {
    border-radius: 6px;
    background: rgba(15, 23, 42, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.1);
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
</style>
