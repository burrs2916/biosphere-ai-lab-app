//! 能动性的基础定义
//!
//! 这个模块定义了 [`Agency`] trait，它描述了存在对迁移的选择能力。
//!
//! Agency 不是行为，不是调度，不是控制，
//! 而是"在多个可能的迁移中进行选择的资格"。
//!
//! # 核心公理
//!
//! Agency 是一种"对多个 Migration 的选择能力"。
//!
//! # 设计原则
//!
//! - **选择性**：Agency 只做选择，不做执行
//! - **被动性**：Agency 不主动推进世界
//! - **非必然性**：有选择能力 ≠ 一定发生迁移
//! - **最小性**：不包含任何策略或目标
//! - **解耦性**：Agency 不需要知道世界的构造细节
//!
//! # 哲学断言
//!
//! 如果一个存在没有 Agency，
//! 那么所有迁移都只能来自外部条件或高层机制。
//!
//! # 与其他组件的关系
//!
//! - **Migration**：Agency 选择的候选对象
//! - **Environment**：Agency 不能直接修改
//! - **Field**：Agency 不能直接修改
//! - **ExistenceCore**：Agency 是可选层，不是所有生命都有 Agency
//!
//! # Agency ≠ 的铁律
//!
//! - **Agency ≠ 行为**：Agency 不是 doing something，而是 having the capacity to choose
//! - **Agency ≠ 时间**：Agency 不推进时间、不调度、不循环
//! - **Agency ≠ 生命本体**：不是所有生命都有 Agency，Agency 是一种可选的存在层
//! - **Agency ≠ 世界控制**：Agency 不能直接改 Environment，不能直接改 Field，只能选择 Migration
//! - **Agency ≠ 必然性**：有 Agency ≠ 一定发生迁移，只是存在选择的可能性
//!
//! # 示例
//!
//! ```rust
//! use biosphere::agency::Agency;
//! use biosphere::migration::Migration;
//! use biosphere::existential_topology::ExistentialTopology;
//!
//! // 假设有一个具体的 Migration 实现
//! struct MyMigration;
//!
//! impl Migration for MyMigration {
//!     type Embodiment = MyEmbodiment;
//!     type Field = MyField;
//!     type Environment = MyEnvironment;
//!     type Topology = MyTopology;
//!
//!     fn apply(&self, from: &Self::Topology) -> Self::Topology {
//!         MyTopology
//!     }
//! }
//!
//! // 一个具体的 Agency 实现
//! struct MyAgency;
//!
//! impl Agency<MyMigration> for MyAgency {
//!     fn choose(&self, options: &[MyMigration]) -> Option<&MyMigration> {
//!         // 从候选迁移中选择一个
//!         // 或者返回 None 表示不进行任何迁移
//!         options.first()
//!     }
//! }
//! ```

use crate::possibility::migration::Migration;

/// 能动性的核心抽象
///
/// [`Agency`] 描述"谁有资格选择迁移"，
/// 而不是"迁移如何执行"。
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
///
/// # 方法
///
/// - [`choose`](Agency::choose)：在多个可能的迁移中进行选择
///
/// # 设计原则
///
/// - **选择性**：Agency 只做选择，不做执行
/// - **被动性**：Agency 不主动推进世界
/// - **非必然性**：有选择能力 ≠ 一定发生迁移
/// - **最小性**：不包含任何策略或目标
/// - **解耦性**：Agency 不需要知道世界的构造细节
///
/// # 哲学含义
///
/// Agency 不拥有 Migration，
/// Agency 不创造 Migration，
/// Agency 只是从"可能性空间"中选一个。
///
/// 这是"自由意志的最小模型"，
/// 也是"策略接口的最小抽象"。
///
/// Migration 已经是"自描述的可能性实体"，
/// 它内部携带了它所作用的世界类型信息，
/// 所以 Agency 不需要显式感知 E / F / Env / T。
///
/// # 示例
///
/// ```rust
/// use biosphere::agency::Agency;
/// use biosphere::migration::Migration;
/// use biosphere::existential_topology::ExistentialTopology;
///
/// struct MyMigration;
///
/// impl Migration for MyMigration {
///     type Embodiment = MyEmbodiment;
///     type Field = MyField;
///     type Environment = MyEnvironment;
///     type Topology = MyTopology;
///
///     fn apply(&self, from: &Self::Topology) -> Self::Topology {
///         MyTopology
///     }
/// }
///
/// struct MyAgency;
///
/// impl Agency<MyMigration> for MyAgency {
///     fn choose(&self, options: &[MyMigration]) -> Option<&MyMigration> {
///         // 从候选迁移中选择一个
///         // 或者返回 None 表示不进行任何迁移
///         options.first()
///     }
/// }
/// ```
pub trait Agency<M>
where
    M: Migration,
{
    /// 在多个可能的迁移中进行选择
    ///
    /// 返回 `Some(Migration)` 表示选择了一个迁移
    /// 返回 `None` 表示当前不进行任何迁移
    ///
    /// # 参数
    ///
    /// - `options`：候选迁移的集合
    ///
    /// # 返回值
    ///
    /// - `Some(&M)`：选择了一个迁移
    /// - `None`：不进行任何迁移
    ///
    /// # 约束
    ///
    /// - Agency 不拥有 Migration（返回的是引用）
    /// - Agency 不创造 Migration（从候选中选择）
    /// - Agency 不执行 Migration（只选择）
    /// - Agency 不修改候选集合（只读访问）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::agency::Agency;
    /// use biosphere::migration::Migration;
    /// use biosphere::existential_topology::ExistentialTopology;
    ///
    /// struct MyMigration;
    ///
    /// impl Migration for MyMigration {
    ///     type Embodiment = MyEmbodiment;
    ///     type Field = MyField;
    ///     type Environment = MyEnvironment;
    ///     type Topology = MyTopology;
    ///
    ///     fn apply(&self, from: &Self::Topology) -> Self::Topology {
    ///         MyTopology
    ///     }
    /// }
    ///
    /// struct MyAgency;
    ///
    /// impl Agency<MyMigration> for MyAgency {
    ///     fn choose(&self, options: &[MyMigration]) -> Option<&MyMigration> {
    ///         // 从候选迁移中选择一个
    ///         // 或者返回 None 表示不进行任何迁移
    ///         options.first()
    ///     }
    /// }
    /// ```
    fn choose(&self, options: &[M]) -> Option<&M>;
}
