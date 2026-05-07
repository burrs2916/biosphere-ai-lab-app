use crate::protocol::arbiter::Arbiter;
use crate::reality::executor::Executor;
use crate::possibility::migration::Migration;
use crate::protocol::world_step::WorldStep;

/// 世界跃迁协议的组合体
///
/// [`WorldTransition`] 是"世界跃迁能力的描述"，不是调用点。
///
/// 它组合了世界跃迁所需的所有协议：
/// - [`WorldStep`]：一次跃迁的规则
/// - [`Arbiter`]：裁决规则
/// - [`Executor`]：现实承认规则
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
///
/// # 设计原则
///
/// - **能力描述**：WorldTransition 描述"世界允许什么"，不是"世界被谁调用"
/// - **协议组合**：组合多个协议，形成完整的跃迁能力
/// - **可复用性**：一次组装，多次使用
/// - **无状态性**：不持有拓扑或时间状态
///
/// # 哲学含义
///
/// WorldTransition 是"世界自包含跃迁能力"的体现。
///
/// 它回答了这样一个问题：
/// > "世界本身具备哪些跃迁协议？"
///
/// 它不是"外部驱动者"，而是"世界能力的描述"。
///
/// # 约束
///
/// - WorldTransition 不持有 Topology
/// - WorldTransition 不推进时间
/// - WorldTransition 不包含调用逻辑
/// - WorldTransition 不包含循环
/// - WorldTransition 不包含调度
///
/// # 与其他组件的关系
///
/// - **WorldStep**：WorldTransition 组合的协议之一
/// - **Arbiter**：WorldTransition 组合的协议之一
/// - **Executor**：WorldTransition 组合的协议之一
/// - **TemporalEnvironment**：并列的世界能力（时间协议）
///
/// # 使用示例
///
/// ```rust
/// use biosphere::world_transition::WorldTransition;
/// use biosphere::world_step::WorldStep;
/// use biosphere::arbiter::Arbiter;
/// use biosphere::executor::Executor;
/// use biosphere::migration::Migration;
///
/// // 组装世界跃迁协议
/// let transition = WorldTransition {
///     step: &world_step,
///     arbiter: &arbiter,
///     executor: &executor,
/// };
///
/// // 使用协议进行跃迁
/// let new_topology = transition.step.step(
///     &current_topology,
///     &candidates,
///     transition.arbiter,
///     transition.executor,
/// );
/// ```
pub struct WorldTransition<'a, M>
where
    M: Migration,
{
    /// 一次跃迁的规则
    pub step: &'a dyn WorldStep<M>,

    /// 裁决规则
    pub arbiter: &'a dyn Arbiter<M>,

    /// 现实承认规则
    pub executor: &'a dyn Executor<M>,
}
