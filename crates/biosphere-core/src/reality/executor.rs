//! 现实执行器的基础定义
//!
//! 这个模块定义了 [`Executor`] trait，它描述了存在拓扑如何接受迁移并确认为新的现实状态。
//!
//! # 核心公理
//!
//! Executor 是"世界承认迁移为现实"的机制。
//!
//! 它不是主体执行，而是世界层级接受一次迁移映射。
//!
//! # 设计原则
//!
//! - **唯一性**：同一时刻只有一个现实拓扑
//! - **被动性**：Executor 不创造 Migration
//! - **无选择性**：Executor 不选择 Migration
//! - **无策略性**：Executor 不判断好坏
//! - **双语义性**：Executor 支持有状态的现实提交和无状态的现实推演
//!
//! # 哲学含义
//!
//! Executor 回答了这样一个问题：
//! > 可能性如何成为现实？
//!
//! 这个定义连接了：
//! - [`Migration`] - 描述"可能性本身"
//! - [`ExistentialTopology`] - 描述"现实状态"
//!
//! # 执行语义
//!
//! Executor 提供两种执行语义：
//!
//! 1. **现实提交**（`execute`）：世界真的改变了
//!    - 有状态操作，修改内部拓扑
//!    - 等价于"世界承认一个新事实"
//!
//! 2. **现实推演**（`execute_and_return`）：世界"如果"改变会怎样
//!    - 无状态操作，不修改任何状态
//!    - 等价于"推演迁移的结果"
//!
//! 这两种语义是平级的，不是主次关系。
//!
//! # 与其他组件的关系
//!
//! - **Migration**：Executor 执行的对象
//! - **Agency**：Executor 不依赖 Agency，选择权在 Agency
//! - **ExistentialTopology**：Executor 持有并更新的现实状态
//!
//! # 执行 ≠ 选择
//!
//! Executor 不选择 Migration，只执行被选择的 Migration。
//!
//! 这意味着：
//! - 选择权在 Agency
//! - 执行权在 Executor
//! - 两者完全分离
//!
//! # 示例
//!
//! ```rust
//! use biosphere::executor::Executor;
//! use biosphere::migration::Migration;
//!
//! struct WorldExecutor<T> {
//!     topology: T,
//! }
//!
//! impl<M> Executor<M> for WorldExecutor<M::Topology>
//! where
//!     M: Migration,
//! {
//!     fn execute(&mut self, migration: &M) {
//!         self.topology = migration.apply(&self.topology);
//!     }
//!
//!     fn execute_and_return(
//!         &self,
//!         current: &M::Topology,
//!         migration: &M,
//!     ) -> M::Topology {
//!         migration.apply(current)
//!     }
//! }
//! ```

use crate::possibility::migration::Migration;

/// 现实执行器的核心抽象
///
/// [`Executor`] 描述"一个存在拓扑，如何接受一次迁移，并将其结果确认为新的现实状态。"
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
///
/// # 方法
///
/// - [`execute`](Executor::execute)：将一个迁移确认为现实（有状态）
/// - [`execute_and_return`](Executor::execute_and_return)：在给定事实下推演迁移结果（无状态）
///
/// # 设计原则
///
/// - **唯一性**：同一时刻只有一个现实拓扑
/// - **被动性**：Executor 不创造 Migration
/// - **无选择性**：Executor 不选择 Migration
/// - **无策略性**：Executor 不判断好坏
/// - **双语义性**：Executor 支持有状态的现实提交和无状态的现实推演
///
/// # 哲学含义
///
/// Executor 是"现实发生"的最小模型。
///
/// 它不解释为何执行，不决定是否执行，不选择执行哪个迁移。
/// 它只是机械地执行：将迁移应用到当前拓扑，并确认结果为新的现实。
///
/// # 执行 ≠ 选择
///
/// Executor 不选择 Migration，只执行被选择的 Migration。
///
/// - **Agency**：选择 Migration（有资格）
/// - **Executor**：执行 Migration（有权力）
///
/// 这两者完全分离，符合"组合而非绑定"的原则。
///
/// # 约束
///
/// - Executor 不返回旧拓扑
/// - Executor 不解释为何执行
/// - Executor 不决定是否执行
/// - Executor 不选择执行哪个迁移
///
/// # 示例
///
/// ```rust
/// use biosphere::executor::Executor;
/// use biosphere::migration::Migration;
///
/// struct WorldExecutor<T> {
///     topology: T,
/// }
///
/// impl<M> Executor<M> for WorldExecutor<M::Topology>
/// where
///     M: Migration,
/// {
///     fn execute(&mut self, migration: &M) {
///         self.topology = migration.apply(&self.topology);
///     }
/// }
/// ```
pub trait Executor<M>
where
    M: Migration,
{
    /// 将一个迁移确认为现实
    ///
    /// 这个方法执行迁移，将结果确认为新的现实状态。
    ///
    /// # 参数
    ///
    /// - `migration`：被选择的迁移
    ///
    /// # 效果
    ///
    /// - 使用 `migration.apply(...)` 计算新的拓扑
    /// - 更新内部拓扑状态为新的拓扑
    ///
    /// # 前置条件
    ///
    /// - 此方法只适用于持有现实拓扑的 Executor 实现
    /// - Stateless Executor 不应被用于现实提交路径
    ///
    /// # 约束
    ///
    /// - Executor 不返回旧拓扑
    /// - Executor 不解释为何执行
    /// - Executor 不决定是否执行
    /// - Executor 不选择执行哪个迁移
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::executor::Executor;
    /// use biosphere::migration::Migration;
    ///
    /// struct WorldExecutor<T> {
    ///     topology: T,
    /// }
    ///
    /// impl<M> Executor<M> for WorldExecutor<M::Topology>
    /// where
    ///     M: Migration,
    /// {
    ///     fn execute(&mut self, migration: &M) {
    ///         self.topology = migration.apply(&self.topology);
    ///     }
    /// }
    /// ```
    fn execute(&mut self, _migration: &M) {
        panic!("This executor does not support stateful execution");
    }

    /// 在给定事实下推演迁移结果
    ///
    /// 这个方法不提交现实，只返回"如果发生会怎样"。
    ///
    /// # 参数
    ///
    /// - `current`：当前存在拓扑
    /// - `migration`：被选择的迁移
    ///
    /// # 返回值
    ///
    /// 返回迁移应用后的新拓扑
    ///
    /// # 效果
    ///
    /// - 使用 `migration.apply(...)` 计算新的拓扑
    /// - 不修改任何状态
    ///
    /// # 约束
    ///
    /// - Executor 不修改任何状态
    /// - Executor 不提交现实
    /// - Executor 不决定是否执行
    /// - Executor 不选择执行哪个迁移
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::executor::Executor;
    /// use biosphere::migration::Migration;
    ///
    /// struct WorldExecutor<T> {
    ///     topology: T,
    /// }
    ///
    /// impl<M> Executor<M> for WorldExecutor<M::Topology>
    /// where
    ///     M: Migration,
    /// {
    ///     fn execute(&mut self, migration: &M) {
    ///         self.topology = migration.apply(&self.topology);
    ///     }
    /// }
    /// ```
    fn execute_and_return(
        &self,
        _current: &M::Topology,
        _migration: &M,
    ) -> M::Topology {
        panic!("This executor does not support stateless execution");
    }
}
