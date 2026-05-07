//! 存在关系的基础定义
//!
//! 这个模块定义了 [`ExistentialRelationKind`]，它描述世界承认的关系"类型"。
//!
//! # 核心公理
//!
//! 关系被当作"类型"，而不是"事实"。
//!
//! [`ExistentialRelationKind`] 只定义关系的种类，不带引用，不带数据。
//! 关系的"事实实例"只能存在于 biosphere-foundation 的拓扑中。
//!
//! # 设计原则
//!
//! - **类型性**：只定义关系种类，不携带具体数据
//! - **不可伪造**：关系事实只能由 World/Topology 生成
//! - **语义分离**：关系类型（core）与关系事实（foundation）分离
//!
//! # 与其他组件的关系
//!
//! - **ExistentialTopology**：使用 [`ExistentialRelationKind`] 来分类关系事实
//! - **Field**：关系描述了存在与场域之间的连接
//! - **Environment**：关系描述了环境与场域之间的连接
//!
//! # 哲学含义
//!
//! 关系是"世界承认的类型"，而不是"生命声称的事实"。
//!
//! 这意味着：
//! - 任何存在都不能说"我在某个 Field"
//! - 任何代码都不能伪造关系
//! - 只有世界拓扑能说"这是事实"
//!
//! 这是整个系统防止"语义造假"的核心机制。
//!
//! # 示例
//!
//! ```rust
//! use biosphere::ExistentialRelationKind;
//!
//! // 关系类型只是枚举，不携带任何数据
//! let relation_kind = ExistentialRelationKind::EmbodimentInField;
//! ```

/// 存在关系的种类
///
/// [`ExistentialRelationKind`] 定义了世界承认的关系"类型"。
///
/// 它只定义关系的种类，不带引用，不带数据。
/// 关系的"事实实例"只能存在于 biosphere-foundation 的拓扑中。
///
/// # 变体
///
/// - [`EmbodimentInField`](ExistentialRelationKind::EmbodimentInField)：具身存在于场域中
/// - [`EnvironmentInField`](ExistentialRelationKind::EnvironmentInField)：环境存在于场域中
///
/// # 设计约束
///
/// - 不携带任何数据
/// - 不包含引用
/// - 不能被手动构造为"事实"
///
/// # 哲学含义
///
/// 关系是"世界承认的类型"，而不是"生命声称的事实"。
///
/// 这意味着：
/// - 任何存在都不能说"我在某个 Field"
/// - 任何代码都不能伪造关系
/// - 只有世界拓扑能说"这是事实"
///
/// # 示例
///
/// ```rust
/// use biosphere::ExistentialRelationKind;
///
/// // 关系类型只是枚举，不携带任何数据
/// let relation_kind = ExistentialRelationKind::EmbodimentInField;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExistentialRelationKind {
    /// 具身存在于场域中
    ///
    /// 表示某个具身（生命系统）存在于某个场域中。
    EmbodimentInField,

    /// 环境存在于场域中
    ///
    /// 表示某个环境存在于某个场域中。
    EnvironmentInField,
}
