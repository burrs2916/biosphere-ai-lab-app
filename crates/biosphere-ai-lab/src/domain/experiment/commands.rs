use crate::core::config::TrainingConfig;
use crate::types::TaskType;

use super::aggregate::ExperimentId;
use super::ArtifactRef;

#[derive(Debug, Clone)]
pub enum ExperimentCommand {
    CreateExperiment {
        name: String,
        task_type: TaskType,
        config: TrainingConfig,
    },
    TrackMetric {
        experiment_id: ExperimentId,
        metric_name: String,
        value: f64,
        step: u64,
    },
    TrackMetricWithEpoch {
        experiment_id: ExperimentId,
        metric_name: String,
        value: f64,
        step: u64,
        epoch: usize,
    },
    StartExperiment {
        experiment_id: ExperimentId,
    },
    RestartExperiment {
        experiment_id: ExperimentId,
    },
    PauseExperiment {
        experiment_id: ExperimentId,
    },
    ResumeExperiment {
        experiment_id: ExperimentId,
    },
    CompleteExperiment {
        experiment_id: ExperimentId,
        final_metrics: serde_json::Value,
    },
    FailExperiment {
        experiment_id: ExperimentId,
        error: String,
    },
    CancelExperiment {
        experiment_id: ExperimentId,
    },
    SetParam {
        experiment_id: ExperimentId,
        key: String,
        value: serde_json::Value,
    },
    AddTag {
        experiment_id: ExperimentId,
        tag: String,
    },
    DeleteExperiment {
        experiment_id: ExperimentId,
    },
    SetDescription {
        experiment_id: ExperimentId,
        description: String,
    },
    RemoveTag {
        experiment_id: ExperimentId,
        tag: String,
    },
    CloneExperiment {
        experiment_id: ExperimentId,
        new_name: String,
    },
    AddArtifact {
        experiment_id: ExperimentId,
        artifact: ArtifactRef,
    },
    ArchiveExperiment {
        experiment_id: ExperimentId,
    },
    RestoreExperiment {
        experiment_id: ExperimentId,
    },
    LinkDataset {
        experiment_id: ExperimentId,
        dataset_id: String,
        dataset_version: Option<String>,
    },
    SetGroup {
        experiment_id: ExperimentId,
        group: String,
    },
}
