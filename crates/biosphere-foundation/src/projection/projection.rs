use biosphere_core::Conditions;

/// 投射
///
/// [`Projection`] 描述如何将 Conditions 映射为视图模型。
///
/// 它是 Conditions 的视图，可以被复制、投射。
///
/// # 设计约束
///
/// - 可复制：可以被复制
/// - 可投射：可以被 Representation 映射和呈现
/// - 不可反推：无法从 Projection 反推世界的完整状态
/// - 不包含渲染逻辑：只返回视图模型，不进行渲染
///
/// # 哲学含义
///
/// Projection 是"Conditions 的视图"，而不是"世界的完整状态"。
///
/// 这意味着：
/// - Projection 可以被复制
/// - Projection 可以被 Representation 映射和呈现
/// - 无法从 Projection 反推世界的完整状态
/// - Projection 只提供数据接口，不渲染
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::projection::Projection;
/// use biosphere_core::Conditions;
///
/// struct MyViewModel {
///     data: String,
/// }
///
/// struct MyProjection;
///
/// impl Projection for MyProjection {
///     type ViewModel = MyViewModel;
///
///     fn render(&self, _conditions: &dyn Conditions) -> Self::ViewModel {
///         MyViewModel { data: String::new() }
///     }
/// }
/// ```
pub trait Projection {
    /// 视图模型类型
    type ViewModel;

    /// 返回投射的视图模型
    ///
    /// # 参数
    ///
    /// * `conditions` - 条件
    ///
    /// # 返回值
    ///
    /// 返回投射的视图模型
    ///
    /// # 设计约束
    ///
    /// - 只返回视图模型，不进行渲染
    /// - 渲染逻辑由 UI 层负责
    /// - Foundation 层只提供数据接口
    fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::sensed_conditions::SensedConditions;
    use biosphere_core::ConditionSnapshot;

    struct TestViewModel {
        data: String,
    }

    struct TestProjection;

    impl Projection for TestProjection {
        type ViewModel = TestViewModel;

        fn render(&self, _conditions: &dyn Conditions) -> Self::ViewModel {
            TestViewModel { data: String::new() }
        }
    }

    #[test]
    fn test_projection_trait() {
        let projection = TestProjection;
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        let conditions = SensedConditions::new(snapshot);
        let model = projection.render(&conditions);
        assert!(model.data.is_empty());
    }
}
