use biosphere_core::Conditions;

/// ASCII 视图
///
/// [`AsciiView`] 定义了 ASCII 视图的接口。
///
/// # 设计约束
///
/// - 只读访问：只通过 Conditions 访问状态
/// - 不修改状态：不提供任何修改接口
/// - 不持有状态：不包含任何状态
/// - 不依赖 UI 框架：不依赖具体的 UI 框架
/// - 不包含渲染逻辑：不包含具体的渲染逻辑
///
/// # 哲学含义
///
/// AsciiView 是"ASCII 视图"，而不是"ASCII 渲染器"。
///
/// 这意味着：
/// - AsciiView 只提供数据接口，不渲染
/// - AsciiView 是只读计算器
/// - AsciiView 不处理事件
/// - AsciiView 不依赖 UI 框架
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::projection::AsciiView;
/// use biosphere_core::Conditions;
///
/// struct MySignalInfo {
///     intensity: f64,
/// }
///
/// struct MyAsciiViewModel {
///     width: usize,
///     height: usize,
///     signals: Vec<MySignalInfo>,
/// }
///
/// struct MyAsciiView;
///
/// impl AsciiView for MyAsciiView {
///     type ViewModel = MyAsciiViewModel;
///
///     fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel {
///         MyAsciiViewModel {
///             width: 80,
///             height: 24,
///             signals: Vec::new(),
///         }
///     }
/// }
/// ```
pub trait AsciiView {
    /// 视图模型类型
    type ViewModel;

    /// 渲染 ASCII 视图
    ///
    /// # 参数
    ///
    /// * `conditions` - 条件
    ///
    /// # 返回值
    ///
    /// 返回 ASCII 视图的视图模型
    fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::sensed_conditions::SensedConditions;
    use biosphere_core::ConditionSnapshot;

    const DEFAULT_WIDTH: usize = 80;
    const DEFAULT_HEIGHT: usize = 24;

    struct TestSignalInfo {
        _intensity: f64,
    }

    struct TestViewModel {
        width: usize,
        height: usize,
        signals: Vec<TestSignalInfo>,
    }

    struct TestAsciiView;

    impl AsciiView for TestAsciiView {
        type ViewModel = TestViewModel;

        fn render(&self, _conditions: &dyn Conditions) -> Self::ViewModel {
            TestViewModel {
                width: DEFAULT_WIDTH,
                height: DEFAULT_HEIGHT,
                signals: Vec::new(),
            }
        }
    }

    #[test]
    fn test_ascii_view_trait() {
        let view = TestAsciiView;
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        let conditions = SensedConditions::new(snapshot);
        let model = view.render(&conditions);
        assert_eq!(model.width, DEFAULT_WIDTH);
        assert_eq!(model.height, DEFAULT_HEIGHT);
        assert!(model.signals.is_empty());
    }
}
