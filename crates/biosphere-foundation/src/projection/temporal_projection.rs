//! 时间投射
//!
//! TemporalProjection 定义了如何将世界历史映射为视图模型。
//! 它与 Projection 不同，专门处理时间维度的投射。
//!
//! # 设计约束
//!
//! - 历史感知：可以访问世界历史
//! - 时间范围：支持时间范围查询
//! - 与 Projection 分离：不与普通 Projection 混淆
//!
//! # 哲学含义
//!
//! TemporalProjection 是"世界历史的投射"，而不是"当前条件的投射"。
//!
//! 这意味着：
//! - TemporalProjection 知道时间的存在
//! - TemporalProjection 可以查询历史
/// - TemporalProjection 不是 Projection
/// - TemporalProjection 是对"时间"这一存在维度的正式建模

/// 时间投射
///
/// [`TemporalProjection`] 描述如何将世界历史映射为视图模型。
///
/// # 设计约束
///
/// - 历史感知：可以访问世界历史
/// - 时间范围：支持时间范围查询
/// - 与 Projection 分离：不与普通 Projection 混淆
///
/// # 哲学含义
///
/// TemporalProjection 是"世界历史的投射"，而不是"当前条件的投射"。
///
/// 这意味着：
/// - TemporalProjection 知道时间的存在
/// - TemporalProjection 可以查询历史
/// - TemporalProjection 不是 Projection
/// - TemporalProjection 是对"时间"这一存在维度的正式建模
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::projection::TemporalProjection;
/// use biosphere_foundation::projection::TemporalQuery;
/// use biosphere_core::Conditions;
///
/// struct MyViewModel {
///     data: String,
/// }
///
/// struct MyTemporalProjection;
///
/// impl TemporalProjection for MyTemporalProjection {
///     type ViewModel = MyViewModel;
///
///     fn render(&self, history: &dyn TemporalQuery) -> Self::ViewModel {
///         MyViewModel { data: String::new() }
///     }
/// }
/// ```
pub trait TemporalProjection {
    /// 视图模型类型
    type ViewModel;

    /// 返回投射的视图模型
    ///
    /// # 参数
    ///
    /// * `history` - 时间查询接口
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
    fn render(&self, history: &dyn TemporalQuery) -> Self::ViewModel;
}

/// 时间查询接口
///
/// [`TemporalQuery`] 定义了查询世界历史的接口。
///
/// # 设计约束
///
/// - 只读访问：只提供查询接口，不提供修改接口
/// - 时间感知：支持时间范围查询
/// - 抽象存储：不暴露具体的存储实现
///
/// # 哲学含义
///
/// TemporalQuery 是"世界历史的查询接口"，而不是"世界历史的存储"。
///
/// 这意味着：
/// - TemporalQuery 只提供查询能力
/// - TemporalQuery 不暴露存储细节
/// - TemporalQuery 是对"时间查询"这一能力的抽象
pub trait TemporalQuery {
    /// 查询时间范围内的快照
    ///
    /// # 参数
    ///
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回时间范围内的快照列表
    fn query_range(&self, start: u64, end: u64) -> Vec<&ConditionSnapshot>;
}

/// 条件快照
///
/// [`ConditionSnapshot`] 表示某个时间点的条件状态。
///
/// # 设计约束
///
/// - 只读数据：只包含只读数据
/// - 时间标记：包含时间刻信息
/// - 可序列化：可以序列化和反序列化
///
/// # 哲学含义
///
/// ConditionSnapshot 是"时间点的条件"，而不是"可变的状态"。
///
/// 这意味着：
/// - ConditionSnapshot 是时间值对象
/// - ConditionSnapshot 不可修改
/// - ConditionSnapshot 代表某个时间点的"真实"
#[derive(Debug, Clone)]
pub struct ConditionSnapshot {
    /// 时间刻
    pub tick: u64,
    /// 条件信号列表
    pub signals: Vec<biosphere_core::ConditionSignal>,
}

impl ConditionSnapshot {
    /// 创建新的条件快照
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    /// * `signals` - 条件信号列表
    ///
    /// # 返回值
    ///
    /// 返回新的条件快照
    pub fn new(tick: u64, signals: Vec<biosphere_core::ConditionSignal>) -> Self {
        Self { tick, signals }
    }

    /// 获取时间刻
    ///
    /// # 返回值
    ///
    /// 返回时间刻
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// 获取条件信号列表
    ///
    /// # 返回值
    ///
    /// 返回条件信号列表
    pub fn signals(&self) -> &[biosphere_core::ConditionSignal] {
        &self.signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestViewModel {
        data: String,
    }

    struct TestTemporalProjection;

    impl TemporalProjection for TestTemporalProjection {
        type ViewModel = TestViewModel;

        fn render(&self, _history: &dyn TemporalQuery) -> Self::ViewModel {
            TestViewModel { data: String::new() }
        }
    }

    struct MockTemporalQuery {
        snapshots: Vec<ConditionSnapshot>,
    }

    impl TemporalQuery for MockTemporalQuery {
        fn query_range(&self, start: u64, end: u64) -> Vec<&ConditionSnapshot> {
            self.snapshots.iter()
                .filter(|s| s.tick >= start && s.tick <= end)
                .collect()
        }
    }

    #[test]
    fn test_temporal_projection_trait() {
        let projection = TestTemporalProjection;
        let query = MockTemporalQuery {
            snapshots: vec![
                ConditionSnapshot::new(0, Vec::new()),
                ConditionSnapshot::new(1, Vec::new()),
            ],
        };
        let model = projection.render(&query);
        assert!(model.data.is_empty());
    }

    #[test]
    fn test_condition_snapshot() {
        let signals = vec![];
        let snapshot = ConditionSnapshot::new(5, signals);
        assert_eq!(snapshot.tick(), 5);
        assert!(snapshot.signals().is_empty());
    }

    #[test]
    fn test_temporal_query() {
        let snapshots = vec![
            ConditionSnapshot::new(0, Vec::new()),
            ConditionSnapshot::new(5, Vec::new()),
            ConditionSnapshot::new(10, Vec::new()),
        ];
        let query = MockTemporalQuery { snapshots };
        
        let range_0_5 = query.query_range(0, 5);
        assert_eq!(range_0_5.len(), 2);
        
        let range_6_10 = query.query_range(6, 10);
        assert_eq!(range_6_10.len(), 1);
        
        let range_11_20 = query.query_range(11, 20);
        assert!(range_11_20.is_empty());
    }
}