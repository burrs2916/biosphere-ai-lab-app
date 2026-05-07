<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import * as echarts from 'echarts';
	import type { MetricSeries } from '$lib/lab/adapter/types';

	export let series: Record<string, MetricSeries> = {};
	export let height: string = '400px';
	export let title: string = '';
	export let selectedMetrics: Set<string> = new Set(Object.keys(series));
	export let smooth: boolean = true;
	export let smoothFactor: number = 0.3;
	export let logScale: boolean = false;
	export let alignBy: 'step' | 'epoch' | 'time' = 'step';

	$: {
		const allKeys = new Set(Object.keys(series));
		for (const key of allKeys) {
			if (!selectedMetrics.has(key)) {
				selectedMetrics = new Set([...selectedMetrics, ...allKeys]);
				break;
			}
		}
	}

	let chartContainer: HTMLDivElement;
	let chart: echarts.ECharts | null = null;
	let resizeObserver: ResizeObserver | null = null;

	const COLORS = ['#10b981', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	$: activeSeries = Object.entries(series).filter(([name]) => selectedMetrics.has(name));

	function applySmoothing(values: { step: number; value: number }[], factor: number): number[] {
		if (values.length === 0) return [];
		const smoothed: number[] = [values[0].value];
		const alpha = 1 - factor;
		for (let i = 1; i < values.length; i++) {
			smoothed.push(alpha * values[i].value + factor * smoothed[i - 1]);
		}
		return smoothed;
	}

	function buildOption() {
		const seriesList: echarts.SeriesOption[] = [];
		const legendData: string[] = [];

		activeSeries.forEach(([name, s], idx) => {
			const color = COLORS[idx % COLORS.length];
			legendData.push(name);

			const steps = s.values.map((v) => v.step);
			const rawValues = s.values.map((v) => v.value);
			const smoothedValues = smooth ? applySmoothing(s.values, smoothFactor) : rawValues;

			seriesList.push({
				name,
				type: 'line',
				data: smooth ? steps.map((s, i) => [s, smoothedValues[i]]) : steps.map((s, i) => [s, rawValues[i]]),
				smooth: false,
				showSymbol: s.values.length < 50,
				symbolSize: 4,
				lineStyle: { width: 2, color },
				itemStyle: { color },
				areaStyle: {
					color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
						{ offset: 0, color: color + '30' },
						{ offset: 1, color: color + '05' },
					]),
				},
				emphasis: {
					lineStyle: { width: 3 },
				},
			});

			if (smooth && s.values.length > 5) {
				seriesList.push({
					name: `${name} (原始)`,
					type: 'line',
					data: steps.map((s, i) => [s, rawValues[i]]),
					smooth: false,
					showSymbol: false,
					lineStyle: { width: 1, color, type: 'dashed', opacity: 0.3 },
					itemStyle: { color },
				});
			}
		});

		const xAxisName = alignBy === 'epoch' ? 'Epoch' : alignBy === 'time' ? 'Time' : 'Step';

		return {
			title: title ? {
				text: title,
				textStyle: { color: '#e5e7eb', fontSize: 14, fontWeight: 600 },
				left: 10,
				top: 5,
			} : undefined,
			tooltip: {
				trigger: 'axis',
				backgroundColor: 'rgba(15, 23, 42, 0.95)',
				borderColor: 'rgba(255,255,255,0.15)',
				borderWidth: 1,
				textStyle: { color: '#e5e7eb', fontSize: 12, fontFamily: 'monospace' },
				axisPointer: {
					type: 'cross',
					crossStyle: { color: '#6b7280' },
					lineStyle: { color: 'rgba(16, 185, 129, 0.3)', type: 'dashed' },
				},
				formatter: (params: any) => {
					if (!Array.isArray(params)) return '';
					let html = `<div style="margin-bottom:4px;color:#9ca3af">${xAxisName} ${params[0]?.data?.[0] ?? ''}</div>`;
					for (const p of params) {
						if (p.seriesName.includes('(原始)')) continue;
						const val = p.data?.[1];
						html += `<div style="display:flex;align-items:center;gap:6px;margin:2px 0">
							<span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${p.color}"></span>
							<span style="color:#e5e7eb">${p.seriesName}</span>
							<span style="color:#9ca3af;margin-left:auto;font-family:monospace">${typeof val === 'number' ? val.toFixed(4) : val}</span>
						</div>`;
					}
					return html;
				},
			},
			grid: {
				left: 70,
				right: 30,
				top: title ? 45 : 25,
				bottom: 60,
			},
			xAxis: {
				type: 'value',
				name: xAxisName,
				nameTextStyle: { color: '#6b7280', fontSize: 11 },
				axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
				axisLabel: { color: '#6b7280', fontSize: 11, fontFamily: 'monospace' },
				splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
			},
			yAxis: {
				type: logScale ? 'log' : 'value',
				nameTextStyle: { color: '#6b7280', fontSize: 11 },
				axisLine: { lineStyle: { color: 'rgba(255,255,255,0.1)' } },
				axisLabel: {
					color: '#6b7280',
					fontSize: 11,
					fontFamily: 'monospace',
					formatter: (v: number) => {
						if (logScale) {
							if (v >= 1) return v.toFixed(1);
							return v.toExponential(1);
						}
						if (Math.abs(v) >= 100) return v.toFixed(1);
						if (Math.abs(v) >= 1) return v.toFixed(3);
						return v.toFixed(5);
					},
				},
				splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
				min: logScale ? 'dataMin' : undefined,
			},
			dataZoom: [
				{
					type: 'inside',
					xAxisIndex: 0,
					filterMode: 'none',
				},
				{
					type: 'slider',
					xAxisIndex: 0,
					filterMode: 'none',
					height: 20,
					bottom: 5,
					borderColor: 'rgba(255,255,255,0.1)',
					backgroundColor: 'rgba(255,255,255,0.02)',
					fillerColor: 'rgba(16, 185, 129, 0.1)',
					handleStyle: { color: '#10b981', borderColor: '#10b981' },
					textStyle: { color: '#6b7280', fontSize: 10 },
					dataBackground: {
						lineStyle: { color: 'rgba(16, 185, 129, 0.3)' },
						areaStyle: { color: 'rgba(16, 185, 129, 0.05)' },
					},
					selectedDataBackground: {
						lineStyle: { color: '#10b981' },
						areaStyle: { color: 'rgba(16, 185, 129, 0.1)' },
					},
				},
			],
			toolbox: {
				right: 10,
				top: 5,
				iconStyle: { borderColor: '#6b7280' },
				emphasis: { iconStyle: { borderColor: '#10b981' } },
				feature: {
					dataZoom: { yAxisIndex: 'none' },
					restore: {},
					saveAsImage: { backgroundColor: '#0f172a' },
				},
			},
			series: seriesList,
			animation: true,
			animationDuration: 500,
			animationEasing: 'cubicOut',
		};
	}

	function updateChart() {
		if (!chart) return;
		chart.setOption(buildOption() as echarts.EChartsOption, true);
	}

	function toggleMetric(name: string) {
		const next = new Set(selectedMetrics);
		if (next.has(name)) {
			if (next.size > 1) next.delete(name);
		} else {
			next.add(name);
		}
		selectedMetrics = next;
		updateChart();
	}

	function formatNum(n: number): string {
		if (Math.abs(n) >= 100) return n.toFixed(1);
		if (Math.abs(n) >= 1) return n.toFixed(3);
		return n.toFixed(5);
	}

	onMount(() => {
		if (chartContainer) {
			chart = echarts.init(chartContainer, null, { renderer: 'canvas' });
			updateChart();

			resizeObserver = new ResizeObserver(() => {
				chart?.resize();
			});
			resizeObserver.observe(chartContainer);
		}
	});

	$: if (chart) updateChart();

	onDestroy(() => {
		if (resizeObserver) {
			resizeObserver.disconnect();
			resizeObserver = null;
		}
		if (chart) {
			chart.dispose();
			chart = null;
		}
	});
</script>

<div class="metrics-chart-container">
	<div class="chart-toolbar">
		<div class="legend">
			{#each Object.entries(series) as [name, s], idx}
				<button
					class="legend-item"
					class:inactive={!selectedMetrics.has(name)}
					on:click={() => toggleMetric(name)}
				>
					<span class="legend-dot" style="background: {COLORS[idx % COLORS.length]}"></span>
					<span class="legend-name">{name}</span>
					{#if s.values.length > 0}
						<span class="legend-last">{formatNum(s.values[s.values.length - 1].value)}</span>
					{/if}
				</button>
			{/each}
		</div>
		<div class="smooth-control">
			<label class="smooth-label">
				<input type="checkbox" bind:checked={smooth} on:change={updateChart} />
				<span>平滑</span>
			</label>
			{#if smooth}
				<input
					type="range"
					min="0"
					max="0.9"
					step="0.1"
					bind:value={smoothFactor}
					on:input={updateChart}
					class="smooth-slider"
				/>
			{/if}
		</div>
		<div class="scale-control">
			<label class="smooth-label">
				<input type="checkbox" bind:checked={logScale} on:change={updateChart} />
				<span>对数坐标</span>
			</label>
		</div>
		<div class="align-control">
			<select bind:value={alignBy} on:change={updateChart} class="align-select">
				<option value="step">按步数</option>
				<option value="epoch">按Epoch</option>
				<option value="time">按时间</option>
			</select>
		</div>
	</div>
	<div bind:this={chartContainer} class="chart-area" style="height: {height}"></div>
</div>

<style>
	.metrics-chart-container {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1rem;
	}

	.chart-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		padding: 0.3rem 0.6rem;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 0.8rem;
		color: var(--text-primary, #e5e7eb);
	}

	.legend-item:hover {
		background: rgba(255, 255, 255, 0.08);
	}

	.legend-item.inactive {
		opacity: 0.4;
	}

	.legend-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.legend-name {
		font-family: monospace;
	}

	.legend-last {
		color: var(--text-secondary, #9ca3af);
		font-family: monospace;
		font-size: 0.75rem;
	}

	.smooth-control {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.smooth-label {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		cursor: pointer;
	}

	.smooth-label input[type="checkbox"] {
		accent-color: #10b981;
	}

	.smooth-slider {
		width: 80px;
		height: 4px;
		accent-color: #10b981;
	}

	.scale-control {
		display: flex;
		align-items: center;
	}

	.align-control {
		display: flex;
		align-items: center;
	}

	.align-select {
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 6px;
		color: #e5e7eb;
		font-size: 0.8rem;
		padding: 0.2rem 0.4rem;
		outline: none;
		cursor: pointer;
	}

	.align-select:focus {
		border-color: #10b981;
	}

	.chart-area {
		width: 100%;
		min-height: 300px;
	}
</style>
