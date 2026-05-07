use crate::temporal::state::payload::StatePayload;
use crate::temporal::Tick;

/// 世界在某一时间点的状态快照
///
/// ❗这是值对象，不包含任何世界引用
///
/// # 设计约束
///
/// - 值对象：不包含任何世界引用
/// - 不可变：一旦创建就不可修改
/// - 时间绑定：每个快照都与唯一的时间刻绑定
/// - 不提供 diff：不提供状态差异计算
/// - 不提供 mutate：不提供状态修改接口
///
/// # 哲学含义
///
/// StateSnapshot 是"世界在某一时间点的状态快照"，而不是"世界的完整状态"。
///
/// 这意味着：
/// - 快照是值对象，不包含任何世界引用
/// - 快照一旦创建就不可修改
/// - 每个快照都与唯一的时间刻绑定
/// - 快照不提供状态差异计算
/// - 快照不提供状态修改接口
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// 世界时间戳
    tick: Tick,

    /// 状态载荷（中立，不解释）
    payload: StatePayload,
}

impl StateSnapshot {
    /// 创建新的状态快照
    ///
    /// # 参数
    ///
    /// * `tick` - 世界时间戳
    /// * `payload` - 状态载荷
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::{state::{StateSnapshot, StatePayload}, Tick};
    ///
    /// let payload = StatePayload::new("hello world");
    /// let tick = Tick::new(42);
    /// let snapshot = StateSnapshot::new(tick, payload);
    /// ```
    pub fn new(tick: Tick, payload: StatePayload) -> Self {
        Self { tick, payload }
    }

    /// 获取时间戳
    ///
    /// # 返回值
    ///
    /// 返回世界时间戳
    pub fn tick(&self) -> Tick {
        self.tick
    }

    /// 获取状态载荷
    ///
    /// # 返回值
    ///
    /// 返回状态载荷的引用
    pub fn payload(&self) -> &StatePayload {
        &self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::Tick;

    #[test]
    fn test_state_snapshot_creation() {
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        
        assert_eq!(snapshot.tick(), Tick::new(1));
    }

    #[test]
    fn test_state_snapshot_immutability() {
        let payload = StatePayload::new("test");
        let tick = Tick::new(1);
        let snapshot = StateSnapshot::new(tick, payload);
        
        // 快照可以被克隆
        let cloned = snapshot.clone();
        assert_eq!(cloned.tick(), snapshot.tick());
    }
}