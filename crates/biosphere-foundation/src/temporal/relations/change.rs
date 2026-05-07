use biosphere_core::RelationFact;
use crate::temporal::Tick;

/// 关系变化类型
///
/// 定义了关系可能的变化类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationChangeKind {
    /// 关系建立
    Added,
    /// 关系移除
    Removed,
}

/// 关系变化
///
/// ❗Foundation 不理解变化的含义
///
/// # 设计约束
///
/// - 中立容器：Foundation 不理解变化的含义
/// - 不可构造：外部代码无法构造 RelationChange（只能通过 new）
/// - 只读访问：只提供访问接口，不提供修改接口
///
/// # 哲学含义
///
/// RelationChange 是"关系变化（中立容器）"，而不是"可解释的变化"。
///
/// 这意味着：
/// - Foundation 不解释变化
/// - UI 不能构造变化
/// - 生命不能反推世界
/// - 这是整个系统抗污染的关键器官
#[derive(Debug, Clone)]
pub struct RelationChange {
    tick: Tick,
    kind: RelationChangeKind,
    fact: RelationFact,
}

impl RelationChange {
    /// 创建新的关系变化
    ///
    /// # 参数
    ///
    /// * `tick` - 世界时间戳
    /// * `kind` - 变化类型
    /// * `fact` - 关系事实
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{relations::{RelationChange, RelationChangeKind}, Tick};
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let tick = Tick::new(1);
    /// let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
    /// ```
    pub fn new(tick: Tick, kind: RelationChangeKind, fact: RelationFact) -> Self {
        Self { tick, kind, fact }
    }

    /// 获取时间戳
    ///
    /// # 返回值
    ///
    /// 返回世界时间戳
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// 获取变化类型
    ///
    /// # 返回值
    ///
    /// 返回变化类型
    pub fn kind(&self) -> RelationChangeKind {
        self.kind
    }

    /// 获取关系事实
    ///
    /// # 返回值
    ///
    /// 返回关系事实的引用
    pub fn fact(&self) -> &RelationFact {
        &self.fact
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biosphere_core::{ExistentialRelationKind, EntityId};
    use crate::temporal::Tick;

    #[test]
    fn test_relation_change_creation() {
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        
        assert_eq!(change.tick(), Tick::new(1));
        assert_eq!(change.kind(), RelationChangeKind::Added);
    }

    #[test]
    fn test_relation_change_clone() {
        let fact = RelationFact::new(
            ExistentialRelationKind::EmbodimentInField,
            EntityId::new(1),
            EntityId::new(2),
        );
        let tick = Tick::new(1);
        let change = RelationChange::new(tick, RelationChangeKind::Added, fact);
        
        let cloned = change.clone();
        assert_eq!(cloned.tick(), change.tick());
        assert_eq!(cloned.kind(), change.kind());
    }
}
