<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { experimentStore } from '$lib/lab/stores/experiment';
	const experimentLogs = experimentStore.logs;
	import { modelStore } from '$lib/lab/stores/model';
	import { progressStore } from '$lib/lab/stores/progress';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import MetricsChart from '$lib/lab/components/MetricsChart.svelte';
	import TrainingProgress from '$lib/lab/components/TrainingProgress.svelte';
	import type { ExperimentDetail, ExperimentStatus, InferenceResult, EvaluationResult, MetricsTimeline, CheckpointInfo } from '$lib/lab/adapter/types';
	import { t } from '$lib/i18n';

	let detail: ExperimentDetail | null = null;
	let loading = true;
	let activeTab: 'metrics' | 'params' | 'config' | 'artifacts' | 'environment' | 'inference' | 'logs' | 'notes' = 'metrics';
	let showRegisterModal = false;
	let registering = false;
	let registerName = '';
	let registerVersion = '1.0.0';
	let registerError: string | null = null;

	let inferenceInput = '';
	let inferenceResult: InferenceResult | null = null;
	let inferenceRunning = false;
	let inferenceError: string | null = null;

	let evalTestDataPath = '';
	let evalResult: EvaluationResult | null = null;
	let evalRunning = false;
	let evalError: string | null = null;
	let evalHistory: any[] = [];
	let evalHistoryLoading = false;

	let batchInferenceData = '';
	let batchInferenceResults: Array<{ input: number[]; result: InferenceResult } | null> = [];
	let batchInferenceRunning = false;
	let batchInferenceError: string | null = null;

	let newTag = '';
	let newParamKey = '';
	let newParamValue = '';
	let tagAdding = false;
	let editGroup = '';
	let showEditGroup = false;
	let groupSaving = false;
	let logLevelFilter: 'all' | 'error' | 'warn' | 'info' | 'debug' = 'all';
	let autoScrollLogs = true;
	let logsContainer: HTMLElement | null = null;
	let paramAdding = false;

	let editDescription = '';
	let showEditDescription = false;
	let descriptionSaving = false;

	let notesText = '';
	let notesSaving = false;
	let notesPreviewMode = false;

	let exportFormat = 'burn_record';
	let exportRunning = false;
	let exportResult: any | null = null;
	let exportError: string | null = null;
	let availableExportFormats: string[] = [];
	let notesLoaded = false;

	let metricsTimeline: MetricsTimeline = { series: {} };
	let metricsSmoothAlpha = 0.0;
	let metricsMaxPoints = 500;
	let metricsLoading = false;

	let artifactPreviewPath: string | null = null;

	let showCheckpointResumeModal = false;
	let checkpointList: CheckpointInfo[] = [];
	let checkpointLoading = false;
	let selectedCheckpointEpoch: number | null = null;
	let checkpointResuming = false;
	let artifactPreviewContent: string | null = null;
	let artifactPreviewLoading = false;
	let artifactPreviewError: string | null = null;
	let scanningArtifacts = false;

	let unsubDetail: (() => void) | null = null;
	let unsubMetrics: (() => void) | null = null;
	let unsubLogs: (() => void) | null = null;
	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let lastHeartbeat: Date | null = null;
	let heartbeatElapsed: number = 0;
	let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
	let heartbeatUnlisten: (() => void) | null = null;

	$: experimentId = $page.params.id ?? '';

	async function refreshMetrics() {
		if (!experimentId || !detail) return;
		metricsLoading = true;
		try {
			const client = getLabClient();
			const metricNames = Object.keys(detail.metrics.series);
			if (metricNames.length > 0) {
				metricsTimeline = await client.queryMetricsDownsampled(
					experimentId,
					metricNames,
					metricsMaxPoints,
					metricsSmoothAlpha > 0 ? metricsSmoothAlpha : undefined,
				);
			}
		} catch (e) {
			console.error('Failed to refresh metrics:', e);
		} finally {
			metricsLoading = false;
		}
	}

	$: if (metricsSmoothAlpha !== 0.0 || metricsMaxPoints !== 500) {
		refreshMetrics();
	}

	function startAutoRefresh() {
		if (refreshInterval) return;
		refreshInterval = setInterval(async () => {
			if (!experimentId) return;
			try {
				const d = await experimentStore.loadDetail(experimentId);
				if (d) {
					if (activeTab === 'logs') {
						await experimentStore.loadPersistedLogs(experimentId);
					}
					if (d.status !== 'running' && d.status !== 'paused' && refreshInterval) {
						clearInterval(refreshInterval);
						refreshInterval = null;
					}
				}
			} catch (e) {
				console.error('Auto-refresh failed:', e);
			}
		}, 5000);
		startHeartbeatMonitor();
	}

	function stopAutoRefresh() {
		if (refreshInterval) {
			clearInterval(refreshInterval);
			refreshInterval = null;
		}
		stopHeartbeatMonitor();
	}

	function startHeartbeatMonitor() {
		stopHeartbeatMonitor();
		lastHeartbeat = null;
		heartbeatElapsed = 0;
		heartbeatTimer = setInterval(() => {
			if (lastHeartbeat) {
				heartbeatElapsed = Math.floor((Date.now() - lastHeartbeat.getTime()) / 1000);
			}
		}, 1000);
		try {
			const client = getLabClient();
			client.onLabEvent((event: any) => {
				if (event.type === 'Heartbeat' && detail?.status === 'running') {
					lastHeartbeat = new Date();
					heartbeatElapsed = 0;
				}
			}).then((fn) => { heartbeatUnlisten = fn; });
		} catch (e) {
			console.warn('Failed to subscribe to heartbeat events:', e);
		}
	}

	function stopHeartbeatMonitor() {
		if (heartbeatTimer) {
			clearInterval(heartbeatTimer);
			heartbeatTimer = null;
		}
		if (heartbeatUnlisten) {
			heartbeatUnlisten();
			heartbeatUnlisten = null;
		}
		lastHeartbeat = null;
		heartbeatElapsed = 0;
	}

	onMount(async () => {
		if (!experimentId) return;
		try {
			detail = await experimentStore.loadDetail(experimentId);
			if (detail) {
				const metricNames = Object.keys(detail.metrics.series);
				if (metricNames.length > 0) {
					await experimentStore.loadMetrics(experimentId, metricNames);
				}
				await experimentStore.loadPersistedLogs(experimentId);
				if (detail.status === 'running') {
					startAutoRefresh();
				}
				loadEvalHistory();
				loadExportFormats();
			}
		} catch (e) {
			console.error('Failed to load experiment detail:', e);
		} finally {
			loading = false;
		}

		unsubDetail = experimentStore.details.subscribe((map) => {
			const d = map.get(experimentId);
			if (d) {
				const wasRunning = detail?.status === 'running';
				const nowRunning = d.status === 'running';
				detail = d;
				if (nowRunning && !wasRunning) {
					startAutoRefresh();
				} else if (!nowRunning && wasRunning) {
					stopAutoRefresh();
				}
			}
		});

		unsubMetrics = experimentStore.metrics.subscribe((map) => {
			const t = map.get(experimentId);
			if (t) metricsTimeline = t;
		});

		unsubLogs = experimentStore.logs.subscribe(() => {
			if (autoScrollLogs && logsContainer) {
				requestAnimationFrame(() => {
					logsContainer?.scrollTo({ top: logsContainer.scrollHeight, behavior: 'smooth' });
				});
			}
		});
	});

	onDestroy(() => {
		if (unsubDetail) {
			unsubDetail();
			unsubDetail = null;
		}
		if (unsubMetrics) {
			unsubMetrics();
			unsubMetrics = null;
		}
		if (unsubLogs) {
			unsubLogs();
			unsubLogs = null;
		}
		stopAutoRefresh();
	});

	function statusColor(status: ExperimentStatus): string {
		switch (status) {
			case 'running': return '#10b981';
			case 'completed': return '#3b82f6';
			case 'failed': return '#ef4444';
			case 'paused': return '#f59e0b';
			case 'cancelled': return '#6b7280';
			case 'created': return '#8b5cf6';
			default: return '#6b7280';
		}
	}

	function formatTime(iso: string | null): string {
		if (!iso) return '-';
		return new Date(iso).toLocaleString('zh-CN');
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / 1048576).toFixed(1) + ' MB';
	}

	function duration(start: string, end: string | null): string {
		if (!end) return $t('experimentDetail.ongoing');
		const ms = new Date(end).getTime() - new Date(start).getTime();
		if (ms < 60000) return `${Math.floor(ms / 1000)} ${$t('experimentDetail.seconds')}`;
		if (ms < 3600000) return `${Math.floor(ms / 60000)} ${$t('experimentDetail.minutes')}`;
		return `${(ms / 3600000).toFixed(1)} ${$t('experimentDetail.hours')}`;
	}

	async function openRegisterModal() {
		if (!detail) return;
		registerName = detail.name;
		registerVersion = '1.0.0';
		registerError = null;
		showRegisterModal = true;
	}

	async function archiveExperiment() {
		if (!detail) return;
		try {
			const client = getLabClient();
			await client.archiveExperiment(experimentId);
			detail = await experimentStore.loadDetail(experimentId);
		} catch (e) {
			console.error('Failed to archive experiment:', e);
		}
	}

	async function restoreExperiment() {
		if (!detail) return;
		try {
			const client = getLabClient();
			await client.restoreExperiment(experimentId);
			detail = await experimentStore.loadDetail(experimentId);
		} catch (e) {
			console.error('Failed to restore experiment:', e);
		}
	}

	async function registerModel() {
		if (!detail) return;
		if (!registerName.trim() || !registerVersion.trim()) {
			registerError = $t('models.nameVersionRequired');
			return;
		}
		registering = true;
		registerError = null;
		try {
			await modelStore.registerFromExperiment(detail.id, registerName, registerVersion);
			showRegisterModal = false;
			await experimentStore.refresh();
		} catch (e: any) {
			registerError = e?.message || $t('models.registerFailed');
		} finally {
			registering = false;
		}
	}

	async function openCheckpointResumeModal() {
		if (!detail) return;
		checkpointLoading = true;
		showCheckpointResumeModal = true;
		selectedCheckpointEpoch = null;
		try {
			const client = getLabClient();
			checkpointList = await client.listCheckpoints(detail.id);
		} catch (e) {
			console.error('Failed to load checkpoints:', e);
			checkpointList = [];
		} finally {
			checkpointLoading = false;
		}
	}

	async function resumeFromCheckpoint() {
		if (!detail || selectedCheckpointEpoch === null) return;
		checkpointResuming = true;
		try {
			const client = getLabClient();
			await client.resumeFromCheckpoint(detail.id, selectedCheckpointEpoch);
			showCheckpointResumeModal = false;
			await experimentStore.refresh();
		} catch (e: any) {
			console.error('Failed to resume from checkpoint:', e);
		} finally {
			checkpointResuming = false;
		}
	}

	async function runInference() {
		if (!detail || !inferenceInput.trim()) return;
		inferenceRunning = true;
		inferenceError = null;
		inferenceResult = null;
		try {
			const values = inferenceInput
				.trim()
				.split(/[\s,]+/)
				.map(Number)
				.filter(v => !isNaN(v));
			if (values.length === 0) {
				inferenceError = $t('experimentDetail.invalidNumericInput');
				return;
			}
			const client = getLabClient();
			inferenceResult = await client.runInference(experimentId, [values]);
		} catch (e: any) {
			inferenceError = e?.message || $t('models.serveFailed');
		} finally {
			inferenceRunning = false;
		}
	}

	async function runEvaluation() {
		if (!detail || !evalTestDataPath.trim()) return;
		evalRunning = true;
		evalError = null;
		evalResult = null;
		try {
			const client = getLabClient();
			evalResult = await client.evaluateModel(experimentId, evalTestDataPath);
			try {
				await client.saveEvaluation(experimentId, evalResult, evalTestDataPath);
			} catch (saveErr: any) {
				console.warn('Failed to save evaluation result:', saveErr);
			}
			loadEvalHistory();
		} catch (e: any) {
			evalError = e?.message || $t('experimentDetail.evalFailed');
		} finally {
			evalRunning = false;
		}
	}

	async function loadEvalHistory() {
		evalHistoryLoading = true;
		try {
			const client = getLabClient();
			evalHistory = await client.listEvaluations(experimentId);
		} catch (e: any) {
			console.warn('Failed to load evaluation history:', e);
			evalHistory = [];
		} finally {
			evalHistoryLoading = false;
		}
	}

	async function selectTestDataFile() {
		try {
			const client = getLabClient();
			const path = await client.selectFile([
				{ name: 'CSV', extensions: ['csv'] },
				{ name: 'All', extensions: ['*'] },
			]);
			if (path) {
				evalTestDataPath = path;
			}
		} catch (e: any) {
			console.error('File selection failed:', e);
		}
	}

	async function runBatchInference() {
		if (!detail || !batchInferenceData.trim()) return;
		batchInferenceRunning = true;
		batchInferenceError = null;
		batchInferenceResults = [];
		try {
			const lines = batchInferenceData.trim().split('\n').filter(l => l.trim());
			const inputs: number[][] = [];
			for (const line of lines) {
				const values = line.trim().split(/[\s,]+/).map(Number).filter(v => !isNaN(v));
				if (values.length > 0) {
					inputs.push(values);
				}
			}
			if (inputs.length === 0) {
				batchInferenceError = $t('experimentDetail.invalidBatchData');
				return;
			}
			const client = getLabClient();
			const results: Array<{ input: number[]; result: InferenceResult } | null> = [];
			for (let i = 0; i < inputs.length; i++) {
				try {
					const result = await client.batchInference(experimentId, [inputs[i]]);
					results.push({ input: inputs[i], result });
				} catch {
					results.push(null);
				}
			}
			batchInferenceResults = results;
		} catch (e: any) {
			batchInferenceError = e?.message || $t('experimentDetail.batchInferenceFailed');
		} finally {
			batchInferenceRunning = false;
		}
	}

	async function addTag() {
		if (!detail || !newTag.trim()) return;
		tagAdding = true;
		try {
			const client = getLabClient();
			await client.experimentAddTag(experimentId, newTag.trim());
			detail = await experimentStore.loadDetail(experimentId);
			newTag = '';
		} catch (e: any) {
			console.error('Failed to add tag:', e);
		} finally {
			tagAdding = false;
		}
	}

	async function saveGroup() {
		if (!detail) return;
		groupSaving = true;
		try {
			const client = getLabClient();
			await client.setExperimentGroup(experimentId, editGroup.trim());
			detail = await experimentStore.loadDetail(experimentId);
			showEditGroup = false;
		} catch (e: any) {
			console.error('Failed to save group:', e);
		} finally {
			groupSaving = false;
		}
	}

	async function saveDescription() {
		if (!detail) return;
		descriptionSaving = true;
		try {
			const client = getLabClient();
			await client.experimentSetDescription(experimentId, editDescription);
			detail = await experimentStore.loadDetail(experimentId);
			showEditDescription = false;
		} catch (e: any) {
			console.error('Failed to save description:', e);
		} finally {
			descriptionSaving = false;
		}
	}

	async function addParam() {
		if (!detail || !newParamKey.trim()) return;
		paramAdding = true;
		try {
			const client = getLabClient();
			let parsedValue: unknown = newParamValue;
			try {
				parsedValue = JSON.parse(newParamValue);
			} catch {}
			await client.experimentSetParam(experimentId, newParamKey.trim(), parsedValue);
			detail = await experimentStore.loadDetail(experimentId);
			newParamKey = '';
			newParamValue = '';
		} catch (e: any) {
			console.error('Failed to add param:', e);
		} finally {
			paramAdding = false;
		}
	}

	async function loadNotes() {
		if (notesLoaded || !detail) return;
		const params = detail.params;
		notesText = (params['_notes'] as string) || '';
		notesLoaded = true;
	}

	async function saveNotes() {
		if (!detail) return;
		notesSaving = true;
		try {
			const client = getLabClient();
			await client.experimentSetParam(experimentId, '_notes', notesText);
			detail = await experimentStore.loadDetail(experimentId);
		} catch (e: any) {
			console.error('Failed to save notes:', e);
		} finally {
			notesSaving = false;
		}
	}

	function exportNotesAsFile() {
		if (!notesText.trim()) return;
		const blob = new Blob([notesText], { type: 'text/markdown' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${detail?.name || 'experiment'}_notes.md`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	function exportExperimentSummary() {
		if (!detail) return;
		const lines: string[] = [];
		lines.push(`# ${$t('experimentDetail.reportTitle')}: ${detail.name}`);
		lines.push('');
		lines.push(`- **ID**: ${detail.id}`);
		lines.push(`- **${$t('experimentDetail.statusLabel')}**: ${detail.status}`);
		lines.push(`- **${$t('experimentDetail.taskTypeLabel')}**: ${detail.task_type}`);
		lines.push(`- **${$t('experimentDetail.createdLabel')}**: ${formatTime(detail.created_at)}`);
		lines.push(`- **${$t('experimentDetail.completedLabel')}**: ${formatTime(detail.completed_at) || $t('experimentDetail.ongoing')}`);
		lines.push(`- **${$t('experimentDetail.durationLabel')}**: ${duration(detail.created_at, detail.completed_at)}`);
		if (detail.description) {
			lines.push(`- **${$t('experimentDetail.descLabel')}**: ${detail.description}`);
		}
		lines.push('');
		lines.push(`## ${$t('experimentDetail.trainingConfig')}`);
		lines.push('');
		lines.push(`- **${$t('experimentDetail.engineLabel')}**: ${detail.config.engine_id}`);
		lines.push(`- **Epochs**: ${detail.config.epochs}`);
		lines.push(`- **${$t('experimentDetail.learningRateLabel')}**: ${detail.config.learning_rate}`);
		lines.push(`- **Batch Size**: ${detail.config.batch_size}`);
		lines.push(`- **${$t('experimentDetail.optimizerLabel')}**: ${optimizerName(detail.config.optimizer)}`);
		if (detail.config.lr_scheduler) {
			lines.push(`- **${$t('experimentDetail.lrSchedulerLabel')}**: ${lrSchedulerName(detail.config.lr_scheduler)}`);
		}
		lines.push('');
		lines.push(`## ${$t('experimentDetail.hyperparams')}`);
		lines.push('');
		const skipKeys = ['_notes', 'engine_id', 'epochs', 'learning_rate', 'batch_size', 'optimizer', 'lr_scheduler'];
		for (const [key, value] of Object.entries(detail.params)) {
			if (!skipKeys.includes(key)) {
				lines.push(`- **${key}**: ${typeof value === 'object' ? JSON.stringify(value) : value}`);
			}
		}
		lines.push('');
		lines.push(`## ${$t('experimentDetail.metrics')}`);
		lines.push('');
		for (const [name, series] of Object.entries(metricsTimeline.series)) {
			if (series.values && series.values.length > 0) {
				const last = series.values[series.values.length - 1].value;
				const best = series.values.reduce((a: number, b) => name.includes('loss') ? Math.min(a, b.value) : Math.max(a, b.value), series.values[0].value);
				lines.push(`- **${name}**: ${$t('experimentDetail.final')}=${last.toFixed(6)}, ${$t('experimentDetail.best')}=${best.toFixed(6)} (${series.values.length} ${$t('experimentDetail.steps')})`);
			}
		}
		if (notesText.trim()) {
			lines.push('');
			lines.push(`## ${$t('experimentDetail.notes')}`);
			lines.push('');
			lines.push(notesText);
		}
		const blob = new Blob([lines.join('\n')], { type: 'text/markdown' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${detail.name}_report.md`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	function markdownToHtml(md: string): string {
		let html = md
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;');
		html = html.replace(/^### (.+)$/gm, '<h4>$1</h4>');
		html = html.replace(/^## (.+)$/gm, '<h3>$1</h3>');
		html = html.replace(/^# (.+)$/gm, '<h2>$1</h2>');
		html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
		html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
		html = html.replace(/`(.+?)`/g, '<code>$1</code>');
		html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
		html = html.replace(/(<li>.*<\/li>\n?)+/g, (match) => `<ul>${match}</ul>`);
		html = html.replace(/\n{2,}/g, '</p><p>');
		html = `<p>${html}</p>`;
		html = html.replace(/<p><\/p>/g, '');
		html = html.replace(/<p>(<h[2-4]>)/g, '$1');
		html = html.replace(/(<\/h[2-4]>)<\/p>/g, '$1');
		html = html.replace(/<p>(<ul>)/g, '$1');
		html = html.replace(/(<\/ul>)<\/p>/g, '$1');
		return html;
	}

	async function loadExportFormats() {
		if (!detail) return;
		try {
			const client = getLabClient();
			availableExportFormats = await client.listExportFormats(detail.config.engine_id);
			if (availableExportFormats.length > 0 && !availableExportFormats.includes(exportFormat)) {
				exportFormat = availableExportFormats[0];
			}
		} catch (e) {
			availableExportFormats = ['burn_record'];
		}
	}

	async function handleExportModel() {
		if (!detail) return;
		exportRunning = true;
		exportResult = null;
		exportError = null;
		try {
			const client = getLabClient();
			exportResult = await client.exportModel(experimentId, exportFormat, null, null, []);
		} catch (e: any) {
			exportError = e?.toString() || $t('experimentDetail.exportFailed');
		} finally {
			exportRunning = false;
		}
	}

	function optimizerName(opt: any): string {
		if (!opt) return '-';
		if (opt.Sgd) return `SGD (momentum=${opt.Sgd.momentum ?? '-'}, weight_decay=${opt.Sgd.weight_decay ?? '-'})`;
		if (opt.Adam) return `Adam (β1=${opt.Adam.beta1 ?? '-'}, β2=${opt.Adam.beta2 ?? '-'}, weight_decay=${opt.Adam.weight_decay ?? '-'})`;
		if (opt.AdamW) return `AdamW (β1=${opt.AdamW.beta1 ?? '-'}, β2=${opt.AdamW.beta2 ?? '-'}, weight_decay=${opt.AdamW.weight_decay ?? '-'})`;
		if (opt.Rmsprop) return `Rmsprop (alpha=${opt.Rmsprop.alpha ?? '-'}, weight_decay=${opt.Rmsprop.weight_decay ?? '-'})`;
		return JSON.stringify(opt);
	}

	function lrSchedulerName(config: any): string {
		if (!config) return '-';
		if (config === 'Constant') return 'Constant';
		if (config.Constant !== undefined) return 'Constant';
		if (config.Step) return `Step (step_size=${config.Step.step_size ?? '-'}, γ=${config.Step.gamma ?? '-'})`;
		if (config.Exponential) return `Exponential (γ=${config.Exponential.gamma ?? '-'})`;
		if (config.CosineAnnealing) return `CosineAnnealing (min_lr=${config.CosineAnnealing.min_lr ?? '-'}, num_iters=${config.CosineAnnealing.num_iters ?? '-'})`;
		if (config.Linear) return `Linear (final_lr=${config.Linear.final_lr ?? '-'}, num_iters=${config.Linear.num_iters ?? '-'})`;
		return JSON.stringify(config);
	}

	interface ArtifactGroup {
		type: string;
		icon: string;
		items: any[];
	}

	function buildArtifactTree(artifacts: any[]): ArtifactGroup[] {
		const groups: Map<string, ArtifactGroup> = new Map();
		const typeConfig: Record<string, { icon: string; label: string }> = {
			model: { icon: '🧠', label: $t('experimentDetail.artifactModel') },
			checkpoint: { icon: '💾', label: $t('experimentDetail.artifactCheckpoint') },
			log: { icon: '📄', label: $t('experimentDetail.artifactLog') },
			metric: { icon: '📊', label: $t('experimentDetail.artifactMetric') },
			config: { icon: '⚙️', label: $t('experimentDetail.artifactConfig') },
			other: { icon: '📁', label: $t('experimentDetail.artifactOther') },
		};

		for (const a of artifacts) {
			const t = a.artifact_type || 'other';
			if (!groups.has(t)) {
				const cfg = typeConfig[t] || typeConfig.other;
				groups.set(t, { type: cfg.label, icon: cfg.icon, items: [] });
			}
			groups.get(t)!.items.push(a);
		}

		return Array.from(groups.values());
	}

	function fileIcon(path: string): string {
		const ext = path.split('.').pop()?.toLowerCase() || '';
		const icons: Record<string, string> = {
			'bin': '🗄️', 'pt': '🧠', 'pth': '🧠', 'onnx': '🧠', 'safetensors': '🧠',
			'json': '📋', 'yaml': '📋', 'yml': '📋', 'toml': '📋',
			'csv': '📊', 'tsv': '📊', 'parquet': '📊',
			'log': '📄', 'txt': '📄', 'md': '📝',
			'png': '🖼️', 'jpg': '🖼️', 'jpeg': '🖼️', 'svg': '🖼️',
		};
		return icons[ext] || '📄';
	}

	async function previewArtifact(artifact: any) {
		const ext = artifact.path.split('.').pop()?.toLowerCase() || '';
		const textExts = ['json', 'yaml', 'yml', 'toml', 'csv', 'tsv', 'log', 'txt', 'md', 'rs', 'py', 'js', 'ts'];
		if (!textExts.includes(ext)) {
			downloadArtifact(artifact);
			return;
		}
		artifactPreviewPath = artifact.path;
		artifactPreviewContent = null;
		artifactPreviewError = null;
		artifactPreviewLoading = true;
		try {
			const client = getLabClient();
			const bytes = await client.getArtifactContent(experimentId, artifact.path);
			const decoder = new TextDecoder('utf-8');
			artifactPreviewContent = decoder.decode(new Uint8Array(bytes));
		} catch (e: any) {
			artifactPreviewError = e.message || $t('experimentDetail.cannotLoadFile');
		} finally {
			artifactPreviewLoading = false;
		}
	}

	function closeArtifactPreview() {
		artifactPreviewPath = null;
		artifactPreviewContent = null;
		artifactPreviewError = null;
	}

	async function downloadArtifact(artifact: any) {
		try {
			const client = getLabClient();
			const bytes = await client.getArtifactContent(experimentId, artifact.path);
			const blob = new Blob([new Uint8Array(bytes)]);
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = artifact.path.split('/').pop() || 'artifact';
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		} catch (e: any) {
			console.error('Download failed:', e);
		}
	}

	async function scanArtifactsAction() {
		scanningArtifacts = true;
		try {
			const client = getLabClient();
			await client.scanArtifacts(experimentId);
			detail = await experimentStore.loadDetail(experimentId);
		} catch (e: any) {
			console.error('Scan artifacts failed:', e);
		} finally {
			scanningArtifacts = false;
		}
	}
</script>

<div class="experiment-detail">
	{#if loading}
		<div class="loading-state">
			<div class="spinner"></div>
			<p>{$t('experimentDetail.loadingDetail')}</p>
		</div>
	{:else if detail}
		<div class="detail-header">
			<a href="/lab" class="back-link">← {$t('experimentDetail.backToList')}</a>
			<div class="header-content">
				<div class="header-left">
					<h1 class="exp-title">{detail.name}</h1>
					<div class="exp-meta">
						<span class="status-badge" style="color: {statusColor(detail.status)}; border-color: {statusColor(detail.status)}30; background: {statusColor(detail.status)}10">
							{detail.status}
						</span>
						{#if detail.status === 'running'}
							<span class="heartbeat-indicator" class:heartbeat-stale={heartbeatElapsed > 30} class:heartbeat-dead={heartbeatElapsed > 60}>
								<span class="heartbeat-dot"></span>
								{#if lastHeartbeat}
									{heartbeatElapsed <= 30 ? $t('experimentDetail.active') : `${heartbeatElapsed}s ${$t('experimentDetail.noResponse')}`}
								{:else}
									{$t('experimentDetail.waitingHeartbeat')}
								{/if}
							</span>
						{/if}
						<span class="meta-item">{detail.task_type}</span>
						<span class="meta-item">{detail.id}</span>
						<span class="meta-item">{$t('experimentDetail.created')}: {formatTime(detail.created_at)}</span>
						<span class="meta-item">{$t('experimentDetail.duration')}: {duration(detail.created_at, detail.completed_at)}</span>
					</div>
					{#if detail.description || showEditDescription}
						<div class="description-section">
							{#if showEditDescription}
								<textarea bind:value={editDescription} class="desc-textarea" placeholder={$t('experimentDetail.enterDesc')}></textarea>
								<div class="desc-actions">
									<button class="btn-sm btn-save" on:click={saveDescription} disabled={descriptionSaving}>
										{descriptionSaving ? $t('experimentDetail.saving') : $t('experimentDetail.save')}
									</button>
									<button class="btn-sm btn-cancel" on:click={() => showEditDescription = false}>{$t('confirm.cancel')}</button>
								</div>
							{:else}
								<p class="description-text">{detail.description}</p>
								<button class="btn-sm btn-edit" on:click={() => { editDescription = detail?.description || ''; showEditDescription = true; }}>{$t('experimentDetail.editDesc')}</button>
							{/if}
						</div>
					{:else}
						<button class="btn-sm btn-edit" on:click={() => { editDescription = ''; showEditDescription = true; }}>+ {$t('experimentDetail.addDesc')}</button>
					{/if}
					{#if detail.tags.length > 0}
						<div class="tags">
							{#each detail.tags as tag}
								<span class="tag">{tag}</span>
							{/each}
						</div>
					{/if}

					<hr class="section-divider" />

					<h3 class="subsection-title">{$t('experimentDetail.batchInference')}</h3>
					<p class="inference-hint">{$t('experimentDetail.batchInferenceHint')}</p>

					<div class="inference-form">
						<textarea
							bind:value={batchInferenceData}
							placeholder="5.1, 3.5, 1.4, 0.2&#10;6.2, 2.9, 4.3, 1.3&#10;7.7, 3.8, 6.7, 2.2"
							class="inference-textarea"
							rows="5"
						></textarea>
						<button
							class="btn-inference"
							on:click={runBatchInference}
							disabled={batchInferenceRunning || !batchInferenceData.trim()}
						>
							{batchInferenceRunning ? $t('experimentDetail.inferring') : `📦 ${$t('experimentDetail.batchInference')}`}
						</button>
					</div>

					{#if batchInferenceError}
						<div class="error-banner">
							<span class="error-icon">✗</span>
							<span>{batchInferenceError}</span>
						</div>
					{/if}

					{#if batchInferenceResults.length > 0}
						<div class="inference-result">
							<h4>{$t('experimentDetail.batchResults')} ({batchInferenceResults.length})</h4>
							<div class="batch-results-table">
								<div class="batch-table-header">
									<span>#</span>
									<span>{$t('experimentDetail.input')}</span>
									<span>{$t('experimentDetail.predictedClass')}</span>
									<span>{$t('experimentDetail.predictedValue')}</span>
								</div>
								{#each batchInferenceResults as item, i}
									<div class="batch-table-row">
										<span>{i + 1}</span>
										<span class="batch-input">{item ? item.input.map(v => v.toFixed(2)).join(', ') : '-'}</span>
										<span class="batch-predicted">{item && item.result.predicted_classes.length > 0 ? item.result.predicted_classes[0] : '-'}</span>
										<span class="batch-value">{item && item.result.predictions.length > 0 ? item.result.predictions.map(v => v.toFixed(4)).join(', ') : '-'}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
				<div class="header-right">
					{#if detail.status === 'running'}
						<button class="pause-btn" on:click={async () => { await experimentStore.pauseTraining(experimentId); }}>⏸ {$t('experimentDetail.pause')}</button>
						<button class="stop-btn" on:click={async () => { await experimentStore.stopTraining(experimentId); }}>⏹ {$t('experimentDetail.stopTraining')}</button>
					{/if}
					{#if detail.status === 'paused'}
						<button class="resume-btn" on:click={async () => { await experimentStore.resumeTraining(experimentId); }}>▶ {$t('experimentDetail.resumeTraining')}</button>
						<button class="stop-btn" on:click={async () => { await experimentStore.stopTraining(experimentId); }}>⏹ {$t('experimentDetail.stopTraining')}</button>
					{/if}
					{#if detail.status === 'failed' || detail.status === 'cancelled'}
						<button class="resume-btn" on:click={openCheckpointResumeModal}>🔄 {$t('experimentDetail.resumeFromCheckpoint')}</button>
					{/if}
					{#if detail.status === 'completed' && !detail.model_id}
						<button class="register-btn" on:click={openRegisterModal}>📦 {$t('experimentDetail.registerModel')}</button>
					{/if}
					{#if detail.status !== 'running' && detail.status !== 'paused' && detail.status !== 'archived'}
						<button class="archive-btn" on:click={archiveExperiment}>📁 {$t('experimentDetail.archive')}</button>
					{/if}
					{#if detail.status === 'archived'}
						<button class="restore-btn" on:click={restoreExperiment}>♻️ {$t('experimentDetail.restore')}</button>
					{/if}
				</div>
			</div>
		</div>

		{#if detail.error_message}
			<div class="error-banner">
				<span class="error-icon">✗</span>
				<span>{detail.error_message}</span>
			</div>
		{/if}

		{#if detail.status === 'running' || detail.status === 'paused'}
			{#if detail.status === 'paused'}
				<div class="paused-banner">⏸ {$t('experimentDetail.pausedHint')}</div>
			{/if}
			<div class="progress-section">
				<TrainingProgress sessionId={experimentId} />
			</div>
		{/if}

		<div class="tab-bar">
			<button class="tab-btn" class:active={activeTab === 'metrics'} on:click={() => activeTab = 'metrics'}>📈 {$t('experimentDetail.metrics')}</button>
			<button class="tab-btn" class:active={activeTab === 'logs'} on:click={() => activeTab = 'logs'}>📋 {$t('experimentDetail.logs')}</button>
			<button class="tab-btn" class:active={activeTab === 'params'} on:click={() => activeTab = 'params'}>🔧 {$t('experimentDetail.params')}</button>
			<button class="tab-btn" class:active={activeTab === 'config'} on:click={() => activeTab = 'config'}>⚙️ {$t('experimentDetail.config')}</button>
			<button class="tab-btn" class:active={activeTab === 'notes'} on:click={() => { activeTab = 'notes'; loadNotes(); }}>📝 {$t('experimentDetail.notes')}</button>
			<button class="tab-btn" class:active={activeTab === 'artifacts'} on:click={() => activeTab = 'artifacts'}>📦 {$t('experimentDetail.artifacts')}</button>
			<button class="tab-btn" class:active={activeTab === 'environment'} on:click={() => activeTab = 'environment'}>🖥️ {$t('experimentDetail.environment')}</button>
			{#if detail.status === 'completed'}
				<button class="tab-btn" class:active={activeTab === 'inference'} on:click={() => activeTab = 'inference'}>🔮 {$t('experimentDetail.inference')}</button>
			{/if}
		</div>

		<div class="tab-content">
			{#if activeTab === 'metrics'}
				<div class="metrics-section">
					<div class="metrics-controls">
						<label class="control-item">
							<span>{$t('experimentDetail.smoothAlpha')}</span>
							<input type="range" min="0" max="0.9" step="0.1" bind:value={metricsSmoothAlpha} />
							<span class="control-value">{metricsSmoothAlpha.toFixed(1)}</span>
						</label>
						<label class="control-item">
							<span>{$t('experimentDetail.maxPoints')}</span>
							<input type="range" min="100" max="2000" step="100" bind:value={metricsMaxPoints} />
							<span class="control-value">{metricsMaxPoints}</span>
						</label>
						<button class="btn btn-sm" on:click={refreshMetrics} disabled={metricsLoading}>
							{metricsLoading ? $t('experimentDetail.loading') : $t('experimentDetail.refresh')}
						</button>
					</div>
					{#if Object.keys(metricsTimeline.series).length > 0}
						<MetricsChart
							series={metricsTimeline.series}
							height="400px"
							title={$t('experimentDetail.trainingMetrics')}
						/>
					{:else if Object.keys(detail.metrics.series).length > 0}
						<MetricsChart
							series={detail.metrics.series}
							height="400px"
							title={$t('experimentDetail.trainingMetrics')}
						/>
					{:else}
						<div class="empty-metrics">
							<p>{$t('experimentDetail.noMetrics')}</p>
						</div>
					{/if}

					<div class="metrics-summary">
						<h3 class="subsection-title">{$t('experimentDetail.metricsOverview')}</h3>
						<div class="metrics-grid">
							{#each Object.entries(detail.metrics.series) as [name, s]}
								{@const values = s.values.map((v: any) => v.value)}
								{@const latest = values.length > 0 ? values[values.length - 1] : null}
								{@const minVal = values.length > 0 ? Math.min(...values) : null}
								{@const maxVal = values.length > 0 ? Math.max(...values) : null}
								{@const isLoss = name.toLowerCase().includes('loss')}
								{@const bestVal = isLoss ? minVal : maxVal}
								{@const trend = values.length >= 2 ? latest - values[values.length - 2] : 0}
								<div class="metric-card">
									<span class="metric-name">{name}</span>
									{#if latest !== null}
										<span class="metric-value">{latest.toFixed(4)}</span>
										<div class="metric-stats">
											<span class="metric-stat best">
												{$t('experimentDetail.best')}: {bestVal !== null ? bestVal.toFixed(4) : '-'}
											</span>
											<span class="metric-stat">
												{$t('experimentDetail.min')}: {minVal !== null ? minVal.toFixed(4) : '-'}
											</span>
											<span class="metric-stat">
												{$t('experimentDetail.max')}: {maxVal !== null ? maxVal.toFixed(4) : '-'}
											</span>
										</div>
										<span class="metric-info">
											{#if trend !== 0}
												<span class="trend" class:trend-down={isLoss ? trend < 0 : trend < 0} class:trend-up={isLoss ? trend > 0 : trend > 0}>
													{trend > 0 ? '↑' : '↓'} {Math.abs(trend).toFixed(4)}
												</span>
												·
											{/if}
											{values.length} {$t('experimentDetail.dataPoints')}
										</span>
									{:else}
										<span class="metric-value">-</span>
									{/if}
								</div>
							{/each}
						</div>
					</div>
				</div>
			{:else if activeTab === 'logs'}
				<div class="logs-section">
					{#if $experimentLogs.get(experimentId)?.length}
						<div class="logs-toolbar">
							<span class="logs-count">{$t('experimentDetail.totalLogs', { count: $experimentLogs.get(experimentId)?.length ?? 0 })}</span>
							<div class="logs-filters">
								<button class="btn-small" class:active={logLevelFilter === 'all'} on:click={() => logLevelFilter = 'all'}>{$t('experimentDetail.all')}</button>
								<button class="btn-small" class:active={logLevelFilter === 'error'} on:click={() => logLevelFilter = 'error'}>{$t('experimentDetail.error')}</button>
								<button class="btn-small" class:active={logLevelFilter === 'warn'} on:click={() => logLevelFilter = 'warn'}>{$t('experimentDetail.warn')}</button>
								<button class="btn-small" class:active={logLevelFilter === 'info'} on:click={() => logLevelFilter = 'info'}>{$t('experimentDetail.info')}</button>
							</div>
							<label class="auto-scroll-label">
								<input type="checkbox" bind:checked={autoScrollLogs} />
								{$t('experimentDetail.autoScroll')}
							</label>
							<button class="btn-small" on:click={() => {
								const logs = ($experimentLogs.get(experimentId) || [])
									.filter((l: { level: string; message: string; timestamp: number }) => logLevelFilter === 'all' || l.level === logLevelFilter);
								const text = logs.map((l: { timestamp: number; level: string; message: string }) => `[${new Date(l.timestamp).toLocaleTimeString()}] [${l.level.toUpperCase()}] ${l.message}`).join('\n');
								navigator.clipboard.writeText(text);
							}}>{$t('experimentDetail.copy')}</button>
						</div>
						<div class="logs-container" bind:this={logsContainer}>
							{#each ($experimentLogs.get(experimentId) || []).filter((l: { level: string }) => logLevelFilter === 'all' || l.level === logLevelFilter) as logEntry}
								<div class="log-line" class:log-error={logEntry.level === 'error'} class:log-warn={logEntry.level === 'warn'} class:log-info={logEntry.level === 'info'} class:log-debug={logEntry.level === 'debug'}>
									<span class="log-time">{new Date(logEntry.timestamp).toLocaleTimeString()}</span>
									<span class="log-level {logEntry.level}">[{logEntry.level.toUpperCase()}]</span>
									<span class="log-msg">{logEntry.message}</span>
								</div>
							{/each}
						</div>
					{:else}
						<p class="empty-hint">{$t('experimentDetail.noLogs')}</p>
					{/if}
				</div>
			{:else if activeTab === 'params'}
				<div class="params-section">
					<div class="params-subsection">
						<h3 class="subsection-title">{$t('experimentDetail.tags')}</h3>
						<div class="tags-management">
							{#if detail.tags.length > 0}
								<div class="tags-list">
									{#each detail.tags as tag}
										<span class="tag">{tag}</span>
									{/each}
								</div>
							{:else}
								<p class="empty-hint">{$t('experimentDetail.noTags')}</p>
							{/if}
							<div class="add-tag-form">
								<input
									type="text"
									bind:value={newTag}
									placeholder={$t('experimentDetail.enterTagName')}
									class="input-sm"
									on:keydown={(e) => { if (e.key === 'Enter') addTag(); }}
								/>
								<button class="btn-sm" on:click={addTag} disabled={tagAdding || !newTag.trim()}>
									{tagAdding ? '...' : `+ ${$t('experimentDetail.add')}`}
								</button>
							</div>
						</div>
					</div>

					<div class="params-subsection">
						<h3 class="subsection-title">{$t('experimentDetail.group')}</h3>
						{#if showEditGroup}
							<div class="add-tag-form">
								<input
									type="text"
									bind:value={editGroup}
									placeholder={$t('experimentDetail.enterGroupName')}
									class="input-sm"
									on:keydown={(e) => { if (e.key === 'Enter') saveGroup(); }}
								/>
								<button class="btn-sm btn-save" on:click={saveGroup} disabled={groupSaving}>
									{groupSaving ? '...' : $t('experimentDetail.save')}
								</button>
								<button class="btn-sm" on:click={() => showEditGroup = false}>{$t('confirm.cancel')}</button>
							</div>
						{:else}
							<div class="add-tag-form">
								{#if detail.group}
									<span class="tag">{detail.group}</span>
								{:else}
									<span class="empty-hint">{$t('experimentDetail.ungrouped')}</span>
								{/if}
								<button class="btn-sm btn-edit" on:click={() => { editGroup = detail?.group || ''; showEditGroup = true; }}>
									{detail.group ? $t('experimentDetail.modify') : $t('experimentDetail.setGroup')}
								</button>
							</div>
						{/if}
					</div>

					<div class="params-subsection">
						<h3 class="subsection-title">{$t('experimentDetail.params')}</h3>
						{#if Object.keys(detail.params).length > 0}
							<div class="params-table">
								{#each Object.entries(detail.params) as [key, value]}
									<div class="param-row">
										<span class="param-key">{key}</span>
										<span class="param-value">{JSON.stringify(value)}</span>
									</div>
								{/each}
							</div>
						{:else}
							<p class="empty-hint">{$t('experimentDetail.noParams')}</p>
						{/if}
						<div class="add-param-form">
							<input
								type="text"
								bind:value={newParamKey}
								placeholder={$t('experimentDetail.paramName')}
								class="input-sm"
							/>
							<input
								type="text"
								bind:value={newParamValue}
								placeholder={$t('experimentDetail.paramValueJson')}
								class="input-sm"
								on:keydown={(e) => { if (e.key === 'Enter') addParam(); }}
							/>
							<button class="btn-sm" on:click={addParam} disabled={paramAdding || !newParamKey.trim()}>
								{paramAdding ? '...' : `+ ${$t('experimentDetail.add')}`}
							</button>
						</div>
					</div>
				</div>
			{:else if activeTab === 'config'}
				<div class="config-section">
					<h3 class="subsection-title">{$t('experimentDetail.trainingConfig')}</h3>
					<div class="config-grid">
						<div class="config-group">
							<h4 class="config-group-title">{$t('experimentDetail.basicSettings')}</h4>
							<div class="params-table">
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.engineLabel')}</span>
									<span class="param-value">{detail.config.engine_id}</span>
								</div>
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.taskTypeLabel')}</span>
									<span class="param-value">{detail.config.task_type}</span>
								</div>
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.modelLabel')}</span>
									<span class="param-value">{detail.config.model_id}</span>
								</div>
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.dataSourceLabel')}</span>
									<span class="param-value">{detail.config.data_source_id}</span>
								</div>
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.dataPathLabel')}</span>
									<span class="param-value">{detail.config.data_path || '-'}</span>
								</div>
							</div>
						</div>
						<div class="config-group">
							<h4 class="config-group-title">{$t('experimentDetail.trainingParams')}</h4>
							<div class="params-table">
								<div class="param-row">
									<span class="param-key">Epochs</span>
									<span class="param-value">{detail.config.epochs}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Batch Size</span>
									<span class="param-value">{detail.config.batch_size}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Learning Rate</span>
									<span class="param-value">{detail.config.learning_rate}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Loss Function</span>
									<span class="param-value">{detail.config.loss_function}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Compute Backend</span>
									<span class="param-value">{detail.config.compute_backend}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Validation Split</span>
									<span class="param-value">{detail.config.validation_split}</span>
								</div>
								<div class="param-row">
									<span class="param-key">Test Split</span>
									<span class="param-value">{detail.config.test_split}</span>
								</div>
							</div>
						</div>
						<div class="config-group">
							<h4 class="config-group-title">{$t('experimentDetail.optimizer')}</h4>
							<div class="params-table">
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.type')}</span>
									<span class="param-value">{optimizerName((detail.config as any).optimizer)}</span>
								</div>
							</div>
						</div>
						<div class="config-group">
							<h4 class="config-group-title">{$t('experimentDetail.lrScheduler')}</h4>
							<div class="params-table">
								<div class="param-row">
									<span class="param-key">{$t('experimentDetail.strategy')}</span>
									<span class="param-value">{lrSchedulerName((detail.config as any).lr_scheduler)}</span>
								</div>
							</div>
						</div>
					</div>
				</div>
			{:else if activeTab === 'notes'}
				<div class="notes-section">
					<div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
						<h3 class="subsection-title" style="margin: 0;">{$t('experimentDetail.experimentNotes')}</h3>
						<div style="display: flex; gap: 8px;">
							<button class="btn-sm" class:btn-active={notesPreviewMode} on:click={() => notesPreviewMode = !notesPreviewMode}>
								{notesPreviewMode ? `✏️ ${$t('experimentDetail.edit')}` : `👁️ ${$t('experimentDetail.preview')}`}
							</button>
							<button class="btn-sm" on:click={exportNotesAsFile} disabled={!notesText.trim()}>📄 {$t('experimentDetail.exportNotes')}</button>
							<button class="btn-sm" on:click={exportExperimentSummary}>📊 {$t('experimentDetail.exportReport')}</button>
						</div>
					</div>
					<p class="notes-hint">{$t('experimentDetail.notesHint')}</p>
					{#if notesPreviewMode}
						<div class="notes-preview">
							{#if notesText.trim()}
								{@html markdownToHtml(notesText)}
							{:else}
								<p style="color: var(--text-secondary);">{$t('experimentDetail.noNotes')}</p>
							{/if}
						</div>
					{:else}
						<textarea
							class="notes-editor"
							bind:value={notesText}
							placeholder={$t('experimentDetail.notesPlaceholder')}
							rows="12"
						></textarea>
					{/if}
					<div class="notes-actions">
						<button class="btn-save-notes" on:click={saveNotes} disabled={notesSaving}>
							{notesSaving ? $t('experimentDetail.saving') : `💾 ${$t('experimentDetail.saveNotes')}`}
						</button>
						<span class="notes-char-count">{$t('experimentDetail.charCount', { count: notesText.length })}</span>
					</div>
				</div>
			{:else if activeTab === 'artifacts'}
				<div class="artifacts-section">
					<div class="artifacts-toolbar">
						<div class="artifacts-toolbar-left">
							<span class="artifacts-count">{$t('experimentDetail.artifactCount', { count: detail.artifacts.length })}</span>
							<span class="artifacts-total-size">{$t('experimentDetail.totalSize')}: {formatSize(detail.artifacts.reduce((s, a) => s + a.size_bytes, 0))}</span>
						</div>
						<button class="btn-scan" on:click={scanArtifactsAction} disabled={scanningArtifacts}>
							{scanningArtifacts ? $t('experimentDetail.scanning') : `🔍 ${$t('experimentDetail.scanNewArtifacts')}`}
						</button>
					</div>
					{#if detail.artifacts.length > 0}
						<div class="artifacts-tree">
							{#each buildArtifactTree(detail.artifacts) as group}
								<div class="artifact-group">
									<div class="artifact-group-header">
										<span class="group-icon">{group.icon}</span>
										<span class="group-name">{group.type}</span>
										<span class="group-count">{group.items.length}</span>
									</div>
									<div class="artifact-group-items">
										{#each group.items as artifact}
											<div class="artifact-card">
												<div class="artifact-icon" role="button" tabindex="0" aria-label={$t('experimentDetail.previewFile')} on:click={() => previewArtifact(artifact)} on:keydown={(e) => e.key === 'Enter' && previewArtifact(artifact)}>{fileIcon(artifact.path)}</div>
												<div class="artifact-info" role="button" tabindex="0" aria-label={`${$t('experimentDetail.preview')} ${artifact.path.split('/').pop() || artifact.path}`} on:click={() => previewArtifact(artifact)} on:keydown={(e) => e.key === 'Enter' && previewArtifact(artifact)}>
													<span class="artifact-path">{artifact.path.split('/').pop() || artifact.path}</span>
													<span class="artifact-dir">{artifact.path.includes('/') ? artifact.path.substring(0, artifact.path.lastIndexOf('/')) : ''}</span>
												</div>
												<div class="artifact-meta">
													<span class="artifact-size">{formatSize(artifact.size_bytes)}</span>
													<span class="artifact-time">{formatTime(artifact.created_at)}</span>
												</div>
												<div class="artifact-actions">
													<button class="btn-artifact-action" on:click|stopPropagation={() => downloadArtifact(artifact)} title={$t('experimentDetail.download')}>⬇️</button>
												</div>
											</div>
										{/each}
									</div>
								</div>
							{/each}
						</div>
					{:else}
						<div class="empty-metrics">
							<p>{$t('experimentDetail.noArtifacts')}</p>
							<p class="empty-hint">{$t('experimentDetail.noArtifactsHint')}</p>
						</div>
					{/if}

					<div class="export-section" style="margin-top: 24px; padding: 16px; border: 1px solid var(--border); border-radius: 8px;">
						<h3 style="margin: 0 0 12px 0; font-size: 14px; font-weight: 600;">{$t('experimentDetail.modelExport')}</h3>
						<div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
							<select bind:value={exportFormat} class="form-input" style="width: auto; min-width: 160px;">
								{#each availableExportFormats as fmt}
									<option value={fmt}>{fmt}</option>
								{/each}
								{#if availableExportFormats.length === 0}
									<option value="burn_record">burn_record</option>
								{/if}
							</select>
							<button class="btn-primary" on:click={handleExportModel} disabled={exportRunning || detail.status === 'running'}>
								{exportRunning ? $t('experimentDetail.exporting') : $t('experimentDetail.exportModel')}
							</button>
						</div>
						{#if exportError}
							<div class="error-banner" style="margin-top: 12px;">{exportError}</div>
						{/if}
						{#if exportResult}
							<div style="margin-top: 12px; padding: 12px; background: var(--bg-secondary); border-radius: 6px;">
								<div style="font-weight: 600; margin-bottom: 8px;">
									{exportResult.success ? `✅ ${$t('experimentDetail.exportSuccess')}` : `❌ ${$t('experimentDetail.exportFailed')}`}
								</div>
								<div style="font-size: 13px;">
									<div>{$t('experimentDetail.format')}: {exportResult.format}</div>
									<div>{$t('experimentDetail.path')}: {exportResult.output_path}</div>
									<div>{$t('experimentDetail.size')}: {(exportResult.file_size_bytes / 1024).toFixed(1)} KB</div>
									{#if exportResult.message}
										<div style="margin-top: 4px; color: var(--text-secondary);">{exportResult.message}</div>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{:else if activeTab === 'environment'}
				<div class="environment-section">
					{#if detail.environment}
						{@const env = detail.environment}
						<div class="env-grid">
							{#if env.git}
								<div class="env-card">
									<h4 class="env-card-title">🔀 {$t('experimentDetail.gitInfo')}</h4>
									<div class="env-card-body">
										{#if env.git.commit_hash}
											<div class="env-row">
												<span class="env-label">Commit</span>
												<span class="env-value env-mono">{env.git.commit_hash.substring(0, 8)}</span>
											</div>
										{/if}
										{#if env.git.branch}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.branch')}</span>
												<span class="env-value">{env.git.branch}</span>
											</div>
										{/if}
										{#if env.git.commit_message}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.commitMsg')}</span>
												<span class="env-value">{env.git.commit_message}</span>
											</div>
										{/if}
										{#if env.git.is_dirty !== null}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.workspaceStatus')}</span>
												<span class="env-value" style="color: {env.git.is_dirty ? '#f59e0b' : '#10b981'}">
													{env.git.is_dirty ? $t('experimentDetail.uncommittedChanges') : $t('experimentDetail.clean')}
												</span>
											</div>
										{/if}
										{#if env.git.remote_url}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.remoteRepo')}</span>
												<span class="env-value env-mono env-small">{env.git.remote_url}</span>
											</div>
										{/if}
									</div>
								</div>
							{/if}
							{#if env.dependencies}
								<div class="env-card">
									<h4 class="env-card-title">📚 {$t('experimentDetail.dependencyInfo')}</h4>
									<div class="env-card-body">
										{#if env.dependencies.rust_version}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.rustVersion')}</span>
												<span class="env-value env-mono">{env.dependencies.rust_version}</span>
											</div>
										{/if}
										{#if env.dependencies.burn_version}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.burnVersion')}</span>
												<span class="env-value env-mono">{env.dependencies.burn_version}</span>
											</div>
										{/if}
										{#if Object.keys(env.dependencies.crates).length > 0}
											<div class="env-row env-row-stack">
												<span class="env-label">{$t('experimentDetail.keyDependencies')}</span>
												<div class="env-deps">
													{#each Object.entries(env.dependencies.crates) as [name, version]}
														<span class="dep-chip">{name}@{version}</span>
													{/each}
												</div>
											</div>
										{/if}
									</div>
								</div>
							{/if}
							{#if env.system}
								<div class="env-card">
									<h4 class="env-card-title">💻 {$t('experimentDetail.systemInfo')}</h4>
									<div class="env-card-body">
										<div class="env-row">
											<span class="env-label">{$t('experimentDetail.os')}</span>
											<span class="env-value">{env.system.os}</span>
										</div>
										{#if env.system.os_version}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.osVersion')}</span>
												<span class="env-value">{env.system.os_version}</span>
											</div>
										{/if}
										<div class="env-row">
											<span class="env-label">{$t('experimentDetail.cpuCores')}</span>
											<span class="env-value">{env.system.cpu_cores}</span>
										</div>
										<div class="env-row">
											<span class="env-label">{$t('experimentDetail.totalMemory')}</span>
											<span class="env-value">{(env.system.total_memory_mb / 1024).toFixed(1)} GB</span>
										</div>
										{#if env.system.hostname}
											<div class="env-row">
												<span class="env-label">{$t('experimentDetail.hostname')}</span>
												<span class="env-value">{env.system.hostname}</span>
											</div>
										{/if}
									</div>
								</div>
							{/if}
						</div>
						<div class="env-captured-at">
							{$t('experimentDetail.envCapturedAt')}: {formatTime(env.captured_at)}
						</div>
					{:else}
						<div class="empty-metrics">
							<p>{$t('experimentDetail.noEnvInfo')}</p>
							<p class="empty-hint">{$t('experimentDetail.noEnvInfoHint')}</p>
						</div>
					{/if}
				</div>
			{:else if activeTab === 'inference'}
				<div class="inference-section">
					<h3 class="subsection-title">{$t('experimentDetail.modelInference')}</h3>
					<p class="inference-hint">{$t('experimentDetail.inferenceHint')}</p>

					<div class="inference-form">
						<textarea
							bind:value={inferenceInput}
							placeholder={$t('experimentDetail.inferencePlaceholder')}
							class="inference-textarea"
							rows="3"
						></textarea>
						<button
							class="btn-inference"
							on:click={runInference}
							disabled={inferenceRunning || !inferenceInput.trim()}
						>
							{inferenceRunning ? $t('experimentDetail.inferring') : `🔮 ${$t('experimentDetail.runInference')}`}
						</button>
					</div>

					{#if inferenceError}
						<div class="error-banner">
							<span class="error-icon">✗</span>
							<span>{inferenceError}</span>
						</div>
					{/if}

					{#if inferenceResult}
						<div class="inference-result">
							<h4>{$t('experimentDetail.inferenceResult')}</h4>
							{#if inferenceResult.predicted_classes.length > 0}
								<div class="result-card">
									<span class="result-label">{$t('experimentDetail.predictedClass')}</span>
									<span class="result-value highlight">{inferenceResult.predicted_classes[0]}</span>
								</div>
							{/if}
							{#if inferenceResult.predictions.length > 0}
								<div class="result-card">
									<span class="result-label">{$t('experimentDetail.predictedValue')}</span>
									<span class="result-value">{inferenceResult.predictions.map(v => v.toFixed(4)).join(', ')}</span>
								</div>
							{/if}
							{#if inferenceResult.probabilities.length > 0 && inferenceResult.probabilities[0].length > 0}
								<div class="result-card">
									<span class="result-label">{$t('experimentDetail.classProbabilities')}</span>
									<div class="prob-bar-container">
										{#each inferenceResult.probabilities[0] as prob, i}
											<div class="prob-bar-item">
												<span class="prob-class">{$t('experimentDetail.classLabel')} {i}</span>
												<div class="prob-bar-track">
													<div class="prob-bar-fill" style="width: {prob * 100}%"></div>
												</div>
												<span class="prob-value">{(prob * 100).toFixed(1)}%</span>
											</div>
										{/each}
									</div>
								</div>
							{/if}
						</div>
					{/if}

					{#if evalHistory.length > 0}
						<div class="eval-history">
							<h5>{$t('experimentDetail.evalHistory')}</h5>
							<div class="eval-history-list">
								{#each evalHistory as ev, idx}
									<div class="eval-history-item">
										<div class="eval-history-header">
											<span class="eval-history-time">{ev.created_at ? new Date(ev.created_at).toLocaleString() : `${$t('experimentDetail.eval')} #${idx + 1}`}</span>
											<span class="eval-history-desc">{ev.description || ''}</span>
										</div>
										{#if ev.result}
											{#if ev.result.task_type === 'classification'}
												<span class="eval-history-badge">{$t('experimentDetail.accuracy')}: {(ev.result.accuracy * 100).toFixed(2)}%</span>
											{:else}
												<span class="eval-history-badge">RMSE: {ev.result.rmse?.toFixed(4) || 'N/A'}</span>
											{/if}
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<hr class="section-divider" />

					<h3 class="subsection-title">{$t('experimentDetail.modelEval')}</h3>
					<p class="inference-hint">{$t('experimentDetail.evalHint')}</p>

					<div class="eval-form">
						<div class="file-select-row">
							<input
								type="text"
								bind:value={evalTestDataPath}
								placeholder={$t('experimentDetail.selectTestDataPath')}
								class="file-input"
								readonly
							/>
							<button class="btn-browse" on:click={selectTestDataFile}>{$t('experimentDetail.browse')}</button>
						</div>
						<button
							class="btn-inference"
							on:click={runEvaluation}
							disabled={evalRunning || !evalTestDataPath.trim()}
						>
							{evalRunning ? $t('experimentDetail.evaluating') : `📊 ${$t('experimentDetail.runEval')}`}
						</button>
					</div>

					{#if evalError}
						<div class="error-banner">
							<span class="error-icon">✗</span>
							<span>{evalError}</span>
						</div>
					{/if}

					{#if evalResult}
						<div class="eval-result">
							<h4>{$t('experimentDetail.evalResult')}</h4>
							{#if evalResult.task_type === 'classification'}
								<div class="eval-summary-grid">
									<div class="eval-metric-card">
										<span class="eval-metric-label">{$t('experimentDetail.totalSamples')}</span>
										<span class="eval-metric-value">{evalResult.total_samples}</span>
									</div>
									<div class="eval-metric-card highlight">
										<span class="eval-metric-label">{$t('experimentDetail.accuracy')}</span>
										<span class="eval-metric-value">{(evalResult.accuracy * 100).toFixed(2)}%</span>
									</div>
									{#if evalResult.macro_f1 !== undefined}
										<div class="eval-metric-card">
											<span class="eval-metric-label">{$t('experimentDetail.macroF1')}</span>
											<span class="eval-metric-value">{(evalResult.macro_f1 * 100).toFixed(2)}%</span>
										</div>
									{/if}
									{#if evalResult.macro_precision !== undefined}
										<div class="eval-metric-card">
											<span class="eval-metric-label">{$t('experimentDetail.macroPrecision')}</span>
											<span class="eval-metric-value">{(evalResult.macro_precision * 100).toFixed(2)}%</span>
										</div>
									{/if}
									{#if evalResult.macro_recall !== undefined}
										<div class="eval-metric-card">
											<span class="eval-metric-label">{$t('experimentDetail.macroRecall')}</span>
											<span class="eval-metric-value">{(evalResult.macro_recall * 100).toFixed(2)}%</span>
										</div>
									{/if}
								</div>

								{#if evalResult.confusion_matrix.length > 0}
									{@const cmMax = Math.max(...evalResult.confusion_matrix.flat())}
									<div class="confusion-matrix-section">
										<h5>{$t('experimentDetail.confusionMatrix')}</h5>
										<div class="confusion-matrix">
											<div class="cm-header">
												<div class="cm-cell cm-corner"></div>
												{#each evalResult.confusion_matrix[0] as _, j}
													<div class="cm-cell cm-col-header">{$t('experimentDetail.predicted')} {j}</div>
												{/each}
											</div>
											{#each evalResult.confusion_matrix as row, i}
												<div class="cm-row">
													<div class="cm-cell cm-row-header">{$t('experimentDetail.actual')} {i}</div>
													{#each row as val, j}
														{@const intensity = cmMax > 0 ? val / cmMax : 0}
														<div
															class="cm-cell cm-value cm-heatmap"
															class:cm-diagonal={i === j}
															style="background: rgba(16, 185, 129, {i === j ? intensity * 0.6 : intensity * 0.35}); color: {intensity > 0.5 ? '#fff' : '#e5e7eb'}"
														>
															{val}
														</div>
													{/each}
												</div>
											{/each}
										</div>
									</div>
								{/if}

								{#if evalResult.class_metrics.length > 0}
									<div class="class-metrics-section">
										<h5>{$t('experimentDetail.classificationReport')}</h5>
										<div class="class-metrics-table">
											<div class="cm-table-header">
												<span>{$t('experimentDetail.classLabel')}</span>
												<span>{$t('experimentDetail.precision')}</span>
												<span>{$t('experimentDetail.recall')}</span>
												<span>F1</span>
												<span>{$t('experimentDetail.sampleCount')}</span>
											</div>
											{#each evalResult.class_metrics as cm}
												<div class="cm-table-row">
													<span>{cm.class}</span>
													<span>{(cm.precision * 100).toFixed(2)}%</span>
													<span>{(cm.recall * 100).toFixed(2)}%</span>
													<span>{(cm.f1_score * 100).toFixed(2)}%</span>
													<span>{cm.support}</span>
												</div>
											{/each}
										</div>
									</div>
								{/if}
							{:else if evalResult.task_type === 'regression'}
								<div class="eval-summary-grid">
									<div class="eval-metric-card">
										<span class="eval-metric-label">{$t('experimentDetail.totalSamples')}</span>
										<span class="eval-metric-value">{evalResult.total_samples}</span>
									</div>
									<div class="eval-metric-card">
										<span class="eval-metric-label">MSE</span>
										<span class="eval-metric-value">{evalResult.mse.toFixed(4)}</span>
									</div>
									<div class="eval-metric-card">
										<span class="eval-metric-label">RMSE</span>
										<span class="eval-metric-value">{evalResult.rmse.toFixed(4)}</span>
									</div>
									<div class="eval-metric-card highlight">
										<span class="eval-metric-label">MAE</span>
										<span class="eval-metric-value">{evalResult.mae.toFixed(4)}</span>
									</div>
									{#if evalResult.r_squared !== undefined}
										<div class="eval-metric-card">
											<span class="eval-metric-label">R²</span>
											<span class="eval-metric-value">{evalResult.r_squared.toFixed(4)}</span>
										</div>
									{/if}
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		<div class="not-found">
			<p>{$t('experimentDetail.experimentNotFound')}</p>
			<a href="/lab" class="back-link">← {$t('experimentDetail.backToExperiments')}</a>
		</div>
	{/if}
</div>

{#if showRegisterModal}
	<div class="modal-overlay" role="presentation" on:click|self={() => showRegisterModal = false} on:keydown={(e) => { if (e.key === 'Escape') showRegisterModal = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1">
			<h3>{$t('experimentDetail.registerModel')}</h3>
			<div class="form-group">
				<label for="reg-model-name">{$t('experimentDetail.modelName')}</label>
				<input id="reg-model-name" type="text" bind:value={registerName} placeholder={$t('experimentDetail.modelNamePlaceholder')} class="form-input" />
			</div>
			<div class="form-group">
				<label for="reg-model-ver">{$t('experimentDetail.version')}</label>
				<input id="reg-model-ver" type="text" bind:value={registerVersion} placeholder={$t('experimentDetail.versionPlaceholder')} class="form-input" />
			</div>
			{#if registerError}
				<div class="form-error">{registerError}</div>
			{/if}
			<div class="modal-actions">
				<button class="btn-cancel" on:click={() => showRegisterModal = false}>{$t('experimentDetail.cancel')}</button>
				<button class="btn-submit" on:click={registerModel} disabled={registering}>
					{registering ? $t('experimentDetail.registering') : $t('experimentDetail.register')}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if showCheckpointResumeModal}
	<div class="modal-overlay" role="presentation" on:click|self={() => showCheckpointResumeModal = false} on:keydown={(e) => { if (e.key === 'Escape') showCheckpointResumeModal = false; }}>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1">
			<p class="modal-hint">{$t('experimentDetail.checkpointResumeHint')}</p>
			{#if checkpointLoading}
				<p class="empty-hint">{$t('experimentDetail.loadingCheckpoints')}</p>
			{:else if checkpointList.length === 0}
				<p class="empty-hint">{$t('experimentDetail.noCheckpoints')}</p>
			{:else}
				<div class="checkpoint-list">
					{#each checkpointList as cp}
						<div class="checkpoint-item" class:selected={selectedCheckpointEpoch === cp.epoch} role="option" tabindex="0" aria-selected={selectedCheckpointEpoch === cp.epoch} on:click={() => selectedCheckpointEpoch = cp.epoch} on:keydown={(e) => e.key === 'Enter' && (selectedCheckpointEpoch = cp.epoch)}>
							<div class="checkpoint-info">
								<span class="checkpoint-name">{cp.name}</span>
								<span class="checkpoint-meta">Epoch {cp.epoch} · {(cp.size_bytes / 1024 / 1024).toFixed(2)} MB</span>
							</div>
							<div class="checkpoint-radio">
								{#if selectedCheckpointEpoch === cp.epoch}
									<span class="radio-selected">●</span>
								{:else}
									<span class="radio-unselected">○</span>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}
			<div class="modal-actions">
				<button class="btn-cancel" on:click={() => showCheckpointResumeModal = false}>{$t('experimentDetail.cancel')}</button>
				<button class="btn-submit" on:click={resumeFromCheckpoint} disabled={checkpointResuming || selectedCheckpointEpoch === null}>
					{checkpointResuming ? $t('experimentDetail.resuming') : `${$t('experimentDetail.resumeFromEpoch')} ${selectedCheckpointEpoch ?? '-'}`}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if artifactPreviewPath}
	<div class="modal-overlay" role="presentation" on:click|self={closeArtifactPreview} on:keydown={(e) => { if (e.key === 'Escape') closeArtifactPreview(); }}>
		<div class="preview-modal" role="dialog" aria-modal="true" tabindex="-1">
			<div class="preview-header">
				<h3>{artifactPreviewPath.split('/').pop() || artifactPreviewPath}</h3>
				<button class="btn-close-preview" on:click={closeArtifactPreview}>✕</button>
			</div>
			<div class="preview-body">
				{#if artifactPreviewLoading}
					<div class="loading-state">
						<div class="spinner"></div>
						<p>{$t('experimentDetail.loadingFileContent')}</p>
					</div>
				{:else if artifactPreviewError}
					<div class="form-error">{artifactPreviewError}</div>
				{:else if artifactPreviewContent}
					<pre class="preview-content">{artifactPreviewContent}</pre>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.experiment-detail {
		max-width: 1100px;
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

	.back-link {
		color: #10b981;
		text-decoration: none;
		font-size: 0.9rem;
		margin-bottom: 1rem;
		display: inline-block;
	}

	.back-link:hover {
		text-decoration: underline;
	}

	.detail-header {
		margin-bottom: 1.5rem;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.header-right {
		display: flex;
		align-items: center;
	}

	.stop-btn {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.stop-btn:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	.pause-btn {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
		border: 1px solid rgba(245, 158, 11, 0.3);
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.pause-btn:hover {
		background: rgba(245, 158, 11, 0.25);
	}

	.resume-btn {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.resume-btn:hover {
		background: rgba(16, 185, 129, 0.25);
	}

	.paused-banner {
		background: rgba(245, 158, 11, 0.1);
		border: 1px solid rgba(245, 158, 11, 0.3);
		color: #f59e0b;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		text-align: center;
		margin-bottom: 1rem;
		font-size: 0.9rem;
	}

	.register-btn {
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		margin-left: 0.75rem;
	}

	.register-btn:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.archive-btn {
		background: linear-gradient(135deg, #6b7280, #4b5563);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		margin-left: 0.75rem;
	}

	.archive-btn:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(107, 114, 128, 0.3);
	}

	.restore-btn {
		background: linear-gradient(135deg, #8b5cf6, #7c3aed);
		color: white;
		border: none;
		border-radius: 8px;
		padding: 0.5rem 1.25rem;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		margin-left: 0.75rem;
	}

	.restore-btn:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
	}

	.exp-title {
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 0.5rem;
	}

	.exp-meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.description-section {
		margin-bottom: 0.75rem;
	}

	.description-text {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		line-height: 1.5;
		margin-bottom: 0.4rem;
	}

	.desc-textarea {
		width: 100%;
		min-height: 80px;
		padding: 0.5rem;
		border-radius: 6px;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-secondary, #1f2937);
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		resize: vertical;
		margin-bottom: 0.4rem;
	}

	.desc-actions {
		display: flex;
		gap: 0.4rem;
	}

	.btn-sm {
		padding: 0.25rem 0.6rem;
		border-radius: 4px;
		font-size: 0.8rem;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-secondary, #1f2937);
		color: var(--text-primary, #e5e7eb);
		cursor: pointer;
	}

	.btn-save {
		background: #3b82f6;
		border-color: #3b82f6;
		color: #fff;
	}

	.btn-cancel {
		background: transparent;
		border-color: var(--border-color, #374151);
		color: var(--text-secondary, #9ca3af);
	}

	.btn-edit {
		background: transparent;
		border-color: var(--border-color, #374151);
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
		cursor: pointer;
	}

	.btn-edit:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
		font-size: 0.85rem;
		font-weight: 500;
		border-width: 1px;
		border-style: solid;
	}

	.heartbeat-indicator {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: 0.78rem;
		color: #10b981;
		padding: 0.15rem 0.5rem;
		background: rgba(16, 185, 129, 0.08);
		border-radius: 4px;
	}

	.heartbeat-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #10b981;
		animation: heartbeat-pulse 2s infinite;
	}

	.heartbeat-stale {
		color: #f59e0b;
		background: rgba(245, 158, 11, 0.08);
	}
	.heartbeat-stale .heartbeat-dot {
		background: #f59e0b;
		animation: none;
	}

	.heartbeat-dead {
		color: #ef4444;
		background: rgba(239, 68, 68, 0.08);
	}
	.heartbeat-dead .heartbeat-dot {
		background: #ef4444;
		animation: none;
	}

	@keyframes heartbeat-pulse {
		0%, 100% { opacity: 1; transform: scale(1); }
		50% { opacity: 0.4; transform: scale(0.8); }
	}

	.meta-item {
		color: var(--text-secondary, #6b7280);
		font-size: 0.85rem;
	}

	.tags {
		display: flex;
		gap: 0.4rem;
		flex-wrap: wrap;
	}

	.tag {
		background: rgba(255, 255, 255, 0.06);
		color: var(--text-secondary, #9ca3af);
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
	}

	.error-banner {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: #fca5a5;
		margin-bottom: 1.5rem;
	}

	.error-icon {
		color: #ef4444;
		font-weight: bold;
	}

	.progress-section {
		background: rgba(16, 185, 129, 0.04);
		border: 1px solid rgba(16, 185, 129, 0.15);
		border-radius: 10px;
		padding: 1rem 1.25rem;
		margin-bottom: 1.5rem;
	}

	.tab-bar {
		display: flex;
		gap: 0.25rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		margin-bottom: 1.5rem;
	}

	.tab-btn {
		background: none;
		border: none;
		color: var(--text-secondary, #9ca3af);
		padding: 0.75rem 1.25rem;
		font-size: 0.9rem;
		cursor: pointer;
		border-bottom: 2px solid transparent;
		transition: all 0.2s;
	}

	.tab-btn:hover {
		color: var(--text-primary, #e5e7eb);
	}

	.tab-btn.active {
		color: #10b981;
		border-bottom-color: #10b981;
	}

	.tab-content {
		min-height: 300px;
	}

	.empty-metrics {
		text-align: center;
		padding: 3rem;
		color: var(--text-secondary, #6b7280);
	}

	.metrics-section {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.metrics-controls {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		padding: 0.75rem 1rem;
		background: var(--bg-secondary, #1e293b);
		border-radius: 8px;
		border: 1px solid var(--border-color, #334155);
		flex-wrap: wrap;
	}

	.control-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
		color: var(--text-secondary, #94a3b8);
	}

	.control-item input[type="range"] {
		width: 100px;
		accent-color: var(--accent, #3b82f6);
	}

	.control-value {
		min-width: 2.5rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.subsection-title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 0.75rem;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.metric-card {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.metric-name {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
		font-family: monospace;
	}

	.metric-value {
		color: var(--text-primary, #e5e7eb);
		font-size: 1.5rem;
		font-weight: 600;
		font-family: monospace;
	}

	.metric-stats {
		display: flex;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.metric-stat {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
		font-family: monospace;
	}

	.metric-stat.best {
		color: var(--accent, #3b82f6);
		font-weight: 600;
	}

	.trend {
		font-weight: 600;
		font-size: 0.75rem;
	}

	.trend-up {
		color: #ef4444;
	}

	.trend-down {
		color: #22c55e;
	}

	.metric-info {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
	}

	.params-table {
		display: flex;
		flex-direction: column;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		overflow: hidden;
	}

	.param-row {
		display: flex;
		justify-content: space-between;
		padding: 0.6rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		align-items: center;
	}

	.param-row:last-child {
		border-bottom: none;
	}

	.param-key {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		font-family: monospace;
		min-width: 150px;
	}

	.param-value {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.85rem;
		font-family: monospace;
		text-align: right;
		word-break: break-all;
	}

	.artifacts-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.artifacts-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.artifacts-toolbar-left {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.btn-scan {
		padding: 0.4rem 0.8rem;
		background: var(--surface-2, #1e293b);
		border: 1px solid var(--border, #334155);
		border-radius: 6px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.8rem;
		cursor: pointer;
		transition: background 0.2s;
	}

	.btn-scan:hover:not(:disabled) {
		background: var(--surface-3, #0f172a);
	}

	.btn-scan:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.artifacts-count {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		font-weight: 500;
	}

	.artifacts-total-size {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
	}

	.artifacts-tree {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.artifact-group {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 10px;
		overflow: hidden;
	}

	.artifact-group-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: rgba(255, 255, 255, 0.03);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.group-icon {
		font-size: 1rem;
	}

	.group-name {
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		font-weight: 600;
		flex: 1;
	}

	.group-count {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
		padding: 0.1rem 0.5rem;
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.artifact-group-items {
		display: flex;
		flex-direction: column;
	}

	.artifact-card {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.6rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.03);
		cursor: pointer;
		transition: background 0.15s;
	}

	.artifact-card:last-child {
		border-bottom: none;
	}

	.artifact-card:hover {
		background: rgba(16, 185, 129, 0.04);
	}

	.artifact-actions {
		flex-shrink: 0;
		display: flex;
		gap: 0.3rem;
	}

	.btn-artifact-action {
		padding: 0.25rem 0.4rem;
		background: transparent;
		border: 1px solid transparent;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		opacity: 0.5;
		transition: opacity 0.15s, background 0.15s;
	}

	.artifact-card:hover .btn-artifact-action {
		opacity: 1;
	}

	.btn-artifact-action:hover {
		background: var(--surface-2, #1e293b);
		border-color: var(--border, #334155);
	}

	.artifact-icon {
		font-size: 1.1rem;
		flex-shrink: 0;
	}

	.artifact-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.artifact-dir {
		color: var(--text-secondary, #6b7280);
		font-size: 0.75rem;
		font-family: monospace;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.artifact-path {
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
		font-size: 0.85rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.artifact-meta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 0.1rem;
		flex-shrink: 0;
	}

	.artifact-size {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.8rem;
	}

	.artifact-time {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
	}

	.environment-section {
		padding: 0;
	}

	.env-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
		gap: 1.25rem;
		margin-bottom: 1rem;
	}

	.env-card {
		background: var(--surface-2, #1e293b);
		border: 1px solid var(--border, #334155);
		border-radius: 10px;
		overflow: hidden;
	}

	.env-card-title {
		margin: 0;
		padding: 0.75rem 1rem;
		font-size: 0.95rem;
		font-weight: 600;
		background: var(--surface-3, #0f172a);
		border-bottom: 1px solid var(--border, #334155);
	}

	.env-card-body {
		padding: 0.75rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.env-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
	}

	.env-row-stack {
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.env-label {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		white-space: nowrap;
		flex-shrink: 0;
		min-width: 80px;
	}

	.env-value {
		color: var(--text-primary, #f1f5f9);
		font-size: 0.85rem;
		text-align: right;
		word-break: break-all;
	}

	.env-mono {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: 0.8rem;
	}

	.env-small {
		font-size: 0.75rem;
	}

	.env-deps {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}

	.dep-chip {
		display: inline-block;
		padding: 0.2rem 0.5rem;
		background: var(--surface-1, #0f172a);
		border: 1px solid var(--border, #334155);
		border-radius: 4px;
		font-size: 0.75rem;
		font-family: 'SF Mono', 'Fira Code', monospace;
		color: var(--text-secondary, #9ca3af);
	}

	.env-captured-at {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		text-align: right;
		padding-top: 0.5rem;
		border-top: 1px solid var(--border, #334155);
	}

	.not-found {
		text-align: center;
		padding: 4rem;
		color: var(--text-secondary, #9ca3af);
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		backdrop-filter: blur(4px);
	}

	.inference-hint {
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 1rem;
	}

	.inference-form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.inference-textarea {
		width: 100%;
		padding: 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
		font-size: 0.9rem;
		resize: vertical;
	}

	.inference-textarea:focus {
		outline: none;
		border-color: #10b981;
	}

	.btn-inference {
		align-self: flex-start;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, #8b5cf6, #6d28d9);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.9rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-inference:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
	}

	.btn-inference:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.inference-result {
		margin-top: 1.5rem;
		padding: 1rem;
		background: rgba(139, 92, 246, 0.05);
		border: 1px solid rgba(139, 92, 246, 0.2);
		border-radius: 10px;
	}

	.inference-result h4 {
		font-size: 1rem;
		color: #8b5cf6;
		margin-bottom: 0.75rem;
	}

	.batch-results-table {
		width: 100%;
		font-size: 0.85rem;
	}

	.batch-table-header {
		display: grid;
		grid-template-columns: 40px 1fr 100px 1fr;
		gap: 0.5rem;
		padding: 0.5rem;
		background: rgba(139, 92, 246, 0.1);
		border-radius: 6px 6px 0 0;
		font-weight: 600;
		color: #8b5cf6;
	}

	.batch-table-row {
		display: grid;
		grid-template-columns: 40px 1fr 100px 1fr;
		gap: 0.5rem;
		padding: 0.4rem 0.5rem;
		border-bottom: 1px solid rgba(139, 92, 246, 0.1);
	}

	.batch-table-row:hover {
		background: rgba(139, 92, 246, 0.05);
	}

	.batch-input {
		font-family: monospace;
		font-size: 0.8rem;
		color: #94a3b8;
	}

	.batch-predicted {
		font-weight: 600;
		color: #8b5cf6;
	}

	.batch-value {
		font-family: monospace;
		font-size: 0.8rem;
	}

	.result-card {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		padding: 0.75rem;
		margin-bottom: 0.5rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 8px;
	}

	.result-label {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
	}

	.result-value {
		font-size: 1rem;
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
	}

	.result-value.highlight {
		font-size: 1.5rem;
		font-weight: 700;
		color: #8b5cf6;
	}

	.prob-bar-container {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.prob-bar-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.prob-class {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		min-width: 60px;
	}

	.prob-bar-track {
		flex: 1;
		height: 8px;
		background: rgba(255, 255, 255, 0.05);
		border-radius: 4px;
		overflow: hidden;
	}

	.prob-bar-fill {
		height: 100%;
		background: linear-gradient(90deg, #8b5cf6, #6d28d9);
		border-radius: 4px;
		transition: width 0.3s ease;
	}

	.prob-value {
		font-size: 0.8rem;
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
		min-width: 50px;
		text-align: right;
	}

	.section-divider {
		border: none;
		border-top: 1px solid var(--border-color, #374151);
		margin: 1.5rem 0;
	}

	.eval-form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.file-select-row {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.file-input {
		flex: 1;
		padding: 0.5rem 0.75rem;
		border-radius: 6px;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-secondary, #1f2937);
		color: var(--text-primary, #e5e7eb);
		font-size: 0.85rem;
	}

	.btn-browse {
		padding: 0.5rem 1rem;
		border-radius: 6px;
		border: 1px solid var(--border-color, #374151);
		background: var(--bg-secondary, #1f2937);
		color: var(--text-primary, #e5e7eb);
		cursor: pointer;
		font-size: 0.85rem;
		white-space: nowrap;
	}

	.btn-browse:hover {
		border-color: #3b82f6;
		color: #3b82f6;
	}

	.eval-result {
		margin-top: 1.5rem;
		padding: 1rem;
		background: rgba(16, 185, 129, 0.05);
		border: 1px solid rgba(16, 185, 129, 0.2);
		border-radius: 10px;
	}

	.eval-result h4 {
		font-size: 1rem;
		color: #10b981;
		margin-bottom: 0.75rem;
	}

	.eval-result h5 {
		font-size: 0.9rem;
		color: var(--text-primary, #e5e7eb);
		margin: 1rem 0 0.5rem;
	}

	.eval-summary-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.eval-metric-card {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border-radius: 8px;
		border: 1px solid transparent;
	}

	.eval-metric-card.highlight {
		border-color: rgba(16, 185, 129, 0.3);
		background: rgba(16, 185, 129, 0.08);
	}

	.eval-metric-label {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
	}

	.eval-metric-value {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
	}

	.confusion-matrix-section {
		margin-top: 1rem;
	}

	.confusion-matrix {
		display: inline-block;
		border: 1px solid var(--border-color, #374151);
		border-radius: 6px;
		overflow: hidden;
	}

	.cm-header, .cm-row {
		display: flex;
	}

	.cm-cell {
		min-width: 48px;
		padding: 0.35rem 0.5rem;
		text-align: center;
		font-size: 0.8rem;
		border: 1px solid var(--border-color, #374151);
	}

	.cm-corner {
		background: var(--bg-secondary, #1f2937);
	}

	.cm-col-header, .cm-row-header {
		background: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
		font-weight: 500;
	}

	.cm-value {
		color: var(--text-primary, #e5e7eb);
		transition: all 0.2s;
	}

	.cm-heatmap {
		border-radius: 3px;
		min-width: 36px;
		text-align: center;
	}

	.cm-diagonal {
		font-weight: 600;
	}

	.eval-history {
		margin-top: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.eval-history h5 {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		margin-bottom: 0.5rem;
	}

	.eval-history-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.eval-history-item {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 8px;
		padding: 0.6rem 0.8rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.eval-history-header {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.eval-history-time {
		font-size: 0.8rem;
		color: var(--text-primary, #e5e7eb);
		font-family: monospace;
	}

	.eval-history-desc {
		font-size: 0.75rem;
		color: var(--text-secondary, #9ca3af);
	}

	.eval-history-badge {
		font-size: 0.8rem;
		color: #10b981;
		font-family: monospace;
		background: rgba(16, 185, 129, 0.1);
		padding: 0.2rem 0.5rem;
		border-radius: 4px;
	}

	.class-metrics-section {
		margin-top: 1rem;
	}

	.class-metrics-table {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border-color, #374151);
		border-radius: 6px;
		overflow: hidden;
	}

	.cm-table-header, .cm-table-row {
		display: grid;
		grid-template-columns: 60px 1fr 1fr 1fr 60px;
		gap: 0;
	}

	.cm-table-header span, .cm-table-row span {
		padding: 0.4rem 0.5rem;
		font-size: 0.8rem;
		text-align: center;
		border-bottom: 1px solid var(--border-color, #374151);
	}

	.cm-table-header span {
		background: rgba(59, 130, 246, 0.1);
		color: #3b82f6;
		font-weight: 500;
	}

	.cm-table-row span {
		color: var(--text-primary, #e5e7eb);
	}

	.modal {
		background: linear-gradient(135deg, #1a1a2e, #16213e);
		border: 1px solid rgba(16, 185, 129, 0.2);
		border-radius: 16px;
		padding: 2rem;
		width: 90%;
		max-width: 450px;
	}

	.modal h3 {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1.5rem;
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	.form-group label {
		display: block;
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.4rem;
	}

	.form-input {
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		outline: none;
		transition: border-color 0.2s;
	}

	.form-input:focus {
		border-color: #10b981;
	}

	.form-error {
		padding: 0.5rem 0.75rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 6px;
		color: #ef4444;
		font-size: 0.85rem;
		margin-bottom: 1rem;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		margin-top: 1.5rem;
	}

	.modal-hint {
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 1rem;
	}

	.checkpoint-list {
		max-height: 300px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.checkpoint-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.checkpoint-item:hover {
		border-color: rgba(16, 185, 129, 0.4);
		background: rgba(16, 185, 129, 0.05);
	}

	.checkpoint-item.selected {
		border-color: #10b981;
		background: rgba(16, 185, 129, 0.1);
	}

	.checkpoint-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.checkpoint-name {
		font-size: 0.9rem;
		color: var(--text-primary, #e5e7eb);
		font-weight: 500;
	}

	.checkpoint-meta {
		font-size: 0.75rem;
		color: var(--text-secondary, #9ca3af);
	}

	.checkpoint-radio {
		font-size: 1.2rem;
	}

	.radio-selected {
		color: #10b981;
	}

	.radio-unselected {
		color: var(--text-muted, #6b7280);
	}

	.btn-cancel {
		padding: 0.5rem 1.25rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		cursor: pointer;
		font-size: 0.9rem;
		transition: all 0.2s;
	}

	.btn-cancel:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.btn-submit {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #10b981, #059669);
		border: none;
		border-radius: 8px;
		color: white;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 500;
		transition: all 0.2s;
	}

	.btn-submit:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.btn-submit:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.params-subsection {
		margin-bottom: 1.5rem;
	}

	.tags-management {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.tags-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.add-tag-form,
	.add-param-form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.input-sm {
		padding: 0.4rem 0.6rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 6px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.85rem;
	}

	.input-sm:focus {
		outline: none;
		border-color: #10b981;
	}

	.btn-sm {
		padding: 0.4rem 0.8rem;
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 6px;
		color: #10b981;
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.btn-sm:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.25);
		border-color: #10b981;
	}

	.btn-sm:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.empty-hint {
		color: var(--text-muted, #6b7280);
		font-size: 0.85rem;
	}

	.logs-section {
		padding: 0;
	}

	.logs-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 0;
		margin-bottom: 0.5rem;
		border-bottom: 1px solid var(--border-color, #e5e7eb);
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.logs-filters {
		display: flex;
		gap: 0.25rem;
	}

	.logs-filters .btn-small.active {
		background: var(--primary-color, #3b82f6);
		color: #fff;
		border-color: var(--primary-color, #3b82f6);
	}

	.auto-scroll-label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-muted, #6b7280);
		cursor: pointer;
	}

	.auto-scroll-label input {
		margin: 0;
	}

	.logs-count {
		font-size: 0.8rem;
		color: var(--text-muted, #6b7280);
	}

	.btn-small {
		font-size: 0.75rem;
		padding: 0.25rem 0.75rem;
		border: 1px solid var(--border-color, #e5e7eb);
		border-radius: 4px;
		background: var(--bg-secondary, #f9fafb);
		color: var(--text-primary, #111827);
		cursor: pointer;
	}

	.btn-small:hover {
		background: var(--bg-hover, #f3f4f6);
	}

	.logs-container {
		max-height: 500px;
		overflow-y: auto;
		font-family: 'SF Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
		font-size: 0.8rem;
		line-height: 1.6;
		background: var(--bg-secondary, #1e1e1e);
		border-radius: 6px;
		padding: 0.75rem;
	}

	.log-line {
		display: flex;
		gap: 0.5rem;
		padding: 0.1rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.log-line:last-child {
		border-bottom: none;
	}

	.log-error {
		color: #ef4444;
	}

	.log-warn {
		color: #f59e0b;
	}

	.log-info {
		color: #d1d5db;
	}

	.log-debug {
		color: #6b7280;
		font-size: 0.78rem;
	}

	.log-time {
		color: var(--text-muted, #6b7280);
		flex-shrink: 0;
		min-width: 70px;
	}

	.log-level {
		flex-shrink: 0;
		min-width: 50px;
		font-weight: 600;
	}

	.log-level.error {
		color: #ef4444;
	}

	.log-level.warn {
		color: #f59e0b;
	}

	.log-level.info {
		color: #60a5fa;
	}

	.log-msg {
		flex: 1;
		word-break: break-all;
	}

	.config-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 1.5rem;
	}

	.config-group {
		background: rgba(255, 255, 255, 0.02);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 10px;
		padding: 1rem;
	}

	.config-group-title {
		font-size: 0.85rem;
		font-weight: 600;
		color: #10b981;
		margin-bottom: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.notes-section {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.notes-hint {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
	}

	.notes-preview {
		width: 100%;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 10px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		line-height: 1.7;
		min-height: 200px;
	}

	.notes-preview :global(h2) {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 1rem 0 0.5rem;
		color: var(--text-primary, #e5e7eb);
	}

	.notes-preview :global(h3) {
		font-size: 1.1rem;
		font-weight: 600;
		margin: 0.8rem 0 0.4rem;
		color: var(--text-primary, #e5e7eb);
	}

	.notes-preview :global(h4) {
		font-size: 1rem;
		font-weight: 600;
		margin: 0.6rem 0 0.3rem;
		color: var(--text-primary, #e5e7eb);
	}

	.notes-preview :global(ul) {
		padding-left: 1.5rem;
		margin: 0.5rem 0;
	}

	.notes-preview :global(li) {
		margin: 0.2rem 0;
	}

	.notes-preview :global(code) {
		background: rgba(0, 0, 0, 0.3);
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
		font-size: 0.85rem;
	}

	.notes-preview :global(strong) {
		color: #10b981;
	}

	.btn-active {
		background: rgba(16, 185, 129, 0.2) !important;
		border-color: #10b981 !important;
	}

	.notes-editor {
		width: 100%;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 10px;
		color: var(--text-primary, #e5e7eb);
		font-family: 'SF Mono', 'Menlo', monospace;
		font-size: 0.9rem;
		line-height: 1.6;
		resize: vertical;
		min-height: 200px;
	}

	.notes-editor:focus {
		outline: none;
		border-color: #10b981;
		box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.1);
	}

	.notes-editor::placeholder {
		color: var(--text-secondary, #6b7280);
	}

	.notes-actions {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.btn-save-notes {
		padding: 0.5rem 1.25rem;
		background: linear-gradient(135deg, #10b981, #059669);
		border: none;
		border-radius: 8px;
		color: white;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-save-notes:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.btn-save-notes:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.notes-char-count {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
	}

	.preview-modal {
		background: var(--surface-2, #1e293b);
		border: 1px solid var(--border, #334155);
		border-radius: 12px;
		width: 90vw;
		max-width: 800px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.preview-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border, #334155);
		background: var(--surface-3, #0f172a);
	}

	.preview-header h3 {
		margin: 0;
		font-size: 0.95rem;
		font-family: 'SF Mono', 'Fira Code', monospace;
		color: var(--text-primary, #f1f5f9);
	}

	.btn-close-preview {
		background: transparent;
		border: none;
		color: var(--text-secondary, #9ca3af);
		font-size: 1.1rem;
		cursor: pointer;
		padding: 0.2rem 0.4rem;
		border-radius: 4px;
	}

	.btn-close-preview:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary, #f1f5f9);
	}

	.preview-body {
		flex: 1;
		overflow: auto;
		padding: 1rem;
	}

	.preview-content {
		margin: 0;
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: 0.8rem;
		line-height: 1.5;
		color: var(--text-primary, #e5e7eb);
		white-space: pre-wrap;
		word-break: break-all;
		background: var(--surface-1, #0f172a);
		padding: 1rem;
		border-radius: 8px;
		border: 1px solid var(--border, #334155);
		max-height: 60vh;
		overflow: auto;
	}
</style>
