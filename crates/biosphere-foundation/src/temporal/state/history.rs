use crate::temporal::state::snapshot::StateSnapshot;
use crate::temporal::Tick;

/// 状态历史（只追加）
///
/// ❗不可删除、不可回滚
///
/// # 设计约束
///
/// - append-only：只能追加，不能删除或修改
/// - 不可删除：不提供 remove / delete 接口
/// - 不可回滚：不提供 rewrite / rollback 接口
/// - 只读访问：不提供 mutable access
/// - 时间顺序：快照按时间顺序存储
///
/// # 哲学含义
///
/// StateHistory 是"状态历史（只追加）"，而不是"可修改的历史"。
///
/// 这意味着：
/// - 历史只能追加，不能删除或修改
/// - 历史不可删除、不可回滚
/// - 历史只提供只读访问
/// - 历史按时间顺序存储
///
/// # 与 WorldAxioms 的一致性
///
/// StateHistory 的 append-only 特性与 WorldAxioms 的"时间不可逆"公理一致：
/// - 世界时间只能向前推进，不能倒流
/// - 状态历史只能追加，不能删除或修改
#[derive(Debug, Default)]
pub struct StateHistory {
    snapshots: Vec<StateSnapshot>,
}

impl StateHistory {
    /// 创建新的状态历史
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::StateHistory;
    ///
    /// let history = StateHistory::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一个新状态
    ///
    /// # 参数
    ///
    /// * `snapshot` - 要记录的状态快照
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// let payload = StatePayload::new("test");
    /// let tick = Tick::new(1);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// history.record(snapshot);
    /// ```
    pub fn record(&mut self, snapshot: StateSnapshot) {
        if let Some(last) = self.snapshots.last() {
            debug_assert!(
                snapshot.tick() > last.tick(),
                "Temporal violation: snapshot tick {} must be greater than last tick {}",
                snapshot.tick(),
                last.tick()
            );
        }
        self.snapshots.push(snapshot);
    }

    /// 获取最新状态
    ///
    /// # 返回值
    ///
    /// 如果历史不为空，返回最新状态的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// let payload = StatePayload::new("test");
    /// let tick = Tick::new(1);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// history.record(snapshot.clone());
    ///
    /// assert!(history.latest().is_some());
    /// ```
    pub fn latest(&self) -> Option<&StateSnapshot> {
        self.snapshots.last()
    }

    /// 按时间查询
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    ///
    /// # 返回值
    ///
    /// 如果找到匹配时间刻的快照，返回其引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// let payload = StatePayload::new("test");
    /// let tick = Tick::new(1);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// history.record(snapshot);
    ///
    /// assert!(history.get_at(Tick::new(1)).is_some());
    /// assert!(history.get_at(Tick::new(2)).is_none());
    /// ```
    pub fn get_at(&self, tick: Tick) -> Option<&StateSnapshot> {
        self.snapshots.iter().find(|s| s.tick() == tick)
    }

    /// 只读遍历
    ///
    /// # 返回值
    ///
    /// 返回快照的只读迭代器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// for i in 1..=3 {
    ///     let payload = StatePayload::new(i);
    ///     let tick = Tick::new(i);
    ///     let snapshot = StateSnapshot::new(tick, payload);
    ///     history.record(snapshot);
    /// }
    ///
    /// let count = history.iter().count();
    /// assert_eq!(count, 3);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &StateSnapshot> {
        self.snapshots.iter()
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
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// assert_eq!(history.len(), 0);
    ///
    /// let payload = StatePayload::new("test");
    /// let tick = Tick::new(1);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// history.record(snapshot);
    ///
    /// assert_eq!(history.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.snapshots.len()
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
    /// use biosphere_foundation::temporal::state::StateHistory;
    ///
    /// let history = StateHistory::new();
    /// assert!(history.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::state::payload::StatePayload;
    use crate::temporal::Tick;

    #[test]
    fn test_state_history_creation() {
        let history = StateHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_state_history_record() {
        let mut history = StateHistory::new();
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        history.record(snapshot);
        
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_state_history_latest() {
        let mut history = StateHistory::new();
        
        let payload1 = StatePayload::new("test1");
        let tick1 = Tick::new(1);
        let snapshot1 = StateSnapshot::new(tick1, payload1);
        history.record(snapshot1);
        
        let payload2 = StatePayload::new("test2");
        let tick2 = Tick::new(2);
        let snapshot2 = StateSnapshot::new(tick2, payload2);
        history.record(snapshot2);
        
        let latest = history.latest();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().tick(), Tick::new(2));
    }

    #[test]
    fn test_state_history_get_at() {
        let mut history = StateHistory::new();
        
        for i in 1..=3 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            history.record(snapshot);
        }
        
        assert!(history.get_at(Tick::new(1)).is_some());
        assert!(history.get_at(Tick::new(2)).is_some());
        assert!(history.get_at(Tick::new(3)).is_some());
        assert!(history.get_at(Tick::new(4)).is_none());
    }

    #[test]
    fn test_state_history_iter() {
        let mut history = StateHistory::new();
        
        for i in 1..=3 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            history.record(snapshot);
        }
        
        let ticks: Vec<u64> = history.iter().map(|s| s.tick().value()).collect();
        assert_eq!(ticks, vec![1, 2, 3]);
    }

    #[test]
    fn test_state_history_append_only() {
        let mut history = StateHistory::new();
        
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        history.record(snapshot);
        
        // 验证只能追加，不能删除或修改
        assert_eq!(history.len(), 1);
        // StateHistory 不提供 remove / rewrite 接口
        // 这是编译时保证的
    }
}