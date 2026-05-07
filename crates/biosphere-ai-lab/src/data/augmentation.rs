use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentationPipeline {
    pub name: String,
    pub target_type: AugmentationTarget,
    pub steps: Vec<AugmentationStep>,
    pub probability: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentationTarget {
    Image,
    Text,
    Tabular,
    Audio,
}

impl std::fmt::Display for AugmentationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image => write!(f, "image"),
            Self::Text => write!(f, "text"),
            Self::Tabular => write!(f, "tabular"),
            Self::Audio => write!(f, "audio"),
        }
    }
}

impl std::str::FromStr for AugmentationTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "image" => Ok(Self::Image),
            "text" => Ok(Self::Text),
            "tabular" => Ok(Self::Tabular),
            "audio" => Ok(Self::Audio),
            _ => Err(format!("Unknown augmentation target: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentationStep {
    pub operation: AugmentationOp,
    pub params: serde_json::Value,
    pub probability: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AugmentationOp {
    RandomHorizontalFlip,
    RandomVerticalFlip,
    RandomRotation,
    RandomCrop,
    RandomBrightness,
    RandomContrast,
    RandomNoise,
    RandomErasing,
    SynonymReplacement,
    RandomDeletion,
    RandomSwap,
    BackTranslation,
    GaussianNoise,
    FeatureDropout,
    Mixup,
    Cutmix,
    Smote,
}

impl std::fmt::Display for AugmentationOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RandomHorizontalFlip => write!(f, "random_horizontal_flip"),
            Self::RandomVerticalFlip => write!(f, "random_vertical_flip"),
            Self::RandomRotation => write!(f, "random_rotation"),
            Self::RandomCrop => write!(f, "random_crop"),
            Self::RandomBrightness => write!(f, "random_brightness"),
            Self::RandomContrast => write!(f, "random_contrast"),
            Self::RandomNoise => write!(f, "random_noise"),
            Self::RandomErasing => write!(f, "random_erasing"),
            Self::SynonymReplacement => write!(f, "synonym_replacement"),
            Self::RandomDeletion => write!(f, "random_deletion"),
            Self::RandomSwap => write!(f, "random_swap"),
            Self::BackTranslation => write!(f, "back_translation"),
            Self::GaussianNoise => write!(f, "gaussian_noise"),
            Self::FeatureDropout => write!(f, "feature_dropout"),
            Self::Mixup => write!(f, "mixup"),
            Self::Cutmix => write!(f, "cutmix"),
            Self::Smote => write!(f, "smote"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentationPreset {
    pub name: String,
    pub description: String,
    pub target: AugmentationTarget,
    pub pipeline: AugmentationPipeline,
}

pub struct AugmentationPresets;

impl AugmentationPresets {
    pub fn image_classification_light() -> AugmentationPreset {
        AugmentationPreset {
            name: "image_classification_light".to_string(),
            description: "轻量图像分类增强：水平翻转 + 轻微亮度/对比度".to_string(),
            target: AugmentationTarget::Image,
            pipeline: AugmentationPipeline {
                name: "image_light".to_string(),
                target_type: AugmentationTarget::Image,
                probability: 0.5,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::RandomHorizontalFlip,
                        params: serde_json::json!({}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomBrightness,
                        params: serde_json::json!({"delta": 0.1}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomContrast,
                        params: serde_json::json!({"delta": 0.1}),
                        probability: 0.3,
                    },
                ],
            },
        }
    }

    pub fn image_classification_standard() -> AugmentationPreset {
        AugmentationPreset {
            name: "image_classification_standard".to_string(),
            description: "标准图像分类增强：翻转 + 旋转 + 裁剪 + 亮度/对比度/噪声".to_string(),
            target: AugmentationTarget::Image,
            pipeline: AugmentationPipeline {
                name: "image_standard".to_string(),
                target_type: AugmentationTarget::Image,
                probability: 0.8,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::RandomHorizontalFlip,
                        params: serde_json::json!({}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomRotation,
                        params: serde_json::json!({"degrees": 15}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomCrop,
                        params: serde_json::json!({"padding": 4}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomBrightness,
                        params: serde_json::json!({"delta": 0.2}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomContrast,
                        params: serde_json::json!({"delta": 0.2}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomNoise,
                        params: serde_json::json!({"std": 0.05}),
                        probability: 0.2,
                    },
                ],
            },
        }
    }

    pub fn image_classification_heavy() -> AugmentationPreset {
        AugmentationPreset {
            name: "image_classification_heavy".to_string(),
            description: "重度图像分类增强：全部图像增强操作 + CutMix/MixUp".to_string(),
            target: AugmentationTarget::Image,
            pipeline: AugmentationPipeline {
                name: "image_heavy".to_string(),
                target_type: AugmentationTarget::Image,
                probability: 1.0,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::RandomHorizontalFlip,
                        params: serde_json::json!({}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomVerticalFlip,
                        params: serde_json::json!({}),
                        probability: 0.2,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomRotation,
                        params: serde_json::json!({"degrees": 30}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomCrop,
                        params: serde_json::json!({"padding": 8}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomBrightness,
                        params: serde_json::json!({"delta": 0.3}),
                        probability: 0.4,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomContrast,
                        params: serde_json::json!({"delta": 0.3}),
                        probability: 0.4,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomNoise,
                        params: serde_json::json!({"std": 0.1}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomErasing,
                        params: serde_json::json!({"scale": [0.02, 0.1], "ratio": [0.3, 3.3]}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::Cutmix,
                        params: serde_json::json!({"alpha": 1.0}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::Mixup,
                        params: serde_json::json!({"alpha": 0.2}),
                        probability: 0.5,
                    },
                ],
            },
        }
    }

    pub fn text_classification_light() -> AugmentationPreset {
        AugmentationPreset {
            name: "text_classification_light".to_string(),
            description: "轻量文本增强：同义词替换 + 随机删除".to_string(),
            target: AugmentationTarget::Text,
            pipeline: AugmentationPipeline {
                name: "text_light".to_string(),
                target_type: AugmentationTarget::Text,
                probability: 0.3,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::SynonymReplacement,
                        params: serde_json::json!({"ratio": 0.1}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomDeletion,
                        params: serde_json::json!({"ratio": 0.05}),
                        probability: 0.3,
                    },
                ],
            },
        }
    }

    pub fn text_classification_standard() -> AugmentationPreset {
        AugmentationPreset {
            name: "text_classification_standard".to_string(),
            description: "标准文本增强：同义词替换 + 随机删除 + 随机交换".to_string(),
            target: AugmentationTarget::Text,
            pipeline: AugmentationPipeline {
                name: "text_standard".to_string(),
                target_type: AugmentationTarget::Text,
                probability: 0.5,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::SynonymReplacement,
                        params: serde_json::json!({"ratio": 0.15}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomDeletion,
                        params: serde_json::json!({"ratio": 0.1}),
                        probability: 0.3,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::RandomSwap,
                        params: serde_json::json!({"ratio": 0.1}),
                        probability: 0.3,
                    },
                ],
            },
        }
    }

    pub fn tabular_standard() -> AugmentationPreset {
        AugmentationPreset {
            name: "tabular_standard".to_string(),
            description: "标准表格数据增强：高斯噪声 + 特征Dropout".to_string(),
            target: AugmentationTarget::Tabular,
            pipeline: AugmentationPipeline {
                name: "tabular_standard".to_string(),
                target_type: AugmentationTarget::Tabular,
                probability: 0.3,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::GaussianNoise,
                        params: serde_json::json!({"std": 0.01}),
                        probability: 0.5,
                    },
                    AugmentationStep {
                        operation: AugmentationOp::FeatureDropout,
                        params: serde_json::json!({"ratio": 0.05}),
                        probability: 0.3,
                    },
                ],
            },
        }
    }

    pub fn tabular_imbalanced() -> AugmentationPreset {
        AugmentationPreset {
            name: "tabular_imbalanced".to_string(),
            description: "不平衡表格数据增强：SMOTE 过采样".to_string(),
            target: AugmentationTarget::Tabular,
            pipeline: AugmentationPipeline {
                name: "tabular_smote".to_string(),
                target_type: AugmentationTarget::Tabular,
                probability: 1.0,
                enabled: true,
                steps: vec![
                    AugmentationStep {
                        operation: AugmentationOp::Smote,
                        params: serde_json::json!({"k_neighbors": 5, "sampling_ratio": 1.0}),
                        probability: 1.0,
                    },
                ],
            },
        }
    }

    pub fn all_presets() -> Vec<AugmentationPreset> {
        vec![
            Self::image_classification_light(),
            Self::image_classification_standard(),
            Self::image_classification_heavy(),
            Self::text_classification_light(),
            Self::text_classification_standard(),
            Self::tabular_standard(),
            Self::tabular_imbalanced(),
        ]
    }

    pub fn presets_for_target(target: AugmentationTarget) -> Vec<AugmentationPreset> {
        Self::all_presets()
            .into_iter()
            .filter(|p| p.target == target)
            .collect()
    }
}
