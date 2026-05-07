<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { experimentStore } from '$lib/lab/stores/experiment';
	import { statusBarStore } from '$lib/lab/stores/status';
	import { progressStore } from '$lib/lab/stores/progress';
	import { resourceStore } from '$lib/lab/stores/hardware';
	import { dashboardStore } from '$lib/lab/stores/dashboard';
	import { datasetRegistryStore } from '$lib/lab/stores/dataset';
	import Toast from '$lib/lab/components/Toast.svelte';
	import TaskManager from '$lib/lab/components/TaskManager.svelte';
	import LocaleSwitch from '$lib/i18n/LocaleSwitch.svelte';
	import { i18n, t, formatUptime, formatLastRefresh } from '$lib/i18n';

	interface NavItem {
		href: string;
		labelKey: string;
		icon: string;
	}

	interface NavGroup {
		titleKey: string;
		icon: string;
		items: NavItem[];
		expanded: boolean;
	}

	let navGroups: NavGroup[] = [
		{
			titleKey: 'nav.overview',
			icon: '🏠',
			items: [{ href: '/lab', labelKey: 'nav.dashboard', icon: '📊' }],
			expanded: true
		},
		{
			titleKey: 'nav.dataManagement',
			icon: '📁',
			items: [
				{ href: '/lab/data/list', labelKey: 'nav.datasetList', icon: '📊' },
				{ href: '/lab/data/workshop', labelKey: 'nav.dataWorkshop', icon: '🔧' },
				{ href: '/lab/plans', labelKey: 'nav.trainingPlans', icon: '📋' },
				{ href: '/lab/plan', labelKey: 'nav.createPlan', icon: '➕' }
			],
			expanded: true
		},
		{
			titleKey: 'nav.modelDevelopment',
			icon: '🧠',
			items: [
				{ href: '/lab/build', labelKey: 'nav.modelBuild', icon: '🏗️' },
				{ href: '/lab/train/new', labelKey: 'nav.trainingConfig', icon: '🚀' },
				{ href: '/lab/tune', labelKey: 'nav.hyperparameterTuning', icon: '🎯' },
				{ href: '/lab/models', labelKey: 'nav.modelManagement', icon: '📦' }
			],
			expanded: true
		},
		{
			titleKey: 'nav.experimentTracking',
			icon: '🔬',
			items: [
				{ href: '/lab/experiments', labelKey: 'nav.experimentList', icon: '📋' },
				{ href: '/lab/compare', labelKey: 'nav.compareAnalysis', icon: '📈' },
				{ href: '/lab/lineage', labelKey: 'nav.lineageTracking', icon: '🔗' }
			],
			expanded: true
		},
		{
			titleKey: 'nav.systemSettings',
			icon: '⚙️',
			items: [{ href: '/lab/settings', labelKey: 'nav.systemConfig', icon: '🔧' }],
			expanded: true
		}
	];

	function toggleGroup(index: number) {
		navGroups[index].expanded = !navGroups[index].expanded;
		navGroups = navGroups;
	}

	function isActive(href: string): boolean {
		const currentPath = $page.url.pathname;
		if (href === '/lab') {
			return currentPath === '/lab' || currentPath === '/lab/';
		}
		return currentPath.startsWith(href);
	}

	$: connectionLabel = $t('status.connected');
	$: idleLabel = $t('status.idle');
	$: experimentsRunningLabel = $t('status.experimentsRunning');
	$: refreshLabel = $t('status.refresh');
	$: runtimeLabel = $t('status.runtime');

	$: uptimeLabel = formatUptime($statusBarStore.uptime, $i18n);
	$: lastRefreshLabel = $statusBarStore.lastRefreshAt
		? formatLastRefresh(Math.floor((Date.now() - $statusBarStore.lastRefreshAt.getTime()) / 1000), $i18n)
		: $t('status.notRefreshed');

	onMount(async () => {
		experimentStore.startListening();
		progressStore.startListening();
		resourceStore.startListening();
		dashboardStore.startListening();
		datasetRegistryStore.startListening();
		await statusBarStore.initialize();
		statusBarStore.startAutoRefresh(5);
	});

	onDestroy(() => {
		experimentStore.stopListening();
		progressStore.stopListening();
		resourceStore.stopListening();
		dashboardStore.stopListening();
		datasetRegistryStore.stopListening();
		statusBarStore.stopAutoRefresh();
	});
</script>

<div class="lab-layout">
	<Toast />
	<TaskManager />
	<nav class="lab-nav">
		<div class="nav-header">
			<div class="logo">
				<span class="logo-icon">🧬</span>
				<span class="logo-text">Biosphere AI Lab</span>
			</div>
			<div class="nav-locale">
				<LocaleSwitch />
			</div>
		</div>

		<div class="nav-groups">
			{#each navGroups as group, gIndex}
				<div class="nav-group">
					<button
						class="nav-group-header"
						on:click={() => toggleGroup(gIndex)}
						class:active={group.items.some(item => isActive(item.href))}
					>
						<span class="group-icon">{group.icon}</span>
						<span class="group-title">{$t(group.titleKey)}</span>
						<span class="group-toggle" class:expanded={group.expanded}>▼</span>
					</button>

					{#if group.expanded}
						<div class="nav-items">
							{#each group.items as item}
								<a
									href={item.href}
									class="nav-item"
									class:active={isActive(item.href)}
								>
									<span class="item-icon">{item.icon}</span>
									<span class="item-label">{$t(item.labelKey)}</span>
								</a>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<div class="nav-footer">
			<div class="version">v0.1.0</div>
		</div>
	</nav>

	<div class="lab-main">
		<main class="lab-content">
			<slot />
		</main>

		<footer class="lab-statusbar">
			<div class="statusbar-left">
				<span class="status-item connection" class:connected={$statusBarStore.connectionStatus === 'connected'} class:connecting={$statusBarStore.connectionStatus === 'connecting'} class:disconnected={$statusBarStore.connectionStatus === 'disconnected'}>
					<span class="status-dot"></span>
					{#if $statusBarStore.connectionStatus === 'connected'}
						{$t('status.connected')}
					{:else if $statusBarStore.connectionStatus === 'connecting'}
						{$t('status.connecting')}
					{:else}
						{$t('status.disconnected')}
					{/if}
				</span>
				<span class="status-divider">|</span>
				<span class="status-item">
					<span class="status-icon">⚡</span>
					{$statusBarStore.computeBackend.toUpperCase()}
				</span>
				{#if $statusBarStore.backendInfo}
					<span class="status-divider">|</span>
					<span class="status-item">{$statusBarStore.backendInfo}</span>
				{/if}
				{#if $resourceStore}
					<span class="status-divider">|</span>
					<span class="status-item resource" class:warning={$resourceStore.cpu_usage_percent > 80} class:critical={$resourceStore.cpu_usage_percent > 95}>
						CPU {$resourceStore.cpu_usage_percent.toFixed(0)}%
					</span>
					<span class="status-divider">|</span>
					<span class="status-item resource" class:warning={$resourceStore.memory_usage_percent > 80} class:critical={$resourceStore.memory_usage_percent > 95}>
						MEM {$resourceStore.memory_usage_percent.toFixed(0)}%
					</span>
					{#if $resourceStore.gpu_usage_percent !== null}
						<span class="status-divider">|</span>
						<span class="status-item resource gpu" class:warning={$resourceStore.gpu_usage_percent > 80} class:critical={$resourceStore.gpu_usage_percent > 95}>
							GPU {$resourceStore.gpu_usage_percent.toFixed(0)}%
						</span>
					{/if}
				{/if}
			</div>

			<div class="statusbar-center">
				{#if $statusBarStore.runningExperiments > 0}
					<span class="status-item running">
						<span class="pulse-dot"></span>
						{$statusBarStore.runningExperiments} {$t('status.experimentsRunning')}
					</span>
				{:else}
					<span class="status-item idle">{$t('status.idle')}</span>
				{/if}
			</div>

			<div class="statusbar-right">
				<span class="status-item">
					{$t('status.refresh')}: {lastRefreshLabel}
				</span>
				<span class="status-divider">|</span>
				<span class="status-item">
					{$t('status.runtime')}: {uptimeLabel}
				</span>
				<span class="status-divider">|</span>
				<span class="status-item">
					v0.1.0
				</span>
			</div>
		</footer>
	</div>
</div>

<style>
	.lab-layout {
		display: flex;
		height: 100%;
		gap: 0;
	}

	.lab-nav {
		width: 240px;
		min-width: 240px;
		background: linear-gradient(180deg, #0f172a 0%, #1e293b 100%);
		border-right: 1px solid rgba(16, 185, 129, 0.15);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.nav-header {
		padding: 1.25rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.logo-icon {
		font-size: 1.5rem;
	}

	.logo-text {
		font-size: 1rem;
		font-weight: 600;
		color: #e5e7eb;
		letter-spacing: -0.025em;
	}

	.nav-locale {
		margin-top: 0.75rem;
		display: flex;
		justify-content: flex-start;
	}

	.nav-groups {
		flex: 1;
		overflow-y: auto;
		padding: 0.5rem 0;
	}

	.nav-group {
		margin-bottom: 0.25rem;
	}

	.nav-group-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.625rem 1rem;
		background: transparent;
		border: none;
		color: #9ca3af;
		font-size: 0.8rem;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		cursor: pointer;
		transition: all 0.2s ease;
	}

	.nav-group-header:hover {
		background: rgba(255, 255, 255, 0.03);
		color: #e5e7eb;
	}

	.nav-group-header.active {
		color: #10b981;
	}

	.group-icon {
		font-size: 0.9rem;
	}

	.group-title {
		flex: 1;
		text-align: left;
	}

	.group-toggle {
		font-size: 0.6rem;
		transition: transform 0.2s ease;
		opacity: 0.5;
	}

	.group-toggle.expanded {
		transform: rotate(180deg);
	}

	.nav-items {
		padding: 0.25rem 0;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 1rem 0.625rem 2.5rem;
		color: #9ca3af;
		text-decoration: none;
		font-size: 0.875rem;
		transition: all 0.2s ease;
		border-left: 3px solid transparent;
	}

	.nav-item:hover {
		background: rgba(16, 185, 129, 0.05);
		color: #e5e7eb;
	}

	.nav-item.active {
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
		border-left-color: #10b981;
	}

	.item-icon {
		font-size: 0.85rem;
		opacity: 0.8;
	}

	.nav-item.active .item-icon {
		opacity: 1;
	}

	.item-label {
		flex: 1;
	}

	.nav-footer {
		padding: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		text-align: center;
	}

	.version {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.lab-main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.lab-content {
		flex: 1;
		overflow-y: auto;
		padding: 2rem;
		background: #0f172a;
	}

	.lab-statusbar {
		height: 28px;
		min-height: 28px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 1rem;
		background: #0c1222;
		border-top: 1px solid rgba(16, 185, 129, 0.12);
		font-size: 0.72rem;
		color: #6b7280;
		user-select: none;
	}

	.statusbar-left,
	.statusbar-center,
	.statusbar-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.status-item {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		white-space: nowrap;
	}

	.status-divider {
		color: rgba(107, 114, 128, 0.3);
	}

	.status-item.resource {
		font-family: monospace;
		font-size: 0.7rem;
		font-weight: 600;
		color: #10b981;
	}

	.status-item.resource.gpu {
		color: #8b5cf6;
	}

	.status-item.resource.warning {
		color: #f59e0b;
	}

	.status-item.resource.critical {
		color: #ef4444;
	}

	.status-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #6b7280;
		flex-shrink: 0;
	}

	.connection.connected .status-dot {
		background: #10b981;
		box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
	}

	.connection.connecting .status-dot {
		background: #f59e0b;
		animation: blink 1s ease-in-out infinite;
	}

	.connection.disconnected .status-dot {
		background: #ef4444;
	}

	.connection.connected {
		color: #10b981;
	}

	.connection.connecting {
		color: #f59e0b;
	}

	.connection.disconnected {
		color: #ef4444;
	}

	.status-icon {
		font-size: 0.7rem;
	}

	.pulse-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #10b981;
		animation: pulse 1.5s ease-in-out infinite;
		flex-shrink: 0;
	}

	.status-item.running {
		color: #10b981;
	}

	.status-item.idle {
		color: #6b7280;
	}

	@keyframes blink {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.3; }
	}

	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.5); }
		50% { box-shadow: 0 0 0 4px rgba(16, 185, 129, 0); }
	}
</style>
