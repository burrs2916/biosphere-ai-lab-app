use std::sync::Arc;

use crate::core::EventBus;
use crate::domain::CommandBus;
use crate::domain::experiment::repository::ExperimentRepository;
use crate::domain::experiment::handler::ExperimentCommandHandler;
use crate::domain::training::handler::TrainingCommandHandler;
use crate::domain::training::service::TrainingService;
use crate::domain::model::repository::ModelRepository;
use crate::domain::model::handler::ModelCommandHandler;
use crate::domain::model::serving::ModelServer;
use crate::domain::dataset::repository::DatasetRepository;
use crate::domain::dataset::handler::DatasetCommandHandler;
use crate::domain::hardware::service::HardwareService;
use crate::domain::hardware::monitor::ResourceMonitor;
use crate::engine::EngineRegistry;
use crate::task::TaskRegistry;
use crate::model::ModelRegistry;
use crate::data::DataSourceRegistry;
use crate::data::DataConnectorRegistry;
use crate::data::DataAccessorRegistry;
use crate::infrastructure::persistence::sqlite::SqliteSettingsRepository;
use crate::domain::experiment::ExperimentFilter;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateSnapshot {
    pub active_experiments: usize,
    pub registered_engines: usize,
    pub registered_tasks: usize,
    pub registered_models: usize,
    pub registered_data_sources: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_experiments: usize,
    pub running_experiments: usize,
    pub completed_experiments: usize,
    pub failed_experiments: usize,
    pub total_models: usize,
    pub production_models: usize,
    pub status_counts: HashMap<String, usize>,
    pub task_type_counts: HashMap<String, usize>,
}

pub struct AppState {
    pub event_bus: Arc<EventBus>,
    pub command_bus: Arc<CommandBus>,
    pub experiment_repo: Arc<dyn ExperimentRepository>,
    pub model_repo: Arc<dyn ModelRepository>,
    pub dataset_repo: Arc<dyn DatasetRepository>,
    pub settings_repo: Arc<SqliteSettingsRepository>,
    pub training_service: Arc<TrainingService>,
    pub hardware_service: Arc<HardwareService>,
    pub resource_monitor: Arc<ResourceMonitor>,
    pub experiment_handler: Arc<dyn ExperimentCommandHandler>,
    pub training_handler: Arc<dyn TrainingCommandHandler>,
    pub model_handler: Arc<dyn ModelCommandHandler>,
    pub dataset_handler: Arc<dyn DatasetCommandHandler>,

    pub engine_registry: Arc<EngineRegistry>,
    pub model_server: Arc<ModelServer>,
    pub task_registry: Arc<TaskRegistry>,
    pub model_registry: Arc<ModelRegistry>,
    pub data_source_registry: Arc<DataSourceRegistry>,
    pub data_connector_registry: Arc<DataConnectorRegistry>,
    pub data_accessor_registry: Arc<DataAccessorRegistry>,
}

impl AppState {
    pub fn new() -> Self {
        let db_path = Self::get_db_path();
        Self::with_db_path(&db_path)
    }

    fn get_db_path() -> String {
        #[cfg(debug_assertions)]
        {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                .unwrap_or_else(|_| ".".to_string());
            let project_root = std::path::PathBuf::from(&manifest_dir)
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from(&manifest_dir));
            let data_dir = project_root.join("biosphere-ai-lab-app").join("data");
            if !data_dir.exists() {
                let _ = std::fs::create_dir_all(&data_dir);
            }
            data_dir.join("biosphere.db").to_string_lossy().into_owned()
        }
        #[cfg(not(debug_assertions))]
        {
            let data_dir = dirs::data_local_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("biosphere-ai-lab");
            if !data_dir.exists() {
                let _ = std::fs::create_dir_all(&data_dir);
            }
            data_dir.join("biosphere.db").to_string_lossy().into_owned()
        }
    }

    pub fn with_db_path(db_path: &str) -> Self {
        let event_bus = Arc::new(EventBus::new(2048));

        let sqlite_repo = crate::infrastructure::persistence::sqlite::SqliteExperimentRepository::new(db_path)
            .expect("Failed to initialize SQLite experiment repository");
        let conn = sqlite_repo.conn().clone();

        {
            let guard = conn.lock().expect("DB lock");
            crate::infrastructure::persistence::sqlite::SqliteModelRepository::init_schema(&guard)
                .expect("Failed to initialize model schema");
            crate::infrastructure::persistence::sqlite::SqliteDatasetRepository::init_schema(&guard)
                .expect("Failed to initialize dataset schema");
        }

        let experiment_repo: Arc<dyn ExperimentRepository> = Arc::new(sqlite_repo);
        let model_repo: Arc<dyn ModelRepository> = Arc::new(
            crate::infrastructure::persistence::sqlite::SqliteModelRepository::new(conn.clone())
        );
        let dataset_repo: Arc<dyn DatasetRepository> = Arc::new(
            crate::infrastructure::persistence::sqlite::SqliteDatasetRepository::new(conn.clone())
        );
        let settings_repo = Arc::new(
            crate::infrastructure::persistence::sqlite::SqliteSettingsRepository::new(conn.clone())
                .expect("Failed to initialize settings repository")
        );

        let experiment_handler: Arc<dyn ExperimentCommandHandler> = Arc::new(
            crate::domain::experiment::handler::DefaultExperimentCommandHandler::new(
                experiment_repo.clone(),
                event_bus.clone(),
            )
            .with_model_repo(model_repo.clone())
            .with_dataset_repo(dataset_repo.clone())
        );

        let training_handler: Arc<dyn TrainingCommandHandler> = Arc::new(
            crate::domain::training::handler::DefaultTrainingCommandHandler::new(
                experiment_repo.clone(),
                experiment_handler.clone(),
                event_bus.clone(),
            )
        );

        let model_handler: Arc<dyn ModelCommandHandler> = Arc::new(
            crate::domain::model::handler::DefaultModelCommandHandler::new(
                model_repo.clone(),
                experiment_repo.clone(),
                event_bus.clone(),
            )
        );

        let dataset_handler: Arc<dyn DatasetCommandHandler> = Arc::new(
            crate::domain::dataset::handler::DefaultDatasetCommandHandler::new(
                dataset_repo.clone(),
                event_bus.clone(),
            )
        );

        let command_bus = Arc::new(CommandBus::new(
            event_bus.clone(),
            experiment_handler.clone(),
            training_handler.clone(),
            model_handler.clone(),
        ));

        let engine_registry = Arc::new(EngineRegistry::new());

        let training_service = Arc::new(TrainingService::with_engine_registry(
            event_bus.clone(),
            experiment_repo.clone(),
            experiment_handler.clone(),
            model_handler.clone(),
            engine_registry.clone(),
        ).with_dataset_repo(dataset_repo.clone()));

        let hardware_service = Arc::new(HardwareService::new());
        let resource_monitor = Arc::new(ResourceMonitor::new(event_bus.clone()));

        let model_server = Arc::new(ModelServer::new(
            model_repo.clone(),
            engine_registry.clone(),
            experiment_repo.clone(),
        ).with_db_conn(conn.clone()));

        let state = Self {
            event_bus,
            command_bus,
            experiment_repo,
            model_repo,
            dataset_repo,
            settings_repo,
            training_service,
            hardware_service,
            resource_monitor,
            experiment_handler,
            training_handler,
            model_handler,
            dataset_handler,
            engine_registry,
            model_server,
            task_registry: Arc::new(TaskRegistry::new()),
            model_registry: Arc::new(ModelRegistry::new()),
            data_source_registry: Arc::new(DataSourceRegistry::new()),
            data_connector_registry: Arc::new(DataConnectorRegistry::new()),
            data_accessor_registry: Arc::new(DataAccessorRegistry::new()),
        };

        state
    }

    pub async fn recover_orphan_experiments(&self) {
        let filter = ExperimentFilter::default();
        let repo = self.experiment_repo.clone();
        let experiments = match repo.list(&filter).await {
            Ok(exps) => exps,
            Err(e) => {
                crate::infrastructure::log("APP_INIT", "WARN", Some(&format!("加载实验列表失败: {}", e)));
                return;
            }
        };

        let mut orphan_count = 0u32;
        for exp in &experiments {
            if exp.status == crate::domain::experiment::aggregate::ExperimentStatus::Running
                || exp.status == crate::domain::experiment::aggregate::ExperimentStatus::Paused
            {
                orphan_count += 1;
                crate::infrastructure::log("APP_INIT", &format!(
                    "发现孤儿实验: id='{}', name='{}', status={:?}，标记为Failed",
                    exp.id, exp.name, exp.status
                ), None);

                if let Ok(Some(mut experiment)) = repo.load(&exp.id).await {
                    experiment.status = crate::domain::experiment::aggregate::ExperimentStatus::Failed;
                    experiment.error_message = Some("Training session lost (app restart or crash). Auto-recovered on startup.".to_string());
                    experiment.completed_at = Some(chrono::Utc::now());
                    let _ = repo.save(&experiment).await;
                }
            }
        }

        if orphan_count > 0 {
            crate::infrastructure::log("APP_INIT", &format!("已恢复 {} 个孤儿实验（标记为Failed）", orphan_count), None);
        } else {
            crate::infrastructure::log("APP_INIT", "未发现孤儿实验", None);
        }
    }

    pub async fn register_default_plugins(&self) {
        crate::infrastructure::log("PLUGINS", "开始注册默认插件", None);

        self.engine_registry.register(
            crate::engine::BurnEngine::new().with_event_bus(self.event_bus.clone())
        ).await;
        crate::infrastructure::log("PLUGINS", "已注册引擎: BurnEngine", None);

        #[cfg(feature = "tch-engine")]
        {
            self.engine_registry.register(
                crate::engine::TchEngine::new().with_event_bus(self.event_bus.clone())
            ).await;
            crate::infrastructure::log("PLUGINS", "已注册引擎: TchEngine (PyTorch)", None);
        }

        self.task_registry.register(crate::task::ClassificationTask::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册任务: ClassificationTask", None);

        self.data_source_registry.register(crate::data::CsvLoader::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册数据源: CsvLoader", None);

        self.data_source_registry.register(crate::data::JsonLoader::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册数据源: JsonLoader", None);

        self.data_source_registry.register(crate::data::ParquetLoader::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册数据源: ParquetLoader", None);

        self.data_connector_registry.register(crate::data::LocalFsConnector::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册数据连接器: LocalFsConnector", None);

        self.data_connector_registry.register(crate::data::HttpConnector::new()).await;
        crate::infrastructure::log("PLUGINS", "已注册数据连接器: HttpConnector", None);

        self.model_registry.register(
            crate::model::MlpModel::default_classifier(784, 10)
        ).await;
        crate::infrastructure::log("PLUGINS", "已注册模型: MLP", None);

        self.model_registry.register(
            crate::model::CnnModel::default_classifier(1, 28, 28, 10)
        ).await;
        crate::infrastructure::log("PLUGINS", "已注册模型: CNN", None);

        self.model_server.restore_deployed_models().await;
    }

    pub async fn snapshot(&self) -> AppStateSnapshot {
        AppStateSnapshot {
            active_experiments: self.training_service.active_session_count().await,
            registered_engines: self.engine_registry.list().await.len(),
            registered_tasks: self.task_registry.list().await.len(),
            registered_models: self.model_registry.list().await.len(),
            registered_data_sources: self.data_source_registry.list().await.len(),
        }
    }

    pub async fn dashboard_stats(&self) -> DashboardStats {
        let experiments = self.experiment_repo.list(&ExperimentFilter::default())
            .await
            .unwrap_or_default();

        let mut status_counts = HashMap::new();
        let mut task_type_counts = HashMap::new();
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;

        for exp in &experiments {
            let status_str = exp.status.to_string();
            *status_counts.entry(status_str.clone()).or_insert(0) += 1;
            let tt_str = serde_json::to_string(&exp.task_type)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            *task_type_counts.entry(tt_str).or_insert(0) += 1;

            match exp.status {
                crate::domain::experiment::aggregate::ExperimentStatus::Running => running += 1,
                crate::domain::experiment::aggregate::ExperimentStatus::Completed => completed += 1,
                crate::domain::experiment::aggregate::ExperimentStatus::Failed => failed += 1,
                _ => {}
            }
        }

        let models = self.model_repo.list(None).await.unwrap_or_default();
        let production_models = models.iter()
            .filter(|m| m.status == crate::domain::model::aggregate::ModelStatus::Production)
            .count();

        DashboardStats {
            total_experiments: experiments.len(),
            running_experiments: running,
            completed_experiments: completed,
            failed_experiments: failed,
            total_models: models.len(),
            production_models,
            status_counts,
            task_type_counts,
        }
    }
    pub async fn start_resource_monitor(&self) {
        self.resource_monitor.start().await;
    }

    pub async fn stop_resource_monitor(&self) {
        self.resource_monitor.stop().await;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
