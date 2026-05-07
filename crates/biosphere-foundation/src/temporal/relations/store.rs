use crate::temporal::relations::{change::RelationChange, history::RelationHistory, query::RelationQuery};
use crate::temporal::Tick;

/// 关系存储器（世界侧）
///
/// ❗只有 WorldRuntime 可以持有
///
/// # 设计约束
///
/// - 世界侧：只有 WorldRuntime 可以持有
/// - 追加接口：只提供 commit 接口
/// - 不提供 set_relation：不提供 set_relation 接口
/// - 不提供 update_relation：不提供 update_relation 接口
/// - 不提供 replace：不提供 replace 接口
///
/// # 哲学含义
///
/// RelationStore 是"关系存储器（世界侧）"，而不是"可修改的关系存储"。
///
/// 这意味着：
/// - 只有 WorldRuntime 可以持有 RelationStore
/// - RelationStore 只提供 commit 接口（追加）
/// - RelationStore 不提供 set_relation / update_relation / replace 接口
///
/// # 与 WorldAxioms 的一致性
///
/// RelationStore 的设计确保了世界关系的不可变性：
/// - 世界只能提交新的关系变化
/// - 世界不能修改历史关系
/// - 世界不能替换当前关系
#[derive(Debug)]
pub struct RelationStore {
    history: RelationHistory,
}

impl RelationStore {
    /// 创建新的关系存储器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::RelationStore;
    ///
    /// let store = RelationStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            history: RelationHistory::new(),
        }
    }

    /// 提交关系变化
    ///
    /// # 参数
    ///
    /// * `change` - 要提交的关系变化
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationStore, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut store = RelationStore::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// store.commit(change);
    /// ```
    pub fn commit(&mut self, change: RelationChange) {
        self.history.record(change);
    }

    /// 获取历史记录
    ///
    /// # 返回值
    ///
    /// 返回历史记录的引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationStore, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut store = RelationStore::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// store.commit(change);
    ///
    /// let history = store.history();
    /// assert_eq!(history.len(), 1);
    /// ```
    pub fn history(&self) -> &RelationHistory {
        &self.history
    }
}

impl Default for RelationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationQuery for RelationStore {
    fn get_relation_at(&self, tick: Tick) -> Option<&RelationChange> {
        self.history.get_at(tick)
    }

    fn query_relations_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange> {
        self.history.query_range(start, end)
    }

    fn latest_relation_change(&self) -> Option<&RelationChange> {
        self.history.latest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    use crate::temporal::relations::change::RelationChangeKind;

    #[test]
    fn test_relation_store_creation() {
        let store = RelationStore::new();
        assert_eq!(store.history().len(), 0);
    }

    #[test]
    fn test_relation_store_commit() {
        let mut store = RelationStore::new();
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        store.commit(change);
        
        assert_eq!(store.history().len(), 1);
    }

    #[test]
    fn test_relation_store_history() {
        let mut store = RelationStore::new();
        
        for i in 1..=3 {
            let fact = RelationFact::new(
                ExistentialRelationKind::EmbodimentInField,
                EntityId::new(i),
                EntityId::new(i + 1),
            );
            let tick = Tick::new(i);
            let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
            store.commit(change);
        }
        
        let history = store.history();
        assert_eq!(history.len(), 3);
        assert!(history.latest().is_some());
        assert_eq!(history.latest().unwrap().tick(), Tick::new(3));
        
        // 测试 RelationQuery trait
        assert!(store.get_relation_at(Tick::new(1)).is_some());
        assert!(store.get_relation_at(Tick::new(2)).is_some());
        assert!(store.get_relation_at(Tick::new(3)).is_some());
        
        let range = store.query_relations_range(Tick::new(1), Tick::new(3));
        assert_eq!(range.len(), 3);
        
        assert!(store.latest_relation_change().is_some());
    }

    #[test]
    fn test_relation_store_append_only() {
        let mut store = RelationStore::new();
        
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        store.commit(change);
        
        // 验证只能追加，不能删除或修改
        assert_eq!(store.history().len(), 1);
        // RelationStore 不提供 set_relation / update_relation / replace 接口
        // 这是编译时保证的
    }
}
