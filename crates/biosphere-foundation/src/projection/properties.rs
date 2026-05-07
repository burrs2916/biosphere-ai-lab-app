use biosphere_core::{EntityId, Conditions};

/// 属性视图
///
/// [`PropertiesView`] 定义了属性视图的接口。
///
/// # 设计约束
///
/// - 只读访问：只通过 Conditions 访问状态
/// - 不修改状态：不提供任何修改接口
/// - 不持有状态：不包含任何状态
/// - 不依赖 UI 框架：不依赖具体的 UI 框架
///
/// # 哲学含义
///
/// PropertiesView 是"属性视图"，而不是"属性编辑器"。
///
/// 这意味着：
/// - PropertiesView 只显示属性，不修改属性
/// - PropertiesView 是只读计算器
/// - PropertiesView 不处理事件
/// - PropertiesView 不依赖 UI 框架
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::projection::PropertiesView;
/// use biosphere_core::{Conditions, EntityId};
///
/// struct MyPropertiesViewModel {
///     entity_id: EntityId,
/// }
///
/// struct MyPropertiesView;
///
/// impl PropertiesView for MyPropertiesView {
///     type ViewModel = MyPropertiesViewModel;
///
///     fn render(&self, conditions: &dyn Conditions, entity_id: EntityId) -> Self::ViewModel {
///         MyPropertiesViewModel { entity_id }
///     }
/// }
/// ```
pub trait PropertiesView {
    /// 视图模型类型
    type ViewModel;

    /// 渲染属性视图
    ///
    /// # 参数
    ///
    /// * `conditions` - 条件
    /// * `entity_id` - 实体 ID
    ///
    /// # 返回值
    ///
    /// 返回属性视图的视图模型
    fn render(&self, conditions: &dyn Conditions, entity_id: EntityId) -> Self::ViewModel;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::sensed_conditions::SensedConditions;
    use biosphere_core::ConditionSnapshot;

    struct TestViewModel {
        entity_id: EntityId,
    }

    impl TestViewModel {
        fn new(entity_id: EntityId) -> Self {
            Self { entity_id }
        }
    }

    struct TestPropertiesView;

    impl PropertiesView for TestPropertiesView {
        type ViewModel = TestViewModel;

        fn render(&self, _conditions: &dyn Conditions, entity_id: EntityId) -> Self::ViewModel {
            TestViewModel::new(entity_id)
        }
    }

    #[test]
    fn test_properties_view_trait() {
        let view = TestPropertiesView;
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        let conditions = SensedConditions::new(snapshot);
        let model = view.render(&conditions, EntityId::new(1));
        assert_eq!(model.entity_id, EntityId::new(1));
    }
}
