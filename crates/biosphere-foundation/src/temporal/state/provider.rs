use crate::temporal::state::{snapshot::StateSnapshot, history::StateHistory};

/// 状态提供者（只读）
///
/// # 设计约束
///
/// - 只读接口：只提供读操作
/// - 没有写操作：不提供任何写操作
/// - 没有修改接口：不提供任何修改接口
///
/// # 哲学含义
///
/// StateProvider 是"状态提供者（只读）"，而不是"状态修改器"。
///
/// 这意味着：
/// - StateProvider 只提供读操作
/// - StateProvider 不提供任何写操作
/// - StateProvider 不提供任何修改接口
///
/// # 使用场景
///
/// StateProvider 用于：
/// - UI 层读取世界状态
/// - Projection 层投射世界状态
/// - Observer 观察世界状态
/// - 调试和日志记录
///
/// # 注意
///
/// ⚠️ StateProvider 不提供：
/// - set_state
/// - update_state
/// - replace
/// - 任何写操作
/// - 任何修改接口
pub trait StateProvider {
    /// 获取当前状态
    ///
    /// # 返回值
    ///
    /// 如果历史不为空，返回最新状态的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::{StateProvider, StateStore, StateSnapshot, StatePayload};
    /// use biosphere_foundation::temporal::Tick;
    ///
    /// let mut store = StateStore::new();
    /// let payload = StatePayload::new("test");
    /// let snapshot = StateSnapshot::new(Tick::new(1), payload);
    /// store.commit(snapshot);
    ///
    /// assert!(store.current_state().is_some());
    /// ```
    fn current_state(&self) -> Option<&StateSnapshot>;

    /// 获取状态历史
    ///
    /// # 返回值
    ///
    /// 返回状态历史的引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::{StateProvider, StateStore, StateSnapshot, StatePayload};
    /// use biosphere_foundation::temporal::Tick;
    ///
    /// let mut store = StateStore::new();
    /// for i in 1..=3 {
    ///     let payload = StatePayload::new(i);
    ///     let snapshot = StateSnapshot::new(Tick::new(i), payload);
    ///     store.commit(snapshot);
    /// }
    ///
    /// let history = store.state_history();
    /// assert_eq!(history.len(), 3);
    /// ```
    fn state_history(&self) -> &StateHistory;
}

impl StateProvider for super::store::StateStore {
    fn current_state(&self) -> Option<&StateSnapshot> {
        self.history().latest()
    }

    fn state_history(&self) -> &StateHistory {
        self.history()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::Tick;
    use super::super::store::StateStore;
    use crate::temporal::state::payload::StatePayload;

    #[test]
    fn test_state_provider_current_state() {
        let mut store = StateStore::new();
        assert!(store.current_state().is_none());
        
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        store.commit(snapshot);
        
        assert!(store.current_state().is_some());
        assert_eq!(store.current_state().unwrap().tick(), Tick::new(1));
    }

    #[test]
    fn test_state_provider_state_history() {
        let mut store = StateStore::new();
        
        for i in 1..=3 {
            let payload = StatePayload::new(i);
            let tick = Tick::new(i);
            let snapshot = StateSnapshot::new(tick, payload);
            store.commit(snapshot);
        }
        
        let history = store.state_history();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_state_provider_read_only() {
        let store = StateStore::new();
        
        // StateProvider 只提供读操作
        // 不提供任何写操作
        // 这是编译时保证的
        let _ = store.current_state();
        let _ = store.state_history();
    }
}