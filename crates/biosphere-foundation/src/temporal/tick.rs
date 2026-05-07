//! 时间刻（Tick）定义
//!
//! Tick 是世界时间的基本单位，代表一个单调递增的时间刻度。
//! 它是 Temporal 系统的核心抽象，为世界状态和关系变化提供时间坐标。

use std::fmt;

/// 单调、权威的时间线标识符
///
/// # 设计哲学
///
/// Tick 代表一个**单一线性时间**上的点，具有以下特性：
///
/// 1. **单调性** - 每个 Tick 值严格大于前一个
/// 2. **权威性** - Tick 由单一权威源分配，保证全局唯一
/// 3. **连续性** - Tick 序列是连续的，不允许空洞（当前假设）
/// 4. **不可变性** - Tick 一旦分配，永不改变
///
/// # 时间线假设
///
/// **当前 Foundation 假设：**
/// - 存在单一全局线性时间线
/// - 所有状态变化都在同一条时间线上排序
/// - 不支持时间分叉或并行时间线
///
/// **未来扩展方向（非当前实现）：**
/// - 多时间线支持（what-if 场景）
/// - 时间分叉与合并
/// - 分布式时间同步
///
/// # 类型安全
///
/// Tick 是一个新类型（newtype），封装 u64 值，提供类型安全保证。
/// 这防止了与其他 u64 值（如 ID、计数器等）的混淆。
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::temporal::tick::Tick;
///
/// // 创建新的 Tick
/// let tick1 = Tick::new(0);
/// let tick2 = Tick::new(1);
///
/// // Tick 支持比较和排序
/// assert!(tick2 > tick1);
///
/// // 获取内部值
/// assert_eq!(tick1.value(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(u64);

impl Tick {
    /// 创建新的 Tick
    ///
    /// # 参数
    ///
    /// * `value` - Tick 的内部值
    ///
    /// # 注意
    ///
    /// 此方法不验证 Tick 的单调性或其他约束。
    /// Tick 的有效性由创建上下文保证。
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// 获取 Tick 的内部值
    pub const fn value(self) -> u64 {
        self.0
    }

    /// 创建下一个 Tick
    ///
    /// 返回比当前 Tick 大 1 的新 Tick
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// 检查是否为初始 Tick（值为 0）
    pub const fn is_initial(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tick({})", self.0)
    }
}

impl From<u64> for Tick {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Tick> for u64 {
    fn from(tick: Tick) -> Self {
        tick.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_creation() {
        let tick = Tick::new(42);
        assert_eq!(tick.value(), 42);
    }

    #[test]
    fn test_tick_comparison() {
        let tick1 = Tick::new(1);
        let tick2 = Tick::new(2);
        
        assert!(tick2 > tick1);
        assert!(tick1 < tick2);
        assert_eq!(tick1, Tick::new(1));
    }

    #[test]
    fn test_tick_next() {
        let tick = Tick::new(5);
        let next = tick.next();
        
        assert_eq!(next.value(), 6);
        assert!(next > tick);
    }

    #[test]
    fn test_tick_is_initial() {
        assert!(Tick::new(0).is_initial());
        assert!(!Tick::new(1).is_initial());
    }

    #[test]
    fn test_tick_from_u64() {
        let tick = Tick::from(10u64);
        assert_eq!(tick.value(), 10);
    }

    #[test]
    fn test_u64_from_tick() {
        let tick = Tick::new(20);
        let value: u64 = tick.into();
        assert_eq!(value, 20);
    }

    #[test]
    fn test_tick_display() {
        let tick = Tick::new(123);
        assert_eq!(tick.to_string(), "Tick(123)");
    }
}