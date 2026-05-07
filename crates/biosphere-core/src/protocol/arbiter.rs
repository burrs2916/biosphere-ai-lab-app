use crate::possibility::migration::Migration;

/// 裁决器的核心抽象
///
/// [`Arbiter`] 描述"在多个主体给出的迁移选择中，
/// 是否、以及选择哪一个提交给现实执行器。"
///
/// Arbiter 是社会结构的最小模型，
/// 但不包含任何策略、伦理或目标。
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
///
/// # 方法
///
/// - [`arbitrate`](Arbiter::arbitrate)：对多个候选迁移进行裁决
///
/// # 设计原则
///
/// - **非执行性**：Arbiter 不执行迁移
/// - **非选择性**：Arbiter 不生成迁移
/// - **裁决性**：Arbiter 决定是否接受一个候选
/// - **单一性**：一次只能提交一个迁移
///
/// # 哲学含义
///
/// Arbiter 是"多重可能性 → 单一现实"的闸门。
///
/// 它不解释为何裁决，不决定如何裁决，不选择裁决策略。
/// 它只是机械地裁决：从多个候选迁移中选择一个（或都不选择）。
///
/// # 裁决 ≠ 决策
///
/// Arbiter 不做决策，只做裁决。
///
/// - **决策**：基于目标、策略、价值判断
/// - **裁决**：基于规则、冲突解决、优先级
///
/// Arbiter 是裁决层，不是决策层。
///
/// # 约束
///
/// - Arbiter 不拥有 Migration
/// - Arbiter 不修改候选
/// - Arbiter 不执行迁移
/// - Arbiter 不持有 Embodiment
/// - Arbiter 不修改 Environment
/// - Arbiter 不推进时间
/// - Arbiter 不产生新 Migration
///
/// # 与其他组件的关系
///
/// - **Migration**：Arbiter 裁决的对象
/// - **Agency**：Arbiter 不依赖 Agency，选择权在 Agency
/// - **Executor**：Arbiter 的输出是 Executor 的输入
/// - **EmbodiedAgency**：Arbiter 不直接与 EmbodiedAgency 交互
///
/// # 示例
///
/// ```rust
/// use biosphere::arbiter::Arbiter;
/// use biosphere::migration::Migration;
///
/// struct FirstComeFirstServedArbiter;
///
/// impl<M> Arbiter<M> for FirstComeFirstServedArbiter
/// where
///     M: Migration,
/// {
///     fn arbitrate(&self, candidates: &[M]) -> Option<&M> {
///         candidates.first()
///     }
/// }
/// ```
pub trait Arbiter<M>
where
    M: Migration,
{
    /// 对多个候选迁移进行裁决
    ///
    /// 这个方法从多个候选迁移中选择一个（或都不选择），
    /// 作为提交给现实执行器的迁移。
    ///
    /// # 参数
    ///
    /// - `candidates`：来自不同主体的迁移选择
    ///
    /// # 返回值
    ///
    /// - `Some(&M)`：裁决通过的迁移
    /// - `None`：本轮不发生任何迁移
    ///
    /// # 效果
    ///
    /// - 从多个候选迁移中选择一个
    /// - 或者决定本轮不发生任何迁移
    ///
    /// # 约束
    ///
    /// - Arbiter 不拥有 Migration（返回的是引用）
    /// - Arbiter 不修改候选（只读访问）
    /// - Arbiter 不执行迁移（只选择）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::arbiter::Arbiter;
    /// use biosphere::migration::Migration;
    ///
    /// struct FirstComeFirstServedArbiter;
    ///
    /// impl<M> Arbiter<M> for FirstComeFirstServedArbiter
    /// where
    ///     M: Migration,
    /// {
    ///     fn arbitrate(&self, candidates: &[M]) -> Option<&M> {
    ///         candidates.first()
    ///     }
    /// }
    /// ```
    fn arbitrate(&self, candidates: &[M]) -> Option<&M>;
}
