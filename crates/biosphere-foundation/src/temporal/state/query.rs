use crate::temporal::state::snapshot::StateSnapshot;
use crate::temporal::Tick;

/// 状态查询接口
///
/// [`StateQuery`] 定义了对状态历史的只读查询操作。
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
/// StateQuery 是"状态历史的只读观察者"，而不是"状态修改者"。
///
/// 这意味着：
/// - 查询不会改变历史
/// - 查询不会影响世界演化
/// - 查询是安全的、可重复的
/// - 查询是"观察"而非"干预"
///
/// # 与 StateHistory 的关系
///
/// StateHistory 实现了 StateQuery trait，提供对内部快照的查询能力。
/// StateQuery 将查询逻辑抽象为接口，使得其他类型也可以提供类似的查询能力。
pub trait StateQuery {
    /// 按时间刻查询
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
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateQuery, StateSnapshot, StatePayload}, Tick};
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
    fn get_at(&self, tick: Tick) -> Option<&StateSnapshot>;

    /// 按时间范围查询
    ///
    /// # 参数
    ///
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回时间范围内的所有快照
    ///
    /// # 契约
    ///
    /// 返回的快照按时间刻递增排序
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateQuery, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// for i in 1..=5 {
    ///     let payload = StatePayload::new(i);
    ///     let tick = Tick::new(i);
    ///     let snapshot = StateSnapshot::new(tick, payload);
    ///     history.record(snapshot);
    /// }
    ///
    /// let range = history.query_range(Tick::new(2), Tick::new(4));
    /// assert_eq!(range.len(), 3);
    /// ```
    fn query_range(&self, start: Tick, end: Tick) -> Vec<&StateSnapshot>;

    /// 获取最新状态
    ///
    /// # 返回值
    ///
    /// 如果历史不为空，返回最新状态的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateHistory, StateQuery, StateSnapshot, StatePayload}, Tick};
    ///
    /// let mut history = StateHistory::new();
    /// let payload = StatePayload::new("test");
    /// let tick = Tick::new(1);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// history.record(snapshot);
    ///
    /// assert!(history.latest_snapshot().is_some());
    /// ```
    fn latest_snapshot(&self) -> Option<&StateSnapshot>;
}

impl StateQuery for super::StateHistory {
    fn get_at(&self, tick: Tick) -> Option<&StateSnapshot> {
        super::StateHistory::get_at(self, tick)
    }

    fn query_range(&self, start: Tick, end: Tick) -> Vec<&StateSnapshot> {
        self.iter()
            .filter(|s| s.tick() >= start && s.tick() <= end)
            .collect()
    }

    fn latest_snapshot(&self) -> Option<&StateSnapshot> {
        super::StateHistory::latest(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::state::payload::StatePayload;
    use crate::temporal::Tick;

    #[test]
    fn test_state_query_get_at() {
        let mut history = super::super::StateHistory::new();
        
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
    fn test_state_query_query_range() {
        let mut history = super::super::StateHistory::new();
        
        for i in 1..=5 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            history.record(snapshot);
        }
        
        let range = history.query_range(Tick::new(2), Tick::new(4));
        assert_eq!(range.len(), 3);
        
        let ticks: Vec<u64> = range.iter().map(|s| s.tick().value()).collect();
        assert_eq!(ticks, vec![2, 3, 4]);
    }

    #[test]
    fn test_state_query_query_range_empty() {
        let history = super::super::StateHistory::new();
        
        let range = history.query_range(Tick::new(1), Tick::new(10));
        assert!(range.is_empty());
    }

    #[test]
    fn test_state_query_query_range_no_match() {
        let mut history = super::super::StateHistory::new();
        
        for i in 1..=3 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            history.record(snapshot);
        }
        
        let range = history.query_range(Tick::new(10), Tick::new(20));
        assert!(range.is_empty());
    }

    #[test]
    fn test_state_query_latest_snapshot() {
        let mut history = super::super::StateHistory::new();
        
        let payload1 = StatePayload::new("test1");
        let tick1 = Tick::new(1);
        let snapshot1 = StateSnapshot::new(tick1, payload1);
        history.record(snapshot1);
        
        let payload2 = StatePayload::new("test2");
        let tick2 = Tick::new(2);
        let snapshot2 = StateSnapshot::new(tick2, payload2);
        history.record(snapshot2);
        
        let latest = history.latest_snapshot();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().tick(), Tick::new(2));
    }

    #[test]
    fn test_state_query_latest_snapshot_empty() {
        let history = super::super::StateHistory::new();
        
        let latest = history.latest_snapshot();
        assert!(latest.is_none());
    }

    #[test]
    fn test_state_query_read_only() {
        let mut history = super::super::StateHistory::new();
        
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        history.record(snapshot);
        
        // StateQuery 只提供只读查询
        let _ = history.get_at(Tick::new(1));
        let _ = history.query_range(Tick::new(1), Tick::new(1));
        let _ = history.latest_snapshot();
        
        // 验证历史没有被修改
        assert_eq!(history.len(), 1);
    }
}
