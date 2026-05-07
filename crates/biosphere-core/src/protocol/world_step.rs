use crate::protocol::arbiter::Arbiter;
use crate::reality::executor::Executor;
use crate::possibility::migration::Migration;

/// 世界跃迁的最小协议
///
/// [`WorldStep`] 描述"在给定当前世界状态下，
/// 是否发生一次存在拓扑的跃迁。"
///
/// 它不关心时间、循环或调度，
/// 只关心一次现实是否被提交。
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
///
/// # 方法
///
/// - [`step`](WorldStep::step)：尝试进行一次世界跃迁
///
/// # 设计原则
///
/// - **不拥有任何状态**：不保存 Topology、时间或历史
/// - **不包含策略**：不判断好坏、不计算收益、不比较主体
/// - **不循环**：一次 WorldStep = 一次尝试
/// - **可被完全替换**：不同世界可以有不同 WorldStep 规则
///
/// # 哲学含义
///
/// WorldStep 是"现实跃迁的协议"，不是"世界机制"。
///
/// 它回答了这样一个问题：
/// > "这一刻，世界是否承认一个新事实？"
///
/// 它不是"世界在流动"，而是"世界被允许改变一次"。
///
/// # 约束
///
/// - WorldStep 不持有 Topology
/// - WorldStep 不推进时间
/// - WorldStep 不修改世界
/// - WorldStep 不包含策略
/// - WorldStep 不循环
///
/// # 与其他组件的关系
///
/// - **Migration**：WorldStep 处理的对象
/// - **Arbiter**：WorldStep 使用的裁决器
/// - **Executor**：WorldStep 使用的执行器
/// - **Topology**：WorldStep 接收和返回的对象
///
/// # 示例
///
/// ```rust
/// use biosphere::world_step::WorldStep;
/// use biosphere::arbiter::Arbiter;
/// use biosphere::executor::Executor;
/// use biosphere::migration::Migration;
///
/// struct SimpleWorldStep;
///
/// impl<M> WorldStep<M> for SimpleWorldStep
/// where
///     M: Migration,
/// {
///     fn step(
///         &self,
///         current: &M::Topology,
///         candidates: &[M],
///         arbiter: &dyn Arbiter<M>,
///         executor: &dyn Executor<M>,
///     ) -> Option<M::Topology> {
///         let chosen = arbiter.arbitrate(candidates)?;
///         Some(executor.execute_and_return(current, chosen))
///     }
/// }
/// ```
pub trait WorldStep<M>
where
    M: Migration,
{
    /// 尝试进行一次世界跃迁
    ///
    /// 这个方法决定是否发生一次存在拓扑的跃迁。
    ///
    /// # 参数
    ///
    /// - `current`：当前存在拓扑
    /// - `candidates`：候选迁移（已被主体选择）
    /// - `arbiter`：裁决器
    /// - `executor`：执行器
    ///
    /// # 返回值
    ///
    /// - `Some(new_topology)`：世界发生了改变
    /// - `None`：本次世界保持不变
    ///
    /// # 效果
    ///
    /// - 使用 Arbiter 从候选迁移中选择一个
    /// - 使用 Executor 推演迁移的结果
    /// - 返回新的拓扑，或 None 表示不改变
    ///
    /// # 约束
    ///
    /// - WorldStep 不修改任何状态
    /// - WorldStep 不推进时间
    /// - WorldStep 不包含策略
    /// - WorldStep 不循环
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::world_step::WorldStep;
    /// use biosphere::arbiter::Arbiter;
    /// use biosphere::executor::Executor;
    /// use biosphere::migration::Migration;
    ///
    /// struct SimpleWorldStep;
    ///
    /// impl<M> WorldStep<M> for SimpleWorldStep
    /// where
    ///     M: Migration,
    /// {
    ///     fn step(
    ///         &self,
    ///         current: &M::Topology,
    ///         candidates: &[M],
    ///         arbiter: &dyn Arbiter<M>,
    ///         executor: &dyn Executor<M>,
    ///     ) -> Option<M::Topology> {
    ///         let chosen = arbiter.arbitrate(candidates)?;
    ///         Some(executor.execute_and_return(current, chosen))
    ///     }
    /// }
    /// ```
    fn step(
        &self,
        current: &M::Topology,
        candidates: &[M],
        arbiter: &dyn Arbiter<M>,
        executor: &dyn Executor<M>,
    ) -> Option<M::Topology>;
}
