use crate::temporal::relations::change::RelationChange;
use crate::temporal::Tick;

/// 关系查询接口
///
/// [`RelationQuery`] 定义了对关系历史的只读查询操作。
///
/// # 设计约束
///
/// - 只读查询：不提供任何修改接口
/// - 时间维度：所有查询都基于时间刻
/// - 不可变返回：返回的都是不可变引用
/// - 无副作用：查询操作不会改变状态
///
/// # 哲学含义
///
/// RelationQuery 是"关系历史的只读观察者"，而不是"关系修改者"。
///
/// 这意味着：
/// - 查询不会改变历史
/// - 查询不会影响世界演化
/// - 查询是安全的、可重复的
/// - 查询是"观察"而非"干预"
///
/// # 与 RelationHistory 的关系
///
/// RelationHistory 实现了 RelationQuery trait，提供对内部变化的查询能力。
/// RelationQuery 将查询逻辑抽象为接口，使得其他类型也可以提供类似的查询能力。
pub trait RelationQuery {
    /// 按时间刻查询关系
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
    /// use biosphere_foundation::temporal::{relations::{RelationHistory, RelationQuery, RelationChange, RelationChangeKind}, Tick};
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let tick = Tick::new(1);
    /// let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
    /// history.record(change);
    ///
    /// assert!(history.get_relation_at(Tick::new(1)).is_some());
    /// assert!(history.get_relation_at(Tick::new(2)).is_none());
    /// ```
    fn get_relation_at(&self, tick: Tick) -> Option<&RelationChange>;

    /// 按时间范围查询关系
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
    /// use biosphere_foundation::temporal::{relations::{RelationHistory, RelationQuery, RelationChange, RelationChangeKind}, Tick};
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
    /// let range = history.query_relations_range(Tick::new(2), Tick::new(4));
    /// assert_eq!(range.len(), 3);
    /// ```
    fn query_relations_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange>;

    /// 获取最新关系变化
    ///
    /// # 返回值
    ///
    /// 如果历史不为空，返回最新变化的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{relations::{RelationHistory, RelationQuery, RelationChange, RelationChangeKind}, Tick};
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut history = RelationHistory::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let tick = Tick::new(1);
    /// let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
    /// history.record(change);
    ///
    /// assert!(history.latest_relation_change().is_some());
    /// ```
    fn latest_relation_change(&self) -> Option<&RelationChange>;
}

impl RelationQuery for super::RelationHistory {
    fn get_relation_at(&self, tick: Tick) -> Option<&RelationChange> {
        super::RelationHistory::get_at(self, tick)
    }

    fn query_relations_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange> {
        self.iter()
            .filter(|c| c.tick() >= start && c.tick() <= end)
            .collect()
    }

    fn latest_relation_change(&self) -> Option<&RelationChange> {
        super::RelationHistory::latest(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    use crate::temporal::relations::change::RelationChangeKind;
    use crate::temporal::Tick;

    #[test]
    fn test_relation_query_get_at() {
        let mut history = super::super::RelationHistory::new();
        
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
        
        assert!(history.get_relation_at(Tick::new(1)).is_some());
        assert!(history.get_relation_at(Tick::new(2)).is_some());
        assert!(history.get_relation_at(Tick::new(3)).is_some());
        assert!(history.get_relation_at(Tick::new(4)).is_none());
    }

    #[test]
    fn test_relation_query_query_range() {
        let mut history = super::super::RelationHistory::new();
        
        for i in 1..=5 {
            let fact = RelationFact::new(
                ExistentialRelationKind::EmbodimentInField,
                EntityId::new(i),
                EntityId::new(i + 1),
            );
            let tick = Tick::new(i);
            let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
            history.record(change);
        }
        
        let range = history.query_relations_range(Tick::new(2), Tick::new(4));
        assert_eq!(range.len(), 3);
        
        let ticks: Vec<u64> = range.iter().map(|c| c.tick().value()).collect();
        assert_eq!(ticks, vec![2, 3, 4]);
    }

    #[test]
    fn test_relation_query_query_range_empty() {
        let history = super::super::RelationHistory::new();
        
        let range = history.query_relations_range(Tick::new(1), Tick::new(10));
        assert!(range.is_empty());
    }

    #[test]
    fn test_relation_query_query_range_no_match() {
        let mut history = super::super::RelationHistory::new();
        
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
        
        let range = history.query_relations_range(Tick::new(10), Tick::new(20));
        assert!(range.is_empty());
    }

    #[test]
    fn test_relation_query_latest_change() {
        let mut history = super::super::RelationHistory::new();
        
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
        
        let latest = history.latest_relation_change();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().tick(), Tick::new(2));
    }

    #[test]
    fn test_relation_query_latest_change_empty() {
        let history = super::super::RelationHistory::new();
        
        let latest = history.latest_relation_change();
        assert!(latest.is_none());
    }

    #[test]
    fn test_relation_query_read_only() {
        let mut history = super::super::RelationHistory::new();
        
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        history.record(change);
        
        // RelationQuery 只提供只读查询
        let _ = history.get_relation_at(Tick::new(1));
        let _ = history.query_relations_range(Tick::new(1), Tick::new(1));
        let _ = history.latest_relation_change();
        
        // 验证历史没有被修改
        assert_eq!(history.len(), 1);
    }
}
