use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
    pub metadata: LineageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: String,
    pub name: String,
    pub node_type: LineageNodeType,
    pub version: String,
    pub digest: Option<String>,
    pub created_at: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageNodeType {
    RawData,
    ProcessedData,
    Dataset,
    Split,
    Model,
    Experiment,
    Checkpoint,
    Export,
}

impl std::fmt::Display for LineageNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawData => write!(f, "raw_data"),
            Self::ProcessedData => write!(f, "processed_data"),
            Self::Dataset => write!(f, "dataset"),
            Self::Split => write!(f, "split"),
            Self::Model => write!(f, "model"),
            Self::Experiment => write!(f, "experiment"),
            Self::Checkpoint => write!(f, "checkpoint"),
            Self::Export => write!(f, "export"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub from: String,
    pub to: String,
    pub relation: LineageRelation,
    pub transform: Option<String>,
    pub params: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageRelation {
    DerivedFrom,
    TrainedOn,
    EvaluatedOn,
    SplitFrom,
    PreprocessedFrom,
    AugmentedFrom,
    ExportedFrom,
    DependsOn,
}

impl std::fmt::Display for LineageRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DerivedFrom => write!(f, "derived_from"),
            Self::TrainedOn => write!(f, "trained_on"),
            Self::EvaluatedOn => write!(f, "evaluated_on"),
            Self::SplitFrom => write!(f, "split_from"),
            Self::PreprocessedFrom => write!(f, "preprocessed_from"),
            Self::AugmentedFrom => write!(f, "augmented_from"),
            Self::ExportedFrom => write!(f, "exported_from"),
            Self::DependsOn => write!(f, "depends_on"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageMetadata {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_depth: usize,
    pub has_cycles: bool,
    pub root_nodes: Vec<String>,
    pub leaf_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageTrace {
    pub target_id: String,
    pub target_name: String,
    pub upstream: Vec<LineageNode>,
    pub downstream: Vec<LineageNode>,
    pub full_path: Vec<String>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub changed_node: String,
    pub changed_node_name: String,
    pub directly_affected: Vec<String>,
    pub indirectly_affected: Vec<String>,
    pub total_affected: usize,
    pub severity: ImpactSeverity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ImpactSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityReport {
    pub experiment_id: String,
    pub is_reproducible: bool,
    pub missing_inputs: Vec<String>,
    pub missing_transforms: Vec<String>,
    pub data_snapshot_available: bool,
    pub code_version_available: bool,
    pub environment_available: bool,
    pub score: f64,
    pub recommendations: Vec<String>,
}

pub struct LineageTracker {
    nodes: Vec<LineageNode>,
    edges: Vec<LineageEdge>,
}

impl LineageTracker {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(
        &mut self,
        id: &str,
        name: &str,
        node_type: LineageNodeType,
        version: &str,
        digest: Option<&str>,
    ) -> &mut Self {
        self.nodes.push(LineageNode {
            id: id.to_string(),
            name: name.to_string(),
            node_type,
            version: version.to_string(),
            digest: digest.map(|d| d.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        self
    }

    pub fn add_edge(
        &mut self,
        from: &str,
        to: &str,
        relation: LineageRelation,
        transform: Option<&str>,
        params: Option<serde_json::Value>,
    ) -> &mut Self {
        self.edges.push(LineageEdge {
            from: from.to_string(),
            to: to.to_string(),
            relation,
            transform: transform.map(|t| t.to_string()),
            params,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        self
    }

    pub fn build(self) -> Result<LineageGraph, String> {
        let node_ids: HashSet<String> = self.nodes.iter().map(|n| n.id.clone()).collect();

        for edge in &self.edges {
            if !node_ids.contains(&edge.from) {
                return Err(format!("Edge references unknown source node: {}", edge.from));
            }
            if !node_ids.contains(&edge.to) {
                return Err(format!("Edge references unknown target node: {}", edge.to));
            }
        }

        let has_cycles = Self::detect_cycles(&self.nodes, &self.edges);

        let in_degree: HashMap<String, usize> = self.nodes.iter()
            .map(|n| {
                let count = self.edges.iter().filter(|e| e.to == n.id).count();
                (n.id.clone(), count)
            })
            .collect();

        let root_nodes: Vec<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let out_degree: HashMap<String, usize> = self.nodes.iter()
            .map(|n| {
                let count = self.edges.iter().filter(|e| e.from == n.id).count();
                (n.id.clone(), count)
            })
            .collect();

        let leaf_nodes: Vec<String> = out_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let max_depth = Self::compute_max_depth(&self.nodes, &self.edges);

        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();

        Ok(LineageGraph {
            nodes: self.nodes,
            edges: self.edges,
            metadata: LineageMetadata {
                total_nodes,
                total_edges,
                max_depth,
                has_cycles,
                root_nodes,
                leaf_nodes,
            },
        })
    }

    fn detect_cycles(nodes: &[LineageNode], edges: &[LineageEdge]) -> bool {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in nodes {
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in edges {
            adj.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
        }

        let mut visited: HashSet<&str> = HashSet::new();
        let mut in_stack: HashSet<&str> = HashSet::new();

        fn dfs<'a>(
            node: &'a str,
            adj: &HashMap<&str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
            in_stack: &mut HashSet<&'a str>,
        ) -> bool {
            visited.insert(node);
            in_stack.insert(node);
            if let Some(neighbors) = adj.get(node) {
                for &next in neighbors {
                    if !visited.contains(next) {
                        if dfs(next, adj, visited, in_stack) {
                            return true;
                        }
                    } else if in_stack.contains(next) {
                        return true;
                    }
                }
            }
            in_stack.remove(node);
            false
        }

        for node in nodes {
            if !visited.contains(node.id.as_str()) {
                if dfs(node.id.as_str(), &adj, &mut visited, &mut in_stack) {
                    return true;
                }
            }
        }
        false
    }

    fn compute_max_depth(nodes: &[LineageNode], edges: &[LineageEdge]) -> usize {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
            adj.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
        }

        let mut queue: VecDeque<&str> = VecDeque::new();
        let mut depth: HashMap<&str, usize> = HashMap::new();

        for node in nodes {
            if in_degree[node.id.as_str()] == 0 {
                queue.push_back(node.id.as_str());
                depth.insert(node.id.as_str(), 0);
            }
        }

        let mut max_depth = 0;
        while let Some(current) = queue.pop_front() {
            let current_depth = *depth.get(current).unwrap_or(&0);
            max_depth = max_depth.max(current_depth);
            if let Some(neighbors) = adj.get(current) {
                for &next in neighbors {
                    let entry = in_degree.get_mut(next).unwrap();
                    *entry -= 1;
                    let new_depth = current_depth + 1;
                    depth.entry(next)
                        .and_modify(|d| *d = (*d).max(new_depth))
                        .or_insert(new_depth);
                    if *entry == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }
        max_depth
    }
}

impl LineageGraph {
    pub fn trace_upstream(&self, node_id: &str) -> LineageTrace {
        let target = self.nodes.iter().find(|n| n.id == node_id);
        let target_name = target.map(|n| n.name.clone()).unwrap_or_default();

        let mut upstream = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(node_id);

        while let Some(current) = queue.pop_front() {
            for edge in &self.edges {
                if edge.to == current && !visited.contains(edge.from.as_str()) {
                    visited.insert(edge.from.as_str());
                    if let Some(node) = self.nodes.iter().find(|n| n.id == edge.from) {
                        upstream.push(node.clone());
                    }
                    queue.push_back(edge.from.as_str());
                }
            }
        }

        let mut downstream = Vec::new();
        let mut visited_d: HashSet<&str> = HashSet::new();
        let mut queue_d: VecDeque<&str> = VecDeque::new();
        queue_d.push_back(node_id);

        while let Some(current) = queue_d.pop_front() {
            for edge in &self.edges {
                if edge.from == current && !visited_d.contains(edge.to.as_str()) {
                    visited_d.insert(edge.to.as_str());
                    if let Some(node) = self.nodes.iter().find(|n| n.id == edge.to) {
                        downstream.push(node.clone());
                    }
                    queue_d.push_back(edge.to.as_str());
                }
            }
        }

        let depth = upstream.len().max(downstream.len());
        let mut full_path: Vec<String> = upstream.iter().rev().map(|n| n.name.clone()).collect();
        full_path.push(target_name.clone());
        full_path.extend(downstream.iter().map(|n| n.name.clone()));

        LineageTrace {
            target_id: node_id.to_string(),
            target_name,
            upstream,
            downstream,
            full_path,
            depth,
        }
    }

    pub fn analyze_impact(&self, node_id: &str) -> ImpactAnalysis {
        let target = self.nodes.iter().find(|n| n.id == node_id);
        let changed_node_name = target.map(|n| n.name.clone()).unwrap_or_default();

        let mut directly_affected = Vec::new();
        let mut indirectly_affected = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(&str, usize)> = VecDeque::new();
        queue.push_back((node_id, 0));

        while let Some((current, distance)) = queue.pop_front() {
            for edge in &self.edges {
                if edge.from == current && !visited.contains(edge.to.as_str()) {
                    visited.insert(edge.to.as_str());
                    if distance == 0 {
                        directly_affected.push(edge.to.clone());
                    } else {
                        indirectly_affected.push(edge.to.clone());
                    }
                    queue.push_back((edge.to.as_str(), distance + 1));
                }
            }
        }

        let total_affected = directly_affected.len() + indirectly_affected.len();

        let severity = match total_affected {
            0 => ImpactSeverity::Low,
            1..=3 => ImpactSeverity::Medium,
            4..=10 => ImpactSeverity::High,
            _ => ImpactSeverity::Critical,
        };

        let mut recommendations = Vec::new();
        match severity {
            ImpactSeverity::Critical => {
                recommendations.push(format!(
                    "🔴 严重：变更 '{}' 将影响 {} 个下游节点，建议在隔离环境中先验证",
                    changed_node_name, total_affected
                ));
            }
            ImpactSeverity::High => {
                recommendations.push(format!(
                    "🟠 高影响：变更将影响 {} 个节点，建议重新运行受影响的实验",
                    total_affected
                ));
            }
            ImpactSeverity::Medium => {
                recommendations.push(format!(
                    "🟡 中等影响：{} 个节点受影响，建议检查关键下游节点",
                    total_affected
                ));
            }
            ImpactSeverity::Low => {
                recommendations.push("🟢 低影响：变更影响范围有限".to_string());
            }
        }

        ImpactAnalysis {
            changed_node: node_id.to_string(),
            changed_node_name,
            directly_affected,
            indirectly_affected,
            total_affected,
            severity,
            recommendations,
        }
    }

    pub fn check_reproducibility(
        &self,
        experiment_id: &str,
        available_data: &HashSet<String>,
        available_code: bool,
        available_env: bool,
    ) -> ReproducibilityReport {
        let trace = self.trace_upstream(experiment_id);

        let mut missing_inputs = Vec::new();
        let mut missing_transforms = Vec::new();

        for node in &trace.upstream {
            if let Some(ref digest) = node.digest {
                if !available_data.contains(digest) {
                    missing_inputs.push(format!("{} (v{})", node.name, node.version));
                }
            }
        }

        for edge in &self.edges {
            if trace.upstream.iter().any(|n| n.id == edge.from)
                || trace.downstream.iter().any(|n| n.id == edge.to)
                || edge.from == experiment_id
                || edge.to == experiment_id
            {
                if let Some(ref transform) = edge.transform {
                    missing_transforms.push(transform.clone());
                }
            }
        }

        let data_available = missing_inputs.is_empty();
        let is_reproducible = data_available && available_code && available_env;

        let score = if is_reproducible {
            100.0
        } else {
            let mut s = 0.0;
            if data_available { s += 40.0; }
            if available_code { s += 30.0; }
            if available_env { s += 30.0; }
            s
        };

        let mut recommendations = Vec::new();
        if !data_available {
            recommendations.push(format!(
                "缺失 {} 个数据依赖，请确保原始数据未被删除或移动",
                missing_inputs.len()
            ));
        }
        if !available_code {
            recommendations.push("缺少代码版本信息，建议使用 Git 管理训练代码".to_string());
        }
        if !available_env {
            recommendations.push("缺少环境信息，建议记录 Python 版本和依赖列表".to_string());
        }

        ReproducibilityReport {
            experiment_id: experiment_id.to_string(),
            is_reproducible,
            missing_inputs,
            missing_transforms,
            data_snapshot_available: data_available,
            code_version_available: available_code,
            environment_available: available_env,
            score,
            recommendations,
        }
    }

    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph LR\n");

        for node in &self.nodes {
            let icon = match node.node_type {
                LineageNodeType::RawData => "📥",
                LineageNodeType::ProcessedData => "⚙️",
                LineageNodeType::Dataset => "📊",
                LineageNodeType::Split => "✂️",
                LineageNodeType::Model => "🧠",
                LineageNodeType::Experiment => "🔬",
                LineageNodeType::Checkpoint => "💾",
                LineageNodeType::Export => "📤",
            };
            output.push_str(&format!(
                "    {}[\"{} {}<br/>v{}\"]\n",
                node.id, icon, node.name, node.version
            ));
        }

        for edge in &self.edges {
            let label = match edge.relation {
                LineageRelation::DerivedFrom => "derives",
                LineageRelation::TrainedOn => "trains on",
                LineageRelation::EvaluatedOn => "evaluates",
                LineageRelation::SplitFrom => "splits",
                LineageRelation::PreprocessedFrom => "preprocesses",
                LineageRelation::AugmentedFrom => "augments",
                LineageRelation::ExportedFrom => "exports",
                LineageRelation::DependsOn => "depends",
            };
            output.push_str(&format!(
                "    {} -->|\"{}\"| {}\n",
                edge.from, label, edge.to
            ));
        }

        output
    }
}

pub fn create_training_lineage(
    raw_data_id: &str,
    raw_data_name: &str,
    dataset_id: &str,
    dataset_name: &str,
    split_id: &str,
    experiment_id: &str,
    experiment_name: &str,
    model_id: &str,
    model_name: &str,
    raw_digest: &str,
    dataset_digest: &str,
) -> Result<LineageGraph, String> {
    let mut tracker = LineageTracker::new();

    tracker.add_node(raw_data_id, raw_data_name, LineageNodeType::RawData, "v1", Some(raw_digest));
    tracker.add_node(dataset_id, dataset_name, LineageNodeType::Dataset, "v1", Some(dataset_digest));
    tracker.add_node(split_id, &format!("{}_split", dataset_name), LineageNodeType::Split, "v1", None);
    tracker.add_node(experiment_id, experiment_name, LineageNodeType::Experiment, "v1", None);
    tracker.add_node(model_id, model_name, LineageNodeType::Model, "v1", None);
    tracker.add_edge(raw_data_id, dataset_id, LineageRelation::DerivedFrom, Some("import"), None);
    tracker.add_edge(dataset_id, split_id, LineageRelation::SplitFrom, Some("train_test_split"), None);
    tracker.add_edge(split_id, experiment_id, LineageRelation::TrainedOn, Some("training"), None);
    tracker.add_edge(experiment_id, model_id, LineageRelation::DerivedFrom, Some("checkpoint"), None);

    tracker.build()
}
