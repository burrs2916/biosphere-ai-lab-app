<script lang="ts">
	import { onMount } from 'svelte';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import Skeleton from '$lib/lab/components/Skeleton.svelte';

	interface LineageNode {
		id: string;
		name: string;
		node_type: string;
		version: string;
		digest: string | null;
		created_at: string;
		metadata: Record<string, any>;
	}

	interface LineageEdge {
		from: string;
		to: string;
		relation: string;
		transform: string | null;
		params: any;
		created_at: string;
	}

	interface LineageGraph {
		nodes: LineageNode[];
		edges: LineageEdge[];
		metadata: {
			total_nodes: number;
			total_edges: number;
			max_depth: number;
			has_cycles: boolean;
			root_nodes: string[];
			leaf_nodes: string[];
		};
	}

	let graph: LineageGraph | null = null;
	let loading = true;
	let error: string | null = null;

	let selectedNode: LineageNode | null = null;
	let selectedNodeTrace: any = null;
	let selectedNodeImpact: any = null;
	let traceLoading = false;

	let svgWidth = 900;
	let svgHeight = 600;

	const nodeColors: Record<string, string> = {
		dataset: '#10b981',
		raw_data: '#f59e0b',
		processed_data: '#8b5cf6',
		split: '#ec4899',
		model: '#3b82f6',
		experiment: '#ef4444',
		checkpoint: '#06b6d4',
		export: '#84cc16',
	};

	const nodeIcons: Record<string, string> = {
		dataset: '📊',
		raw_data: '📥',
		processed_data: '⚙️',
		split: '✂️',
		model: '🧠',
		experiment: '🔬',
		checkpoint: '💾',
		export: '📤',
	};

	const nodeLabels: Record<string, string> = {
		dataset: '数据集',
		raw_data: '原始数据',
		processed_data: '处理后数据',
		split: '数据划分',
		model: '模型',
		experiment: '实验',
		checkpoint: '检查点',
		export: '导出',
	};

	const relationLabels: Record<string, string> = {
		trained_on: '训练于',
		derived_from: '派生自',
		evaluated_on: '评估于',
		split_from: '划分自',
		preprocessed_from: '预处理自',
		augmented_from: '增强自',
		exported_from: '导出自',
		depends_on: '依赖于',
	};

	function getNodeColor(type: string): string {
		return nodeColors[type] || '#6b7280';
	}

	function getNodeIcon(type: string): string {
		return nodeIcons[type] || '📌';
	}

	function getNodeLabel(type: string): string {
		return nodeLabels[type] || type;
	}

	function getRelationLabel(relation: string): string {
		return relationLabels[relation] || relation;
	}

	function computeLayout(): Map<string, { x: number; y: number }> {
		const positions = new Map<string, { x: number; y: number }>();
		if (!graph) return positions;

		const inDegree = new Map<string, number>();
		const outEdges = new Map<string, string[]>();
		for (const node of graph.nodes) {
			inDegree.set(node.id, 0);
			outEdges.set(node.id, []);
		}
		for (const edge of graph.edges) {
			inDegree.set(edge.to, (inDegree.get(edge.to) || 0) + 1);
			const outs = outEdges.get(edge.from) || [];
			outs.push(edge.to);
			outEdges.set(edge.from, outs);
		}

		const layers: string[][] = [];
		const visited = new Set<string>();
		let queue = graph.nodes.filter((n) => (inDegree.get(n.id) || 0) === 0).map((n) => n.id);

		while (queue.length > 0) {
			layers.push([...queue]);
			const nextQueue: string[] = [];
			for (const id of queue) {
				visited.add(id);
				for (const to of outEdges.get(id) || []) {
					if (!visited.has(to) && !nextQueue.includes(to)) {
						nextQueue.push(to);
					}
				}
			}
			queue = nextQueue;
		}

		for (const node of graph.nodes) {
			if (!visited.has(node.id)) {
				if (layers.length === 0) layers.push([]);
				layers[layers.length - 1].push(node.id);
			}
		}

		const layerHeight = svgHeight / (layers.length + 1);
		for (let i = 0; i < layers.length; i++) {
			const layer = layers[i];
			const layerWidth = svgWidth / (layer.length + 1);
			for (let j = 0; j < layer.length; j++) {
				positions.set(layer[j], {
					x: layerWidth * (j + 1),
					y: layerHeight * (i + 1),
				});
			}
		}

		return positions;
	}

	async function loadGraph() {
		loading = true;
		error = null;
		try {
			const client = getLabClient();
			graph = await client.lineageGraph();
		} catch (e: any) {
			error = e?.toString() || '加载血缘图失败';
		} finally {
			loading = false;
		}
	}

	async function selectNode(node: LineageNode) {
		selectedNode = node;
		selectedNodeTrace = null;
		selectedNodeImpact = null;
		traceLoading = true;
		try {
			const client = getLabClient();
			if (graph) {
				const [trace, impact] = await Promise.all([
					client.lineageTrace(graph, node.id),
					client.lineageImpact(graph, node.id),
				]);
				selectedNodeTrace = trace;
				selectedNodeImpact = impact;
			}
		} catch (e) {
			console.error('Failed to load node details:', e);
		} finally {
			traceLoading = false;
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	function truncateDigest(d: string): string {
		return d.substring(0, 8);
	}

	$: positions = computeLayout();
	$: nodePositions = positions;

	onMount(() => {
		loadGraph();
	});
</script>

<div class="lineage-page">
	<div class="page-header">
		<div>
			<h2>数据血缘追踪</h2>
			<p class="subtitle">数据集 → 实验 → 模型 全链路追踪</p>
		</div>
		<button class="btn-refresh" on:click={loadGraph} disabled={loading}>
			{loading ? '加载中...' : '🔄 刷新'}
		</button>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="loading-container">
			<Skeleton height="400px" />
		</div>
	{:else if graph && graph.nodes.length > 0}
		<div class="lineage-layout">
			<div class="graph-panel">
				<div class="graph-stats">
					<span class="stat-badge">节点: {graph.metadata.total_nodes}</span>
					<span class="stat-badge">边: {graph.metadata.total_edges}</span>
					<span class="stat-badge">深度: {graph.metadata.max_depth}</span>
					{#if graph.metadata.has_cycles}
						<span class="stat-badge warning">⚠ 循环依赖</span>
					{/if}
				</div>
				<svg width={svgWidth} height={svgHeight} viewBox="0 0 {svgWidth} {svgHeight}" class="lineage-svg">
					{#each graph.edges as edge}
						{@const fromPos = nodePositions.get(edge.from)}
						{@const toPos = nodePositions.get(edge.to)}
						{#if fromPos && toPos}
							{@const mx = (fromPos.x + toPos.x) / 2}
							{@const my = (fromPos.y + toPos.y) / 2}
							<line
								x1={fromPos.x} y1={fromPos.y}
								x2={toPos.x} y2={toPos.y}
								stroke="rgba(255,255,255,0.15)"
								stroke-width="2"
								marker-end="url(#arrowhead)"
							/>
							<text
								x={mx} y={my - 6}
								fill="rgba(255,255,255,0.4)"
								font-size="10"
								text-anchor="middle"
							>
								{getRelationLabel(edge.relation)}
							</text>
						{/if}
					{/each}

					<defs>
						<marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
							<polygon points="0 0, 8 3, 0 6" fill="rgba(255,255,255,0.3)" />
						</marker>
					</defs>

					{#each graph.nodes as node}
						{@const pos = nodePositions.get(node.id)}
						{#if pos}
							{@const color = getNodeColor(node.node_type)}
							{@const isSelected = selectedNode?.id === node.id}
							<g
								class="lineage-node"
								class:selected={isSelected}
								role="button"
								tabindex="0"
								aria-label="{node.name || node.id}"
								on:click={() => selectNode(node)}
								on:keydown={(e) => e.key === 'Enter' && selectNode(node)}
								transform="translate({pos.x - 60}, {pos.y - 28})"
							>
								<rect
									x="0" y="0"
									width="120" height="56"
									rx="8" ry="8"
									fill="rgba(15, 23, 42, 0.95)"
									stroke={color}
									stroke-width={isSelected ? 3 : 1.5}
									stroke-opacity={isSelected ? 1 : 0.6}
								/>
								<text x="12" y="22" font-size="14" fill={color}>{getNodeIcon(node.node_type)}</text>
								<text
									x="34" y="22"
									font-size="12"
									fill="#e5e7eb"
									font-weight="500"
									textLength="74"
									lengthAdjust="spacing"
								>
									{node.name.length > 12 ? node.name.substring(0, 11) + '…' : node.name}
								</text>
								<text x="12" y="42" font-size="10" fill="rgba(255,255,255,0.4)">
									{getNodeLabel(node.node_type)} · v{node.version}
								</text>
							</g>
						{/if}
					{/each}
				</svg>
			</div>

			<div class="detail-panel">
				{#if selectedNode}
					<div class="detail-card">
						<h3 class="detail-title" style="color: {getNodeColor(selectedNode.node_type)}">
							{getNodeIcon(selectedNode.node_type)} {selectedNode.name}
						</h3>
						<div class="detail-grid">
							<div class="detail-item">
								<span class="detail-label">类型</span>
								<span class="detail-value">{getNodeLabel(selectedNode.node_type)}</span>
							</div>
							<div class="detail-item">
								<span class="detail-label">版本</span>
								<span class="detail-value">v{selectedNode.version}</span>
							</div>
							<div class="detail-item">
								<span class="detail-label">创建时间</span>
								<span class="detail-value">{formatDate(selectedNode.created_at)}</span>
							</div>
							{#if selectedNode.digest}
								<div class="detail-item">
									<span class="detail-label">数据摘要</span>
									<span class="detail-value mono">{truncateDigest(selectedNode.digest)}</span>
								</div>
							{/if}
						</div>

						{#if traceLoading}
							<Skeleton height="80px" marginTop="12px" />
						{:else}
							{#if selectedNodeTrace}
								<div class="trace-section">
									<h4 class="section-subtitle">上下游分析</h4>
									<div class="trace-row">
										<span class="trace-label">上游 ({selectedNodeTrace.upstream?.length || 0})</span>
										<div class="trace-nodes">
											{#each selectedNodeTrace.upstream || [] as up}
												<span class="trace-chip">{up.name || up}</span>
											{/each}
											{#if !selectedNodeTrace.upstream?.length}
												<span class="trace-empty">无</span>
											{/if}
										</div>
									</div>
									<div class="trace-row">
										<span class="trace-label">下游 ({selectedNodeTrace.downstream?.length || 0})</span>
										<div class="trace-nodes">
											{#each selectedNodeTrace.downstream || [] as down}
												<span class="trace-chip">{down.name || down}</span>
											{/each}
											{#if !selectedNodeTrace.downstream?.length}
												<span class="trace-empty">无</span>
											{/if}
										</div>
									</div>
								</div>
							{/if}

							{#if selectedNodeImpact}
								<div class="impact-section">
									<h4 class="section-subtitle">影响分析</h4>
									<div class="impact-severity" class:high={selectedNodeImpact.severity === 'high'} class:medium={selectedNodeImpact.severity === 'medium'} class:low={selectedNodeImpact.severity === 'low'}>
										严重程度: {selectedNodeImpact.severity === 'high' ? '🔴 高' : selectedNodeImpact.severity === 'medium' ? '🟡 中' : '🟢 低'}
									</div>
									<div class="impact-stat">
										直接影响: {selectedNodeImpact.directly_affected?.length || 0} 个节点
									</div>
									<div class="impact-stat">
										间接影响: {selectedNodeImpact.indirectly_affected?.length || 0} 个节点
									</div>
									{#if selectedNodeImpact.recommendations?.length}
										<div class="impact-recommendations">
											{#each selectedNodeImpact.recommendations as rec}
												<div class="rec-item">{rec}</div>
											{/each}
										</div>
									{/if}
								</div>
							{/if}
						{/if}
					</div>
				{:else}
					<div class="detail-empty">
						<p>👆 点击图中节点查看详情</p>
						<p class="hint">包括上下游分析和影响评估</p>
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<div class="empty-state">
			<div class="empty-icon">🔗</div>
			<h3>暂无血缘数据</h3>
			<p>注册数据集并运行实验后，血缘关系将自动建立</p>
		</div>
	{/if}
</div>

<style>
	.lineage-page {
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
	}

	.page-header h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: #e5e7eb;
		margin: 0;
	}

	.subtitle {
		color: #6b7280;
		font-size: 0.85rem;
		margin: 0.25rem 0 0 0;
	}

	.btn-refresh {
		padding: 0.5rem 1rem;
		background: rgba(255,255,255,0.05);
		border: 1px solid rgba(255,255,255,0.1);
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		font-size: 0.85rem;
		transition: all 0.2s;
	}

	.btn-refresh:hover:not(:disabled) {
		background: rgba(255,255,255,0.08);
		color: #e5e7eb;
	}

	.error-banner {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		margin-bottom: 1rem;
		font-size: 0.85rem;
	}

	.loading-container {
		border-radius: 12px;
		overflow: hidden;
	}

	.lineage-layout {
		display: grid;
		grid-template-columns: 1fr 320px;
		gap: 1rem;
	}

	.graph-panel {
		background: rgba(15, 23, 42, 0.6);
		border: 1px solid rgba(255,255,255,0.06);
		border-radius: 12px;
		padding: 1rem;
		overflow: auto;
	}

	.graph-stats {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
		flex-wrap: wrap;
	}

	.stat-badge {
		padding: 0.2rem 0.6rem;
		background: rgba(255,255,255,0.05);
		border-radius: 4px;
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.stat-badge.warning {
		color: #fbbf24;
		background: rgba(251, 191, 36, 0.1);
	}

	.lineage-svg {
		width: 100%;
		height: auto;
	}

	.lineage-node {
		cursor: pointer;
		transition: opacity 0.2s;
	}

	.lineage-node:hover {
		opacity: 0.9;
	}

	.lineage-node:hover rect {
		stroke-opacity: 1;
	}

	.detail-panel {
		position: sticky;
		top: 0;
	}

	.detail-card {
		background: rgba(15, 23, 42, 0.8);
		border: 1px solid rgba(255,255,255,0.08);
		border-radius: 12px;
		padding: 1.25rem;
	}

	.detail-title {
		font-size: 1rem;
		font-weight: 600;
		margin: 0 0 1rem 0;
	}

	.detail-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.detail-item {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.detail-label {
		font-size: 0.7rem;
		color: #6b7280;
		text-transform: uppercase;
	}

	.detail-value {
		font-size: 0.8rem;
		color: #d1d5db;
	}

	.detail-value.mono {
		font-family: monospace;
		font-size: 0.75rem;
	}

	.section-subtitle {
		font-size: 0.8rem;
		font-weight: 600;
		color: #9ca3af;
		margin: 0.75rem 0 0.5rem 0;
	}

	.trace-section, .impact-section {
		border-top: 1px solid rgba(255,255,255,0.06);
		padding-top: 0.5rem;
	}

	.trace-row {
		margin-bottom: 0.5rem;
	}

	.trace-label {
		font-size: 0.7rem;
		color: #6b7280;
		display: block;
		margin-bottom: 0.25rem;
	}

	.trace-nodes {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}

	.trace-chip {
		padding: 0.15rem 0.5rem;
		background: rgba(255,255,255,0.05);
		border-radius: 4px;
		font-size: 0.7rem;
		color: #9ca3af;
	}

	.trace-empty {
		font-size: 0.7rem;
		color: #4b5563;
	}

	.impact-severity {
		font-size: 0.8rem;
		padding: 0.3rem 0.5rem;
		border-radius: 4px;
		margin-bottom: 0.5rem;
	}

	.impact-severity.high {
		background: rgba(239, 68, 68, 0.1);
		color: #fca5a5;
	}

	.impact-severity.medium {
		background: rgba(251, 191, 36, 0.1);
		color: #fde68a;
	}

	.impact-severity.low {
		background: rgba(16, 185, 129, 0.1);
		color: #6ee7b7;
	}

	.impact-stat {
		font-size: 0.75rem;
		color: #9ca3af;
		margin-bottom: 0.25rem;
	}

	.impact-recommendations {
		margin-top: 0.5rem;
	}

	.rec-item {
		font-size: 0.7rem;
		color: #fbbf24;
		padding: 0.25rem 0;
		border-bottom: 1px solid rgba(255,255,255,0.04);
	}

	.detail-empty {
		background: rgba(15, 23, 42, 0.6);
		border: 1px solid rgba(255,255,255,0.06);
		border-radius: 12px;
		padding: 2rem;
		text-align: center;
		color: #6b7280;
	}

	.detail-empty p {
		margin: 0.25rem 0;
		font-size: 0.85rem;
	}

	.detail-empty .hint {
		font-size: 0.75rem;
		color: #4b5563;
	}

	.empty-state {
		text-align: center;
		padding: 4rem 2rem;
		background: rgba(15, 23, 42, 0.4);
		border: 1px solid rgba(255,255,255,0.06);
		border-radius: 12px;
	}

	.empty-icon {
		font-size: 3rem;
		margin-bottom: 1rem;
	}

	.empty-state h3 {
		color: #9ca3af;
		margin: 0 0 0.5rem 0;
	}

	.empty-state p {
		color: #6b7280;
		font-size: 0.85rem;
		margin: 0;
	}
</style>
