use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelId(String);

impl ModelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ModelId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    #[serde(alias = "None")]
    None,
    #[serde(alias = "Staging")]
    Staging,
    #[serde(alias = "Production")]
    Production,
    #[serde(alias = "Archived")]
    Archived,
}

impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSpec {
    pub name: String,
    pub dtype: String,
    pub shape: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSignature {
    pub inputs: Vec<TensorSpec>,
    pub outputs: Vec<TensorSpec>,
}

impl ModelSignature {
    pub fn new(inputs: Vec<TensorSpec>, outputs: Vec<TensorSpec>) -> Self {
        Self { inputs, outputs }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetLineage {
    pub dataset_id: String,
    pub dataset_name: Option<String>,
    pub dataset_version: Option<String>,
    pub split_name: Option<String>,
    pub data_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLineage {
    pub experiment_id: Option<String>,
    pub experiment_name: Option<String>,
    pub training_config: Option<serde_json::Value>,
    pub parent_model_id: Option<String>,
    pub dataset: Option<String>,
    pub datasets: Vec<DatasetLineage>,
    pub preprocessing_pipeline: Option<String>,
    pub split_name: Option<String>,
}

impl ModelLineage {
    pub fn from_experiment(experiment_id: &str, experiment_name: &str) -> Self {
        Self {
            experiment_id: Some(experiment_id.to_string()),
            experiment_name: Some(experiment_name.to_string()),
            training_config: None,
            parent_model_id: None,
            dataset: None,
            datasets: Vec::new(),
            preprocessing_pipeline: None,
            split_name: None,
        }
    }
}

impl Default for ModelLineage {
    fn default() -> Self {
        Self {
            experiment_id: None,
            experiment_name: None,
            training_config: None,
            parent_model_id: None,
            dataset: None,
            datasets: Vec::new(),
            preprocessing_pipeline: None,
            split_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistration {
    pub id: ModelId,
    pub name: String,
    pub version: String,
    pub status: ModelStatus,
    pub framework: String,
    pub path: Option<String>,
    pub signature: Option<serde_json::Value>,
    pub structured_signature: Option<ModelSignature>,
    pub lineage: Option<ModelLineage>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ModelRegistration {
    pub fn new(name: String, version: String, framework: String) -> Self {
        let now = Utc::now();
        Self {
            id: ModelId::new(),
            name,
            version,
            status: ModelStatus::None,
            framework,
            path: None,
            signature: None,
            structured_signature: None,
            lineage: None,
            metadata: HashMap::new(),
            description: None,
            tags: Vec::new(),
            aliases: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_signature(mut self, signature: ModelSignature) -> Self {
        self.structured_signature = Some(signature.clone());
        self.signature = Some(serde_json::to_value(&signature).unwrap_or_default());
        self.updated_at = Utc::now();
        self
    }

    pub fn with_lineage(mut self, lineage: ModelLineage) -> Self {
        self.lineage = Some(lineage);
        self.updated_at = Utc::now();
        self
    }

    pub fn promote_to_staging(&mut self) -> Result<(), String> {
        match self.status {
            ModelStatus::None | ModelStatus::Production => {
                self.status = ModelStatus::Staging;
                self.updated_at = Utc::now();
                Ok(())
            }
            ModelStatus::Staging => Err("Model is already in Staging".to_string()),
            ModelStatus::Archived => Err("Cannot promote archived model".to_string()),
        }
    }

    pub fn promote_to_production(&mut self) -> Result<(), String> {
        match self.status {
            ModelStatus::Staging => {
                self.status = ModelStatus::Production;
                self.updated_at = Utc::now();
                Ok(())
            }
            ModelStatus::None => Err("Model must be promoted to Staging first".to_string()),
            ModelStatus::Production => Err("Model is already in Production".to_string()),
            ModelStatus::Archived => Err("Cannot promote archived model".to_string()),
        }
    }

    pub fn demote_to_staging(&mut self) -> Result<(), String> {
        match self.status {
            ModelStatus::Production => {
                self.status = ModelStatus::Staging;
                self.updated_at = Utc::now();
                Ok(())
            }
            ModelStatus::Staging => Err("Model is already in Staging".to_string()),
            ModelStatus::None => Err("Model has no stage to demote from".to_string()),
            ModelStatus::Archived => Err("Cannot demote archived model".to_string()),
        }
    }

    pub fn archive(&mut self) -> Result<(), String> {
        match self.status {
            ModelStatus::None | ModelStatus::Staging | ModelStatus::Production => {
                self.status = ModelStatus::Archived;
                self.updated_at = Utc::now();
                Ok(())
            }
            ModelStatus::Archived => Err("Model is already archived".to_string()),
        }
    }

    pub fn add_alias(&mut self, alias: String) {
        if !self.aliases.contains(&alias) {
            self.aliases.push(alias);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_alias(&mut self, alias: &str) {
        self.aliases.retain(|a| a != alias);
        self.updated_at = Utc::now();
    }

    pub fn set_path(&mut self, path: String) {
        self.path = Some(path);
        self.updated_at = Utc::now();
    }

    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = Utc::now();
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = Utc::now();
    }

    pub fn bump_version(&mut self) -> String {
        let cleaned = self.version.trim_start_matches('v').trim_start_matches('V');
        let parts: Vec<&str> = cleaned.split('.').collect();
        let major = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let patch = if parts.len() >= 3 {
            parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0)
        } else if parts.len() == 2 {
            minor
        } else {
            0
        };
        let new_minor = if parts.len() < 3 { minor + 1 } else { minor };
        let new_patch = if parts.len() >= 3 { patch + 1 } else { 0 };
        let new_version = format!("{}.{}.{}", major, new_minor, new_patch);
        self.version = new_version.clone();
        self.updated_at = Utc::now();
        new_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_id_uniqueness() {
        let id1 = ModelId::new();
        let id2 = ModelId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_model_id_from_str() {
        let id = ModelId::from_str("model-123");
        assert_eq!(id.as_str(), "model-123");
    }

    #[test]
    fn test_model_registration_new() {
        let model = ModelRegistration::new("test-model".to_string(), "1.0.0".to_string(), "burn".to_string());
        assert_eq!(model.name, "test-model");
        assert_eq!(model.version, "1.0.0");
        assert_eq!(model.framework, "burn");
        assert_eq!(model.status, ModelStatus::None);
        assert!(model.path.is_none());
        assert!(model.signature.is_none());
    }

    #[test]
    fn test_model_status_transitions() {
        let mut model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string());

        assert!(model.promote_to_staging().is_ok());
        assert_eq!(model.status, ModelStatus::Staging);
        assert!(model.promote_to_staging().is_err());

        assert!(model.promote_to_production().is_ok());
        assert_eq!(model.status, ModelStatus::Production);
        assert!(model.promote_to_production().is_err());

        assert!(model.demote_to_staging().is_ok());
        assert_eq!(model.status, ModelStatus::Staging);

        assert!(model.promote_to_production().is_ok());
        assert!(model.archive().is_ok());
        assert_eq!(model.status, ModelStatus::Archived);
        assert!(model.archive().is_err());
        assert!(model.promote_to_staging().is_err());
    }

    #[test]
    fn test_model_with_signature() {
        let signature = ModelSignature::new(
            vec![TensorSpec { name: "input".to_string(), dtype: "float32".to_string(), shape: vec![1, 28, 28] }],
            vec![TensorSpec { name: "output".to_string(), dtype: "float32".to_string(), shape: vec![1, 10] }],
        );
        let model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string())
            .with_signature(signature);

        assert!(model.structured_signature.is_some());
        assert!(model.signature.is_some());
        assert_eq!(model.structured_signature.unwrap().inputs.len(), 1);
    }

    #[test]
    fn test_model_with_lineage() {
        let lineage = ModelLineage::from_experiment("exp-123", "test-exp");
        let model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string())
            .with_lineage(lineage);

        assert!(model.lineage.is_some());
        assert_eq!(model.lineage.unwrap().experiment_id, Some("exp-123".to_string()));
    }

    #[test]
    fn test_model_metadata() {
        let mut model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string());
        model.set_metadata("accuracy".to_string(), serde_json::json!(0.95));
        model.set_metadata("f1_score".to_string(), serde_json::json!(0.93));

        assert_eq!(model.metadata.len(), 2);
        assert_eq!(model.metadata["accuracy"], serde_json::json!(0.95));
    }

    #[test]
    fn test_model_tags() {
        let mut model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string());
        model.add_tag("baseline".to_string());
        model.add_tag("v1".to_string());
        model.add_tag("baseline".to_string());

        assert_eq!(model.tags.len(), 2);
        model.remove_tag("baseline");
        assert_eq!(model.tags.len(), 1);
    }

    #[test]
    fn test_model_aliases() {
        let mut model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string());
        model.add_alias("champion".to_string());
        model.add_alias("production-ready".to_string());
        model.add_alias("champion".to_string());

        assert_eq!(model.aliases.len(), 2);
        assert!(model.aliases.contains(&"champion".to_string()));
        model.remove_alias("champion");
        assert_eq!(model.aliases.len(), 1);
        assert!(!model.aliases.contains(&"champion".to_string()));
    }

    #[test]
    fn test_model_bump_version_semver() {
        let mut model = ModelRegistration::new("test".to_string(), "1.2.3".to_string(), "burn".to_string());
        let new_ver = model.bump_version();
        assert_eq!(new_ver, "1.2.4");
        assert_eq!(model.version, "1.2.4");
    }

    #[test]
    fn test_model_bump_version_two_part() {
        let mut model = ModelRegistration::new("test".to_string(), "2.5".to_string(), "burn".to_string());
        let new_ver = model.bump_version();
        assert_eq!(new_ver, "2.6.0");
    }

    #[test]
    fn test_model_bump_version_single() {
        let mut model = ModelRegistration::new("test".to_string(), "3".to_string(), "burn".to_string());
        let new_ver = model.bump_version();
        assert_eq!(new_ver, "3.1.0");
    }

    #[test]
    fn test_model_bump_version_with_v_prefix() {
        let mut model = ModelRegistration::new("test".to_string(), "v1.0.0".to_string(), "burn".to_string());
        let new_ver = model.bump_version();
        assert_eq!(new_ver, "1.0.1");
    }

    #[test]
    fn test_model_set_path() {
        let mut model = ModelRegistration::new("test".to_string(), "1.0.0".to_string(), "burn".to_string());
        model.set_path("/models/test.mpk.gz".to_string());
        assert_eq!(model.path, Some("/models/test.mpk.gz".to_string()));
    }

    #[test]
    fn test_model_status_display() {
        assert_eq!(ModelStatus::None.to_string(), "none");
        assert_eq!(ModelStatus::Staging.to_string(), "staging");
        assert_eq!(ModelStatus::Production.to_string(), "production");
        assert_eq!(ModelStatus::Archived.to_string(), "archived");
    }
}
