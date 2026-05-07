use crate::manifest::Manifest;
use crate::perception::Perception;

/// 感知构建器
///
/// [`PerceptionBuilder`] 定义了从 Manifest 构建 Perception 的接口。
///
/// # 设计约束
///
/// - 纯函数：构建过程不产生副作用
/// - 确定性：相同的 Manifest 总是产生相同的 Perception
/// - 感知顺序：按照人类感知的顺序构建
/// - 不可变：不修改原始 Manifest
///
/// # 哲学含义
///
/// PerceptionBuilder 是"人类如何一步一步'看'Manifest 的策略"，而不是"渲染策略"。
///
/// 这意味着：
/// - PerceptionBuilder 定义感知顺序，不是渲染顺序
/// - PerceptionBuilder 是纯函数，不是有状态的对象
/// - PerceptionBuilder 可以有不同的实现策略
pub trait PerceptionBuilder {
    /// 从 Manifest 构建 Perception
    ///
    /// # 参数
    ///
    /// * `manifest` - 要构建的 Manifest
    ///
    /// # 返回值
    ///
    /// 返回构建的 Perception
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{PerceptionBuilder, Perception};
    /// use biosphere_foundation::manifest::Manifest;
    ///
    /// let builder = MyPerceptionBuilder;
    /// let perception = builder.build(&manifest);
    /// ```
    fn build(&self, manifest: &Manifest) -> Perception;
}