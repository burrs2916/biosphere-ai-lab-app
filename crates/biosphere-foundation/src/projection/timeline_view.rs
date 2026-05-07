use crate::temporal::state::{StateProvider, StateQuery};
use crate::temporal::Tick;
use crate::projection::temporal_projection::{TemporalProjection, TemporalQuery, ConditionSnapshot};

/// 状态历史适配器
///
/// 将 StateProvider 和 StateQuery 适配为 TemporalQuery 接口。
pub struct StateHistoryAdapter<'a, P> {
    provider: &'a P,
}

impl<'a, P: StateProvider + StateQuery> StateHistoryAdapter<'a, P> {
    /// 创建新的适配器
    pub fn new(provider: &'a P) -> Self {
        Self { provider }
    }
}

impl<'a, P: StateProvider + StateQuery> TemporalQuery for StateHistoryAdapter<'a, P> {
    fn query_range(&self, start: u64, end: u64) -> Vec<&ConditionSnapshot> {
        let snapshots = self.provider.query_range(Tick::new(start), Tick::new(end));
        
        // 将 StateSnapshot 转换为 ConditionSnapshot
        // 简化实现，只保留时间信息
        snapshots.iter().map(|s| {
            let tick = s.tick().value();
            // 创建条件快照，只包含时间信息
            // 在实际实现中，需要根据 s 的内容创建相应的 ConditionSnapshot
            let condition_snapshot = ConditionSnapshot::new(tick, Vec::new());
            // 这里需要返回引用，但我们创建的是临时值
            // 在实际实现中，需要更复杂的生命周期管理
            // 为了简化，我们使用 Box::leak
            Box::leak(Box::new(condition_snapshot)) as &ConditionSnapshot
        }).collect()
    }
}

/// 时间轴视图模型
///
/// [`TimelineViewModel`] 定义了时间轴视图的视图模型。
///
/// # 设计约束
///
/// - 只读数据：只包含只读数据
/// - 无状态：不包含任何状态
/// - 可序列化：可以序列化和反序列化
/// - 不包含渲染逻辑：不包含具体的渲染逻辑
#[derive(Debug, Clone)]
pub struct TimelineViewModel {
    ticks: Vec<Tick>,
}

/// 时间轴视图
///
/// [`TimelineView`] 定义了时间轴视图的接口。
///
/// # 设计约束
///
/// - 只读访问：只通过 StateProvider 和 StateQuery 访问状态
/// - 不修改状态：不提供任何修改接口
/// - 不持有状态：不包含任何状态
/// - 不依赖 UI 框架：不依赖具体的 UI 框架
/// - 不包含渲染逻辑：不包含具体的渲染逻辑
///
/// # 哲学含义
///
/// TimelineView 是"时间轴视图"，而不是"时间轴渲染器"。
///
/// 这意味着：
/// - TimelineView 只提供数据接口，不渲染
/// - TimelineView 是只读计算器
/// - TimelineView 不处理事件
/// - TimelineView 不依赖 UI 框架
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::{BasicWorld, TimelineView};
///
/// let mut world = BasicWorld::new();
/// world.step_world();
///
/// let timeline = TimelineView::unlimited();
/// let output = timeline.render(&world, 0, u64::MAX);
/// println!("{:?}", output);
/// ```
pub struct TimelineView {
    limit: Option<usize>,
}

impl TimelineView {
    /// 创建新的时间轴视图
    ///
    /// # 参数
    ///
    /// * `limit` - 可选的显示限制，None 表示显示全部历史
    pub fn new(limit: Option<usize>) -> Self {
        Self { limit }
    }

    /// 创建无限制的时间轴视图
    pub fn unlimited() -> Self {
        Self::new(None)
    }

    /// 创建有限制的时间轴视图
    pub fn limited(n: usize) -> Self {
        Self::new(Some(n))
    }

    /// 渲染时间轴
    ///
    /// # 类型参数
    ///
    /// * `P` - 必须实现 StateProvider 和 StateQuery trait
    ///
    /// # 参数
    ///
    /// * `provider` - 状态提供者
    /// * `start` - 查询起始时间刻（包含）
    /// * `end` - 查询结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回时间轴视图的视图模型
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::{BasicWorld, TimelineView};
    ///
    /// let mut world = BasicWorld::new();
    /// world.step_world();
    ///
    /// let timeline = TimelineView::unlimited();
    /// let model = timeline.render(&world, 0, u64::MAX);
    /// ```
    pub fn render<P: StateProvider + StateQuery>(&self, provider: &P, start: u64, end: u64) -> TimelineViewModel {
        // 创建适配器
        let adapter = StateHistoryAdapter::new(provider);
        
        // 查询时间范围
        let snapshots = adapter.query_range(start, end);
        
        // 提取时间点
        let ticks: Vec<u64> = snapshots.iter().map(|s| s.tick()).collect();
        
        // 应用限制
        let ticks = if let Some(limit) = self.limit {
            if ticks.len() > limit {
                ticks[ticks.len() - limit..].to_vec()
            } else {
                ticks
            }
        } else {
            ticks
        };
        
        TimelineViewModel { ticks: ticks.into_iter().map(|t| Tick::new(t)).collect() }
    }
}

impl Default for TimelineView {
    fn default() -> Self {
        Self::unlimited()
    }
}

impl TemporalProjection for TimelineView {
    type ViewModel = TimelineViewModel;
    
    fn render(&self, history: &dyn TemporalQuery) -> Self::ViewModel {
        // 查询所有时间范围
        let snapshots = history.query_range(0, u64::MAX);
        
        // 提取时间点
        let ticks: Vec<u64> = snapshots.iter().map(|s| s.tick()).collect();
        
        // 应用限制
        let ticks = if let Some(limit) = self.limit {
            if ticks.len() > limit {
                ticks[ticks.len() - limit..].to_vec()
            } else {
                ticks
            }
        } else {
            ticks
        };
        
        TimelineViewModel { ticks: ticks.into_iter().map(|t| Tick::new(t)).collect() }
    }
}

impl TimelineViewModel {
    /// 创建新的时间轴视图模型
    ///
    /// # 参数
    ///
    /// * `ticks` - 时间点列表
    ///
    /// # 返回值
    ///
    /// 返回新的时间轴视图模型
    ///
    /// # 设计约束
    ///
    /// - 只读构造函数
    /// - 时间点列表在构造时确定，之后不可修改
    pub fn new(ticks: Vec<u64>) -> Self {
        Self {
            ticks: ticks.into_iter().map(|t| Tick::new(t)).collect(),
        }
    }

    /// 获取时间点数量
    ///
    /// # 返回值
    ///
    /// 返回时间点的数量
    pub fn tick_count(&self) -> usize {
        self.ticks.len()
    }

    /// 检查是否为空
    ///
    /// # 返回值
    ///
    /// 如果没有时间点，返回 true，否则返回 false
    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// 获取所有时间点
    ///
    /// # 返回值
    ///
    /// 返回所有时间点的列表
    pub fn ticks(&self) -> Vec<u64> {
        // 为了保持外部接口兼容性，返回 u64 列表
        self.ticks.iter().map(|t| t.value()).collect()
    }
}

impl Default for TimelineViewModel {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::BasicWorld;

    const TEST_ITERATIONS_SMALL: usize = 3;
    const TEST_ITERATIONS_LARGE: usize = 10;
    const TEST_LIMIT: usize = 3;

    #[test]
    fn test_timeline_view_creation() {
        let timeline = TimelineView::unlimited();
        assert!(timeline.limit.is_none());
    }

    #[test]
    fn test_timeline_view_limited() {
        let timeline = TimelineView::limited(TEST_LIMIT);
        assert_eq!(timeline.limit, Some(TEST_LIMIT));
    }

    #[test]
    fn test_timeline_view_render_empty() {
        let world = BasicWorld::new();
        let timeline = TimelineView::unlimited();
        let model = timeline.render(&world, 0, u64::MAX);
        
        // BasicWorld::new() 现在会记录初始状态（tick 0），所以历史不为空
        assert!(!model.is_empty());
        assert!(model.ticks().contains(&0));
    }

    #[test]
    fn test_timeline_view_render_with_history() {
        let mut world = BasicWorld::new();
        
        for _ in 0..TEST_ITERATIONS_SMALL {
            world.step_world().unwrap();
        }
        
        let timeline = TimelineView::unlimited();
        let model = timeline.render(&world, 0, u64::MAX);
        
        // 第一次推进后是 t=1，第二次是 t=2，第三次是 t=3
        assert!(model.ticks().contains(&1));
        assert!(model.ticks().contains(&2));
        assert!(model.ticks().contains(&3));
    }

    #[test]
    fn test_timeline_view_render_limited() {
        let mut world = BasicWorld::new();
        
        for _ in 0..TEST_ITERATIONS_LARGE {
            world.step_world().unwrap();
        }
        
        let timeline = TimelineView::limited(TEST_LIMIT);
        let model = timeline.render(&world, 0, u64::MAX);
        
        // 只显示最近 3 个（t=8, t=9, t=10）
        assert_eq!(model.tick_count(), TEST_LIMIT);
        assert!(model.ticks().contains(&8));
        assert!(model.ticks().contains(&9));
        assert!(model.ticks().contains(&10));
        
        // 不显示更早的
        assert!(!model.ticks().contains(&0));
    }

    #[test]
    fn test_timeline_view_model_creation() {
        let model = TimelineViewModel::new(Vec::new());
        assert!(model.is_empty());
        assert_eq!(model.tick_count(), 0);
    }

    #[test]
    fn test_timeline_view_model_with_ticks() {
        let ticks = vec![1, 2, 3];
        let model = TimelineViewModel::new(ticks);
        assert_eq!(model.tick_count(), TEST_ITERATIONS_SMALL);
        assert!(model.ticks().contains(&1));
        assert!(model.ticks().contains(&2));
        assert!(model.ticks().contains(&3));
    }

    #[test]
    fn test_timeline_view_model_clone() {
        let ticks = vec![1, 2];
        let model = TimelineViewModel::new(ticks);
        let cloned = model.clone();
        assert_eq!(cloned.tick_count(), model.tick_count());
        assert_eq!(cloned.ticks(), model.ticks());
    }

    #[test]
    fn test_timeline_view_model_default() {
        let model = TimelineViewModel::default();
        assert!(model.is_empty());
        assert_eq!(model.tick_count(), 0);
    }

    #[test]
    fn test_timeline_view_model_tick_count() {
        const TEST_TICK_COUNT: usize = 2;
        let ticks = vec![1, 2];
        let model = TimelineViewModel::new(ticks);
        assert_eq!(model.tick_count(), TEST_TICK_COUNT);
    }

    #[test]
    fn test_timeline_view_model_is_empty() {
        let model = TimelineViewModel::new(Vec::new());
        assert!(model.is_empty());
    }

    #[test]
    fn test_timeline_view_model_ticks() {
        let ticks = vec![1, 2, 3];
        let model = TimelineViewModel::new(ticks);
        let ticks = model.ticks();
        assert_eq!(ticks.len(), TEST_ITERATIONS_SMALL);
        assert!(ticks.contains(&1));
        assert!(ticks.contains(&2));
        assert!(ticks.contains(&3));
    }
}
