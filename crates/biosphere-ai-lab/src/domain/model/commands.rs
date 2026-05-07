use super::aggregate::ModelId;
use crate::domain::experiment::aggregate::ExperimentId;

#[derive(Debug, Clone)]
pub enum ModelCommand {
    RegisterModel {
        name: String,
        version: String,
        framework: String,
    },
    RegisterModelFromExperiment {
        experiment_id: ExperimentId,
        name: String,
        version: String,
    },
    AddModelVersion {
        name: String,
        version: String,
        framework: String,
        source_model_id: ModelId,
    },
    PromoteToStaging {
        model_id: ModelId,
    },
    PromoteToProduction {
        model_id: ModelId,
    },
    DemoteToStaging {
        model_id: ModelId,
    },
    ArchiveModel {
        model_id: ModelId,
    },
    SetModelPath {
        model_id: ModelId,
        path: String,
    },
    SetModelMetadata {
        model_id: ModelId,
        key: String,
        value: serde_json::Value,
    },
    SetModelDescription {
        model_id: ModelId,
        description: String,
    },
    AddModelTag {
        model_id: ModelId,
        tag: String,
    },
    RemoveModelTag {
        model_id: ModelId,
        tag: String,
    },
    AddModelAlias {
        model_id: ModelId,
        alias: String,
    },
    RemoveModelAlias {
        model_id: ModelId,
        alias: String,
    },
    DeleteModel {
        model_id: ModelId,
    },
}
