<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { dashboardStore, successRate } from '$lib/lab/stores/dashboard';
	import { resourceStore } from '$lib/lab/stores/hardware';
	import { experimentStore } from '$lib/lab/stores/experiment';

	let statsLoaded = false;
	let resourceLoaded = false;
	let statsError = '';
	let resourceError = '';

	const resourceHistory = resourceStore.history;

	const CHART_W = 600;
	const CHART_H = 120;
	const CHART_PAD = 2;

	function buildSparkline(history: number[]): string {
		if (history.length < 2) return '';
		const step = (CHART_W - CHART_PAD * 2) / (history.length - 1);
		const points = history.map((v, i) => {
			const x = CHART_PAD + i * step;
			const y = CHART_H - CHART_PAD - (Math.min(v, 100) / 100) * (CHART_H - CHART_PAD * 2);
			return `${x},${y}`;
		});
		return `M${points.join(' L')}`;
	}

	function buildAreaPath(history: number[]): string {
		if (history.length < 2) return '';
		const step = (CHART_W - CHART_PAD * 2) / (history.length - 1);
		const baseline = CHART_H - CHART_PAD;
		const points = history.map((v, i) => {
			const x = CHART_PAD + i * step;
			const y = CHART_H - CHART_PAD - (Math.min(v, 100) / 100) * (CHART_H - CHART_PAD * 2);
			return `${x},${y}`;
		});
		const last = history.length - 1;
		const lastX = CHART_PAD + last * step;
		return `M${points.join(' L')} L${lastX},${baseline} L${CHART_PAD},${baseline} Z`;
	}

	$: cpuHistory = $resourceHistory.map((s: any) => s.cpu_usage_percent);
	$: memHistory = $resourceHistory.map((s: any) => s.memory_usage_percent);
	$: diskHistory = $resourceHistory.map((s: any) => s.disk_usage_percent ?? 0);
	$: gpuHistory = $resourceHistory.map((s: any) => s.gpu_usage_percent).filter((v: any) => v !== null);

	onMount(async () => {
		try {
			await dashboardStore.refresh();
		} catch (e) {
			statsError = String(e);
		} finally {
			statsLoaded = true;
		}

		resourceStore.refresh().then(() => {
			resourceLoaded = true;
		}).catch((e) => {
			resourceError = String(e);
			resourceLoaded = true;
		});
		resourceStore.startAutoRefresh(5);

		try {
			await experimentStore.refresh();
		} catch (e) {
			console.error('Failed to load experiments:', e);
		}
	});

	onDestroy(() => {
		resourceStore.stopAutoRefresh();
	});

	$: loaded = statsLoaded;
	$: runningExperiments = Array.from($experimentStore.values()).filter(e => e.status === 'running');
	$: recentExperiments = Array.from($experimentStore.values())
		.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
		.slice(0, 5);
	$: taskTypeStats = buildTaskTypeStats(Array.from($experimentStore.values()));

	function buildTaskTypeStats(experiments: any[]): { type: string; count: number; color: string }[] {
		const map = new Map<string, number>();
		for (const e of experiments) {
			const t = e.task_type || 'unknown';
			map.set(t, (map.get(t) || 0) + 1);
		}
		const colors: Record<string, string> = {
			classification: '#10b981',
			regression: '#3b82f6',
			image_classification: '#8b5cf6',
			text_classification: '#f59e0b',
			object_detection: '#ef4444',
		};
		return Array.from(map.entries()).map(([type, count]) => ({
			type,
			count,
			color: colors[type] || '#6b7280',
		})).sort((a, b) => b.count - a.count);
	}

	function formatTimeAgo(dateStr: string): string {
		const now = Date.now();
		const then = new Date(dateStr).getTime();
		const diff = now - then;
		if (diff < 60000) return '刚刚';
		if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;
		return `${Math.floor(diff / 86400000)} 天前`;
	}

	const navCards = [
		{ href: '/lab/train/new', icon: '🚀', title: '新建训练', desc: '配置并启动新的训练实验' },
		{ href: '/lab/experiments', icon: '🔬', title: '实验管理', desc: '查看和管理所有训练实验' },
		{ href: '/lab/models', icon: '📦', title: '模型仓库', desc: '管理已注册的模型版本' },
		{ href: '/lab/data', icon: '📊', title: '数据管理', desc: '加载和预览训练数据' },
		{ href: '/lab/compare', icon: '⚖️', title: '实验对比', desc: '对比不同实验的性能指标' },
		{ href: '/lab/settings', icon: '⚙️', title: '系统设置', desc: '配置引擎、存储和显示选项' },
	];
</script>

<div class="lab-dashboard">
	<div class="lab-header">
		<div class="header-row">
			<div>
				<h1 class="title">AI Lab</h1>
				<p class="subtitle">通用模型训练可视化平台</p>
			</div>
			<a href="/lab/train/new" class="new-experiment-btn">
				+ 新建实验
			</a>
		</div>
	</div>

	{#if !loaded}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>加载中...</p>
		</div>
	{:else}
		<section class="stats-cards">
			<div class="stat-card">
				<div class="stat-icon">🔬</div>
				<div class="stat-content">
					<span class="stat-value">{$dashboardStore.total_experiments}</span>
					<span class="stat-label">总实验数</span>
				</div>
			</div>
			<div class="stat-card running">
				<div class="stat-icon">⚡</div>
				<div class="stat-content">
					<span class="stat-value">{$dashboardStore.running_experiments}</span>
					<span class="stat-label">运行中</span>
				</div>
				{#if $dashboardStore.running_experiments > 0}
					<div class="stat-pulse"></div>
				{/if}
			</div>
			<div class="stat-card completed">
				<div class="stat-icon">✅</div>
				<div class="stat-content">
					<span class="stat-value">{$dashboardStore.completed_experiments}</span>
					<span class="stat-label">已完成</span>
				</div>
			</div>
			<div class="stat-card failed">
				<div class="stat-icon">❌</div>
				<div class="stat-content">
					<span class="stat-value">{$dashboardStore.failed_experiments}</span>
					<span class="stat-label">失败</span>
				</div>
			</div>
			<div class="stat-card models">
				<div class="stat-icon">📦</div>
				<div class="stat-content">
					<span class="stat-value">{$dashboardStore.total_models}</span>
					<span class="stat-label">注册模型</span>
				</div>
			</div>
			<div class="stat-card success-rate">
				<div class="stat-icon">📊</div>
				<div class="stat-content">
					<span class="stat-value">{$successRate}%</span>
					<span class="stat-label">成功率</span>
				</div>
			</div>
		</section>

		{#if statsError}
			<div class="error-banner">统计加载失败: {statsError}</div>
		{/if}

		<section class="resource-section">
			<div class="section-header">
				<h2 class="section-title">系统资源</h2>
			</div>
			{#if $resourceStore}
				<div class="resource-cards">
					<div class="resource-card">
						<div class="resource-label">CPU</div>
						<div class="resource-bar-container">
							<div class="resource-bar-track">
								<div
									class="resource-bar-fill"
									class:warning={$resourceStore.cpu_usage_percent > 80}
									class:critical={$resourceStore.cpu_usage_percent > 95}
									style="width: {$resourceStore.cpu_usage_percent}%"
								></div>
							</div>
							<span class="resource-value">{$resourceStore.cpu_usage_percent.toFixed(1)}%</span>
						</div>
					</div>
					<div class="resource-card">
						<div class="resource-label">内存</div>
						<div class="resource-bar-container">
							<div class="resource-bar-track">
								<div
									class="resource-bar-fill memory"
									class:warning={$resourceStore.memory_usage_percent > 80}
									class:critical={$resourceStore.memory_usage_percent > 95}
									style="width: {$resourceStore.memory_usage_percent}%"
								></div>
							</div>
							<span class="resource-value">{$resourceStore.memory_usage_percent.toFixed(1)}%</span>
						</div>
						<div class="resource-detail">
							{($resourceStore.memory_total_mb - $resourceStore.memory_available_mb).toLocaleString()} / {$resourceStore.memory_total_mb.toLocaleString()} MB 已使用
						</div>
					</div>
					{#if $resourceStore.gpu_usage_percent !== null}
						<div class="resource-card">
							<div class="resource-label">GPU</div>
							<div class="resource-bar-container">
								<div class="resource-bar-track">
									<div
										class="resource-bar-fill gpu"
										class:warning={$resourceStore.gpu_usage_percent > 80}
										class:critical={$resourceStore.gpu_usage_percent > 95}
										style="width: {$resourceStore.gpu_usage_percent}%"
									></div>
								</div>
								<span class="resource-value">{$resourceStore.gpu_usage_percent.toFixed(1)}%</span>
							</div>
							{#if $resourceStore.gpu_memory_total_mb !== null}
								<div class="resource-detail">
									{($resourceStore.gpu_memory_used_mb ?? 0).toLocaleString()} / {$resourceStore.gpu_memory_total_mb?.toLocaleString() ?? '—'} MB 已使用
								</div>
							{/if}
						</div>
					{/if}
					<div class="resource-card">
						<div class="resource-label">磁盘</div>
						<div class="resource-bar-container">
							<div class="resource-bar-track">
								<div
									class="resource-bar-fill disk"
									class:warning={$resourceStore.disk_usage_percent > 85}
									class:critical={$resourceStore.disk_usage_percent > 95}
									style="width: {$resourceStore.disk_usage_percent}%"
								></div>
							</div>
							<span class="resource-value">{$resourceStore.disk_usage_percent.toFixed(1)}%</span>
						</div>
						<div class="resource-detail">
							{($resourceStore.disk_total_gb - $resourceStore.disk_available_gb).toLocaleString()} / {$resourceStore.disk_total_gb.toLocaleString()} GB 已使用
						</div>
					</div>
				</div>
				{#if cpuHistory.length >= 2}
					<div class="resource-chart">
						<div class="chart-row">
							<span class="chart-label cpu-label">CPU</span>
							<svg viewBox="0 0 {CHART_W} {CHART_H}" class="sparkline" preserveAspectRatio="none">
								<defs>
									<linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
										<stop offset="0%" stop-color="#10b981" stop-opacity="0.3"/>
										<stop offset="100%" stop-color="#10b981" stop-opacity="0.02"/>
									</linearGradient>
								</defs>
								<path d={buildAreaPath(cpuHistory)} fill="url(#cpuGrad)" />
								<path d={buildSparkline(cpuHistory)} fill="none" stroke="#10b981" stroke-width="1.5" />
							</svg>
						</div>
						<div class="chart-row">
							<span class="chart-label mem-label">MEM</span>
							<svg viewBox="0 0 {CHART_W} {CHART_H}" class="sparkline" preserveAspectRatio="none">
								<defs>
									<linearGradient id="memGrad" x1="0" y1="0" x2="0" y2="1">
										<stop offset="0%" stop-color="#3b82f6" stop-opacity="0.3"/>
										<stop offset="100%" stop-color="#3b82f6" stop-opacity="0.02"/>
									</linearGradient>
								</defs>
								<path d={buildAreaPath(memHistory)} fill="url(#memGrad)" />
								<path d={buildSparkline(memHistory)} fill="none" stroke="#3b82f6" stroke-width="1.5" />
							</svg>
						</div>
						<div class="chart-row">
							<span class="chart-label disk-label">DISK</span>
							<svg viewBox="0 0 {CHART_W} {CHART_H}" class="sparkline" preserveAspectRatio="none">
								<defs>
									<linearGradient id="diskGrad" x1="0" y1="0" x2="0" y2="1">
										<stop offset="0%" stop-color="#f59e0b" stop-opacity="0.3"/>
										<stop offset="100%" stop-color="#f59e0b" stop-opacity="0.02"/>
									</linearGradient>
								</defs>
								<path d={buildAreaPath(diskHistory)} fill="url(#diskGrad)" />
								<path d={buildSparkline(diskHistory)} fill="none" stroke="#f59e0b" stroke-width="1.5" />
							</svg>
						</div>
						{#if gpuHistory.length >= 2}
							<div class="chart-row">
								<span class="chart-label gpu-label">GPU</span>
								<svg viewBox="0 0 {CHART_W} {CHART_H}" class="sparkline" preserveAspectRatio="none">
									<defs>
										<linearGradient id="gpuGrad" x1="0" y1="0" x2="0" y2="1">
											<stop offset="0%" stop-color="#8b5cf6" stop-opacity="0.3"/>
											<stop offset="100%" stop-color="#8b5cf6" stop-opacity="0.02"/>
										</linearGradient>
									</defs>
									<path d={buildAreaPath(gpuHistory)} fill="url(#gpuGrad)" />
									<path d={buildSparkline(gpuHistory)} fill="none" stroke="#8b5cf6" stroke-width="1.5" />
								</svg>
							</div>
						{/if}
					</div>
				{/if}
			{:else if resourceError}
				<div class="error-banner">资源监控不可用: {resourceError}</div>
			{:else}
				<div class="resource-cards">
					<div class="resource-card">
						<div class="resource-label">CPU</div>
						<div class="resource-bar-container">
							<div class="resource-bar-track"><div class="resource-bar-fill" style="width: 0%"></div></div>
							<span class="resource-value">—</span>
						</div>
					</div>
					<div class="resource-card">
						<div class="resource-label">内存</div>
						<div class="resource-bar-container">
							<div class="resource-bar-track"><div class="resource-bar-fill memory" style="width: 0%"></div></div>
							<span class="resource-value">—</span>
						</div>
					</div>
				</div>
			{/if}
		</section>

		<section class="nav-section">
			<div class="section-header">
				<h2 class="section-title">快速导航</h2>
			</div>
			<div class="nav-grid">
				{#each navCards as card}
					<a href={card.href} class="nav-card">
						<div class="nav-icon">{card.icon}</div>
						<div class="nav-content">
							<span class="nav-title">{card.title}</span>
							<span class="nav-desc">{card.desc}</span>
						</div>
						<span class="nav-arrow">→</span>
					</a>
				{/each}
			</div>
		</section>

		<div class="two-col">
			<section class="active-section">
				<div class="section-header">
					<h2 class="section-title">活跃运行</h2>
					{#if runningExperiments.length > 0}
						<span class="badge-running">{runningExperiments.length} 运行中</span>
					{/if}
				</div>
				{#if runningExperiments.length > 0}
					<div class="active-list">
						{#each runningExperiments as exp}
							<a href="/lab/experiments/{exp.id}" class="active-row">
								<div class="active-pulse"></div>
								<div class="active-info">
									<span class="active-name">{exp.name}</span>
									<span class="active-type">{exp.task_type}</span>
								</div>
								<span class="active-time">{formatTimeAgo(exp.updated_at)}</span>
							</a>
						{/each}
					</div>
				{:else}
					<div class="empty-section">
						<p>当前没有运行中的实验</p>
						<a href="/lab/train/new" class="link-start">开始新实验 →</a>
					</div>
				{/if}
			</section>

			<section class="recent-section">
				<div class="section-header">
					<h2 class="section-title">最近实验</h2>
					<a href="/lab/experiments" class="link-all">查看全部 →</a>
				</div>
				{#if recentExperiments.length > 0}
					<div class="recent-list">
						{#each recentExperiments as exp}
							<a href="/lab/experiments/{exp.id}" class="recent-row">
								<span class="recent-status-dot" class:running={exp.status === 'running'} class:completed={exp.status === 'completed'} class:failed={exp.status === 'failed'}></span>
								<div class="recent-info">
									<span class="recent-name">{exp.name}</span>
									<span class="recent-meta">{exp.task_type} · {formatTimeAgo(exp.updated_at)}</span>
								</div>
							</a>
						{/each}
					</div>
				{:else}
					<div class="empty-section">
						<p>暂无实验记录</p>
					</div>
				{/if}
			</section>
		</div>

		{#if taskTypeStats.length > 0}
			<section class="task-stats-section">
				<div class="section-header">
					<h2 class="section-title">任务类型分布</h2>
				</div>
				<div class="task-stats-bar">
					{#each taskTypeStats as stat}
						<div class="task-stat-item" style="flex: {stat.count}">
							<div class="task-stat-fill" style="background: {stat.color}"></div>
							<span class="task-stat-label">{stat.type}</span>
							<span class="task-stat-count">{stat.count}</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>

<style>
	.lab-dashboard {
		max-width: 1200px;
		margin: 0 auto;
	}

	.lab-header {
		margin-bottom: 2rem;
	}

	.header-row {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.title {
		font-size: 2rem;
		font-weight: 700;
		background: linear-gradient(135deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin-bottom: 0.25rem;
	}

	.subtitle {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.95rem;
	}

	.new-experiment-btn {
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		padding: 0.6rem 1.5rem;
		border-radius: 8px;
		font-weight: 600;
		font-size: 0.9rem;
		text-decoration: none;
		transition: all 0.2s ease;
	}

	.new-experiment-btn:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.4);
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

	.error-banner {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		color: #fca5a5;
		font-size: 0.85rem;
		margin-bottom: 1.5rem;
	}

	.stats-cards {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.25rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		position: relative;
		overflow: hidden;
		transition: all 0.2s ease;
	}

	.stat-card:hover {
		background: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.12);
		transform: translateY(-2px);
	}

	.stat-card.running {
		border-color: rgba(16, 185, 129, 0.3);
		background: rgba(16, 185, 129, 0.06);
	}

	.stat-card.completed {
		border-color: rgba(59, 130, 246, 0.3);
		background: rgba(59, 130, 246, 0.06);
	}

	.stat-card.failed {
		border-color: rgba(239, 68, 68, 0.3);
		background: rgba(239, 68, 68, 0.06);
	}

	.stat-card.models {
		border-color: rgba(139, 92, 246, 0.3);
		background: rgba(139, 92, 246, 0.06);
	}

	.stat-card.success-rate {
		border-color: rgba(245, 158, 11, 0.3);
		background: rgba(245, 158, 11, 0.06);
	}

	.stat-icon {
		font-size: 1.5rem;
		flex-shrink: 0;
	}

	.stat-content {
		display: flex;
		flex-direction: column;
	}

	.stat-value {
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--text-primary, #e5e7eb);
		line-height: 1.2;
	}

	.stat-label {
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		margin-top: 0.15rem;
	}

	.stat-pulse {
		position: absolute;
		top: 0.75rem;
		right: 0.75rem;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #10b981;
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.5); }
		50% { box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
	}

	.resource-section {
		margin-bottom: 2rem;
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.section-title {
		font-size: 1.15rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.resource-cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: 1rem;
	}

	.resource-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 1rem 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.resource-label {
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.resource-bar-container {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.resource-bar-track {
		flex: 1;
		height: 8px;
		background: rgba(255, 255, 255, 0.08);
		border-radius: 4px;
		overflow: hidden;
	}

	.resource-bar-fill {
		height: 100%;
		border-radius: 4px;
		background: #10b981;
		transition: width 0.5s ease, background 0.3s ease;
	}

	.resource-bar-fill.memory {
		background: #3b82f6;
	}

	.resource-bar-fill.disk {
		background: #f59e0b;
	}

	.resource-bar-fill.gpu {
		background: #8b5cf6;
	}

	.resource-bar-fill.warning {
		background: #f59e0b;
	}

	.resource-bar-fill.critical {
		background: #ef4444;
	}

	.resource-value {
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
		min-width: 50px;
		text-align: right;
	}

	.resource-detail {
		font-size: 0.75rem;
		color: var(--text-secondary, #6b7280);
	}

	.resource-chart {
		margin-top: 1rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 0.75rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.chart-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.chart-label {
		font-size: 0.7rem;
		font-weight: 700;
		letter-spacing: 0.05em;
		min-width: 32px;
		text-align: center;
		border-radius: 4px;
		padding: 2px 6px;
	}

	.cpu-label {
		color: #10b981;
		background: rgba(16, 185, 129, 0.12);
	}

	.mem-label {
		color: #3b82f6;
		background: rgba(59, 130, 246, 0.12);
	}

	.disk-label {
		color: #f59e0b;
		background: rgba(245, 158, 11, 0.12);
	}

	.gpu-label {
		color: #8b5cf6;
		background: rgba(139, 92, 246, 0.12);
	}

	.sparkline {
		flex: 1;
		height: 50px;
	}

	.nav-section {
		margin-bottom: 2rem;
	}

	.nav-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
		gap: 1rem;
	}

	.nav-card {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.25rem 1.5rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		text-decoration: none;
		color: inherit;
		transition: all 0.2s ease;
	}

	.nav-card:hover {
		background: rgba(16, 185, 129, 0.06);
		border-color: rgba(16, 185, 129, 0.3);
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.nav-icon {
		font-size: 2rem;
		flex-shrink: 0;
		width: 3rem;
		height: 3rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(255, 255, 255, 0.06);
		border-radius: 10px;
	}

	.nav-content {
		display: flex;
		flex-direction: column;
		flex: 1;
	}

	.nav-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.nav-desc {
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		margin-top: 0.2rem;
	}

	.nav-arrow {
		color: var(--text-secondary, #6b7280);
		font-size: 1.2rem;
		transition: transform 0.2s;
	}

	.nav-card:hover .nav-arrow {
		transform: translateX(4px);
		color: #10b981;
	}

	@media (max-width: 900px) {
		.stats-cards {
			grid-template-columns: repeat(3, 1fr);
		}
		.two-col {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.stats-cards {
			grid-template-columns: repeat(2, 1fr);
		}
		.nav-grid {
			grid-template-columns: 1fr;
		}
	}

	.two-col {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.active-section,
	.recent-section {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 12px;
		padding: 1.25rem;
	}

	.badge-running {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
		padding: 0.15rem 0.6rem;
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.active-list,
	.recent-list {
		display: flex;
		flex-direction: column;
	}

	.active-row,
	.recent-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 0.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		text-decoration: none;
		color: inherit;
		transition: background 0.15s;
	}

	.active-row:last-child,
	.recent-row:last-child {
		border-bottom: none;
	}

	.active-row:hover,
	.recent-row:hover {
		background: rgba(16, 185, 129, 0.04);
	}

	.active-pulse {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #10b981;
		animation: pulse 1.5s ease-in-out infinite;
		flex-shrink: 0;
	}

	.active-info,
	.recent-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.active-name,
	.recent-name {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.active-type,
	.recent-meta {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
	}

	.active-time {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
		flex-shrink: 0;
	}

	.recent-status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #6b7280;
		flex-shrink: 0;
	}

	.recent-status-dot.running {
		background: #10b981;
	}

	.recent-status-dot.completed {
		background: #3b82f6;
	}

	.recent-status-dot.failed {
		background: #ef4444;
	}

	.empty-section {
		text-align: center;
		padding: 2rem 1rem;
		color: var(--text-secondary, #6b7280);
		font-size: 0.9rem;
	}

	.link-start,
	.link-all {
		color: #10b981;
		font-size: 0.85rem;
		text-decoration: none;
		font-weight: 500;
	}

	.link-start:hover,
	.link-all:hover {
		text-decoration: underline;
	}

	.task-stats-section {
		margin-bottom: 2rem;
	}

	.task-stats-bar {
		display: flex;
		border-radius: 8px;
		overflow: hidden;
		height: 40px;
	}

	.task-stat-item {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		overflow: hidden;
	}

	.task-stat-fill {
		position: absolute;
		inset: 0;
		opacity: 0.2;
	}

	.task-stat-label {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.8rem;
		font-weight: 500;
		z-index: 1;
	}

	.task-stat-count {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.75rem;
		z-index: 1;
	}
</style>
