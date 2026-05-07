use crate::temporal::relations::change::RelationChange;
use crate::temporal::Tick;

/// 关系历史（只追加）
///
/// ❗不可删除、不可回滚
///
/// # 设计约束
///
/// - append-only：只能追加，不能删除或修改
/// - 不可删除：不提供 remove / delete 接口
/// - 不可回滚：不提供 rewrite / rollback 接口
/// - 只读访问：不提供 mutable access
/// - 时间顺序：变化按时间顺序存储
///
/// # 哲学含义
///
/// RelationHistory 是"关系历史（只追加）"，而不是"可修改的历史"。
///
/// 这意味着：
/// - 历史只能追加，不能删除或修改
/// - 历史不可删除、不可回滚
/// - 历史只提供只读访问
/// - 历史按时间顺序存储
///
/// # 与 WorldAxioms 的一致性
///
/// RelationHistory 的 append-only 特性与 WorldAxioms 的"时间不可逆"公理一致：
/// - 世界时间只能向前推进，不能倒流
/// - 关系历史只能追加，不能删除或修改
#[derive(Debug, Default)]
pub struct RelationHistory {
    changes: Vec<RelationChange>,
}

impl RelationHistory {
    /// 创建新的关系历史
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::RelationHistory;
    ///
    /// let history = RelationHistory::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一个新关系变化
    ///
    /// # 参数
    ///
    /// * `change` - 要记录的关系变化
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationHistory, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// history.record(change);
    /// ```
    pub fn record(&mut self, change: RelationChange) {
        if let Some(last) = self.changes.last() {
            debug_assert!(
                change.tick() > last.tick(),
                "Temporal violation: change tick {} must be greater than last tick {}",
                change.tick(),
                last.tick()
            );
        }
        self.changes.push(change);
    }

    /// 获取最新变化
    ///
    /// # 返回值
    ///
    /// 如果历史不为空，返回最新变化的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationHistory, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// history.record(change.clone());
    ///
    /// assert!(history.latest().is_some());
    /// ```
    pub fn latest(&self) -> Option<&RelationChange> {
        self.changes.last()
    }

    /// 按时间查询
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    ///
    /// # 返回值
    ///
    /// 如果找到匹配时间刻的变化，返回其引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationHistory, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// history.record(change);
    ///
    /// assert!(history.get_at(Tick::new(1)).is_some());
    /// assert!(history.get_at(Tick::new(2)).is_none());
    /// ```
    pub fn get_at(&self, tick: Tick) -> Option<&RelationChange> {
        self.changes.iter().find(|c| c.tick() == tick)
    }

    /// 只读遍历
    ///
    /// # 返回值
    ///
    /// 返回变化的只读迭代器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationHistory, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// for i in 1..=3 {
    ///     let fact = RelationFact::new(
    ///         ExistentialRelationKind::EmbodimentInField,
    ///         EntityId::new(i),
    ///         EntityId::new(i + 1),
    ///     );
    ///     let change = RelationChange::new(Tick::new(i), RelationChangeKind::Added, fact);
    ///     history.record(change);
    /// }
    ///
    /// let count = history.iter().count();
    /// assert_eq!(count, 3);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &RelationChange> {
        self.changes.iter()
    }

    /// 查询时间范围内的变化
    ///
    /// # 参数
    ///
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回时间范围内的所有变化
    ///
    /// # 契约
    ///
    /// 返回的变化按时间刻递增排序
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{relations::{RelationHistory, RelationChange, RelationChangeKind}, Tick};
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// for i in 1..=5 {
    ///     let fact = RelationFact::new(
    ///         ExistentialRelationKind::EmbodimentInField,
    ///         EntityId::new(i),
    ///         EntityId::new(i + 1),
    ///     );
    ///     let tick = Tick::new(i);
    ///     let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
    ///     history.record(change);
    /// }
    ///
    /// let range = history.query_range(Tick::new(2), Tick::new(4));
    /// assert_eq!(range.len(), 3);
    /// ```
    pub fn query_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange> {
        self.changes
            .iter()
            .filter(|c| c.tick() >= start && c.tick() <= end)
            .collect()
    }

    /// 获取历史记录数量
    ///
    /// # 返回值
    ///
    /// 返回历史记录的数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::{RelationHistory, RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// assert_eq!(history.len(), 0);
    ///
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// history.record(change);
    ///
    /// assert_eq!(history.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// 检查历史是否为空
    ///
    /// # 返回值
    ///
    /// 如果历史为空，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::relations::RelationHistory;
    ///
    /// let history = RelationHistory::new();
    /// assert!(history.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    use crate::temporal::relations::change::RelationChangeKind;
    use crate::temporal::Tick;

    #[test]
    fn test_relation_history_creation() {
        let history = RelationHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_relation_history_record() {
        let mut history = RelationHistory::new();
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        history.record(change);
        
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_relation_history_latest() {
        let mut history = RelationHistory::new();
        
        let fact1 = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick1 = Tick::new(1);
        let change1 = RelationChange::new(tick1, RelationChangeKind::Added, fact1);
        history.record(change1);
        
        let fact2 = RelationFact::new(
            ExistentialRelationKind::EnvironmentInField,
            EntityId::new(3),
            EntityId::new(4),
        );
        let tick2 = Tick::new(2);
        let change2 = RelationChange::new(tick2, RelationChangeKind::Added, fact2);
        history.record(change2);
        
        let latest = history.latest();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().tick(), Tick::new(2));
    }

    #[test]
    fn test_relation_history_get_at() {
        let mut history = RelationHistory::new();
        
        for i in 1..=3 {
            let fact = RelationFact::new(
                ExistentialRelationKind::EmbodimentInField,
                EntityId::new(i),
                EntityId::new(i + 1),
            );
            let tick = Tick::new(i);
            let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
            history.record(change);
        }
        
        assert!(history.get_at(Tick::new(1)).is_some());
        assert!(history.get_at(Tick::new(2)).is_some());
        assert!(history.get_at(Tick::new(3)).is_some());
        assert!(history.get_at(Tick::new(4)).is_none());
    }

    #[test]
    fn test_relation_history_iter() {
        let mut history = RelationHistory::new();
        
        for i in 1..=3 {
            let fact = RelationFact::new(
                ExistentialRelationKind::EmbodimentInField,
                EntityId::new(i),
                EntityId::new(i + 1),
            );
            let tick = Tick::new(i);
            let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
            history.record(change);
        }
        
        let ticks: Vec<u64> = history.iter().map(|c| c.tick().value()).collect();
        assert_eq!(ticks, vec![1, 2, 3]);
    }

    #[test]
    fn test_relation_history_append_only() {
        let mut history = RelationHistory::new();
        
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        history.record(change);
        
        // 验证只能追加，不能删除或修改
        assert_eq!(history.len(), 1);
        // RelationHistory 不提供 remove / rewrite 接口
        // 这是编译时保证的
    }
}
