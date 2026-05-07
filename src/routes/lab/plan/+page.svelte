<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { getLabClient } from '$lib/lab/stores/plugins';
	import { datasetRegistryStore, activeDatasets } from '$lib/lab/stores/dataset';
	import type {
		TrainingPlan, DataRecipe, PlanType, PlanPhase, QualityGate,
		DataBudget, PreprocessingConfig, ValidationConfig, ExperimentTracking,
		DedupConfig, RecipeDataset, PlanSummary, PlanValidationResult,
		DatasetSummary
	} from '$lib/lab/adapter/types';

	let currentStep = 0;
	const steps = ['基本信息', '数据配方', '训练阶段', '质量门禁', '预算配置', '确认创建'];
	let submitting = false;
	let error: string | null = null;
	let success: string | null = null;

	// Step 1: 基本信息
	let planName = '';
	let planVersion = '1.0';
	let planDescription = '';
	let planType: PlanType = 'Pretraining';
	let outputDir = './training_output';
	let planSeed = 42;

	// Step 2: 数据配方
	let recipeMode: 'preset' | 'custom' = 'preset';
	let selectedPreset = 'llm_pretraining';
	let customRecipe: DataRecipe = {
		name: 'custom_recipe',
		version: '1.0',
		description: null,
		datasets: [],
		mixing_strategy: 'Proportional',
		curriculum: null,
		dynamic_ratio: null,
		quality_thresholds: null,
		total_samples_target: null,
		seed: 42
	};
	let recipeJson = '';
	let recipeValid: boolean | null = null;
	let recipeError: string | null = null;
	let recipeLoading = false;

	// Step 3: 训练阶段
	let phases: PlanPhase[] = [{
		name: 'phase_1',
		recipe: {
			name: 'default_recipe',
			version: '1.0',
			description: null,
			datasets: [],
			mixing_strategy: 'Proportional',
			curriculum: null,
			dynamic_ratio: null,
			quality_thresholds: null,
			total_samples_target: 1000000,
			seed: 42
		},
		steps: 10000,
		batch_size: 512,
		learning_rate: 0.0003,
		warmup_steps: 500,
		weight_decay: 0.1,
		max_seq_length: 2048
	}];

	// Step 4: 质量门禁
	let qualityGates: QualityGate[] = [
		{ name: 'min_text_length', metric: 'text_length', threshold: 50, action: 'skip', phase: 'all' },
		{ name: 'dedup_check', metric: 'duplicate_ratio', threshold: 0.01, action: 'warn', phase: 'all' },
		{ name: 'lang_check', metric: 'language_score', threshold: 0.8, action: 'skip', phase: 'all' }
	];

	// Step 5: 预算配置
	let dataBudget: DataBudget = {
		max_tokens: 1000000000000,
		max_samples: 100000000,
		max_cost_usd: 10000,
		token_budget_per_phase: {}
	};

	// 预处理 & 验证
	let enablePreprocessing = true;
	let preprocessing: PreprocessingConfig = {
		tokenizer_path: '',
		max_seq_length: 2048,
		packing: true,
		add_special_tokens: true,
		truncation: 'right',
		padding: 'max_length'
	};

	let enableValidation = true;
	let validation: ValidationConfig = {
		val_split: 0.01,
		val_datasets: [],
		metrics: ['perplexity', 'accuracy'],
		eval_interval_steps: 1000
	};

	let enableTracking = true;
	let experimentTracking: ExperimentTracking = {
		project_name: '',
		tags: [],
		notes: null,
		log_interval_steps: 100
	};

	let enableDedup = true;
	let dedupConfig: DedupConfig = {
		similarity_threshold: 0.8,
		num_perm: 128,
		n_gram: 5,
		minhash_seed: 42,
		num_bands: 16
	};

	// 验证结果
	let validationResult: PlanValidationResult | null = null;
	let planSummary: PlanSummary | null = null;
	let validating = false;

	// 已注册数据集
	let registeredDatasets: DatasetSummary[] = [];
	let datasetsLoaded = false;

	let recommendLoading = false;
	let recommendResult: any = null;
	let recommendError: string | null = null;

	// 配方数据集编辑
	let newRecipeDataset: RecipeDataset = {
		name: '',
		weight: 1.0,
		source: '',
		split: 'train',
		local_path: null,
		max_samples: null,
		filters: null
	};

	// 质量门禁编辑
	let newGate: QualityGate = {
		name: '',
		metric: '',
		threshold: 0,
		action: 'warn',
		phase: 'all'
	};

	// 标签编辑
	let newTag = '';

	const planTypeLabels: Record<PlanType, string> = {
		'Pretraining': '预训练',
		'FineTuning': '微调',
		'InstructionTuning': '指令微调',
		'RLHF': 'RLHF',
		'Custom': '自定义'
	};

	const presetLabels: Record<string, string> = {
		'llm_pretraining': 'LLM 预训练',
		'sft': 'SFT 监督微调',
		'rlhf': 'RLHF 偏好对齐'
	};

	onMount(() => {
		datasetRegistryStore.fetchDatasets();
		const unsub = activeDatasets.subscribe(datasets => {
			registeredDatasets = datasets;
			datasetsLoaded = true;
		});

		const planId = $page.url.searchParams.get('id');
		if (planId) {
			(async () => {
				try {
					const client = getLabClient();
					const planJson = await client.trainingPlanLoad(planId);
					const plan: TrainingPlan = JSON.parse(planJson);
					planName = plan.name;
					planVersion = plan.version;
					planDescription = plan.description || '';
					planType = plan.plan_type;
					outputDir = plan.output_dir || './training_output';
					planSeed = plan.seed || 42;
					if (plan.phases && plan.phases.length > 0) {
						phases = plan.phases;
					}
					if (plan.quality_gates && plan.quality_gates.length > 0) {
						qualityGates = plan.quality_gates;
					}
					if (plan.data_budget) {
						dataBudget = plan.data_budget;
					}
					if (plan.preprocessing) {
						preprocessing = plan.preprocessing;
					}
					if (plan.validation) {
						validation = plan.validation;
					}
					if (plan.experiment_tracking) {
						experimentTracking = plan.experiment_tracking;
					}
					if (plan.dedup_config) {
						dedupConfig = plan.dedup_config;
					}
				} catch (e: any) {
					error = '加载训练计划失败: ' + (e?.toString() || '未知错误');
				}
			})();
		}

		return () => unsub();
	});

	function buildPlan(): TrainingPlan {
		return {
			name: planName,
			version: planVersion,
			description: planDescription || null,
			plan_type: planType,
			phases,
			data_budget: dataBudget,
			quality_gates: qualityGates,
			dedup_config: enableDedup ? dedupConfig : null,
			preprocessing: enablePreprocessing ? preprocessing : null,
			validation: enableValidation ? validation : null,
			experiment_tracking: enableTracking ? experimentTracking : null,
			output_dir: outputDir,
			seed: planSeed,
			metadata: {}
		};
	}

	async function loadPreset() {
		recipeLoading = true;
		recipeError = null;
		try {
			const client = getLabClient();
			recipeJson = await client.dataRecipePresets(selectedPreset);
			recipeValid = true;
		} catch (e: any) {
			recipeError = e?.toString() || '加载预设失败';
			recipeValid = false;
		} finally {
			recipeLoading = false;
		}
	}

	async function validateRecipe() {
		recipeLoading = true;
		recipeError = null;
		try {
			const client = getLabClient();
			const json = recipeMode === 'preset' ? recipeJson : JSON.stringify(customRecipe);
			const resultStr = await client.dataRecipeValidate(json);
			const result = JSON.parse(resultStr);
			recipeValid = result.valid;
			if (!result.valid) {
				recipeError = result.error || '配方验证失败';
			}
		} catch (e: any) {
			recipeError = e?.toString() || '验证失败';
			recipeValid = false;
		} finally {
			recipeLoading = false;
		}
	}

	async function getRecommendations() {
		recommendLoading = true;
		recommendError = null;
		recommendResult = null;
		try {
			const client = getLabClient();
			const plan = buildPlan();
			recommendResult = await client.datasetRecommendForPlan(JSON.stringify(plan));
		} catch (e: any) {
			recommendError = e?.toString() || '推荐失败';
		} finally {
			recommendLoading = false;
		}
	}

	function applyRecommendation(ds: any) {
		const existing = customRecipe.datasets.find(d => d.name === ds.dataset_name);
		if (!existing) {
			customRecipe.datasets = [...customRecipe.datasets, {
				name: ds.dataset_name,
				weight: 1.0,
				source: ds.dataset_id,
				split: 'train',
				local_path: null,
				max_samples: null,
				filters: null
			}];
		}
	}

	function getPhaseRecipe(phaseIndex: number): DataRecipe {
		if (recipeMode === 'preset' && recipeJson) {
			try {
				return JSON.parse(recipeJson);
			} catch {
				return phases[phaseIndex].recipe;
			}
		}
		return customRecipe;
	}

	function applyRecipeToPhase(phaseIndex: number) {
		phases[phaseIndex].recipe = getPhaseRecipe(phaseIndex);
		phases = phases;
	}

	function addPhase() {
		phases = [...phases, {
			name: `phase_${phases.length + 1}`,
			recipe: getPhaseRecipe(0),
			steps: 5000,
			batch_size: 512,
			learning_rate: 0.0001,
			warmup_steps: 200,
			weight_decay: 0.1,
			max_seq_length: 2048
		}];
	}

	function removePhase(index: number) {
		phases = phases.filter((_, i) => i !== index);
	}

	function addRecipeDataset() {
		if (!newRecipeDataset.name || !newRecipeDataset.source) return;
		customRecipe = {
			...customRecipe,
			datasets: [...customRecipe.datasets, { ...newRecipeDataset }]
		};
		newRecipeDataset = { name: '', weight: 1.0, source: '', split: 'train', local_path: null, max_samples: null, filters: null };
	}

	function removeRecipeDataset(index: number) {
		customRecipe = {
			...customRecipe,
			datasets: customRecipe.datasets.filter((_, i) => i !== index)
		};
	}

	function addQualityGate() {
		if (!newGate.name || !newGate.metric) return;
		qualityGates = [...qualityGates, { ...newGate }];
		newGate = { name: '', metric: '', threshold: 0, action: 'warn', phase: 'all' };
	}

	function removeQualityGate(index: number) {
		qualityGates = qualityGates.filter((_, i) => i !== index);
	}

	function addTag() {
		if (!newTag.trim()) return;
		experimentTracking = {
			...experimentTracking,
			tags: [...experimentTracking.tags, newTag.trim()]
		};
		newTag = '';
	}

	function removeTag(index: number) {
		experimentTracking = {
			...experimentTracking,
			tags: experimentTracking.tags.filter((_, i) => i !== index)
		};
	}

	async function validatePlan() {
		validating = true;
		error = null;
		validationResult = null;
		planSummary = null;
		try {
			const client = getLabClient();
			const plan = buildPlan();
			const planJson = JSON.stringify(plan);

			const [valStr, summaryStr] = await Promise.all([
				client.trainingPlanValidate(planJson),
				client.trainingPlanSummarize(planJson)
			]);

			validationResult = JSON.parse(valStr);
			planSummary = JSON.parse(summaryStr);
		} catch (e: any) {
			error = e?.toString() || '验证失败';
		} finally {
			validating = false;
		}
	}

	async function createPlan() {
		submitting = true;
		error = null;
		success = null;
		try {
			const client = getLabClient();
			const plan = buildPlan();
			const planJson = JSON.stringify(plan);

			const validationResult = await client.trainingPlanValidate(planJson);
			const parsed = JSON.parse(validationResult);
			if (!parsed.is_valid) {
				error = '计划验证失败:\n' + (parsed.errors || []).join('\n');
				submitting = false;
				return;
			}
			if (parsed.warnings && parsed.warnings.length > 0) {
				console.warn('计划警告:', parsed.warnings);
			}

			const planId = await client.trainingPlanSave(planJson);
			success = `训练计划 "${planName}" 保存成功！`;
			setTimeout(() => goto('/lab/plans'), 1500);
		} catch (e: any) {
			error = e?.toString() || '保存失败';
		} finally {
			submitting = false;
		}
	}

	function nextStep() {
		if (currentStep < steps.length - 1) {
			currentStep++;
		}
	}

	function prevStep() {
		if (currentStep > 0) {
			currentStep--;
		}
	}

	function formatTokens(n: number): string {
		if (n >= 1e12) return (n / 1e12).toFixed(1) + 'T';
		if (n >= 1e9) return (n / 1e9).toFixed(1) + 'B';
		if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
		if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
		return n.toString();
	}

	function formatCost(n: number): string {
		return '$' + n.toLocaleString();
	}
</script>

<div class="plan-page">
	<h2>训练计划配置</h2>
	<p class="subtitle">创建声明式训练计划，定义数据配方、训练阶段和质量门禁</p>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}
	{#if success}
		<div class="success-banner">{success}</div>
	{/if}

	<!-- 步骤指示器 -->
	<div class="step-indicator">
		{#each steps as step, i}
			<div class="step-item" class:active={i === currentStep} class:completed={i < currentStep}>
				<div class="step-circle">{i < currentStep ? '✓' : i + 1}</div>
				<span class="step-label">{step}</span>
				{#if i < steps.length - 1}
					<div class="step-line" class:filled={i < currentStep}></div>
				{/if}
			</div>
		{/each}
	</div>

	<div class="step-content">
		<!-- Step 1: 基本信息 -->
		{#if currentStep === 0}
			<h3>基本信息</h3>
			<div class="form-grid">
				<div class="form-group">
					<label for="auto-f1">计划名称</label>
					<input id="auto-f1" class="input" type="text" bind:value={planName} placeholder="例如: llama3-8b-pretraining" />
				</div>
				<div class="form-group">
					<label for="auto-f2">版本</label>
					<input id="auto-f2" class="input" type="text" bind:value={planVersion} placeholder="1.0" />
				</div>
				<div class="form-group">
					<label for="auto-f3">计划类型</label>
					<select id="auto-f3" class="input" bind:value={planType}>
						{#each Object.entries(planTypeLabels) as [key, label]}
							<option value={key}>{label}</option>
						{/each}
					</select>
				</div>
				<div class="form-group">
					<label for="auto-f4">输出目录</label>
					<input id="auto-f4" class="input" type="text" bind:value={outputDir} placeholder="./training_output" />
				</div>
				<div class="form-group">
					<label for="auto-f5">随机种子</label>
					<input id="auto-f5" class="input" type="number" bind:value={planSeed} />
				</div>
			</div>
			<div class="form-group full-width">
				<label for="auto-f6">描述</label>
				<textarea id="auto-f6" class="input textarea" bind:value={planDescription} placeholder="描述训练计划的目标和范围..." rows="3"></textarea>
			</div>

		<!-- Step 2: 数据配方 -->
		{:else if currentStep === 1}
			<h3>数据配方</h3>
			<div class="form-group">
				<span class="field-label">配方模式</span>
				<div class="mode-toggle" role="group" aria-label="配方模式">
					<button class="mode-btn" class:active={recipeMode === 'preset'} on:click={() => recipeMode = 'preset'}>使用预设</button>
					<button class="mode-btn" class:active={recipeMode === 'custom'} on:click={() => recipeMode = 'custom'}>自定义配方</button>
				</div>
			</div>

			{#if recipeMode === 'preset'}
				<div class="form-group">
					<span class="field-label">选择预设配方</span>
					<div class="preset-grid">
						{#each Object.entries(presetLabels) as [key, label]}
							<button
								class="preset-card"
								class:selected={selectedPreset === key}
								on:click={() => { selectedPreset = key; loadPreset(); }}
							>
								<span class="preset-name">{label}</span>
								<span class="preset-key">{key}</span>
							</button>
						{/each}
					</div>
				</div>

				<div class="form-group">
					<span class="field-label">智能推荐</span>
					<button class="btn-recommend" on:click={getRecommendations} disabled={recommendLoading}>
						{recommendLoading ? '分析中...' : '🤖 智能推荐数据集'}
					</button>
					{#if recommendError}
						<div class="error-text">{recommendError}</div>
					{/if}
					{#if recommendResult?.recommendations?.length > 0}
						<div class="recommend-list">
							{#each recommendResult.recommendations as rec}
								<div class="recommend-item" class:excellent={rec.suitability === 'excellent'} class:good={rec.suitability === 'good'} class:fair={rec.suitability === 'fair'} class:poor={rec.suitability === 'poor'}>
									<div class="rec-header">
										<span class="rec-name">{rec.dataset_name}</span>
										<span class="rec-score">{rec.score.toFixed(0)}分</span>
										<span class="rec-level">
											{rec.suitability === 'excellent' ? '⭐ 强烈推荐' : rec.suitability === 'good' ? '👍 推荐' : rec.suitability === 'fair' ? '👌 可选' : '⚠️ 不推荐'}
										</span>
									</div>
									<div class="rec-reasons">
										{#each rec.reasons as reason}
											<div class="rec-reason">• {reason}</div>
										{/each}
									</div>
									<div class="rec-match">
										<span class="match-item">类型: {rec.match_details.type_match.toFixed(0)}%</span>
										<span class="match-item">规模: {rec.match_details.size_match.toFixed(0)}%</span>
										<span class="match-item">质量: {rec.match_details.quality_match.toFixed(0)}%</span>
										<span class="match-item">特征: {rec.match_details.feature_match.toFixed(0)}%</span>
									</div>
									<button class="btn-apply" on:click={() => applyRecommendation(rec)}>
										+ 添加到配方
									</button>
								</div>
							{/each}
						</div>
					{/if}
				</div>
				{#if recipeLoading}
					<div class="loading-text">加载预设配方...</div>
				{/if}
				{#if recipeJson}
					<div class="recipe-preview">
						<div class="preview-header">
							<span class="preview-label">配方预览</span>
							<button class="btn-text" on:click={validateRecipe}>验证配方</button>
						</div>
						<pre class="recipe-json">{recipeJson.substring(0, 500)}{recipeJson.length > 500 ? '...' : ''}</pre>
						{#if recipeValid !== null}
							<span class="validation-badge" class:valid={recipeValid} class:invalid={!recipeValid}>
								{recipeValid ? '✓ 配方有效' : '✗ 配方无效'}
							</span>
						{/if}
						{#if recipeError}
							<div class="error-text">{recipeError}</div>
						{/if}
					</div>
				{/if}
			{:else}
				<div class="form-group">
					<label for="auto-f7">配方名称</label>
					<input id="auto-f7" class="input" type="text" bind:value={customRecipe.name} />
				</div>
				<div class="form-group">
					<label for="auto-f8">混合策略</label>
					<select id="auto-f8" class="input" bind:value={customRecipe.mixing_strategy}>
						<option value="Proportional">按比例混合</option>
						<option value="Interleaved">交错混合</option>
					</select>
				</div>
				<div class="form-group">
					<label for="auto-f9">目标样本数</label>
					<input id="auto-f9" class="input" type="number" bind:value={customRecipe.total_samples_target} placeholder="留空表示不限制" />
				</div>

				<h4>数据集列表</h4>
				<div class="dataset-list">
					{#each customRecipe.datasets as ds, i}
						<div class="dataset-item">
							<span class="ds-name">{ds.name}</span>
							<span class="ds-source">{ds.source}</span>
							<span class="ds-weight">权重: {ds.weight}</span>
							<button class="btn-icon-danger" on:click={() => removeRecipeDataset(i)}>✕</button>
						</div>
					{/each}
				</div>

				<div class="add-dataset-form">
					<input class="input" type="text" bind:value={newRecipeDataset.name} placeholder="数据集名称" />
					<input class="input" type="text" bind:value={newRecipeDataset.source} placeholder="数据源 (如 hf://org/dataset)" />
					<input class="input" type="number" bind:value={newRecipeDataset.weight} placeholder="权重" step="0.1" />
					<select class="input" bind:value={newRecipeDataset.split}>
						<option value="train">train</option>
						<option value="validation">validation</option>
						<option value="test">test</option>
					</select>
					<button class="btn-primary btn-sm" on:click={addRecipeDataset}>添加数据集</button>
				</div>

				<button class="btn-secondary" on:click={validateRecipe}>验证配方</button>
				{#if recipeValid !== null}
					<span class="validation-badge" class:valid={recipeValid} class:invalid={!recipeValid}>
						{recipeValid ? '✓ 配方有效' : '✗ 配方无效'}
					</span>
				{/if}
				{#if recipeError}
					<div class="error-text">{recipeError}</div>
				{/if}
			{/if}

		<!-- Step 3: 训练阶段 -->
		{:else if currentStep === 2}
			<h3>训练阶段</h3>
			<p class="hint">每个阶段使用一个数据配方，可以有不同的学习率和步数</p>

			<div class="phases-list">
				{#each phases as phase, i}
					<div class="phase-card">
						<div class="phase-header">
							<h4>阶段 {i + 1}</h4>
							{#if phases.length > 1}
								<button class="btn-icon-danger" on:click={() => removePhase(i)}>删除</button>
							{/if}
						</div>
						<div class="form-grid">
							<div class="form-group">
								<label for="auto-f10">阶段名称</label>
								<input id="auto-f10" class="input" type="text" bind:value={phase.name} />
							</div>
							<div class="form-group">
								<label for="auto-f11">训练步数</label>
								<input id="auto-f11" class="input" type="number" bind:value={phase.steps} />
							</div>
							<div class="form-group">
								<label for="auto-f12">批次大小</label>
								<input id="auto-f12" class="input" type="number" bind:value={phase.batch_size} />
							</div>
							<div class="form-group">
								<label for="auto-f13">学习率</label>
								<input id="auto-f13" class="input" type="number" bind:value={phase.learning_rate} step="0.0001" />
							</div>
							<div class="form-group">
								<label for="auto-f14">预热步数</label>
								<input id="auto-f14" class="input" type="number" bind:value={phase.warmup_steps} />
							</div>
							<div class="form-group">
								<label for="auto-f15">权重衰减</label>
								<input id="auto-f15" class="input" type="number" bind:value={phase.weight_decay} step="0.01" />
							</div>
							<div class="form-group">
								<label for="auto-f16">最大序列长度</label>
								<input id="auto-f16" class="input" type="number" bind:value={phase.max_seq_length} />
							</div>
						</div>
						<button class="btn-text" on:click={() => applyRecipeToPhase(i)}>应用当前配方到此阶段</button>
					</div>
				{/each}
			</div>
			<button class="btn-secondary" on:click={addPhase}>+ 添加阶段</button>

		<!-- Step 4: 质量门禁 -->
		{:else if currentStep === 3}
			<h3>质量门禁</h3>
			<p class="hint">设置数据质量检查规则，不满足条件的数据将被警告、跳过或中止</p>

			<div class="gates-list">
				{#each qualityGates as gate, i}
					<div class="gate-item">
						<div class="gate-info">
							<span class="gate-name">{gate.name}</span>
							<span class="gate-metric">指标: {gate.metric}</span>
							<span class="gate-threshold">阈值: {gate.threshold}</span>
							<span class="gate-action badge-{gate.action}">{gate.action}</span>
						</div>
						<button class="btn-icon-danger" on:click={() => removeQualityGate(i)}>✕</button>
					</div>
				{/each}
			</div>

			<div class="add-gate-form">
				<input class="input" type="text" bind:value={newGate.name} placeholder="门禁名称" />
				<input class="input" type="text" bind:value={newGate.metric} placeholder="指标 (如 text_length)" />
				<input class="input" type="number" bind:value={newGate.threshold} placeholder="阈值" step="0.01" />
				<select class="input" bind:value={newGate.action}>
					<option value="warn">警告</option>
					<option value="skip">跳过</option>
					<option value="abort">中止</option>
				</select>
				<button class="btn-primary btn-sm" on:click={addQualityGate}>添加门禁</button>
			</div>

			<h4 style="margin-top: 1.5rem;">高级配置</h4>
			<div class="form-grid">
				<div class="form-group">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={enableDedup} />
						启用全局去重
					</label>
				</div>
				{#if enableDedup}
					<div class="form-group">
						<label for="auto-f17">相似度阈值</label>
						<input id="auto-f17" class="input" type="number" bind:value={dedupConfig.similarity_threshold} step="0.05" />
					</div>
				{/if}
				<div class="form-group">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={enablePreprocessing} />
						启用预处理
					</label>
				</div>
				{#if enablePreprocessing}
					<div class="form-group">
						<label for="auto-f18">分词器路径</label>
						<input id="auto-f18" class="input" type="text" bind:value={preprocessing.tokenizer_path} placeholder="tokenizer.json 路径" />
					</div>
					<div class="form-group">
						<label for="auto-f19">最大序列长度</label>
						<input id="auto-f19" class="input" type="number" bind:value={preprocessing.max_seq_length} />
					</div>
					<div class="form-group">
						<label class="checkbox-label">
							<input type="checkbox" bind:checked={preprocessing.packing} />
							序列打包
						</label>
					</div>
				{/if}
				<div class="form-group">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={enableValidation} />
						启用验证集
					</label>
				</div>
				{#if enableValidation}
					<div class="form-group">
						<label for="auto-f20">验证集比例</label>
						<input id="auto-f20" class="input" type="number" bind:value={validation.val_split} step="0.01" />
					</div>
				{/if}
			</div>

		<!-- Step 5: 预算配置 -->
		{:else if currentStep === 4}
			<h3>数据预算</h3>
			<p class="hint">设置训练数据的总预算限制</p>

			<div class="form-grid">
				<div class="form-group">
					<label for="auto-f21">最大 Token 数</label>
					<input id="auto-f21" class="input" type="number" bind:value={dataBudget.max_tokens} />
					<span class="hint">{formatTokens(dataBudget.max_tokens)} tokens</span>
				</div>
				<div class="form-group">
					<label for="auto-f22">最大样本数</label>
					<input id="auto-f22" class="input" type="number" bind:value={dataBudget.max_samples} />
				</div>
				<div class="form-group">
					<label for="auto-f23">最大费用 (USD)</label>
					<input id="auto-f23" class="input" type="number" bind:value={dataBudget.max_cost_usd} />
				</div>
			</div>

			<h4 style="margin-top: 1.5rem;">实验追踪</h4>
			<div class="form-group">
				<label class="checkbox-label">
					<input type="checkbox" bind:checked={enableTracking} />
					启用实验追踪
				</label>
			</div>
			{#if enableTracking}
				<div class="form-grid">
					<div class="form-group">
						<label for="auto-f24">项目名称</label>
						<input id="auto-f24" class="input" type="text" bind:value={experimentTracking.project_name} placeholder="例如: llama3-experiments" />
					</div>
					<div class="form-group">
						<label for="auto-f25">日志间隔 (步)</label>
						<input id="auto-f25" class="input" type="number" bind:value={experimentTracking.log_interval_steps} />
					</div>
				</div>
				<div class="form-group">
				<span class="field-label">标签</span>
					<div class="tag-list">
						{#each experimentTracking.tags as tag, i}
							<span class="tag">
								{tag}
								<button class="tag-remove" on:click={() => removeTag(i)}>✕</button>
							</span>
						{/each}
					</div>
					<div class="add-tag-form">
						<input class="input" type="text" bind:value={newTag} placeholder="添加标签..." />
						<button class="btn-primary btn-sm" on:click={addTag}>添加</button>
					</div>
				</div>
			{/if}

		<!-- Step 6: 确认创建 -->
		{:else if currentStep === 5}
			<h3>确认创建</h3>

			<div class="summary-section">
				<h4>计划概览</h4>
				<div class="summary-grid">
					<div class="summary-item">
						<span class="summary-label">名称</span>
						<span class="summary-value">{planName || '(未设置)'}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">类型</span>
						<span class="summary-value">{planTypeLabels[planType]}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">版本</span>
						<span class="summary-value">{planVersion}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">阶段数</span>
						<span class="summary-value">{phases.length}</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">质量门禁</span>
						<span class="summary-value">{qualityGates.length} 条规则</span>
					</div>
					<div class="summary-item">
						<span class="summary-label">Token 预算</span>
						<span class="summary-value">{formatTokens(dataBudget.max_tokens)}</span>
					</div>
				</div>
			</div>

			{#if planSummary}
				<div class="summary-section">
					<h4>后端验证结果</h4>
					<div class="summary-grid">
						<div class="summary-item">
							<span class="summary-label">预估 Token</span>
							<span class="summary-value">{formatTokens(planSummary.total_estimated_tokens)}</span>
						</div>
						<div class="summary-item">
							<span class="summary-label">预估样本</span>
							<span class="summary-value">{planSummary.total_estimated_samples.toLocaleString()}</span>
						</div>
						<div class="summary-item">
							<span class="summary-label">预估费用</span>
							<span class="summary-value">{formatCost(planSummary.total_estimated_cost_usd)}</span>
						</div>
						<div class="summary-item">
							<span class="summary-label">数据集数</span>
							<span class="summary-value">{planSummary.datasets_count}</span>
						</div>
					</div>
				</div>
			{/if}

			{#if validationResult}
				<div class="validation-section">
					<h4>验证检查</h4>
					{#if validationResult.is_valid}
						<div class="valid-badge">✓ 计划有效</div>
					{:else}
						<div class="invalid-badge">✗ 计划存在问题</div>
					{/if}
					{#if validationResult.errors.length > 0}
						<div class="check-list errors">
							<h5>错误 ({validationResult.errors.length})</h5>
							{#each validationResult.errors as err}
								<div class="check-item error">✗ {err}</div>
							{/each}
						</div>
					{/if}
					{#if validationResult.warnings.length > 0}
						<div class="check-list warnings">
							<h5>警告 ({validationResult.warnings.length})</h5>
							{#each validationResult.warnings as warn}
								<div class="check-item warning">⚠ {warn}</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}

			<div class="action-buttons">
				<button class="btn-secondary" on:click={validatePlan} disabled={validating}>
					{validating ? '验证中...' : '验证计划'}
				</button>
				<button class="btn-primary" on:click={createPlan} disabled={submitting || !planName}>
					{submitting ? '创建中...' : '创建训练计划'}
				</button>
			</div>
		{/if}
	</div>

	<!-- 导航按钮 -->
	<div class="step-nav">
		<button class="btn-secondary" on:click={prevStep} disabled={currentStep === 0}>上一步</button>
		<span class="step-counter">{currentStep + 1} / {steps.length}</span>
		{#if currentStep < steps.length - 1}
			<button class="btn-primary" on:click={nextStep}>下一步</button>
		{/if}
	</div>
</div>

<style>
	.plan-page {
		max-width: 900px;
		margin: 0 auto;
	}

	h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 0.25rem;
	}

	.subtitle {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		margin-bottom: 1.5rem;
	}

	.error-banner {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: #ef4444;
		font-size: 0.9rem;
		margin-bottom: 1rem;
	}

	.success-banner {
		padding: 0.75rem 1rem;
		background: rgba(16, 185, 129, 0.1);
		border: 1px solid rgba(16, 185, 129, 0.3);
		border-radius: 8px;
		color: #10b981;
		font-size: 0.9rem;
		margin-bottom: 1rem;
	}

	.step-indicator {
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 1.5rem;
		gap: 0;
	}

	.step-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.step-circle {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.8rem;
		font-weight: 600;
		background: rgba(255, 255, 255, 0.08);
		color: var(--text-secondary, #9ca3af);
		border: 2px solid rgba(255, 255, 255, 0.12);
		flex-shrink: 0;
	}

	.step-item.active .step-circle {
		background: rgba(16, 185, 129, 0.15);
		border-color: #10b981;
		color: #10b981;
	}

	.step-item.completed .step-circle {
		background: #10b981;
		border-color: #10b981;
		color: white;
	}

	.step-label {
		font-size: 0.8rem;
		color: var(--text-secondary, #9ca3af);
		white-space: nowrap;
	}

	.step-item.active .step-label {
		color: #10b981;
		font-weight: 500;
	}

	.step-item.completed .step-label {
		color: #10b981;
	}

	.step-line {
		width: 40px;
		height: 2px;
		background: rgba(255, 255, 255, 0.12);
		margin: 0 0.5rem;
	}

	.step-line.filled {
		background: #10b981;
	}

	.step-content {
		background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
		border: 1px solid rgba(16, 185, 129, 0.1);
		border-radius: 12px;
		padding: 1.5rem 2rem;
		min-height: 350px;
	}

	h3 {
		font-size: 1.15rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 1.25rem;
	}

	h4 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--text-primary, #e5e7eb);
		margin-bottom: 0.75rem;
	}

	h5 {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.5rem;
	}

	.form-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.form-group {
		margin-bottom: 0.75rem;
	}

	.form-group.full-width {
		grid-column: 1 / -1;
	}

	.form-group label {
		display: block;
		font-size: 0.85rem;
		color: var(--text-secondary, #9ca3af);
		margin-bottom: 0.35rem;
		font-weight: 500;
	}

	.checkbox-label {
		display: flex !important;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.checkbox-label input[type="checkbox"] {
		width: 16px;
		height: 16px;
		accent-color: #10b981;
	}

	.input {
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: rgba(0, 0, 0, 0.3);
		border: 1px solid rgba(107, 114, 128, 0.3);
		border-radius: 8px;
		color: var(--text-primary, #e5e7eb);
		font-size: 0.9rem;
		transition: border-color 0.2s;
		box-sizing: border-box;
	}

	.input:focus {
		outline: none;
		border-color: #10b981;
	}

	select.input {
		cursor: pointer;
	}

	.textarea {
		resize: vertical;
		font-family: inherit;
	}

	.hint {
		display: block;
		font-size: 0.8rem;
		color: var(--text-secondary, #6b7280);
		margin-top: 0.3rem;
	}

	.btn-primary {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, #10b981, #059669);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 0.9rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
	}

	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn-secondary {
		padding: 0.6rem 1.5rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.9rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		color: var(--text-primary, #e5e7eb);
	}

	.btn-secondary:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.btn-sm {
		padding: 0.4rem 0.75rem;
		font-size: 0.8rem;
	}

	.btn-text {
		background: none;
		border: none;
		color: #10b981;
		font-size: 0.85rem;
		cursor: pointer;
		padding: 0.2rem 0.5rem;
	}

	.btn-text:hover {
		text-decoration: underline;
	}

	.btn-icon-danger {
		background: none;
		border: none;
		color: #ef4444;
		cursor: pointer;
		font-size: 0.9rem;
		padding: 0.2rem 0.4rem;
		border-radius: 4px;
	}

	.btn-icon-danger:hover {
		background: rgba(239, 68, 68, 0.15);
	}

	.step-nav {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 1.5rem;
	}

	.step-counter {
		color: var(--text-secondary, #6b7280);
		font-size: 0.85rem;
	}

	/* Mode toggle */
	.mode-toggle {
		display: flex;
		gap: 0.5rem;
	}

	.mode-btn {
		padding: 0.5rem 1.25rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 8px;
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.mode-btn.active {
		background: rgba(16, 185, 129, 0.15);
		border-color: #10b981;
		color: #10b981;
	}

	/* Preset grid */
	.preset-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
	}

	.preset-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(107, 114, 128, 0.2);
		border-radius: 10px;
		cursor: pointer;
		transition: all 0.2s;
		color: var(--text-secondary, #9ca3af);
	}

	.preset-card:hover {
		border-color: rgba(16, 185, 129, 0.3);
		background: rgba(16, 185, 129, 0.05);
	}

	.preset-card.selected {
		border-color: #10b981;
		background: rgba(16, 185, 129, 0.1);
		color: #10b981;
	}

	.preset-name {
		font-weight: 600;
		font-size: 0.9rem;
	}

	.preset-key {
		font-size: 0.7rem;
		opacity: 0.7;
		font-family: monospace;
	}

	/* Recipe preview */
	.recipe-preview {
		margin-top: 1rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 8px;
		padding: 1rem;
	}

	.preview-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.preview-label {
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--text-secondary, #9ca3af);
	}

	.recipe-json {
		font-size: 0.75rem;
		color: var(--text-secondary, #6b7280);
		background: rgba(0, 0, 0, 0.3);
		padding: 0.75rem;
		border-radius: 6px;
		overflow-x: auto;
		white-space: pre-wrap;
		word-break: break-all;
		max-height: 200px;
		overflow-y: auto;
	}

	.validation-badge {
		display: inline-block;
		font-size: 0.8rem;
		font-weight: 500;
		margin-top: 0.5rem;
		padding: 0.2rem 0.6rem;
		border-radius: 4px;
	}

	.validation-badge.valid {
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
	}

	.validation-badge.invalid {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.error-text {
		color: #ef4444;
		font-size: 0.85rem;
		margin-top: 0.5rem;
	}

	.loading-text {
		color: var(--text-secondary, #9ca3af);
		font-size: 0.85rem;
		margin-top: 0.5rem;
	}

	/* Dataset list */
	.dataset-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.dataset-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 6px;
		font-size: 0.85rem;
	}

	.ds-name {
		color: var(--text-primary, #e5e7eb);
		font-weight: 500;
	}

	.ds-source {
		color: var(--text-secondary, #6b7280);
		font-family: monospace;
		font-size: 0.8rem;
	}

	.ds-weight {
		color: #10b981;
		font-size: 0.8rem;
	}

	.add-dataset-form {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		margin-bottom: 1rem;
	}

	.add-dataset-form .input {
		flex: 1;
		min-width: 120px;
	}

	/* Phases */
	.phases-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.phase-card {
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 1rem;
	}

	.phase-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	/* Quality gates */
	.gates-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.gate-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 6px;
	}

	.gate-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.85rem;
	}

	.gate-name {
		color: var(--text-primary, #e5e7eb);
		font-weight: 500;
	}

	.gate-metric, .gate-threshold {
		color: var(--text-secondary, #6b7280);
		font-size: 0.8rem;
	}

	.gate-action {
		font-size: 0.75rem;
		padding: 0.15rem 0.5rem;
		border-radius: 4px;
		font-weight: 500;
	}

	.badge-warn {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.badge-skip {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.badge-abort {
		background: rgba(239, 68, 68, 0.25);
		color: #ef4444;
	}

	.add-gate-form {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.add-gate-form .input {
		flex: 1;
		min-width: 100px;
	}

	/* Tags */
	.tag-list {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		margin-bottom: 0.5rem;
	}

	.tag {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.2rem 0.6rem;
		background: rgba(16, 185, 129, 0.1);
		border: 1px solid rgba(16, 185, 129, 0.2);
		border-radius: 4px;
		color: #10b981;
		font-size: 0.8rem;
	}

	.tag-remove {
		background: none;
		border: none;
		color: #10b981;
		cursor: pointer;
		font-size: 0.7rem;
		padding: 0;
		opacity: 0.7;
	}

	.tag-remove:hover {
		opacity: 1;
	}

	.add-tag-form {
		display: flex;
		gap: 0.5rem;
	}

	.add-tag-form .input {
		flex: 1;
	}

	/* Summary */
	.summary-section {
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 10px;
		padding: 1rem;
		margin-bottom: 1rem;
	}

	.summary-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
	}

	.summary-item {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.summary-label {
		font-size: 0.75rem;
		color: var(--text-secondary, #6b7280);
	}

	.summary-value {
		font-size: 0.95rem;
		color: var(--text-primary, #e5e7eb);
		font-weight: 500;
	}

	/* Validation */
	.validation-section {
		margin-bottom: 1rem;
	}

	.valid-badge {
		display: inline-block;
		padding: 0.3rem 0.75rem;
		background: rgba(16, 185, 129, 0.15);
		color: #10b981;
		border-radius: 6px;
		font-weight: 500;
		font-size: 0.9rem;
		margin-bottom: 0.75rem;
	}

	.invalid-badge {
		display: inline-block;
		padding: 0.3rem 0.75rem;
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
		border-radius: 6px;
		font-weight: 500;
		font-size: 0.9rem;
		margin-bottom: 0.75rem;
	}

	.check-list {
		margin-bottom: 0.75rem;
	}

	.check-item {
		font-size: 0.85rem;
		padding: 0.3rem 0;
	}

	.check-item.error {
		color: #ef4444;
	}

	.check-item.warning {
		color: #f59e0b;
	}

	.action-buttons {
		display: flex;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	@media (max-width: 700px) {
		.form-grid {
			grid-template-columns: 1fr;
		}
		.summary-grid {
			grid-template-columns: repeat(2, 1fr);
		}
		.preset-grid {
			grid-template-columns: 1fr;
		}
		.step-label {
			display: none;
		}
	}

	.btn-recommend {
		padding: 0.5rem 1rem;
		background: linear-gradient(135deg, #8b5cf6, #6366f1);
		border: none;
		border-radius: 6px;
		color: #fff;
		font-size: 0.82rem;
		cursor: pointer;
		transition: opacity 0.2s;
	}
	.btn-recommend:hover:not(:disabled) { opacity: 0.9; }
	.btn-recommend:disabled { opacity: 0.5; cursor: not-allowed; }

	.recommend-list { margin-top: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
	.recommend-item {
		padding: 0.6rem 0.75rem;
		background: rgba(255,255,255,0.03);
		border: 1px solid rgba(255,255,255,0.08);
		border-radius: 6px;
	}
	.recommend-item.excellent { border-color: rgba(16,185,129,0.3); }
	.recommend-item.good { border-color: rgba(59,130,246,0.3); }
	.recommend-item.fair { border-color: rgba(245,158,11,0.3); }
	.recommend-item.poor { border-color: rgba(239,68,68,0.3); }

	.rec-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.35rem; }
	.rec-name { font-weight: 600; font-size: 0.82rem; color: #e5e7eb; }
	.rec-score { font-size: 0.78rem; font-weight: 700; color: #60a5fa; }
	.rec-level { font-size: 0.68rem; margin-left: auto; }
	.recommend-item.excellent .rec-level { color: #34d399; }
	.recommend-item.good .rec-level { color: #60a5fa; }
	.recommend-item.fair .rec-level { color: #fbbf24; }
	.recommend-item.poor .rec-level { color: #f87171; }

	.rec-reasons { margin-bottom: 0.35rem; }
	.rec-reason { font-size: 0.68rem; color: #9ca3af; line-height: 1.4; }

	.rec-match { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
	.match-item {
		font-size: 0.62rem;
		padding: 0.1rem 0.35rem;
		background: rgba(255,255,255,0.04);
		border-radius: 3px;
		color: #6b7280;
	}

	.btn-apply {
		padding: 0.25rem 0.6rem;
		background: rgba(59,130,246,0.15);
		border: 1px solid rgba(59,130,246,0.3);
		border-radius: 4px;
		color: #60a5fa;
		font-size: 0.68rem;
		cursor: pointer;
	}
	.btn-apply:hover { background: rgba(59,130,246,0.25); }
</style>
