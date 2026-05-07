use crate::manifest::{Value, NodeKind};
use crate::perception::ManifestPath;
use std::fmt;

/// 感知条目
///
/// [`PerceptionEntry`] 是 Manifest 中单个节点的感知表示。
///
/// # 设计约束
///
/// - 路径信息：包含在 Manifest 中的路径
/// - 深度信息：表示感知的层次深度
/// - 类型信息：包含节点的类型
/// - 值信息：包含节点的值
///
/// # 哲学含义
///
/// PerceptionEntry 是"人类感知 Manifest 中单个节点的方式"，而不是"Manifest 节点本身"。
///
/// 这意味着：
/// - PerceptionEntry 是感知表示，不是原始数据
/// - PerceptionEntry 包含感知上下文（路径、深度）
/// - PerceptionEntry 是人类可读的，不是机器可读的
#[derive(Debug, Clone, PartialEq)]
pub struct PerceptionEntry {
    /// 在 Manifest 中的路径
    pub path: ManifestPath,
    /// 感知深度
    pub depth: usize,
    /// 节点类型
    pub kind: NodeKind,
    /// 节点值
    pub value: Value,
}

impl PerceptionEntry {
    /// 创建新的感知条目
    ///
    /// # 参数
    ///
    /// * `path` - 在 Manifest 中的路径
    /// * `depth` - 感知深度
    /// * `kind` - 节点类型
    /// * `value` - 节点值
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{PerceptionEntry, ManifestPath};
    /// use biosphere_foundation::manifest::{Value, NodeKind};
    ///
    /// let entry = PerceptionEntry::new(
    ///     vec![0, 1],
    ///     2,
    ///     NodeKind::Scalar,
    ///     Value::text("Hello")
    /// );
    /// ```
    pub fn new(path: ManifestPath, depth: usize, kind: NodeKind, value: Value) -> Self {
        Self { path, depth, kind, value }
    }

    /// 获取路径
    ///
    /// # 返回值
    ///
    /// 返回在 Manifest 中的路径
    pub fn path(&self) -> &ManifestPath {
        &self.path
    }

    /// 获取深度
    ///
    /// # 返回值
    ///
    /// 返回感知深度
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// 获取类型
    ///
    /// # 返回值
    ///
    /// 返回节点类型
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// 获取值
    ///
    /// # 返回值
    ///
    /// 返回节点值
    pub fn value(&self) -> &Value {
        &self.value
    }
}

impl fmt::Display for PerceptionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] {:?}: {:?} (path: {:?})",
            self.depth, self.kind, self.value, self.path
        )
    }
}