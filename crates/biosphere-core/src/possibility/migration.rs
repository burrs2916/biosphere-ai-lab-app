//! 存在迁移的基础定义
//!
//! 这个模块定义了 [`Migration`] trait，它描述存在承认关系如何发生变化。
//!
//! 迁移不是行为，不是过程，不是调度，
//! 而是从一个存在拓扑到另一个存在拓扑的映射。
//!
//! # 核心公理
//!
//! 迁移不是存在的动作，而是存在承认关系的改变。
//!
//! Migration 是一个"自包含的世界变换可能性"。
//!
//! # 设计原则
//!
//! - **自描述性**：Migration 内部携带它所作用的世界类型信息
//! - **描述性**：Migration 只描述"关系如何改变"，不执行改变
//! - **被动性**：Migration 不决定"谁去改变"或"何时改变"
//! - **无因果**：Migration 不解释关系为何改变
//! - **无时间**：Migration 不记录改变发生的时间或顺序
//!
//! # 与其他组件的关系
//!
//! - **ExistentialTopology**：Migration 的输入和输出类型（通过关联类型）
//! - **Field**：Migration 不改变 Field 的定义
//! - **Environment**：Migration 不改变 Environment 的状态
//! - **Embodiment**：Migration 不改变 Embodiment 的本质
//!
//! # 哲学含义
//!
//! Migration 是一个完整的存在描述，
//! 它内部已经携带了：
//! - 它适用于哪种世界
//! - 它映射的是哪类存在事实
//!
//! 这意味着：
//! - Migration 不是一个"需要被外部世界补完的模板"
//! - Migration 是一个"自洽的可能性"
//! - Agency 只需要面对 Migration 本身，不需要知道世界的构造细节
//!
//! # 示例
//!
//! ```rust
//! use biosphere::migration::Migration;
//! use biosphere::existential_topology::ExistentialTopology;
//!
//! // 假设有一个具体的 ExistentialTopology 实现
//! struct MyTopology;
//!
//! // 一个具体的 Migration 实现
//! struct MyMigration;
//!
//! impl Migration for MyMigration {
//!     type Embodiment = MyEmbodiment;
//!     type Field = MyField;
//!     type Environment = MyEnvironment;
//!     type Topology = MyTopology;
//!
//!     fn apply(&self, from: &Self::Topology) -> Self::Topology {
//!         // 描述如何从一个拓扑映射到另一个拓扑
//!         // 这里不执行任何"动作"，只返回新的拓扑
//!         MyTopology
//!     }
//! }
//! ```

use crate::topology::existential_topology::ExistentialTopology;

/// 存在迁移的核心抽象
///
/// [`Migration`] 描述"存在承认关系如何改变"，
/// 而不是"谁去改变"。
///
/// Migration 是一个"自描述的可能性实体"，
/// 它内部携带了它所作用的世界类型信息。
///
/// # 关联类型
///
/// - [`Embodiment`](Migration::Embodiment)：具身类型，必须实现 [`Embodiment`]
/// - [`Field`](Migration::Field)：场域类型，必须实现 [`Field`]
/// - [`Environment`](Migration::Environment)：环境类型，必须实现 [`Environment`]
/// - [`Topology`](Migration::Topology)：存在拓扑的类型，必须实现 [`ExistentialTopology`]
///
/// # 方法
///
/// - [`apply`](Migration::apply)：给定一个存在拓扑，产生一个新的存在拓扑
///
/// # 设计约束
///
/// - Migration 不包含额外的 Field、Environment、Embodiment 的引用
/// - Migration 不记录时间信息
/// - Migration 不触发任何副作用
/// - Migration 是纯函数式的映射
///
/// # 哲学含义
///
/// 迁移是"形式"，而不是"观察视角"。
/// Migration 不关心观察视角，只关心结构本身。
/// 生命周期属于具体的存在拓扑实现，而不属于 Migration。
///
/// Migration 是一个完整的存在描述，
/// 它内部已经携带了：
/// - 它适用于哪种世界
/// - 它映射的是哪类存在事实
///
/// 这意味着：
/// - Migration 不是一个"需要被外部世界补完的模板"
/// - Migration 是一个"自洽的可能性"
/// - Agency 只需要面对 Migration 本身，不需要知道世界的构造细节
///
/// # 类型系统约束
///
/// `Topology: ExistentialTopology` 表示：
/// "这个拓扑是一个合法的存在拓扑。"
///
/// 这保证了 Migration 可以接受任何实现了 ExistentialTopology 的拓扑，
/// 而不需要知道具体的实现细节。
///
/// # 示例
///
/// ```rust
/// use biosphere::migration::Migration;
/// use biosphere::existential_topology::ExistentialTopology;
///
/// struct MyTopology;
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
///         // 纯函数式映射
///         MyTopology
///     }
/// }
/// ```
pub trait Migration
where
    Self::Embodiment: crate::ontology::embodiment::Embodiment,
    Self::Field: crate::ontology::field::Field<Environment = Self::Environment, Embodiment = Self::Embodiment>,
    Self::Environment: crate::ontology::environment::Environment,
    Self::Topology: ExistentialTopology,
{
    /// 具身类型
    type Embodiment;

    /// 场域类型
    type Field;

    /// 环境类型
    type Environment;

    /// 存在拓扑类型
    type Topology;

    /// 给定一个存在拓扑，产生一个新的存在拓扑
    ///
    /// 这个方法不要求执行迁移，
    /// 只描述迁移后"哪些关系成立"。
    ///
    /// # 参数
    ///
    /// - `from`：原始的存在拓扑
    ///
    /// # 返回值
    ///
    /// 返回一个新的存在拓扑，描述迁移后的关系状态
    ///
    /// # 约束
    ///
    /// - 不修改原始拓扑（`from` 是不可变引用）
    /// - 不触发任何副作用
    /// - 不记录时间信息
    /// - 不解释为何发生改变
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::migration::Migration;
    /// use biosphere::existential_topology::ExistentialTopology;
    ///
    /// struct MyTopology;
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
    ///         // 纯函数式映射
    ///         MyTopology
    ///     }
    /// }
    /// ```
    fn apply(&self, from: &Self::Topology) -> Self::Topology;
}
