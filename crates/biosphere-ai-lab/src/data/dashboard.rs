use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityDashboard {
    pub generated_at: String,
    pub overview: DashboardOverview,
    pub quality_timeline: Vec<QualitySnapshot>,
    pub dataset_rankings: Vec<DatasetRanking>,
    pub alerts: Vec<QualityAlert>,
    pub trends: QualityTrends,
    pub lineage_graph: LineageGraphData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub total_datasets: usize,
    pub total_samples: usize,
    pub total_size_bytes: u64,
    pub avg_quality_score: f64,
    pub datasets_with_issues: usize,
    pub healthy_datasets: usize,
    pub critical_datasets: usize,
    pub quality_distribution: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub timestamp: String,
    pub dataset_id: String,
    pub dataset_name: String,
    pub quality_score: f64,
    pub completeness: f64,
    pub consistency: f64,
    pub uniqueness: f64,
    pub label_quality: Option<f64>,
    pub drift_detected: bool,
    pub bias_level: Option<String>,
    pub sample_count: usize,
    pub issue_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRanking {
    pub rank: usize,
    pub dataset_id: String,
    pub dataset_name: String,
    pub quality_score: f64,
    pub trend: TrendDirection,
    pub top_issue: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Unknown,
}

impl std::fmt::Display for TrendDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Improving => write!(f, "improving"),
            Self::Stable => write!(f, "stable"),
            Self::Declining => write!(f, "declining"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAlert {
    pub alert_id: String,
    pub dataset_id: String,
    pub dataset_name: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub detected_at: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    DriftDetected,
    QualityDrop,
    BiasDetected,
    LabelNoise,
    MissingData,
    SchemaChange,
    DuplicateData,
    OutlierSurge,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DriftDetected => write!(f, "drift_detected"),
            Self::QualityDrop => write!(f, "quality_drop"),
            Self::BiasDetected => write!(f, "bias_detected"),
            Self::LabelNoise => write!(f, "label_noise"),
            Self::MissingData => write!(f, "missing_data"),
            Self::SchemaChange => write!(f, "schema_change"),
            Self::DuplicateData => write!(f, "duplicate_data"),
            Self::OutlierSurge => write!(f, "outlier_surge"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrends {
    pub overall_quality_trend: Vec<TrendPoint>,
    pub completeness_trend: Vec<TrendPoint>,
    pub label_quality_trend: Vec<TrendPoint>,
    pub drift_frequency: Vec<TrendPoint>,
    pub issue_resolution_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: String,
    pub value: f64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraphData {
    pub nodes: Vec<LineageNodeData>,
    pub edges: Vec<LineageEdgeData>,
    pub layout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNodeData {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub status: String,
    pub metrics: HashMap<String, f64>,
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdgeData {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub edge_type: String,
    pub animated: bool,
}

pub struct DashboardBuilder;

impl DashboardBuilder {
    pub fn build(
        snapshots: &[QualitySnapshot],
        alerts: &[QualityAlert],
        lineage: Option<&LineageGraphData>,
    ) -> DataQualityDashboard {
        let _total = snapshots.len();

        let latest_snapshots: HashMap<&str, &QualitySnapshot> = {
            let mut map: HashMap<&str, &QualitySnapshot> = HashMap::new();
            for snap in snapshots {
                map.entry(&snap.dataset_id)
                    .and_modify(|existing| {
                        if snap.timestamp > existing.timestamp {
                            *existing = snap;
                        }
                    })
                    .or_insert(snap);
            }
            map
        };

        let unique_datasets = latest_snapshots.len();
        let total_samples: usize = latest_snapshots.values().map(|s| s.sample_count).sum();
        let avg_quality = if unique_datasets > 0 {
            latest_snapshots.values().map(|s| s.quality_score).sum::<f64>() / unique_datasets as f64
        } else {
            0.0
        };

        let datasets_with_issues = latest_snapshots.values()
            .filter(|s| s.issue_count > 0)
            .count();
        let healthy = unique_datasets - datasets_with_issues;
        let critical = alerts.iter()
            .filter(|a| a.severity == AlertSeverity::Critical && !a.acknowledged)
            .count();

        let mut quality_dist = HashMap::new();
        for snap in latest_snapshots.values() {
            let bucket = if snap.quality_score >= 0.9 {
                "excellent"
            } else if snap.quality_score >= 0.7 {
                "good"
            } else if snap.quality_score >= 0.5 {
                "fair"
            } else {
                "poor"
            };
            *quality_dist.entry(bucket.to_string()).or_insert(0) += 1;
        }

        let overview = DashboardOverview {
            total_datasets: unique_datasets,
            total_samples,
            total_size_bytes: 0,
            avg_quality_score: avg_quality,
            datasets_with_issues,
            healthy_datasets: healthy,
            critical_datasets: critical,
            quality_distribution: quality_dist,
        };

        let mut rankings: Vec<DatasetRanking> = latest_snapshots.values()
            .map(|s| DatasetRanking {
                rank: 0,
                dataset_id: s.dataset_id.clone(),
                dataset_name: s.dataset_name.clone(),
                quality_score: s.quality_score,
                trend: TrendDirection::Unknown,
                top_issue: None,
            })
            .collect();
        rankings.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, r) in rankings.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        let overall_trend: Vec<TrendPoint> = snapshots.iter()
            .map(|s| TrendPoint {
                date: s.timestamp[..10].to_string(),
                value: s.quality_score,
                label: Some(s.dataset_name.clone()),
            })
            .collect();

        let completeness_trend: Vec<TrendPoint> = snapshots.iter()
            .map(|s| TrendPoint {
                date: s.timestamp[..10].to_string(),
                value: s.completeness,
                label: Some(s.dataset_name.clone()),
            })
            .collect();

        let label_trend: Vec<TrendPoint> = snapshots.iter()
            .filter_map(|s| s.label_quality.map(|lq| TrendPoint {
                date: s.timestamp[..10].to_string(),
                value: lq,
                label: Some(s.dataset_name.clone()),
            }))
            .collect();

        let drift_freq: Vec<TrendPoint> = snapshots.iter()
            .map(|s| TrendPoint {
                date: s.timestamp[..10].to_string(),
                value: if s.drift_detected { 1.0 } else { 0.0 },
                label: Some(s.dataset_name.clone()),
            })
            .collect();

        let resolved = alerts.iter().filter(|a| a.acknowledged).count();
        let issue_resolution_rate = if alerts.is_empty() {
            1.0
        } else {
            resolved as f64 / alerts.len() as f64
        };

        let trends = QualityTrends {
            overall_quality_trend: overall_trend,
            completeness_trend,
            label_quality_trend: label_trend,
            drift_frequency: drift_freq,
            issue_resolution_rate,
        };

        let lineage_graph = lineage.cloned().unwrap_or(LineageGraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
            layout: Some("dagre".to_string()),
        });

        DataQualityDashboard {
            generated_at: chrono::Utc::now().to_rfc3339(),
            overview,
            quality_timeline: snapshots.to_vec(),
            dataset_rankings: rankings,
            alerts: alerts.to_vec(),
            trends,
            lineage_graph,
        }
    }

    pub fn create_alert(
        dataset_id: &str,
        dataset_name: &str,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: &str,
    ) -> QualityAlert {
        QualityAlert {
            alert_id: format!("alert_{}_{}", dataset_id, chrono::Utc::now().timestamp_millis()),
            dataset_id: dataset_id.to_string(),
            dataset_name: dataset_name.to_string(),
            alert_type,
            severity,
            message: message.to_string(),
            detected_at: chrono::Utc::now().to_rfc3339(),
            acknowledged: false,
        }
    }

    pub fn create_snapshot(
        dataset_id: &str,
        dataset_name: &str,
        quality_score: f64,
        completeness: f64,
        consistency: f64,
        uniqueness: f64,
        label_quality: Option<f64>,
        drift_detected: bool,
        bias_level: Option<&str>,
        sample_count: usize,
        issue_count: usize,
    ) -> QualitySnapshot {
        QualitySnapshot {
            timestamp: chrono::Utc::now().to_rfc3339(),
            dataset_id: dataset_id.to_string(),
            dataset_name: dataset_name.to_string(),
            quality_score,
            completeness,
            consistency,
            uniqueness,
            label_quality,
            drift_detected,
            bias_level: bias_level.map(|s| s.to_string()),
            sample_count,
            issue_count,
        }
    }

    pub fn build_lineage_from_graph(
        graph: &crate::data::lineage::LineageGraph,
    ) -> LineageGraphData {
        let nodes: Vec<LineageNodeData> = graph.nodes.iter()
            .map(|n| LineageNodeData {
                id: n.id.clone(),
                label: n.name.clone(),
                node_type: n.node_type.to_string(),
                status: "active".to_string(),
                metrics: HashMap::new(),
                position: None,
            })
            .collect();

        let edges: Vec<LineageEdgeData> = graph.edges.iter()
            .enumerate()
            .map(|(i, e)| LineageEdgeData {
                id: format!("edge_{}", i),
                source: e.from.clone(),
                target: e.to.clone(),
                label: e.transform.clone(),
                edge_type: e.relation.to_string(),
                animated: false,
            })
            .collect();

        LineageGraphData {
            nodes,
            edges,
            layout: Some("dagre".to_string()),
        }
    }
}
