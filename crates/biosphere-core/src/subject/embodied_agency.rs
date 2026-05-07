//! 具身能动性的基础定义
//!
//! 这个模块定义了 [`EmbodiedAgency`] trait，它描述了具身存在与能动性能力的组合。
//!
//! # 核心公理
//!
//! EmbodiedAgency 是一种"组合存在"，它不是生命本体，不是世界，不是行为，
//! 而是「一个具身存在 + 一个能动性能力」在当前现实中的联合体。
//!
//! # 设计原则
//!
//! - **组合性**：EmbodiedAgency 通过组合 Embodiment 和 Agency 实现，而非继承
//! - **静态性**：所有类型在编译时确定，提供零运行时开销
//! - **最小性**：只包含必要的组合关系，不引入额外约束
//! - **解耦性**：Embodiment 和 Agency 可以独立变化
//!
//! # 哲学含义
//!
//! EmbodiedAgency 回答了这样一个问题：
//! > 一个具身存在如何拥有选择能力？
//!
//! 这个定义连接了：
//! - [`Embodiment`] - 定义"生命如何站在世界里"
//! - [`Agency`] - 定义"谁有资格选择迁移"
//!
//! # 与其他组件的关系
//!
//! - **Embodiment**：EmbodiedAgency 组合的一个组件
//! - **Agency**：EmbodiedAgency 组合的另一个组件
//! - **Migration**：Agency 选择的候选对象
//!
//! # 组合而非绑定
//!
//! EmbodiedAgency 不要求 Embodiment 直接实现 Agency，
//! 而是通过组合的方式将两者联合起来。
//!
//! 这意味着：
//! - 同一个 Embodiment 可以配合不同的 Agency
//! - 同一个 Agency 可以配合不同的 Embodiment
//!
//! # 示例
//!
//! ```rust
//! use biosphere::embodied_agency::EmbodiedAgency;
//! use biosphere::embodiment::Embodiment;
//! use biosphere::agency::Agency;
//! use biosphere::migration::Migration;
//!
//! struct MyEmbodiedAgency<E, A> {
//!     embodiment: E,
//!     agency: A,
//! }
//!
//! impl<M, E, A> EmbodiedAgency<M, E, A> for MyEmbodiedAgency<E, A>
//! where
//!     E: Embodiment,
//!     A: Agency<M>,
//!     M: Migration,
//! {
//!     fn embodiment(&self) -> &E {
//!         &self.embodiment
//!     }
//!
//!     fn agency(&self) -> &A {
//!         &self.agency
//!     }
//! }
//! ```

use crate::ontology::embodiment::Embodiment;
use crate::subject::agency::Agency;
use crate::possibility::migration::Migration;

/// 具身能动性的核心抽象
///
/// [`EmbodiedAgency`] 描述「一个具身存在 + 一个能动性能力」的组合，
/// 而不是具身存在直接实现能动性。
///
/// # 类型参数
///
/// - `M`：迁移类型，必须实现 [`Migration`]
/// - `E`：具身类型，必须实现 [`Embodiment`]
/// - `A`：能动性类型，必须实现 [`Agency<M>`]
///
/// # 方法
///
/// - [`embodiment`](EmbodiedAgency::embodiment)：获取具身存在的不可变引用
/// - [`agency`](EmbodiedAgency::agency)：获取能动性能力的不可变引用
///
/// # 设计原则
///
/// - **组合性**：通过组合而非继承实现
/// - **静态性**：所有类型在编译时确定，零运行时开销
/// - **最小性**：只包含必要的组合关系
/// - **解耦性**：Embodiment 和 Agency 可以独立变化
///
/// # 哲学含义
///
/// EmbodiedAgency 的方法返回具体类型的引用，
/// 这在存在论上表示：陈述一个事实，而非施加一个动作。
///
/// # 存在论最小形态
///
/// 这是 EmbodiedAgency 的"存在论最小形态"，
/// 它只描述"一个具身存在和一个能动性能力的联合"，
/// 不包含任何执行、调度或控制逻辑。
///
/// # 示例
///
/// ```rust
/// use biosphere::embodied_agency::EmbodiedAgency;
/// use biosphere::embodiment::Embodiment;
/// use biosphere::agency::Agency;
/// use biosphere::migration::Migration;
///
/// struct MyEmbodiedAgency<E, A> {
///     embodiment: E,
///     agency: A,
/// }
///
/// impl<M, E, A> EmbodiedAgency<M, E, A> for MyEmbodiedAgency<E, A>
/// where
///     E: Embodiment,
///     A: Agency<M>,
///     M: Migration,
/// {
///     fn embodiment(&self) -> &E {
///         &self.embodiment
///     }
///
///     fn agency(&self) -> &A {
///         &self.agency
///     }
/// }
/// ```
pub trait EmbodiedAgency<M, E, A>
where
    E: Embodiment,
    A: Agency<M>,
    M: Migration,
{
    /// 获取具身存在的不可变引用
    ///
    /// 这在存在论上表示：陈述一个事实，而非施加一个动作。
    ///
    /// # 返回值
    ///
    /// 返回具身存在的不可变引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::embodied_agency::EmbodiedAgency;
    ///
    /// fn inspect_ea<M, E, A>(ea: &dyn EmbodiedAgency<M, E, A>) {
    ///     let body = ea.embodiment();
    ///     // 陈述一个事实：这个具身存在拥有某个生命定义
    /// }
    /// ```
    fn embodiment(&self) -> &E;

    /// 获取能动性能力的不可变引用
    ///
    /// 这在存在论上表示：陈述一个事实，而非施加一个动作。
    ///
    /// # 返回值
    ///
    /// 返回能动性能力的不可变引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::embodied_agency::EmbodiedAgency;
    ///
    /// fn inspect_ea<M, E, A>(ea: &dyn EmbodiedAgency<M, E, A>) {
    ///     let agency = ea.agency();
    ///     // 陈述一个事实：这个存在拥有选择能力
    /// }
    /// ```
    fn agency(&self) -> &A;
}
