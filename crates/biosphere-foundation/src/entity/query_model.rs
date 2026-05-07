use crate::entity::filter::EntityFilter;

/// 实体查询
///
/// [`EntityQuery`] 扩展 Filter，添加时间范围和查询上下文。
///
/// # 设计约束
///
/// - 时间感知：支持时间范围内的查询
/// - 上下文感知：支持不同查询上下文
/// - 可组合：支持多个查询条件的组合
/// - 类型安全：所有查询都是类型安全的
///
/// # 哲学含义
///
/// EntityQuery 是"实体查询的完整描述"，而不是"简单的过滤条件"。
///
/// 这意味着：
/// - EntityQuery 包含过滤条件和时间范围
/// - EntityQuery 支持不同的查询上下文
/// - EntityQuery 可以被优化和缓存
/// - EntityQuery 支持复杂查询场景
#[derive(Debug, Clone)]
pub struct EntityQuery {
    /// 过滤条件
    filter: EntityFilter,
    /// 时间范围
    time_range: Option<TimeRange>,
    /// 查询上下文
    context: QueryContext,
}

/// 时间范围
///
/// [`TimeRange`] 描述查询的时间范围。
///
/// # 设计约束
///
/// - 包含性：时间范围包含开始和结束时间
/// - 有效性：开始时间不大于结束时间
/// - 可选性：时间范围可以是可选的
/// - 类型安全：所有时间操作都是类型安全的
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// 开始时间
    start: u64,
    /// 结束时间
    end: u64,
}

/// 查询上下文
///
/// [`QueryContext`] 描述查询的上下文。
///
/// # 设计约束
///
/// - 语义明确：每个上下文都有明确的语义
/// - 可扩展：支持添加新的查询上下文
/// - 类型安全：所有上下文都是类型安全的
/// - 可优化：不同上下文可以有不同的优化策略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryContext {
    /// 即时查询
    ///
    /// 立即返回当前状态的查询结果
    Immediate,
    /// 持续订阅
    ///
    /// 返回初始结果，并在状态变化时推送更新
    Subscription,
    /// 历史查询
    ///
    /// 查询历史状态，不关心当前状态
    Historical,
}

impl EntityQuery {
    /// 创建即时查询
    ///
    /// # 参数
    ///
    /// * `filter` - 过滤条件
    ///
    /// # 返回值
    ///
    /// 返回即时查询
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::{EntityQuery, QueryContext};
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let query = EntityQuery::immediate(EntityFilter::All);
    /// assert_eq!(query.context(), QueryContext::Immediate);
    /// ```
    pub fn immediate(filter: EntityFilter) -> Self {
        Self {
            filter,
            time_range: None,
            context: QueryContext::Immediate,
        }
    }

    /// 创建时间范围查询
    ///
    /// # 参数
    ///
    /// * `filter` - 过滤条件
    /// * `start` - 开始时间
    /// * `end` - 结束时间
    ///
    /// # 返回值
    ///
    /// 返回时间范围查询
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::{EntityQuery, QueryContext, TimeRange};
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let query = EntityQuery::time_range(EntityFilter::All, 0, 10);
    /// assert_eq!(query.context(), QueryContext::Historical);
    /// assert_eq!(query.get_time_range(), Some(TimeRange::new(0, 10)));
    /// ```
    pub fn time_range(filter: EntityFilter, start: u64, end: u64) -> Self {
        Self {
            filter,
            time_range: Some(TimeRange::new(start, end)),
            context: QueryContext::Historical,
        }
    }

    /// 创建订阅查询
    ///
    /// # 参数
    ///
    /// * `filter` - 过滤条件
    ///
    /// # 返回值
    ///
    /// 返回订阅查询
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::{EntityQuery, QueryContext};
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let query = EntityQuery::subscription(EntityFilter::All);
    /// assert_eq!(query.context(), QueryContext::Subscription);
    /// ```
    pub fn subscription(filter: EntityFilter) -> Self {
        Self {
            filter,
            time_range: None,
            context: QueryContext::Subscription,
        }
    }

    /// 获取过滤条件
    ///
    /// # 返回值
    ///
    /// 返回过滤条件
    pub fn filter(&self) -> &EntityFilter {
        &self.filter
    }

    /// 获取时间范围
    ///
    /// # 返回值
    ///
    /// 返回时间范围
    pub fn get_time_range(&self) -> Option<TimeRange> {
        self.time_range
    }

    /// 获取查询上下文
    ///
    /// # 返回值
    ///
    /// 返回查询上下文
    pub fn context(&self) -> QueryContext {
        self.context.clone()
    }

    /// 设置时间范围
    ///
    /// # 参数
    ///
    /// * `time_range` - 时间范围
    ///
    /// # 返回值
    ///
    /// 返回修改后的查询
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::{EntityQuery, TimeRange};
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let mut query = EntityQuery::immediate(EntityFilter::All);
    /// query = query.with_time_range(TimeRange::new(0, 10));
    /// assert!(query.get_time_range().is_some());
    /// ```
    pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
        self.time_range = Some(time_range);
        self
    }

    /// 设置查询上下文
    ///
    /// # 参数
    ///
    /// * `context` - 查询上下文
    ///
    /// # 返回值
    ///
    /// 返回修改后的查询
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::{EntityQuery, QueryContext};
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let mut query = EntityQuery::immediate(EntityFilter::All);
    /// query = query.with_context(QueryContext::Historical);
    /// assert_eq!(query.context(), QueryContext::Historical);
    /// assert_eq!(query.get_time_range(), None);
    /// ```
    pub fn with_context(mut self, context: QueryContext) -> Self {
        self.context = context;
        self
    }
}

impl TimeRange {
    /// 创建新的时间范围
    ///
    /// # 参数
    ///
    /// * `start` - 开始时间
    /// * `end` - 结束时间
    ///
    /// # 返回值
    ///
    /// 返回新的时间范围
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::TimeRange;
    ///
    /// let range = TimeRange::new(0, 10);
    /// assert_eq!(range.start(), 0);
    /// assert_eq!(range.end(), 10);
    /// ```
    pub fn new(start: u64, end: u64) -> Self {
        assert!(start <= end, "Start time must be less than or equal to end time");
        Self { start, end }
    }

    /// 获取开始时间
    ///
    /// # 返回值
    ///
    /// 返回开始时间
    pub fn start(&self) -> u64 {
        self.start
    }

    /// 获取结束时间
    ///
    /// # 返回值
    ///
    /// 返回结束时间
    pub fn end(&self) -> u64 {
        self.end
    }

    /// 检查时间是否在范围内
    ///
    /// # 参数
    ///
    /// * `time` - 时间
    ///
    /// # 返回值
    ///
    /// 如果时间在范围内，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::TimeRange;
    ///
    /// let range = TimeRange::new(0, 10);
    /// assert!(range.contains(5));
    /// assert!(!range.contains(15));
    /// ```
    pub fn contains(&self, time: u64) -> bool {
        time >= self.start && time <= self.end
    }

    /// 获取时间范围的持续时间
    ///
    /// # 返回值
    ///
    /// 返回时间范围的持续时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::query_model::TimeRange;
    ///
    /// let range = TimeRange::new(0, 10);
    /// assert_eq!(range.duration(), 10);
    /// ```
    pub fn duration(&self) -> u64 {
        self.end - self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::filter::EntityFilter;

    #[test]
    fn test_entity_query_immediate() {
        let query = EntityQuery::immediate(EntityFilter::All);
        assert_eq!(query.context(), QueryContext::Immediate);
        assert_eq!(query.get_time_range(), None);
    }

    #[test]
    fn test_entity_query_time_range() {
        let query = EntityQuery::time_range(EntityFilter::All, 0, 10);
        assert_eq!(query.context(), QueryContext::Historical);
        assert_eq!(query.get_time_range(), Some(TimeRange::new(0, 10)));
    }

    #[test]
    fn test_entity_query_subscription() {
        let query = EntityQuery::subscription(EntityFilter::All);
        assert_eq!(query.context(), QueryContext::Subscription);
        assert_eq!(query.get_time_range(), None);
    }

    #[test]
    fn test_entity_query_with_time_range() {
        let query = EntityQuery::immediate(EntityFilter::All)
            .with_time_range(TimeRange::new(0, 10));
        assert_eq!(query.context(), QueryContext::Immediate);
        assert_eq!(query.get_time_range(), Some(TimeRange::new(0, 10)));
    }

    #[test]
    fn test_entity_query_with_context() {
        let query = EntityQuery::immediate(EntityFilter::All)
            .with_context(QueryContext::Historical);
        assert_eq!(query.context(), QueryContext::Historical);
        assert_eq!(query.get_time_range(), None);
    }

    #[test]
    fn test_time_range_new() {
        let range = TimeRange::new(0, 10);
        assert_eq!(range.start(), 0);
        assert_eq!(range.end(), 10);
    }

    #[test]
    #[should_panic(expected = "Start time must be less than or equal to end time")]
    fn test_time_range_invalid() {
        TimeRange::new(10, 0);
    }

    #[test]
    fn test_time_range_contains() {
        let range = TimeRange::new(0, 10);
        assert!(range.contains(0));
        assert!(range.contains(5));
        assert!(range.contains(10));
        assert!(!range.contains(11));
    }

    #[test]
    fn test_time_range_duration() {
        let range = TimeRange::new(0, 10);
        assert_eq!(range.duration(), 10);
        
        let range = TimeRange::new(5, 15);
        assert_eq!(range.duration(), 10);
    }

    #[test]
    fn test_query_context_equality() {
        assert_eq!(QueryContext::Immediate, QueryContext::Immediate);
        assert_eq!(QueryContext::Subscription, QueryContext::Subscription);
        assert_eq!(QueryContext::Historical, QueryContext::Historical);
        
        assert_ne!(QueryContext::Immediate, QueryContext::Subscription);
        assert_ne!(QueryContext::Immediate, QueryContext::Historical);
        assert_ne!(QueryContext::Subscription, QueryContext::Historical);
    }

    #[test]
    fn test_query_context_clone() {
        let context = QueryContext::Immediate;
        let cloned = context.clone();
        assert_eq!(context, cloned);
    }
}