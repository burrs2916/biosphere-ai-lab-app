use biosphere_core::{EntityId, Conditions};

/// 场景图视图模型
///
/// [`SceneGraphViewModel`] 定义了场景图视图的视图模型。
///
/// # 设计约束
///
/// - 只读数据：只包含只读数据
/// - 无状态：不包含任何状态
/// - 可序列化：可以序列化和反序列化
#[derive(Debug, Clone)]
pub struct SceneGraphViewModel {
    nodes: Vec<SceneGraphNode>,
    edges: Vec<SceneGraphEdge>,
}

/// 场景图节点
///
/// [`SceneGraphNode`] 定义了场景图的节点。
#[derive(Debug, Clone)]
pub struct SceneGraphNode {
    id: EntityId,
}

impl SceneGraphNode {
    pub fn new(id: EntityId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> EntityId {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct SceneGraphEdge {
    source: EntityId,
    target: EntityId,
}

impl SceneGraphEdge {
    pub fn new(source: EntityId, target: EntityId) -> Self {
        Self { source, target }
    }

    pub fn source(&self) -> EntityId {
        self.source
    }

    pub fn target(&self) -> EntityId {
        self.target
    }
}

/// 场景图视图
///
/// [`SceneGraphView`] 定义了场景图视图的接口。
///
/// # 设计约束
///
/// - 只读访问：只通过 Conditions 访问状态
/// - 不修改状态：不提供任何修改接口
/// - 不持有状态：不包含任何状态
/// - 不依赖 UI 框架：不依赖具体的 UI 框架
///
/// # 哲学含义
///
/// SceneGraphView 是"场景图视图"，而不是"场景图编辑器"。
///
/// 这意味着：
/// - SceneGraphView 只显示场景图，不修改场景图
/// - SceneGraphView 是只读计算器
/// - SceneGraphView 不处理事件
/// - SceneGraphView 不依赖 UI 框架
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::projection::{SceneGraphView, SceneGraphViewModel};
/// use biosphere_core::Conditions;
///
/// struct MySceneGraphView;
///
/// impl SceneGraphView for MySceneGraphView {
///     fn render(&self, conditions: &dyn Conditions) -> SceneGraphViewModel {
///         // 实现场景图视图逻辑
///         SceneGraphViewModel::new(Vec::new(), Vec::new())
///     }
/// }
/// ```
pub trait SceneGraphView {
    /// 渲染场景图视图
    ///
    /// # 参数
    ///
    /// * `conditions` - 条件
    ///
    /// # 返回值
    ///
    /// 返回场景图视图的视图模型
    fn render(&self, conditions: &dyn Conditions) -> SceneGraphViewModel;
}

impl SceneGraphViewModel {
    /// 创建新的场景图视图模型
    ///
    /// # 参数
    ///
    /// * `nodes` - 节点列表
    /// * `edges` - 边列表
    ///
    /// # 返回值
    ///
    /// 返回新的场景图视图模型
    ///
    /// # 设计约束
    ///
    /// - 只读构造函数
    /// - 节点和边列表在构造时确定，之后不可修改
    pub fn new(nodes: Vec<SceneGraphNode>, edges: Vec<SceneGraphEdge>) -> Self {
        Self {
            nodes,
            edges,
        }
    }

    /// 获取节点列表
    ///
    /// # 返回值
    ///
    /// 返回节点列表的引用
    pub fn nodes(&self) -> &[SceneGraphNode] {
        &self.nodes
    }

    /// 获取边列表
    ///
    /// # 返回值
    ///
    /// 返回边列表的引用
    pub fn edges(&self) -> &[SceneGraphEdge] {
        &self.edges
    }

    /// 获取节点数量
    ///
    /// # 返回值
    ///
    /// 返回节点的数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取边数量
    ///
    /// # 返回值
    ///
    /// 返回边的数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for SceneGraphViewModel {
    fn default() -> Self {
        Self::new(Vec::new(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::sensed_conditions::SensedConditions;
    use biosphere_core::ConditionSnapshot;

    #[test]
    fn test_scene_graph_view_model_creation() {
        let model = SceneGraphViewModel::new(Vec::new(), Vec::new());
        assert!(model.nodes().is_empty());
        assert!(model.edges().is_empty());
    }

    #[test]
    fn test_scene_graph_view_model_with_nodes() {
        let nodes = vec![
            SceneGraphNode::new(EntityId::new(1)),
        ];
        let model = SceneGraphViewModel::new(nodes, Vec::new());
        assert_eq!(model.node_count(), 1);
    }

    #[test]
    fn test_scene_graph_view_model_with_edges() {
        let edges = vec![
            SceneGraphEdge::new(EntityId::new(1), EntityId::new(2)),
        ];
        let model = SceneGraphViewModel::new(Vec::new(), edges);
        assert_eq!(model.edge_count(), 1);
    }

    #[test]
    fn test_scene_graph_view_trait() {
        struct TestSceneGraphView;
        
        impl SceneGraphView for TestSceneGraphView {
            fn render(&self, _conditions: &dyn Conditions) -> SceneGraphViewModel {
                SceneGraphViewModel::new(Vec::new(), Vec::new())
            }
        }
        
        let view = TestSceneGraphView;
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        let conditions = SensedConditions::new(snapshot);
        let model = view.render(&conditions);
        assert!(model.nodes().is_empty());
        assert!(model.edges().is_empty());
    }

    #[test]
    fn test_scene_graph_view_model_clone() {
        let nodes = vec![
            SceneGraphNode::new(EntityId::new(1)),
        ];
        let edges = vec![
            SceneGraphEdge::new(EntityId::new(1), EntityId::new(2)),
        ];
        let model = SceneGraphViewModel::new(nodes, edges);
        let cloned = model.clone();
        assert_eq!(cloned.node_count(), model.node_count());
        assert_eq!(cloned.edge_count(), model.edge_count());
    }

    #[test]
    fn test_scene_graph_view_model_default() {
        let model = SceneGraphViewModel::default();
        assert!(model.nodes().is_empty());
        assert!(model.edges().is_empty());
    }

    #[test]
    fn test_scene_graph_node_creation() {
        let node = SceneGraphNode::new(EntityId::new(1));
        assert_eq!(node.id(), EntityId::new(1));
    }

    #[test]
    fn test_scene_graph_edge_creation() {
        let edge = SceneGraphEdge::new(EntityId::new(1), EntityId::new(2));
        assert_eq!(edge.source(), EntityId::new(1));
        assert_eq!(edge.target(), EntityId::new(2));
    }

    #[test]
    fn test_scene_graph_node_clone() {
        let node = SceneGraphNode::new(EntityId::new(1));
        let cloned = node.clone();
        assert_eq!(cloned.id(), node.id());
    }

    #[test]
    fn test_scene_graph_edge_clone() {
        let edge = SceneGraphEdge::new(EntityId::new(1), EntityId::new(2));
        let cloned = edge.clone();
        assert_eq!(cloned.source(), edge.source());
        assert_eq!(cloned.target(), edge.target());
    }
}
