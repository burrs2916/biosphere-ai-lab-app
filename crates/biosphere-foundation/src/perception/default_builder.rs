use crate::manifest::{Manifest, ManifestNode};
use crate::perception::{Perception, PerceptionEntry, PerceptionBuilder, ManifestPath};

/// 默认感知构建器
///
/// [`DefaultPerceptionBuilder`] 是 PerceptionBuilder 的默认实现。
///
/// # 设计约束
///
/// - 深度优先：使用深度优先遍历
/// - 线性展开：将树形结构展开为线性序列
/// - 感知顺序：按照人类感知的顺序排列
/// - 确定性：相同的 Manifest 总是产生相同的 Perception
///
/// # 哲学含义
///
/// DefaultPerceptionBuilder 是"人类如何一步一步'看'Manifest 的默认策略"，而不是"渲染策略"。
///
/// 这意味着：
/// - DefaultPerceptionBuilder 使用深度优先遍历
/// - DefaultPerceptionBuilder 产生线性感知序列
/// - DefaultPerceptionBuilder 是纯函数，不是有状态的对象
#[derive(Debug, Clone, Default)]
pub struct DefaultPerceptionBuilder;

impl PerceptionBuilder for DefaultPerceptionBuilder {
    fn build(&self, manifest: &Manifest) -> Perception {
        let time = manifest.time();
        let entries = self.build_entries(&manifest.root(), vec![], 0);
        Perception::new(time, entries)
    }
}

impl DefaultPerceptionBuilder {
    /// 从 ManifestNode 构建感知条目
    ///
    /// # 参数
    ///
    /// * `node` - 要构建的 ManifestNode
    /// * `path` - 当前路径
    /// * `depth` - 当前深度
    ///
    /// # 返回值
    ///
    /// 返回感知条目向量
    fn build_entries(
        &self,
        node: &ManifestNode,
        path: ManifestPath,
        depth: usize,
    ) -> Vec<PerceptionEntry> {
        let mut entries = Vec::new();
        
        // 添加当前节点
        entries.push(PerceptionEntry::new(
            path.clone(),
            depth,
            node.kind(),
            node.value().clone(),
        ));
        
        // 递归处理子节点
        for (index, child) in node.children().iter().enumerate() {
            let mut child_path = path.clone();
            child_path.push(index);
            entries.extend(self.build_entries(child, child_path, depth + 1));
        }
        
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Value, NodeKind};

    #[test]
    fn test_default_perception_builder() {
        let builder = DefaultPerceptionBuilder;
        
        // 创建一个简单的 Manifest
        let leaf1 = ManifestNode::scalar(Value::text("Leaf 1"));
        let leaf2 = ManifestNode::scalar(Value::text("Leaf 2"));
        let root = ManifestNode::group(
            Value::text("Root"),
            vec![leaf1, leaf2],
        );
        let manifest = Manifest::new(42, root);
        
        // 构建 Perception
        let perception = builder.build(&manifest);
        
        // 验证结果
        assert_eq!(perception.time(), 42);
        assert_eq!(perception.entries().len(), 3); // root + 2 leaves
        
        // 验证第一个条目（root）
        let root_entry = &perception.entries()[0];
        assert_eq!(root_entry.depth(), 0);
        assert_eq!(root_entry.kind(), NodeKind::Group);
        assert_eq!(root_entry.value(), &Value::text("Root"));
        assert_eq!(root_entry.path(), &vec![0]);
        
        // 验证第二个条目（leaf1）
        let leaf1_entry = &perception.entries()[1];
        assert_eq!(leaf1_entry.depth(), 1);
        assert_eq!(leaf1_entry.kind(), NodeKind::Scalar);
        assert_eq!(leaf1_entry.value(), &Value::text("Leaf 1"));
        assert_eq!(leaf1_entry.path(), &vec![0, 0]);
        
        // 验证第三个条目（leaf2）
        let leaf2_entry = &perception.entries()[2];
        assert_eq!(leaf2_entry.depth(), 1);
        assert_eq!(leaf2_entry.kind(), NodeKind::Scalar);
        assert_eq!(leaf2_entry.value(), &Value::text("Leaf 2"));
        assert_eq!(leaf2_entry.path(), &vec![0, 1]);
    }
}