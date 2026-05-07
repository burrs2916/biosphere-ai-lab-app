//! 环境条件的基础定义
//!
//! 这个模块定义了 [`Conditions`] trait 和相关类型，它描述世界允许被感知的那一部分。
//!
//! # 核心公理
//!
//! Conditions 是"世界 → 生命"的单向闸门。
//!
//! [`Conditions`] 是只读能力约束，不可构造，不可修改，不可反推世界。
//!
//! # 设计原则
//!
//! - **只读性**：只能以"快照"方式被读取
//! - **不可构造**：外部代码无法构造 Conditions
//! - **不可修改**：Conditions 是不可变的
//! - **不可反推**：无法从 Conditions 反推世界的完整状态
//! - **单向性**：世界 → 生命，生命无法反向影响世界
//!
//! # 与其他组件的关系
//!
//! - **Environment**：Conditions 是 Environment 暴露给生命的只读视图
//! - **Perception**：生命通过 Perception 感知 Conditions
//! - **Representation**：Conditions 可以被 Representation 映射和呈现
//!
//! # 哲学含义
//!
//! Conditions 是"世界允许被感知的那一部分"，而不是"世界的完整状态"。
//!
//! 这意味着：
//! - UI 无法伪造 Conditions
//! - 生命无法通过 Conditions 修改世界
//! - Conditions 不包含世界的完整信息
//! - Conditions 是"单向闸门"，只允许信息从世界流向生命
//!
//! # 示例
//!
//! ```rust
//! use biosphere::Conditions;
//!
//! // Conditions 只能由 World/Environment 生成
//! let conditions = world.conditions();
//!
//! // 生命只能以快照方式读取条件
//! let snapshot = conditions.snapshot();
//!
//! // 快照可以被复制、投射
//! let snapshot_copy = snapshot.clone();
//! ```

use std::any::Any;

/// 环境条件的核心抽象
///
/// [`Conditions`] 描述"世界允许被感知的那一部分"。
///
/// 它是"世界 → 生命"的单向闸门，只读、不可构造、不可修改。
///
/// # 方法
///
/// - [`snapshot`](Conditions::snapshot)：返回条件的只读快照
///
/// # 设计约束
///
/// - 不可构造：外部代码无法构造 Conditions
/// - 不可修改：Conditions 是不可变的
/// - 不可反推：无法从 Conditions 反推世界的完整状态
/// - 单向性：世界 → 生命，生命无法反向影响世界
///
/// # 哲学含义
///
/// Conditions 是"世界允许被感知的那一部分"，而不是"世界的完整状态"。
///
/// 这意味着：
/// - UI 无法伪造 Conditions
/// - 生命无法通过 Conditions 修改世界
/// - Conditions 不包含世界的完整信息
/// - Conditions 是"单向闸门"，只允许信息从世界流向生命
///
/// # 示例
///
/// ```rust
/// use biosphere::Conditions;
///
/// // Conditions 只能由 World/Environment 生成
/// let conditions = world.conditions();
///
/// // 生命只能以快照方式读取条件
/// let snapshot = conditions.snapshot();
///
/// // 快照可以被复制、投射
/// let snapshot_copy = snapshot.clone();
/// ```
pub trait Conditions: Any {
    /// 返回条件的只读快照
    ///
    /// 这个方法返回当前条件的只读快照。
    ///
    /// # 返回值
    ///
    /// 返回当前条件的只读快照
    ///
    /// # 约束
    ///
    /// - 只读：不修改 Conditions 状态
    /// - 不可反推：无法从快照反推世界的完整状态
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::Conditions;
    ///
    /// let conditions = world.conditions();
    /// let snapshot = conditions.snapshot();
    /// ```
    fn snapshot(&self) -> ConditionSnapshot;
}

/// 条件快照
///
/// [`ConditionSnapshot`] 是条件的只读快照，可以被复制、投射。
///
/// 它是值对象，不包含对 Conditions 或 World 的引用。
///
/// # 字段
///
/// - [`signals`](ConditionSnapshot::signals)：条件信号集合
///
/// # 设计约束
///
/// - 可复制：可以被复制
/// - 可投射：可以被 Representation 映射和呈现
/// - 不可反推：无法从快照反推世界的完整状态
///
/// # 示例
///
/// ```rust
/// use biosphere::Conditions;
///
/// let conditions = world.conditions();
/// let snapshot = conditions.snapshot();
///
/// // 快照可以被复制
/// let snapshot_copy = snapshot.clone();
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ConditionSnapshot {
    /// 条件信号集合
    pub signals: Vec<ConditionSignal>,
}

/// 条件信号
///
/// [`ConditionSignal`] 描述单个条件信号。
///
/// # 字段
///
/// - [`kind`](ConditionSignal::kind)：信号种类
/// - [`intensity`](ConditionSignal::intensity)：信号强度
///
/// # 设计约束
///
/// - 可复制：可以被复制
/// - 可比较：可以被比较
/// - 可哈希：可以被哈希
///
/// # 示例
///
/// ```rust
/// use biosphere::ConditionSignal;
///
/// let signal = ConditionSignal {
///     kind: "temperature",
///     intensity: 25,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConditionSignal {
    /// 信号种类
    pub kind: &'static str,

    /// 信号强度
    pub intensity: i64,
}
