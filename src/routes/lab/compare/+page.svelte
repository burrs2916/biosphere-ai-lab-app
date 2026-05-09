<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import * as echarts from 'echarts';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { t } from '$lib/i18n';
	import MetricsChart from '$lib/lab/components/MetricsChart.svelte';
	import type { ExperimentSummary, ExperimentDetail, MetricSeries } from '$lib/lab/adapter/types';

	interface ConfigDiff {
		key: string;
		values: string[];
		same: boolean;
	}

	function getConfigDiffs(details: Map<string, ExperimentDetail>): ConfigDiff[] {
		const entries = Array.from(details.values());
		if (entries.length < 2) return [];

		const keys = ['epochs', 'batch_size', 'learning_rate', 'compute_backend', 'loss_function', 'optimizer', 'validation_split'];
		const diffs: ConfigDiff[] = [];

		for (const key of keys) {
			const values = entries.map(e => {
				const config: any = e.config;
				if (key === 'optimizer') {
					const opt = config.optimizer;
					if (opt.Sgd) return 'SGD';
					if (opt.Adam) return 'Adam';
					if (opt.AdamW) return 'AdamW';
					if (opt.Rmsprop) return 'Rmsprop';
					return JSON.stringify(opt);
				}
				return String(config[key] ?? '-');
			});
			const same = values.every(v => v === values[0]);
			diffs.push({ key, values, same });
		}

		return diffs;
	}

	let experiments: ExperimentSummary[] = [];
	let selectedIds: Set<string> = new Set();
	let details: Map<string, ExperimentDetail> = new Map();
	let loading = true;
	let comparing = false;
	let error: string | null = null;
	let compareMetric: string = 'loss';
	let availableMetrics: string[] = [];
	let activeView: 'chart' | 'scatter' | 'parallel' | 'table' = 'chart';

	let scatterContainer: HTMLDivElement;
	let scatterChart: echarts.ECharts | null = null;
	let parallelContainer: HTMLDivElement;
	let parallelChart: echarts.ECharts | null = null;
	let scatterResizeObserver: ResizeObserver | null = null;
	let parallelResizeObserver: ResizeObserver | null = null;

	const colors = ['#10b981', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6'];

	function getColor(index: number): string {
		return colors[index % colors.length];
	}

	onMount(async () => {
		const client = getLabClient();
		try {
			experiments = await client.listExperiments();
		} catch (e: any) {
			error = e?.message || $t('compare.loadListFailed');
		} finally {
			loading = false;
		}
	});

	onDestroy(() => {
		if (scatterResizeObserver) scatterResizeObserver.disconnect();
		if (parallelResizeObserver) parallelResizeObserver.disconnect();
		if (scatterChart) { scatterChart.dispose(); scatterChart = null; }
		if (parallelChart) { parallelChart.dispose(); parallelChart = null; }
	});

	function toggleSelect(id: string) {
		const newSet = new Set(selectedIds);
		if (newSet.has(id)) {
			newSet.delete(id);
		} else if (newSet.size < 5) {
			newSet.add(id);
		}
		selectedIds = newSet;
	}

	async function compare() {
		if (selectedIds.size < 2) return;
		comparing = true;
		error = null;
		details = new Map();
		const client = getLabClient();
		const allMetrics = new Set<string>();

		try {
			for (const id of selectedIds) {
				const detail = await client.getExperimentDetail(id);
				details.set(id, detail);
				for (const name of Object.keys(detail.metrics.series)) {
					allMetrics.add(name);
				}
			}
			availableMetrics = Array.from(allMetrics).sort();
			if (availableMetrics.length > 0 && !availableMetrics.includes(compareMetric)) {
				compareMetric = availableMetrics[0];
			}
		} catch (e: any) {
			error = e?.message || $t('compare.loadDetailFailed');
		} finally {
			comparing = false;
		}
	}

	function getSeries(id: string, metricName: string): MetricSeries | null {
		const detail = details.get(id);
		if (!detail) return null;
		return detail.metrics.series[metricName] || null;
	}

	function getBestValue(id: string, metricName: string): number | null {
		const series = getSeries(id, metricName);
		if (!series || series.values.length === 0) return null;
		const isLoss = metricName.toLowerCase().includes('loss') || metricName.toLowerCase().includes('error');
		if (isLoss) {
			return Math.min(...series.values.map(v => v.value));
		}
		return Math.max(...series.values.map(v => v.value));
	}

	function getLastValue(id: string, metricName: string): number | null {
		const series = getSeries(id, metricName);
		if (!series || series.values.length === 0) return null;
		return series.values[series.values.length - 1].value;
	}

	function formatNum(n: number | null): string {
		if (n === null) return '-';
		return n.toFixed(4);
	}

	function buildScatterOption() {
		if (availableMetrics.length < 2) return {};

		const xMetric = availableMetrics[0];
		const yMetric = availableMetrics.length > 1 ? availableMetrics[1] : availableMetrics[0];

		const scatterData: any[] = [];
		const idxArr = Array.from(selectedIds);
		idxArr.forEach((id, i) => {
			const detail = details.get(id);
			if (!detail) return;
			const xSeries = detail.metrics.series[xMetric];
			const ySeries = detail.metrics.series[yMetric];
			if (!xSeries || !ySeries) return;

			const minLen = Math.min(xSeries.values.length, ySeries.values.length);
			for (let j = 0; j < minLen; j++) {
				scatterData.push({
					value: [xSeries.values[j].value, ySeries.values[j].value, xSeries.values[j].step],
					itemStyle: { color: getColor(i) },
					experimentName: detail.name,
				});
			}
		});

		return {
			title: {
				text: `${xMetric} vs ${yMetric}`,
				textStyle: { color: '#e5e7eb', fontSize: 14, fontWeight: 600 },
				left: 10,
				top: 5,
			},
			tooltip: {
				trigger: 'item',
				backgroundColor: 'rgba(15, 23, 42, 0.95)',
				borderColor: 'rgba(255,255,255,0.15)',
				borderWidth: 1,
				textStyle: { color: '#e5e7eb', fontSize: 12 },
				formatter: (p: any) => {
					return `<div style="font-family:monospace">
						<div style="color:#9ca3af;margin-bottom:4px">${p.data.experimentName}</div>
						<div>${xMetric}: ${p.value[0].toFixed(4)}</div>
						<div>${yMetric}: ${p.value[1].toFixed(4)}</div>
						<div style="color:#6b7280">Step: ${p.value[2]}</div>
					</div>`;
				},
			},
			grid: { left: 70, right: 30, top: 50, bottom: 40 },
			xAxis: {
				type: 'value',
				name: xMetric,
				nameTextStyle: { color: '#6b7280', fontSize: 11 },
				axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
				axisLabel: { color: '#6b7280', fontSize: 11, fontFamily: 'monospace' },
				splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
			},
			yAxis: {
				type: 'value',
				name: yMetric,
				nameTextStyle: { color: '#6b7280', fontSize: 11 },
				axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
				axisLabel: { color: '#6b7280', fontSize: 11, fontFamily: 'monospace' },
				splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
			},
			series: [{
				type: 'scatter',
				data: scatterData,
				symbolSize: 8,
				emphasis: { itemStyle: { borderColor: '#fff', borderWidth: 1 } },
			}],
			dataZoom: [{ type: 'inside' }, { type: 'slider', height: 16, bottom: 5, borderColor: 'rgba(255,255,255,0.1)', backgroundColor: 'rgba(255,255,255,0.02)', fillerColor: 'rgba(16, 185, 129, 0.1)', handleStyle: { color: '#10b981' }, textStyle: { color: '#6b7280' } }],
		};
	}

	function buildParallelOption() {
		const detailEntries = Array.from(details.entries());
		if (detailEntries.length === 0) return {};

		const configKeys = ['epochs', 'batch_size', 'learning_rate'];
		const metricKeys = availableMetrics.slice(0, 4);
		const allKeys = [...configKeys, ...metricKeys];

		const parallelAxis: any[] = allKeys.map((key, idx) => {
			const isConfig = idx < configKeys.length;
			if (isConfig) {
				const numericValues = detailEntries.map(([, d]) => Number((d.config as any)[key])).filter(v => !isNaN(v));
				return {
					dim: idx,
					name: key,
					type: 'value',
					nameTextStyle: { color: '#9ca3af', fontSize: 11 },
					axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
					axisLabel: { color: '#6b7280', fontSize: 10, fontFamily: 'monospace' },
				};
			} else {
				const numericValues = detailEntries.map(([, d]) => {
					const s = d.metrics.series[key];
					return s && s.values.length > 0 ? s.values[s.values.length - 1].value : NaN;
				}).filter(v => !isNaN(v));
				return {
					dim: idx,
					name: key,
					type: 'value',
					nameTextStyle: { color: '#9ca3af', fontSize: 11 },
					axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
					axisLabel: { color: '#6b7280', fontSize: 10, fontFamily: 'monospace' },
				};
			}
		});

		const seriesData = detailEntries.map(([id, d], i) => {
			const row: number[] = [];
			for (const key of configKeys) {
				row.push(Number((d.config as any)[key]) || 0);
			}
			for (const key of metricKeys) {
				const s = d.metrics.series[key];
				row.push(s && s.values.length > 0 ? s.values[s.values.length - 1].value : 0);
			}
			return {
				value: row,
				lineStyle: { color: getColor(i), width: 2, opacity: 0.8 },
				name: d.name,
			};
		});

		return {
			title: {
				text: $t('compare.parallelCoordinates'),
				textStyle: { color: '#e5e7eb', fontSize: 14, fontWeight: 600 },
				left: 10,
				top: 5,
			},
			tooltip: {
				trigger: 'item',
				backgroundColor: 'rgba(15, 23, 42, 0.95)',
				borderColor: 'rgba(255,255,255,0.15)',
				borderWidth: 1,
				textStyle: { color: '#e5e7eb', fontSize: 12 },
				formatter: (p: any) => {
					const name = p.data?.name || '';
					const vals = p.data?.value || [];
					let html = `<div style="color:#9ca3af;margin-bottom:4px">${name}</div>`;
					allKeys.forEach((k, i) => {
						html += `<div>${k}: ${typeof vals[i] === 'number' ? vals[i].toFixed(4) : vals[i]}</div>`;
					});
					return html;
				},
			},
			parallelAxis,
			parallel: {
				left: 60,
				right: 30,
				top: 50,
				bottom: 30,
				axisExpandable: true,
				axisExpandCenter: 1,
				axisExpandCount: 3,
			},
			series: {
				type: 'parallel',
				data: seriesData,
				smooth: true,
				lineStyle: { width: 2 },
			},
		};
	}

	function initScatterChart() {
		if (!scatterContainer) return;
		if (scatterChart) { scatterChart.dispose(); }
		scatterChart = echarts.init(scatterContainer, null, { renderer: 'canvas' });
		scatterChart.setOption(buildScatterOption() as echarts.EChartsOption);
		scatterResizeObserver = new ResizeObserver(() => scatterChart?.resize());
		scatterResizeObserver.observe(scatterContainer);
	}

	function initParallelChart() {
		if (!parallelContainer) return;
		if (parallelChart) { parallelChart.dispose(); }
		parallelChart = echarts.init(parallelContainer, null, { renderer: 'canvas' });
		parallelChart.setOption(buildParallelOption() as echarts.EChartsOption);
		parallelResizeObserver = new ResizeObserver(() => parallelChart?.resize());
		parallelResizeObserver.observe(parallelContainer);
	}

	$: hasComparison = details.size >= 2;
	$: compareChartSeries = (() => {
		const result: Record<string, MetricSeries> = {};
		for (const id of Array.from(selectedIds)) {
			const detail = details.get(id);
			if (detail) {
				const s = detail.metrics.series[compareMetric];
				if (s) {
					result[detail.name] = s;
				}
			}
		}
		return result;
	})();

	$: if (hasComparison && activeView === 'scatter') {
		setTimeout(() => initScatterChart(), 50);
	}
	$: if (hasComparison && activeView === 'parallel') {
		setTimeout(() => initParallelChart(), 50);
	}
</script>

<div class="compare-page">
	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>{$t('compare.loadingList')}</p>
		</div>
	{:else}
		<div class="compare-header">
			<h2>{$t('compare.title')}</h2>
			<div class="header-info">{$t('compare.selectHint')}</div>
		</div>

		{#if error}
			<div class="error-banner">{error}</div>
		{/if}

		<div class="compare-layout">
			<div class="experiment-selector">
				<h3>{$t('compare.selectExperiments')}</h3>
				<div class="experiment-list">
					{#each experiments as exp}
						<button
							class="experiment-item"
							class:selected={selectedIds.has(exp.id)}
							on:click={() => toggleSelect(exp.id)}
							disabled={!selectedIds.has(exp.id) && selectedIds.size >= 5}
						>
							<div class="item-header">
								<span class="item-check">
									{#if selectedIds.has(exp.id)}
										✓
									{/if}
								</span>
								<span class="item-name">{exp.name}</span>
								<span class="item-status" data-status={exp.status}>{exp.status}</span>
							</div>
							<div class="item-meta">
								<span>{exp.task_type}</span>
								<span>{new Date(exp.created_at).toLocaleDateString()}</span>
							</div>
						</button>
					{/each}
					{#if experiments.length === 0}
						<div class="empty-state">{$t('compare.noExperimentData')}</div>
					{/if}
				</div>

				<button class="compare-btn" on:click={compare} disabled={selectedIds.size < 2 || comparing}>
					{comparing ? $t('compare.loading') : $t('compare.compareCount', { count: selectedIds.size })}
				</button>
			</div>

			<div class="compare-results">
				{#if !hasComparison}
					<div class="placeholder">
						<div class="placeholder-icon">📊</div>
						<p>{$t('compare.selectAtLeast2')}</p>
					</div>
				{:else}
					<div class="results-section">
						<div class="results-toolbar">
							<h3>{$t('compare.metricsComparison')}</h3>
							<div class="view-tabs">
								<button class="view-tab" class:active={activeView === 'chart'} on:click={() => activeView = 'chart'}>📈 {$t('compare.lineChart')}</button>
								<button class="view-tab" class:active={activeView === 'scatter'} on:click={() => activeView = 'scatter'}>🔵 {$t('compare.scatterPlot')}</button>
								<button class="view-tab" class:active={activeView === 'parallel'} on:click={() => activeView = 'parallel'}>📐 {$t('compare.parallelCoord')}</button>
								<button class="view-tab" class:active={activeView === 'table'} on:click={() => activeView = 'table'}>📋 {$t('compare.table')}</button>
							</div>
						</div>

						{#if availableMetrics.length > 0 && (activeView === 'chart' || activeView === 'scatter')}
							<div class="metric-selector">
								<label for="auto-f87">{$t('compare.selectMetric')}：</label>
								<select id="auto-f87" bind:value={compareMetric} class="metric-select">
									{#each availableMetrics as m}
										<option value={m}>{m}</option>
									{/each}
								</select>
							</div>
						{/if}

						{#if activeView === 'chart'}
							<div class="chart-container">
								<MetricsChart series={compareChartSeries} height="350px" title={compareMetric} />
							</div>
						{:else if activeView === 'scatter'}
							<div bind:this={scatterContainer} class="echart-container"></div>
						{:else if activeView === 'parallel'}
							<div bind:this={parallelContainer} class="echart-container"></div>
						{/if}

						{#if activeView === 'table'}
							<div class="metrics-table-container">
								<table class="metrics-table">
									<thead>
										<tr>
											<th>{$t('compare.experiment')}</th>
											<th>{$t('compare.status')}</th>
											{#each availableMetrics as m}
												<th>{$t('compare.best')} {m}</th>
												<th>{$t('compare.final')} {m}</th>
											{/each}
											<th>Epochs</th>
											<th>{$t('compare.learningRate')}</th>
											<th>{$t('compare.batchSize')}</th>
											<th>{$t('compare.backend')}</th>
										</tr>
									</thead>
									<tbody>
										{#each Array.from(selectedIds) as id, i}
											{@const detail = details.get(id)}
											{#if detail}
												<tr>
													<td>
														<span class="table-color" style="background: {getColor(i)}"></span>
														{detail.name}
													</td>
													<td>
														<span class="status-badge" data-status={detail.status}>{detail.status}</span>
													</td>
													{#each availableMetrics as m}
														<td class="metric-value">{formatNum(getBestValue(id, m))}</td>
														<td class="metric-value">{formatNum(getLastValue(id, m))}</td>
													{/each}
													<td>{detail.config.epochs}</td>
													<td>{detail.config.learning_rate}</td>
													<td>{detail.config.batch_size}</td>
													<td>{detail.config.compute_backend}</td>
												</tr>
											{/if}
										{/each}
									</tbody>
								</table>
							</div>
						{/if}

						<div class="config-comparison">
							<h4>{$t('compare.configDiff')}</h4>
							<div class="diff-grid">
								{#each getConfigDiffs(details) as diff}
									<div class="diff-item" class:diff-same={diff.same} class:diff-different={!diff.same}>
										<span class="diff-key">{diff.key}</span>
										<div class="diff-values">
											{#each diff.values as val, i}
												<span class="diff-val" style="color: {getColor(i)}">{val}</span>
											{/each}
										</div>
									</div>
								{/each}
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.compare-page {
		max-width: 1400px;
		margin: 0 auto;
	}

	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		color: var(--text-secondary, #9ca3af);
		gap: 1rem;
	}

	.spinner {
		width: 2rem;
		height: 2rem;
		border: 3px solid rgba(16, 185, 129, 0.2);
		border-top-color: #10b981;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.compare-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.header-info {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
	}

	.error-banner {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.9rem;
		margin-bottom: 1.5rem;
	}

	.compare-layout {
		display: flex;
		gap: 2rem;
	}

	.experiment-selector {
		width: 320px;
		flex-shrink: 0;
	}

	h3 {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1rem;
	}

	.experiment-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 500px;
		overflow-y: auto;
		margin-bottom: 1rem;
	}

	.experiment-item {
		padding: 0.75rem 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		color: var(--text-primary, #e5e7eb);
	}

	.experiment-item:hover:not(:disabled) {
		border-color: rgba(16, 185, 129, 0.3);
		background: rgba(16, 185, 129, 0.05);
	}

	.experiment-item.selected {
		border-color: rgba(16, 185, 129, 0.5);
		background: rgba(16, 185, 129, 0.1);
	}

	.experiment-item:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.item-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.3rem;
	}

	.item-check {
		width: 18px;
		height: 18px;
		border: 2px solid rgba(255, 255, 255, 0.2);
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.75rem;
		color: #10b981;
		flex-shrink: 0;
	}

	.selected .item-check {
		border-color: #10b981;
		background: rgba(16, 185, 129, 0.2);
	}

	.item-name {
		font-weight: 500;
		font-size: 0.9rem;
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.item-status {
		font-size: 0.75rem;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.1);
	}

	.item-status[data-status="Completed"] {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.item-status[data-status="Running"] {
		background: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}

	.item-status[data-status="Failed"] {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.item-meta {
		display: flex;
		justify-content: space-between;
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		padding-left: 26px;
	}

	.empty-state {
		padding: 2rem;
		text-align: center;
		color: var(--text-secondary, #6b7280);
	}

	.compare-btn {
		width: 100%;
		padding: 0.7rem;
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.compare-btn:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.compare-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.compare-results {
		flex: 1;
		min-width: 0;
	}

	.placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem;
		color: var(--text-secondary, #6b7280);
		gap: 1rem;
	}

	.placeholder-icon {
		font-size: 3rem;
	}

	.results-section {
		background: linear-gradient(135deg, rgba(26, 26, 46, 0.5), rgba(22, 33, 62, 0.5));
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.5rem;
	}

	.results-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.view-tabs {
		display: flex;
		gap: 0.25rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 8px;
		padding: 0.2rem;
	}

	.view-tab {
		padding: 0.4rem 0.75rem;
		background: transparent;
		border: none;
		border-radius: 6px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.view-tab:hover {
		color: var(--text-primary, #e5e7eb);
		background: rgba(255, 255, 255, 0.05);
	}

	.view-tab.active {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.metric-selector {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.metric-selector label {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
	}

	.metric-select {
		padding: 0.4rem 0.6rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 6px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.85rem;
	}

	.chart-container {
		margin-bottom: 1.5rem;
	}

	.echart-container {
		width: 100%;
		height: 400px;
		margin-bottom: 1.5rem;
	}

	.metrics-table-container {
		overflow-x: auto;
		margin-bottom: 1.5rem;
	}

	.metrics-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}

	.metrics-table th {
		text-align: left;
		padding: 0.6rem 0.75rem;
		color: var(--text-secondary, #9ca3af);
		font-weight: 500;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		white-space: nowrap;
	}

	.metrics-table td {
		padding: 0.6rem 0.75rem;
		color: var(--text-primary, #e5e7eb);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.metrics-table td:first-child {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.table-color {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.status-badge {
		font-size: 0.75rem;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.1);
	}

	.status-badge[data-status="Completed"] {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.status-badge[data-status="Running"] {
		background: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}

	.status-badge[data-status="Failed"] {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.metric-value {
		font-variant-numeric: tabular-nums;
	}

	.config-comparison h4 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1rem;
	}

	.diff-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 0.75rem;
	}

	.diff-item {
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 8px;
	}

	.diff-item.diff-different {
		border-color: rgba(245, 158, 11, 0.3);
		background: rgba(245, 158, 11, 0.05);
	}

	.diff-key {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.3rem;
	}

	.diff-values {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}

	.diff-val {
		font-size: 0.85rem;
		font-weight: 500;
	}
</style>
