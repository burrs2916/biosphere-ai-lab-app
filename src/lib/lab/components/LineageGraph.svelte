<script lang="ts">
  import { onMount } from 'svelte';

  export let graph: any = null;
  export let selectedNodeId: string | null = null;
  export let traceResult: any = null;
  export let impactResult: any = null;

  let svgWidth = 800;
  let svgHeight = 500;
  let panX = 0;
  let panY = 0;
  let zoom = 1;
  let dragging = false;
  let dragStart = { x: 0, y: 0 };
  let panStart = { x: 0, y: 0 };

  const NODE_COLORS: Record<string, string> = {
    dataset: '#3b82f6',
    model: '#8b5cf6',
    experiment: '#10b981',
    transform: '#f59e0b',
    source: '#6366f1',
  };

  const NODE_ICONS: Record<string, string> = {
    dataset: '📊',
    model: '🧠',
    experiment: '🔬',
    transform: '⚙️',
    source: '📡',
  };

  $: nodes = computeLayout(graph);

  function computeLayout(g: any) {
    if (!g || !g.nodes) return [];
    const layerMap = new Map<string, number>();
    const adj: Record<string, string[]> = {};

    g.nodes.forEach((n: any) => { adj[n.id] = []; layerMap.set(n.id, 0); });
    (g.edges || []).forEach((e: any) => {
      if (adj[e.source]) adj[e.source].push(e.target);
    });

    let changed = true;
    let iterations = 0;
    while (changed && iterations < 20) {
      changed = false;
      iterations++;
      g.nodes.forEach((n: any) => {
        (adj[n.id] || []).forEach((target: string) => {
          const cur = layerMap.get(target) || 0;
          const newVal = (layerMap.get(n.id) || 0) + 1;
          if (newVal > cur) { layerMap.set(target, newVal); changed = true; }
        });
      });
    }

    const maxLayer = Math.max(...Array.from(layerMap.values()), 0);
    const layerCounts = new Map<number, number>();
    g.nodes.forEach((n: any) => {
      const layer = layerMap.get(n.id) || 0;
      layerCounts.set(layer, (layerCounts.get(layer) || 0) + 1);
    });

    const layerPositions = new Map<number, number>();
    return g.nodes.map((n: any) => {
      const layer = layerMap.get(n.id) || 0;
      const posInLayer = layerPositions.get(layer) || 0;
      layerPositions.set(layer, posInLayer + 1);
      const countInLayer = layerCounts.get(layer) || 1;
      const x = 120 + layer * 200;
      const y = 80 + posInLayer * (svgHeight / (countInLayer + 1));
      return {
        ...n,
        x,
        y,
        layer,
        type: n.type || 'dataset',
        color: NODE_COLORS[n.type] || NODE_COLORS.dataset,
        icon: NODE_ICONS[n.type] || NODE_ICONS.dataset,
      };
    });
  }

  $: edges = (graph?.edges || []).map((e: any) => {
    const source = nodes.find((n: any) => n.id === e.source);
    const target = nodes.find((n: any) => n.id === e.target);
    if (!source || !target) return null;
    return {
      ...e,
      x1: source.x + 80,
      y1: source.y + 20,
      x2: target.x,
      y2: target.y + 20,
    };
  }).filter(Boolean);

  $: tracedNodes = new Set(
    traceResult ? (traceResult.path || []).flatMap((p: any) => [p.source, p.target]) : []
  );
  $: impactedNodes = new Set(
    impactResult ? (impactResult.directly_affected || []).concat(impactResult.indirectly_affected || []) : []
  );

  function nodeClass(node: any): string {
    if (selectedNodeId === node.id) return 'node-selected';
    if (tracedNodes.has(node.id)) return 'node-traced';
    if (impactedNodes.has(node.id)) return 'node-impacted';
    return '';
  }

  function handleNodeClick(node: any) {
    selectedNodeId = selectedNodeId === node.id ? null : node.id;
  }

  function startPan(e: MouseEvent) {
    dragging = true;
    dragStart = { x: e.clientX, y: e.clientY };
    panStart = { x: panX, y: panY };
  }

  function doPan(e: MouseEvent) {
    if (!dragging) return;
    panX = panStart.x + (e.clientX - dragStart.x);
    panY = panStart.y + (e.clientY - dragStart.y);
  }

  function stopPan() { dragging = false; }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    zoom = Math.max(0.3, Math.min(3, zoom - e.deltaY * 0.001));
  }

  function resetView() {
    panX = 0; panY = 0; zoom = 1;
  }
</script>

{#if !graph || !graph.nodes || graph.nodes.length === 0}
  <div class="lineage-empty">
    <span class="empty-icon">🔗</span>
    <p>暂无血缘数据</p>
    <p class="empty-hint">注册数据集并运行实验后，血缘关系将自动生成</p>
  </div>
{:else}
  <div class="lineage-toolbar">
    <button class="toolbar-btn" on:click={resetView} title="重置视图">🔄</button>
    <button class="toolbar-btn" on:click={() => { zoom = Math.min(3, zoom + 0.2); }} title="放大">➕</button>
    <button class="toolbar-btn" on:click={() => { zoom = Math.max(0.3, zoom - 0.2); }} title="缩小">➖</button>
    <span class="zoom-label">{(zoom * 100).toFixed(0)}%</span>
    <span class="node-count">{nodes.length} 节点 · {edges.length} 边</span>
  </div>
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div class="lineage-canvas" role="application" aria-label="数据血缘图 - 可拖拽和缩放"
    on:mousedown={startPan}
    on:mousemove={doPan}
    on:mouseup={stopPan}
    on:mouseleave={stopPan}
    on:wheel={handleWheel}
  >
    <svg viewBox="0 0 {svgWidth} {svgHeight}" class="lineage-svg">
      <defs>
        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="#6b7280" />
        </marker>
        <marker id="arrowhead-traced" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="#3b82f6" />
        </marker>
        <marker id="arrowhead-impacted" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="#f59e0b" />
        </marker>
      </defs>
      <g transform="translate({panX}, {panY}) scale({zoom})">
        {#each edges as edge}
          <line
            x1={edge.x1} y1={edge.y1} x2={edge.x2} y2={edge.y2}
            stroke={tracedNodes.has(edge.source) && tracedNodes.has(edge.target) ? '#3b82f6' : impactedNodes.has(edge.source) && impactedNodes.has(edge.target) ? '#f59e0b' : '#4b5563'}
            stroke-width={tracedNodes.has(edge.source) && tracedNodes.has(edge.target) ? 2.5 : 1.5}
            stroke-dasharray={edge.label === 'derived' ? '6,3' : 'none'}
            marker-end={tracedNodes.has(edge.source) && tracedNodes.has(edge.target) ? 'url(#arrowhead-traced)' : impactedNodes.has(edge.source) && impactedNodes.has(edge.target) ? 'url(#arrowhead-impacted)' : 'url(#arrowhead)'}
            opacity="0.7"
          />
        {/each}
        {#each nodes as node}
          <g class="lineage-node {nodeClass(node)}" role="button" tabindex="0" aria-label="{node.name || node.id}" on:click={() => handleNodeClick(node)} on:keydown={(e) => e.key === 'Enter' && handleNodeClick(node)}>
            <rect
              x={node.x} y={node.y - 20}
              width="160" height="40" rx="8"
              fill={selectedNodeId === node.id ? node.color : 'rgba(30,41,59,0.9)'}
              stroke={node.color}
              stroke-width={selectedNodeId === node.id ? 2.5 : 1.5}
            />
            <text x={node.x + 12} y={node.y + 2} class="node-icon">{node.icon}</text>
            <text x={node.x + 32} y={node.y + 5} class="node-label" fill={selectedNodeId === node.id ? '#fff' : '#e5e7eb'}>
              {(node.name || node.id).substring(0, 14)}
            </text>
            <text x={node.x + 32} y={node.y + 16} class="node-type" fill={node.color}>
              {node.type}
            </text>
          </g>
        {/each}
      </g>
    </svg>
  </div>

  {#if selectedNodeId}
    <div class="node-detail">
      <div class="detail-header">
        <span>📌 选中节点</span>
        <button class="close-btn" on:click={() => { selectedNodeId = null; }}>✕</button>
      </div>
      <div class="detail-id">{selectedNodeId}</div>
      {#if nodes.find((n: any) => n.id === selectedNodeId)}
        {@const sn = nodes.find((n: any) => n.id === selectedNodeId)}
        <div class="detail-type" style="color: {sn.color}">{sn.type}</div>
        <div class="detail-name">{sn.name || sn.id}</div>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .lineage-empty { text-align: center; padding: 2rem 1rem; color: #6b7280; }
  .empty-icon { font-size: 2rem; }
  .lineage-empty p { margin: 0.3rem 0; font-size: 0.85rem; }
  .empty-hint { font-size: 0.72rem; color: #4b5563; }

  .lineage-toolbar {
    display: flex; align-items: center; gap: 0.4rem;
    padding: 0.4rem 0.6rem; background: rgba(30,41,59,0.8);
    border-radius: 6px; margin-bottom: 0.5rem;
  }
  .toolbar-btn {
    width: 28px; height: 28px; border: 1px solid rgba(148,163,184,0.2);
    border-radius: 4px; background: rgba(255,255,255,0.05);
    color: #e5e7eb; font-size: 0.8rem; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
  }
  .toolbar-btn:hover { background: rgba(255,255,255,0.1); }
  .zoom-label { font-size: 0.72rem; color: #9ca3af; margin-left: 0.3rem; }
  .node-count { font-size: 0.72rem; color: #6b7280; margin-left: auto; }

  .lineage-canvas {
    border: 1px solid rgba(148,163,184,0.15);
    border-radius: 8px; overflow: hidden; cursor: grab;
    background: rgba(15,23,42,0.5);
  }
  .lineage-canvas:active { cursor: grabbing; }
  .lineage-svg { width: 100%; height: 420px; }

  .lineage-node { cursor: pointer; }
  .lineage-node:hover rect { filter: brightness(1.2); }
  .node-icon { font-size: 14px; dominant-baseline: middle; }
  .node-label { font-size: 11px; font-weight: 600; dominant-baseline: middle; }
  .node-type { font-size: 9px; dominant-baseline: middle; text-transform: uppercase; letter-spacing: 0.5px; }

  .node-selected rect { filter: drop-shadow(0 0 6px rgba(59,130,246,0.5)); }
  .node-traced rect { stroke: #3b82f6; stroke-width: 2; stroke-dasharray: 4,2; }
  .node-impacted rect { stroke: #f59e0b; stroke-width: 2; }

  .node-detail {
    position: absolute; bottom: 0.5rem; right: 0.5rem;
    background: rgba(30,41,59,0.95); border: 1px solid rgba(148,163,184,0.2);
    border-radius: 8px; padding: 0.6rem; min-width: 200px;
    backdrop-filter: blur(8px);
  }
  .detail-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.4rem; font-size: 0.8rem; color: #e5e7eb; }
  .close-btn { background: none; border: none; color: #9ca3af; cursor: pointer; font-size: 0.8rem; }
  .close-btn:hover { color: #e5e7eb; }
  .detail-id { font-size: 0.7rem; color: #6b7280; font-family: monospace; }
  .detail-type { font-size: 0.75rem; font-weight: 600; text-transform: uppercase; }
  .detail-name { font-size: 0.85rem; color: #e5e7eb; font-weight: 500; margin-top: 0.2rem; }
</style>
