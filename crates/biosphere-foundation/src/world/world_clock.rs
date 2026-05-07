use crate::invariants::world_axioms::{WorldAxioms, WorldAxiomViolation};

/// 世界时钟
///
/// [`WorldClock`] 描述世界的时间推进，是不可逆的。
///
/// # 设计约束
///
/// - 不可逆：时间只能向前推进，不能倒流
/// - 单调递增：时间戳永远递增
/// - 原子性：时间推进是原子的
///
/// # 哲学含义
///
/// WorldClock 是"世界的时间"，是所有事件的时间基准。
///
/// 这意味着：
/// - 时间只能向前推进，不能倒流
/// - 时间戳永远递增，不会重复
/// - 时间推进是原子的，不会出现时间跳跃
#[derive(Debug, PartialEq, Eq)]
pub struct WorldClock {
    tick: u64,
}

impl WorldClock {
    /// 创建新的世界时钟
    ///
    /// ⚠️ 仅允许从 0 开始
    /// 世界不存在"带历史的出生"
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    /// 当前时间刻
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// 推进世界时间
    ///
    /// ❗这是唯一允许推进时间的方式
    /// ❗每一次推进都必须通过 WorldAxioms
    pub fn advance(&mut self) -> Result<u64, WorldAxiomViolation> {
        let next_tick = self.tick + 1;

        WorldAxioms::assert_time_irreversible(self.tick, next_tick)?;

        self.tick = next_tick;
        Ok(self.tick)
    }
}

impl Default for WorldClock {
    fn default() -> Self {
        Self::new()
    }
}
