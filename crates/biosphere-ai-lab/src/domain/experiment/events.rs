use serde::{Deserialize, Serialize};

use super::aggregate::ExperimentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ExperimentEvent {
    ExperimentCreated {
        experiment_id: ExperimentId,
        name: String,
    },
    MetricTracked {
        experiment_id: ExperimentId,
        metric_name: String,
        value: f64,
        step: u64,
    },
    ExperimentStarted {
        experiment_id: ExperimentId,
    },
    ExperimentPaused {
        experiment_id: ExperimentId,
    },
    ExperimentResumed {
        experiment_id: ExperimentId,
    },
    ExperimentCompleted {
        experiment_id: ExperimentId,
        final_metrics: serde_json::Value,
    },
    ExperimentFailed {
        experiment_id: ExperimentId,
        error: String,
    },
    ExperimentCancelled {
        experiment_id: ExperimentId,
    },
    ParamSet {
        experiment_id: ExperimentId,
        key: String,
    },
    TagAdded {
        experiment_id: ExperimentId,
        tag: String,
    },
    ExperimentDeleted {
        experiment_id: ExperimentId,
    },
    ExperimentCloned {
        source_experiment_id: ExperimentId,
        new_experiment_id: ExperimentId,
        new_name: String,
    },
    ExperimentArchived {
        experiment_id: ExperimentId,
    },
    ExperimentRestored {
        experiment_id: ExperimentId,
    },
}
