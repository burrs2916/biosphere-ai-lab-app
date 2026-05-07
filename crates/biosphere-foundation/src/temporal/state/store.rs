use crate::temporal::state::{snapshot::StateSnapshot, history::StateHistory, query::StateQuery};
use crate::temporal::Tick;

/// 状态存储器（世界侧）
///
/// ❗只有 WorldRuntime 可以持有
///
/// # 设计约束
///
/// - 世界侧：只有 WorldRuntime 可以持有
/// - 追加接口：只提供 commit 接口
/// - 不提供 set_state：不提供 set_state 接口
/// - 不提供 update_state：不提供 update_state 接口
/// - 不提供 replace：不提供 replace 接口
///
/// # 哲学含义
///
/// StateStore 是"状态存储器（世界侧）"，而不是"可修改的状态存储"。
///
/// 这意味着：
/// - 只有 WorldRuntime 可以持有 StateStore
/// - StateStore 只提供 commit 接口（追加）
/// - StateStore 不提供 set_state / update_state / replace 接口
///
/// # 与 WorldAxioms 的一致性
///
/// StateStore 的设计确保了世界状态的不可变性：
/// - 世界只能提交新的状态快照
/// - 世界不能修改历史状态
/// - 世界不能替换当前状态
#[derive(Debug)]
pub struct StateStore {
    history: StateHistory,
}

impl StateStore {
    /// 创建新的状态存储器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::StateStore;
    ///
    /// let store = StateStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            history: StateHistory::new(),
        }
    }

    /// 提交状态快照
    ///
    /// # 参数
    ///
    /// * `snapshot` - 要提交的状态快照
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::{StateStore, StateSnapshot, StatePayload};
    /// use biosphere_foundation::temporal::Tick;
    ///
    /// let mut store = StateStore::new();
    /// let payload = StatePayload::new("test");
    /// let snapshot = StateSnapshot::new(Tick::new(1), payload);
    /// store.commit(snapshot);
    /// ```
    pub fn commit(&mut self, snapshot: StateSnapshot) {
        self.history.record(snapshot);
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
    /// use biosphere_foundation::temporal::state::{StateStore, StateSnapshot, StatePayload};
    /// use biosphere_foundation::temporal::Tick;
    ///
    /// let mut store = StateStore::new();
    /// let payload = StatePayload::new("test");
    /// let snapshot = StateSnapshot::new(Tick::new(1), payload);
    /// store.commit(snapshot);
    ///
    /// let history = store.history();
    /// assert_eq!(history.len(), 1);
    /// ```
    pub fn history(&self) -> &StateHistory {
        &self.history
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateQuery for StateStore {
    fn get_at(&self, tick: Tick) -> Option<&StateSnapshot> {
        self.history.get_at(tick)
    }

    fn query_range(&self, start: Tick, end: Tick) -> Vec<&StateSnapshot> {
        self.history.query_range(start, end)
    }

    fn latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.history.latest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::state::payload::StatePayload;

    #[test]
    fn test_state_store_creation() {
        let store = StateStore::new();
        assert_eq!(store.history().len(), 0);
    }

    #[test]
    fn test_state_store_commit() {
        let mut store = StateStore::new();
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        store.commit(snapshot);
        
        assert_eq!(store.history().len(), 1);
    }

    #[test]
    fn test_state_store_history() {
        let mut store = StateStore::new();
        
        for i in 1..=3 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            store.commit(snapshot);
        }
        
        let history = store.history();
        assert_eq!(history.len(), 3);
        assert!(history.latest().is_some());
        assert_eq!(history.latest().unwrap().tick(), Tick::new(3));
    }

    #[test]
    fn test_state_store_append_only() {
        let mut store = StateStore::new();
        
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        store.commit(snapshot);
        
        // 验证只能追加，不能删除或修改
        assert_eq!(store.history().len(), 1);
        // StateStore 不提供 set_state / update_state / replace 接口
        // 这是编译时保证的
    }
}