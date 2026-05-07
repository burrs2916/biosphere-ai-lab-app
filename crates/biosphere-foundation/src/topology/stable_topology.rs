use biosphere_core::{ExistentialTopology, RelationFact, EntityId, ExistentialRelationKind};
use std::sync::{Arc, RwLock};

/// 稳定拓扑
///
/// [`StableTopology`] 是唯一的事实源，记录当前存在的所有关系事实。
///
/// 它是只读的，不提供任何修改接口。
///
/// # 设计约束
///
/// - 唯一事实源：系统中只有一个 StableTopology 实例
/// - 只读性：外部代码只能读取，不能直接修改
/// - 不可伪造：关系事实由应用层生成并注入
/// - 原子性：拓扑更新是原子的（由应用层保证）
///
/// # 哲学含义
///
/// StableTopology 表示当前世界状态下被承认的关系事实，不包含历史，不支持时间回溯。
/// 历史关系由 Temporal / RelationHistory 管理。
///
/// 这意味着：
/// - 任何代码都不能直接修改拓扑
/// - 关系事实由应用层生成并注入
/// - 拓扑更新是原子的，要么全部成功，要么全部失败（由应用层保证）
/// - Topology 是"世界承认什么"的裁决者，而不是"发生过什么"的记录者
#[derive(Clone)]
pub struct StableTopology {
    facts: Arc<RwLock<Vec<RelationFact>>>,
}

impl StableTopology {
    /// 创建新的稳定拓扑
    pub fn new() -> Self {
        Self {
            facts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 返回所有关系事实的快照
    pub fn facts_snapshot(&self) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.clone()
    }

    /// 查询特定关系种类的事实
    pub fn facts_by_kind(&self, kind: ExistentialRelationKind) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.iter().filter(|f| f.kind == kind).cloned().collect()
    }

    /// 查询特定主体的关系事实
    pub fn facts_by_subject(&self, subject_id: EntityId) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.iter().filter(|f| f.subject_id == subject_id).cloned().collect()
    }

    /// 查询特定客体的关系事实
    pub fn facts_by_object(&self, object_id: EntityId) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.iter().filter(|f| f.object_id == object_id).cloned().collect()
    }

    /// 根据谓词查找关系事实
    ///
    /// # 参数
    ///
    /// * `predicate` - 谓词函数，返回 true 表示匹配
    ///
    /// # 返回值
    ///
    /// 返回所有匹配的关系事实
    ///
    /// # 示例
    ///
    /// ```text
    /// let results = topology.find_by_predicate(|fact| {
    ///     fact.kind == ExistentialRelationKind::LocatedIn
    /// });
    /// ```
    pub fn find_by_predicate<F>(&self, predicate: F) -> Vec<RelationFact>
    where
        F: Fn(&RelationFact) -> bool,
    {
        let facts = self.facts.read().unwrap();
        facts.iter().filter(|f| predicate(f)).cloned().collect()
    }

    /// 按关系种类过滤关系事实
    ///
    /// # 参数
    ///
    /// * `kinds` - 要过滤的关系种类列表
    ///
    /// # 返回值
    ///
    /// 返回所有匹配的关系事实
    ///
    /// # 示例
    ///
    /// ```text
    /// let kinds = vec![
    ///     ExistentialRelationKind::LocatedIn,
    ///     ExistentialRelationKind::PartOf,
    /// ];
    /// let results = topology.filter_by_kinds(&kinds);
    /// ```
    pub fn filter_by_kinds(&self, kinds: &[ExistentialRelationKind]) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .filter(|f| kinds.contains(&f.kind))
            .cloned()
            .collect()
    }

    /// 检查是否存在某个关系
    ///
    /// # 参数
    ///
    /// * `subject_id` - 主体 ID
    /// * `object_id` - 客体 ID
    /// * `kind` - 关系种类
    ///
    /// # 返回值
    ///
    /// 如果存在该关系，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```text
    /// let exists = topology.has_relation(
    ///     subject_id,
    ///     object_id,
    ///     ExistentialRelationKind::LocatedIn,
    /// );
    /// ```
    pub fn has_relation(
        &self,
        subject_id: EntityId,
        object_id: EntityId,
        kind: ExistentialRelationKind,
    ) -> bool {
        let facts = self.facts.read().unwrap();
        facts.iter().any(|f| {
            f.subject_id == subject_id
                && f.object_id == object_id
                && f.kind == kind
        })
    }

    /// 获取所有主体
    ///
    /// # 返回值
    ///
    /// 返回所有主体的 ID 列表（去重）
    ///
    /// # 示例
    ///
    /// ```text
    /// let subjects = topology.subjects();
    /// ```
    pub fn subjects(&self) -> Vec<EntityId> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .map(|f| f.subject_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// 获取所有客体
    ///
    /// # 返回值
    ///
    /// 返回所有客体的 ID 列表（去重）
    ///
    /// # 示例
    ///
    /// ```text
    /// let objects = topology.objects();
    /// ```
    pub fn objects(&self) -> Vec<EntityId> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .map(|f| f.object_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// 获取所有关系种类
    ///
    /// # 返回值
    ///
    /// 返回所有关系种类的列表（去重）
    ///
    /// # 示例
    ///
    /// ```text
    /// let kinds = topology.kinds();
    /// ```
    pub fn kinds(&self) -> Vec<ExistentialRelationKind> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .map(|f| f.kind)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// 计算关系事实的数量
    ///
    /// # 返回值
    ///
    /// 返回关系事实的数量
    ///
    /// # 示例
    ///
    /// ```text
    /// let count = topology.count();
    /// ```
    pub fn count(&self) -> usize {
        let facts = self.facts.read().unwrap();
        facts.len()
    }

    /// 检查拓扑是否为空
    ///
    /// # 返回值
    ///
    /// 如果拓扑为空，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```text
    /// let is_empty = topology.is_empty();
    /// ```
    pub fn is_empty(&self) -> bool {
        let facts = self.facts.read().unwrap();
        facts.is_empty()
    }

    /// 验证拓扑一致性
    ///
    /// 检查拓扑是否满足基本的一致性约束：
    /// - 没有重复的关系事实
    ///
    /// # 返回值
    ///
    /// 如果拓扑一致，返回 true，否则返回 false
    ///
    /// # 设计约束
    ///
    /// - 只检查基础一致性约束
    /// - 不包含循环检测逻辑（由应用层负责）
    /// - Foundation 层只提供数据访问接口
    ///
    /// # 示例
    ///
    /// ```text
    /// let is_consistent = topology.validate_consistency();
    /// ```
    pub fn validate_consistency(&self) -> bool {
        let facts = self.facts.read().unwrap();
        
        let mut seen = std::collections::HashSet::new();
        for fact in facts.iter() {
            if !seen.insert(fact) {
                return false;
            }
        }
        
        true
    }
}

impl Default for StableTopology {
    fn default() -> Self {
        Self::new()
    }
}

impl ExistentialTopology for StableTopology {
    fn relations(&self) -> Vec<RelationFact> {
        let facts = self.facts.read().unwrap();
        facts.clone()
    }

    /// 检查世界是否承认某个关系事实
    ///
    /// 这不是一个普通的 contains 方法，而是在询问：
    /// "世界是否承认这个说法"
    ///
    /// # 参数
    ///
    /// * `relation` - 要检查的关系事实
    ///
    /// # 返回值
    ///
    /// 如果世界承认该关系事实，返回 true；否则返回 false
    ///
    /// # 哲学含义
    ///
    /// 这个方法是世界级接口，可以用于：
    /// - Proof / Justification 验证
    /// - Conflict detection
    /// - Agent negotiation
    /// - World-state validation
    ///
    /// 它体现了 Topology 作为"存在裁判"的角色
    fn acknowledges(&self, relation: &RelationFact) -> bool {
        let facts = self.facts.read().unwrap();
        facts.iter().any(|f| f == relation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_topology_creation() {
        let topology = StableTopology::new();
        assert!(topology.is_empty());
        assert_eq!(topology.count(), 0);
    }

    #[test]
    fn test_stable_topology_facts_snapshot() {
        let topology = StableTopology::new();
        let facts = topology.facts_snapshot();
        assert!(facts.is_empty());
    }

    #[test]
    fn test_stable_topology_subjects() {
        let topology = StableTopology::new();
        let subjects = topology.subjects();
        assert!(subjects.is_empty());
    }

    #[test]
    fn test_stable_topology_objects() {
        let topology = StableTopology::new();
        let objects = topology.objects();
        assert!(objects.is_empty());
    }

    #[test]
    fn test_stable_topology_kinds() {
        let topology = StableTopology::new();
        let kinds = topology.kinds();
        assert!(kinds.is_empty());
    }

    #[test]
    fn test_stable_topology_validate_consistency() {
        let topology = StableTopology::new();
        assert!(topology.validate_consistency());
    }
}
