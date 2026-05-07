mod infrastructure;

use std::sync::Arc;
use tauri::Manager;
use infrastructure::{init_logger, log};
use biosphere_ai_lab::AppState;

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info.location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "Unknown location".to_string());

        let entry = format!("[PANIC] {} at {}", message, location);
        eprintln!("{}", entry);

        let log_path = std::env::var("CARGO_MANIFEST_DIR")
            .map(|d| std::path::PathBuf::from(d).parent().map(|p| p.join("logs")).unwrap_or_else(|| std::path::PathBuf::from("/tmp")))
            .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));

        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path.join("panic.log"))
        {
            use std::io::Write;
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), entry);
            let _ = file.flush();
        }
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();

    let app_state = Arc::new(AppState::new());

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state.clone())
        .setup(move |app| {
            if let Err(e) = setup_inner(app, &app_state) {
                eprintln!("[SETUP ERROR] {}", e);
                log("SYSTEM", &format!("Setup error: {}", e), None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_state,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_dashboard_stats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_resource_snapshot,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_engines,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_tasks,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_models,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_data_sources,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_hardware_info,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_recommendations,
            biosphere_ai_lab::gateway::tauri_adapter::lab_load_data,
            biosphere_ai_lab::gateway::tauri_adapter::lab_preview_data,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_model_arch,
            biosphere_ai_lab::gateway::tauri_adapter::lab_create_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_experiments,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_experiment_detail,
            biosphere_ai_lab::gateway::tauri_adapter::lab_query_metrics,
            biosphere_ai_lab::gateway::tauri_adapter::lab_query_metrics_downsampled,
            biosphere_ai_lab::gateway::tauri_adapter::lab_load_logs,
            biosphere_ai_lab::gateway::tauri_adapter::lab_track_metric,
            biosphere_ai_lab::gateway::tauri_adapter::lab_register_model,
            biosphere_ai_lab::gateway::tauri_adapter::lab_register_model_from_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_model_registrations,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_model_registration,
            biosphere_ai_lab::gateway::tauri_adapter::lab_promote_model_staging,
            biosphere_ai_lab::gateway::tauri_adapter::lab_promote_model_production,
            biosphere_ai_lab::gateway::tauri_adapter::lab_archive_model,
            biosphere_ai_lab::gateway::tauri_adapter::lab_demote_model_staging,
            biosphere_ai_lab::gateway::tauri_adapter::lab_add_model_alias,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remove_model_alias,
            biosphere_ai_lab::gateway::tauri_adapter::lab_delete_model_registration,
            biosphere_ai_lab::gateway::tauri_adapter::lab_set_model_path,
            biosphere_ai_lab::gateway::tauri_adapter::lab_add_model_version,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_model_versions,
            biosphere_ai_lab::gateway::tauri_adapter::lab_set_model_description,
            biosphere_ai_lab::gateway::tauri_adapter::lab_add_model_tag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remove_model_tag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_experiment_set_description,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_checkpoints,
            biosphere_ai_lab::gateway::tauri_adapter::lab_delete_checkpoint,
            biosphere_ai_lab::gateway::tauri_adapter::lab_evaluate_model,
            biosphere_ai_lab::gateway::tauri_adapter::lab_save_evaluation,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_evaluations,
            biosphere_ai_lab::gateway::tauri_adapter::lab_batch_inference,
            biosphere_ai_lab::gateway::tauri_adapter::lab_start_training,
            biosphere_ai_lab::gateway::tauri_adapter::lab_stop_training,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pause_training,
            biosphere_ai_lab::gateway::tauri_adapter::lab_resume_training,
            biosphere_ai_lab::gateway::tauri_adapter::lab_resume_from_checkpoint,
            biosphere_ai_lab::gateway::tauri_adapter::lab_run_inference,
            biosphere_ai_lab::gateway::tauri_adapter::lab_preprocess_data,
            biosphere_ai_lab::gateway::tauri_adapter::lab_experiment_add_tag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_experiment_set_param,
            biosphere_ai_lab::gateway::tauri_adapter::lab_delete_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_archive_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_restore_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_batch_delete_experiments,
            biosphere_ai_lab::gateway::tauri_adapter::lab_clone_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_set_experiment_group,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_experiment_groups,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_artifacts,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_artifact_content,
            biosphere_ai_lab::gateway::tauri_adapter::lab_scan_artifacts,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_settings,
            biosphere_ai_lab::gateway::tauri_adapter::lab_save_settings,
            biosphere_ai_lab::gateway::tauri_adapter::lab_register_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_datasets,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_delete_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_archive_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_restore_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_add_tag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_remove_tag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_set_description,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_link_experiment,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_new_version,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_version_history,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_new_version_with_note,
            biosphere_ai_lab::gateway::tauri_adapter::lab_create_dataset_split,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_dataset_splits,
            biosphere_ai_lab::gateway::tauri_adapter::lab_get_dataset_split,
            biosphere_ai_lab::gateway::tauri_adapter::lab_delete_dataset_split,
            biosphere_ai_lab::gateway::tauri_adapter::lab_validate_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_auto_validate_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_version_diff,
            biosphere_ai_lab::gateway::tauri_adapter::lab_validate_dataset_integrity,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_set_card,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_get_card,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_analyze_imbalance,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_analyze_drift,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_analyze_correlation,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_check_leakage,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_check_feature_leakage,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_check_sufficiency,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_check_split_consistency,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_readiness_score,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_create_kfold,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_row_diff,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_list_augmentation_presets,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_lazy_inspect,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_lazy_read_chunk,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_recommend_chunk_size,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_create_standard_dag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_build_dag,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_detect_changes,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_plan_execution,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_to_mermaid,
            biosphere_ai_lab::gateway::tauri_adapter::lab_pipeline_validate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_create_training,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_build,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_trace,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_impact,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_reproducibility,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_to_mermaid,
            biosphere_ai_lab::gateway::tauri_adapter::lab_lineage_graph,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_dedup,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_label_quality,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_slice_analysis,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_bias_detection,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_discovery_search,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_usage_stats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_confident_learning,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_label_quality_summary,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_curation,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_quality_score,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_recommend_for_plan,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_multimodal_images,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_multimodal_texts,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remote_storage_validate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remote_storage_build_url,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remote_storage_estimate_transfer,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remote_storage_recommend_class,
            biosphere_ai_lab::gateway::tauri_adapter::lab_remote_storage_sync_plan,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_influence_tracin,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_influence_loo,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_influence_loss_diff,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dashboard_build,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dashboard_create_alert,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dashboard_create_snapshot,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dashboard_lineage_from_graph,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_preview,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_sample,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_column_stats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_read_split,
            biosphere_ai_lab::gateway::tauri_adapter::lab_dataset_lineage,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_lineage,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_connectors,
            biosphere_ai_lab::gateway::tauri_adapter::lab_scan_data_sources,
            biosphere_ai_lab::gateway::tauri_adapter::lab_test_data_connection,
            biosphere_ai_lab::gateway::tauri_adapter::lab_resolve_data_item,
            biosphere_ai_lab::gateway::tauri_adapter::lab_quick_register_dataset,
            biosphere_ai_lab::gateway::tauri_adapter::lab_start_hyperparameter_tuning,
            biosphere_ai_lab::gateway::tauri_adapter::lab_generate_hparam_combinations,
            biosphere_ai_lab::gateway::tauri_adapter::lab_export_model,
            biosphere_ai_lab::gateway::tauri_adapter::lab_list_export_formats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_deploy,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_undeploy,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_predict,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_list_endpoints,
            biosphere_ai_lab::gateway::tauri_adapter::lab_model_serve_stats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_arrow_table_info,
            biosphere_ai_lab::gateway::tauri_adapter::lab_arrow_table_column_stats,
            biosphere_ai_lab::gateway::tauri_adapter::lab_arrow_table_slice,
            biosphere_ai_lab::gateway::tauri_adapter::lab_arrow_table_select,
            biosphere_ai_lab::gateway::tauri_adapter::lab_streaming_open_csv,
            biosphere_ai_lab::gateway::tauri_adapter::lab_streaming_open_jsonl,
            biosphere_ai_lab::gateway::tauri_adapter::lab_streaming_recommend_chunk,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sharding_plan,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sharding_indices,
            biosphere_ai_lab::gateway::tauri_adapter::lab_query_execute,
            biosphere_ai_lab::gateway::tauri_adapter::lab_query_estimate_cost,
            biosphere_ai_lab::gateway::tauri_adapter::lab_packing_pack,
            biosphere_ai_lab::gateway::tauri_adapter::lab_packing_estimate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_interleave_compute,
            biosphere_ai_lab::gateway::tauri_adapter::lab_provenance_report,
            biosphere_ai_lab::gateway::tauri_adapter::lab_provenance_check_commercial,
            biosphere_ai_lab::gateway::tauri_adapter::lab_checkpoint_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_checkpoint_list,
            biosphere_ai_lab::gateway::tauri_adapter::lab_checkpoint_resume,
            biosphere_ai_lab::gateway::tauri_adapter::lab_tokenizer_load,
            biosphere_ai_lab::gateway::tauri_adapter::lab_tokenizer_encode,
            biosphere_ai_lab::gateway::tauri_adapter::lab_tokenizer_encode_batch,
            biosphere_ai_lab::gateway::tauri_adapter::lab_tokenizer_decode,
            biosphere_ai_lab::gateway::tauri_adapter::lab_chat_template_apply,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_collate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_curation_config,
            biosphere_ai_lab::gateway::tauri_adapter::lab_curation_mask_pii,
            biosphere_ai_lab::gateway::tauri_adapter::lab_prefetch_open_csv,
            biosphere_ai_lab::gateway::tauri_adapter::lab_prefetch_open_jsonl,
            biosphere_ai_lab::gateway::tauri_adapter::lab_prefetch_compression_info,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sampler_distributed,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sampler_split,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sampler_kfold,
            biosphere_ai_lab::gateway::tauri_adapter::lab_sampler_stratified,
            biosphere_ai_lab::gateway::tauri_adapter::lab_monitor_start,
            biosphere_ai_lab::gateway::tauri_adapter::lab_monitor_snapshot,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_loader_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_global_shuffle_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_cloud_streaming_open,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_init,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_commit,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_log,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_checkout,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_diff,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_branches,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_version_create_branch,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_recipe_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_recipe_validate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_recipe_execute,
            biosphere_ai_lab::gateway::tauri_adapter::lab_data_recipe_presets,
            biosphere_ai_lab::gateway::tauri_adapter::lab_hf_hub_search,
            biosphere_ai_lab::gateway::tauri_adapter::lab_hf_hub_info,
            biosphere_ai_lab::gateway::tauri_adapter::lab_hf_hub_download,
            biosphere_ai_lab::gateway::tauri_adapter::lab_hf_hub_cached,
            biosphere_ai_lab::gateway::tauri_adapter::lab_hf_hub_popular,
            biosphere_ai_lab::gateway::tauri_adapter::lab_global_dedup_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_global_dedup_process,
            biosphere_ai_lab::gateway::tauri_adapter::lab_global_dedup_report,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_create,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_validate,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_summarize,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_presets,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_save,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_list,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_load,
            biosphere_ai_lab::gateway::tauri_adapter::lab_training_plan_delete,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = &result {
        eprintln!("[FATAL] Tauri run error: {}", e);
    }

    result.unwrap_or_else(|e| {
        eprintln!("[FATAL] Tauri application exited with error: {}", e);
        std::process::exit(1);
    });
}

fn setup_inner(app: &tauri::App, app_state: &Arc<AppState>) -> Result<(), String> {
    eprintln!("[SETUP] Step 1: getting app data dir...");

    let data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    eprintln!("[SETUP] Step 2: creating data dir {:?}...", data_dir);

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory {:?}: {}", data_dir, e))?;

    eprintln!("[SETUP] Step 3: configuring logger...");

    let mut log_config = infrastructure::LogConfig::default();
    log_config.console_output = true;
    log_config.clear_on_start = true;

    #[cfg(debug_assertions)]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| ".".to_string());
        let project_root = std::path::PathBuf::from(&manifest_dir)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(&manifest_dir));
        log_config.log_dir = project_root.join("data").join("logs");
    }

    #[cfg(not(debug_assertions))]
    {
        log_config.log_dir = data_dir.join("logs");
    }

    eprintln!("[SETUP] Step 4: initializing logger...");
    init_logger(&data_dir, &log_config);
    log("SYSTEM", "Biosphere AI Lab 启动", None);

    eprintln!("[SETUP] Step 5: setting up app state...");
    biosphere_ai_lab::gateway::setup_app(app, app_state);

    log("SYSTEM", &format!("数据目录: {:?}", data_dir), None);
    eprintln!("[SETUP] All steps completed successfully");

    Ok(())
}
