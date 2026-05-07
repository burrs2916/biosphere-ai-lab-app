use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDag {
    pub name: String,
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
    pub metadata: DagMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub name: String,
    pub node_type: DagNodeType,
    pub status: NodeStatus,
    pub input_artifacts: Vec<String>,
    pub output_artifacts: Vec<String>,
    pub config: serde_json::Value,
    pub last_run: Option<String>,
    pub last_digest: Option<String>,
    pub position: NodePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DagNodeType {
    DataSource,
    Preprocess,
    Split,
    Train,
    Evaluate,
    Export,
    Custom,
}

impl std::fmt::Display for DagNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataSource => write!(f, "data_source"),
            Self::Preprocess => write!(f, "preprocess"),
            Self::Split => write!(f, "split"),
            Self::Train => write!(f, "train"),
            Self::Evaluate => write!(f, "evaluate"),
            Self::Export => write!(f, "export"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Stale,
    Skipped,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Stale => write!(f, "stale"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdge {
    pub from: String,
    pub to: String,
    pub artifact: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Data,
    Model,
    Config,
    Trigger,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Data => write!(f, "data"),
            Self::Model => write!(f, "model"),
            Self::Config => write!(f, "config"),
            Self::Trigger => write!(f, "trigger"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagMetadata {
    pub created_at: String,
    pub updated_at: String,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_depth: usize,
    pub has_cycles: bool,
    pub entry_nodes: Vec<String>,
    pub exit_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeDetectionResult {
    pub node_id: String,
    pub has_changed: bool,
    pub changed_inputs: Vec<String>,
    pub downstream_affected: Vec<String>,
    pub needs_rerun: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub dag_name: String,
    pub execution_order: Vec<String>,
    pub parallel_groups: Vec<Vec<String>>,
    pub total_steps: usize,
    pub estimated_duration: String,
}

pub struct PipelineDagBuilder {
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
    name: String,
}

impl PipelineDagBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            name: name.to_string(),
        }
    }

    pub fn add_node(
        mut self,
        id: &str,
        name: &str,
        node_type: DagNodeType,
        inputs: Vec<String>,
        outputs: Vec<String>,
    ) -> Self {
        self.nodes.push(DagNode {
            id: id.to_string(),
            name: name.to_string(),
            node_type,
            status: NodeStatus::Pending,
            input_artifacts: inputs,
            output_artifacts: outputs,
            config: serde_json::Value::Null,
            last_run: None,
            last_digest: None,
            position: NodePosition { x: 0.0, y: 0.0 },
        });
        self
    }

    pub fn add_edge(mut self, from: &str, to: &str, artifact: &str, edge_type: EdgeType) -> Self {
        self.edges.push(DagEdge {
            from: from.to_string(),
            to: to.to_string(),
            artifact: artifact.to_string(),
            edge_type,
        });
        self
    }

    pub fn build(self) -> Result<PipelineDag, String> {
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

        let entry_nodes: Vec<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let out_degree: HashMap<String, usize> = self.nodes.iter()
            .map(|n| {
                let count = self.edges.iter().filter(|e| e.from == n.id).count();
                (n.id.clone(), count)
            })
            .collect();

        let exit_nodes: Vec<String> = out_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let max_depth = Self::compute_max_depth(&self.nodes, &self.edges);

        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();

        let now = chrono::Utc::now().to_rfc3339();

        Ok(PipelineDag {
            name: self.name,
            nodes: self.nodes,
            edges: self.edges,
            metadata: DagMetadata {
                created_at: now.clone(),
                updated_at: now,
                total_nodes,
                total_edges,
                max_depth,
                has_cycles,
                entry_nodes,
                exit_nodes,
            },
        })
    }

    fn detect_cycles(nodes: &[DagNode], edges: &[DagEdge]) -> bool {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in nodes {
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in edges {
            adj.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
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

    fn compute_max_depth(nodes: &[DagNode], edges: &[DagEdge]) -> usize {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
            adj.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
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

impl PipelineDag {
    pub fn detect_changes(
        &self,
        node_digests: &HashMap<String, String>,
    ) -> Vec<ChangeDetectionResult> {
        let mut results = Vec::new();

        for node in &self.nodes {
            let current_digest = node_digests.get(&node.id);
            let has_changed = match (&node.last_digest, current_digest) {
                (Some(old), Some(new)) => old != new,
                (None, Some(_)) => true,
                _ => false,
            };

            let changed_inputs: Vec<String> = if has_changed {
                node.input_artifacts.clone()
            } else {
                Vec::new()
            };

            let downstream = self.get_downstream_nodes(&node.id);

            let needs_rerun = has_changed || downstream.iter().any(|dn| {
                results.iter().any(|r: &ChangeDetectionResult| r.node_id == *dn && r.needs_rerun)
            });

            let reason = if has_changed {
                format!("节点 '{}' 的输入数据已变更", node.name)
            } else if needs_rerun {
                format!("节点 '{}' 的上游节点已变更，需要重新执行", node.name)
            } else {
                format!("节点 '{}' 无变更", node.name)
            };

            results.push(ChangeDetectionResult {
                node_id: node.id.clone(),
                has_changed,
                changed_inputs,
                downstream_affected: downstream,
                needs_rerun,
                reason,
            });
        }

        results
    }

    pub fn get_downstream_nodes(&self, node_id: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(node_id);

        while let Some(current) = queue.pop_front() {
            for edge in &self.edges {
                if edge.from == current && !visited.contains(edge.to.as_str()) {
                    visited.insert(edge.to.as_str());
                    result.push(edge.to.clone());
                    queue.push_back(edge.to.as_str());
                }
            }
        }

        result
    }

    pub fn get_upstream_nodes(&self, node_id: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(node_id);

        while let Some(current) = queue.pop_front() {
            for edge in &self.edges {
                if edge.to == current && !visited.contains(edge.from.as_str()) {
                    visited.insert(edge.from.as_str());
                    result.push(edge.from.clone());
                    queue.push_back(edge.from.as_str());
                }
            }
        }

        result
    }

    pub fn plan_execution(&self) -> ExecutionPlan {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in &self.nodes {
            in_degree.entry(node.id.as_str()).or_insert(0);
            adj.entry(node.id.as_str()).or_default();
        }
        for edge in &self.edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
            adj.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }

        let mut execution_order = Vec::new();
        let mut parallel_groups = Vec::new();
        let mut queue: VecDeque<&str> = VecDeque::new();

        for node in &self.nodes {
            if in_degree[node.id.as_str()] == 0 {
                queue.push_back(node.id.as_str());
            }
        }

        while !queue.is_empty() {
            let group: Vec<String> = queue.iter().map(|&s| s.to_string()).collect();
            parallel_groups.push(group);

            let level_size = queue.len();
            for _ in 0..level_size {
                if let Some(current) = queue.pop_front() {
                    execution_order.push(current.to_string());

                    if let Some(neighbors) = adj.get(current) {
                        for &next in neighbors {
                            let entry = in_degree.get_mut(next).unwrap();
                            *entry -= 1;
                            if *entry == 0 {
                                queue.push_back(next);
                            }
                        }
                    }
                }
            }
        }

        ExecutionPlan {
            dag_name: self.name.clone(),
            execution_order,
            parallel_groups,
            total_steps: self.nodes.len(),
            estimated_duration: "取决于各步骤的实际执行时间".to_string(),
        }
    }

    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph TD\n");

        for node in &self.nodes {
            let icon = match node.node_type {
                DagNodeType::DataSource => "📁",
                DagNodeType::Preprocess => "⚙️",
                DagNodeType::Split => "✂️",
                DagNodeType::Train => "🏋️",
                DagNodeType::Evaluate => "📊",
                DagNodeType::Export => "📤",
                DagNodeType::Custom => "🔧",
            };
            output.push_str(&format!(
                "    {}[\"{} {}<br/>{}<br/>status: {}\"]\n",
                node.id, icon, node.name, node.id, node.status
            ));
        }

        for edge in &self.edges {
            let label = match edge.edge_type {
                EdgeType::Data => "data",
                EdgeType::Model => "model",
                EdgeType::Config => "config",
                EdgeType::Trigger => "trigger",
            };
            output.push_str(&format!(
                "    {} -->|\"{}: {}\"| {}\n",
                edge.from, label, edge.artifact, edge.to
            ));
        }

        output
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.metadata.has_cycles {
            errors.push("DAG 包含循环依赖，无法执行".to_string());
        }

        if self.metadata.entry_nodes.is_empty() && !self.nodes.is_empty() {
            errors.push("DAG 没有入口节点（所有节点都有入边）".to_string());
        }

        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();
        for edge in &self.edges {
            if !node_ids.contains(edge.from.as_str()) {
                errors.push(format!("边引用了不存在的源节点: {}", edge.from));
            }
            if !node_ids.contains(edge.to.as_str()) {
                errors.push(format!("边引用了不存在的目标节点: {}", edge.to));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub fn create_standard_training_dag(
    dataset_name: &str,
    _model_name: &str,
) -> Result<PipelineDag, String> {
    PipelineDagBuilder::new(&format!("{}_training_pipeline", dataset_name))
        .add_node(
            "load_data", "加载数据", DagNodeType::DataSource,
            vec![],
            vec!["raw_data".to_string()],
        )
        .add_node(
            "preprocess", "数据预处理", DagNodeType::Preprocess,
            vec!["raw_data".to_string()],
            vec!["clean_data".to_string()],
        )
        .add_node(
            "split_data", "数据划分", DagNodeType::Split,
            vec!["clean_data".to_string()],
            vec!["train_set".to_string(), "val_set".to_string(), "test_set".to_string()],
        )
        .add_node(
            "train_model", "模型训练", DagNodeType::Train,
            vec!["train_set".to_string(), "val_set".to_string()],
            vec!["trained_model".to_string(), "training_metrics".to_string()],
        )
        .add_node(
            "evaluate", "模型评估", DagNodeType::Evaluate,
            vec!["trained_model".to_string(), "test_set".to_string()],
            vec!["evaluation_report".to_string()],
        )
        .add_node(
            "export", "模型导出", DagNodeType::Export,
            vec!["trained_model".to_string(), "evaluation_report".to_string()],
            vec!["deployable_model".to_string()],
        )
        .add_edge("load_data", "preprocess", "raw_data", EdgeType::Data)
        .add_edge("preprocess", "split_data", "clean_data", EdgeType::Data)
        .add_edge("split_data", "train_model", "train_set", EdgeType::Data)
        .add_edge("split_data", "train_model", "val_set", EdgeType::Data)
        .add_edge("split_data", "evaluate", "test_set", EdgeType::Data)
        .add_edge("train_model", "evaluate", "trained_model", EdgeType::Model)
        .add_edge("train_model", "export", "trained_model", EdgeType::Model)
        .add_edge("evaluate", "export", "evaluation_report", EdgeType::Data)
        .build()
}
