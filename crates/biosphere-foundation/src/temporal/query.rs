use crate::temporal::state::{StateSnapshot, StateQuery};
use crate::temporal::relations::{RelationChange, RelationQuery};
use crate::temporal::Tick;

/// 惰性查询迭代器
///
/// [`LazyQueryIterator`] 提供了对状态历史的惰性查询能力。
///
/// # 设计约束
///
/// - 惰性求值：只在需要时才计算结果
/// - 零分配：不预先分配内存
/// - 链式操作：支持链式操作
///
/// # 哲学含义
///
/// LazyQueryIterator 是"惰性查询器"，而不是"预计算查询器"。
///
/// 这意味着：
/// - 查询不会立即执行
/// - 只在迭代时才计算结果
/// - 支持链式操作和组合
///
/// # 性能优势
///
/// - 避免不必要的内存分配
/// - 支持大范围查询
/// - 可以提前终止迭代
///
/// # 示例
///
/// ```text
/// let world = BasicWorld::new();
/// let lazy_query = LazyQueryIterator::new(&world, 0, 100);
///
/// // 只在迭代时才计算
/// for snapshot in lazy_query {
///     println!("tick: {}", snapshot.tick());
/// }
/// ```
pub struct LazyQueryIterator<'a, P: StateQuery> {
    query: &'a P,
    start: u64,
    end: u64,
}

impl<'a, P: StateQuery> LazyQueryIterator<'a, P> {
    /// 创建新的惰性查询迭代器
    ///
    /// # 参数
    ///
    /// * `query` - 状态查询
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回新的惰性查询迭代器
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let lazy_query = LazyQueryIterator::new(&world, 0, 100);
    /// ```
    pub fn new(query: &'a P, start: u64, end: u64) -> Self {
        Self {
            query,
            start,
            end,
        }
    }

    /// 过滤时间范围
    ///
    /// # 参数
    ///
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回新的惰性查询迭代器
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let lazy_query = LazyQueryIterator::new(&world, 0, 100)
    ///     .filter_range(10, 50);
    /// ```
    pub fn filter_range(self, start: u64, end: u64) -> Self {
        Self {
            start: self.start.max(start),
            end: self.end.min(end),
            ..self
        }
    }
}

impl<'a, P: StateQuery> Iterator for LazyQueryIterator<'a, P> {
    type Item = &'a StateSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let snapshot = self.query.get_at(Tick::new(self.start))?;
        self.start += 1;
        Some(snapshot)
    }
}

/// 惰性关系查询迭代器
///
/// [`LazyRelationQueryIterator`] 提供了对关系历史的惰性查询能力。
///
/// # 设计约束
///
/// - 惰性求值：只在需要时才计算结果
/// - 零分配：不预先分配内存
/// - 链式操作：支持链式操作
///
/// # 哲学含义
///
/// LazyRelationQueryIterator 是"惰性关系查询器"，而不是"预计算查询器"。
///
/// 这意味着：
/// - 查询不会立即执行
/// - 只在迭代时才计算结果
/// - 支持链式操作和组合
///
/// # 性能优势
///
/// - 避免不必要的内存分配
/// - 支持大范围查询
/// - 可以提前终止迭代
///
/// # 示例
///
/// ```text
/// let world = BasicWorld::new();
/// let lazy_query = LazyRelationQueryIterator::new(&world, 0, 100);
///
/// // 只在迭代时才计算
/// for change in lazy_query {
///     println!("tick: {}", change.tick());
/// }
/// ```
pub struct LazyRelationQueryIterator<'a, P: RelationQuery> {
    query: &'a P,
    start: u64,
    end: u64,
}

impl<'a, P: RelationQuery> LazyRelationQueryIterator<'a, P> {
    /// 创建新的惰性关系查询迭代器
    ///
    /// # 参数
    ///
    /// * `query` - 关系查询
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回新的惰性关系查询迭代器
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let lazy_query = LazyRelationQueryIterator::new(&world, 0, 100);
    /// ```
    pub fn new(query: &'a P, start: u64, end: u64) -> Self {
        Self {
            query,
            start,
            end,
        }
    }

    /// 过滤时间范围
    ///
    /// # 参数
    ///
    /// * `start` - 起始时间刻（包含）
    /// * `end` - 结束时间刻（包含）
    ///
    /// # 返回值
    ///
    /// 返回新的惰性关系查询迭代器
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let lazy_query = LazyRelationQueryIterator::new(&world, 0, 100)
    ///     .filter_range(10, 50);
    /// ```
    pub fn filter_range(self, start: u64, end: u64) -> Self {
        Self {
            start: self.start.max(start),
            end: self.end.min(end),
            ..self
        }
    }
}

impl<'a, P: RelationQuery> Iterator for LazyRelationQueryIterator<'a, P> {
    type Item = &'a RelationChange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let change = self.query.get_relation_at(Tick::new(self.start))?;
        self.start += 1;
        Some(change)
    }
}

/// 窗口化查询
///
/// [`WindowedQuery`] 提供了对状态历史的窗口化查询能力。
///
/// # 设计约束
///
/// - 窗口化：只查询指定窗口内的数据
/// - 滚动支持：支持窗口滚动
/// - 零分配：不预先分配内存
///
/// # 哲学含义
///
/// WindowedQuery 是"窗口化查询器"，而不是"全量查询器"。
///
/// 这意味着：
/// - 只查询窗口内的数据
/// - 支持窗口滚动
/// - 适合大范围数据的分页显示
///
/// # 性能优势
///
/// - 减少内存使用
/// - 支持大范围数据
/// - 适合 Timeline scrubber 等场景
///
/// # 示例
///
/// ```text
/// let world = BasicWorld::new();
/// let windowed_query = WindowedQuery::new(&world, 0, 10);
///
/// // 只查询窗口内的数据
/// for snapshot in windowed_query {
///     println!("tick: {}", snapshot.tick());
/// }
/// ```
pub struct WindowedQuery<'a, P: StateQuery> {
    query: &'a P,
    window_start: u64,
    window_size: u64,
}

impl<'a, P: StateQuery> WindowedQuery<'a, P> {
    /// 创建新的窗口化查询
    ///
    /// # 参数
    ///
    /// * `query` - 状态查询
    /// * `window_start` - 窗口起始时间刻
    /// * `window_size` - 窗口大小
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedQuery::new(&world, 0, 10);
    /// ```
    pub fn new(query: &'a P, window_start: u64, window_size: u64) -> Self {
        Self {
            query,
            window_start,
            window_size,
        }
    }

    /// 滚动窗口
    ///
    /// # 参数
    ///
    /// * `offset` - 滚动偏移量（正数向后滚动，负数向前滚动）
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedQuery::new(&world, 0, 10)
    ///     .scroll(5);
    /// ```
    pub fn scroll(self, offset: i64) -> Self {
        let new_start = if offset >= 0 {
            self.window_start + offset as u64
        } else {
            self.window_start.saturating_sub((-offset) as u64)
        };

        Self {
            window_start: new_start,
            ..self
        }
    }

    /// 调整窗口大小
    ///
    /// # 参数
    ///
    /// * `new_size` - 新的窗口大小
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedQuery::new(&world, 0, 10)
    ///     .resize(20);
    /// ```
    pub fn resize(self, new_size: u64) -> Self {
        Self {
            window_size: new_size,
            ..self
        }
    }
}

impl<'a, P: StateQuery> Iterator for WindowedQuery<'a, P> {
    type Item = &'a StateSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.window_size == 0 {
            return None;
        }

        let snapshot = self.query.get_at(Tick::new(self.window_start))?;
        self.window_start += 1;
        self.window_size -= 1;
        Some(snapshot)
    }
}

/// 窗口化关系查询
///
/// [`WindowedRelationQuery`] 提供了对关系历史的窗口化查询能力。
///
/// # 设计约束
///
/// - 窗口化：只查询指定窗口内的数据
/// - 滚动支持：支持窗口滚动
/// - 零分配：不预先分配内存
///
/// # 哲学含义
///
/// WindowedRelationQuery 是"窗口化关系查询器"，而不是"全量查询器"。
///
/// 这意味着：
/// - 只查询窗口内的数据
/// - 支持窗口滚动
/// - 适合大范围数据的分页显示
///
/// # 性能优势
///
/// - 减少内存使用
/// - 支持大范围数据
/// - 适合 Timeline scrubber 等场景
///
/// # 示例
///
/// ```text
/// let world = BasicWorld::new();
/// let windowed_query = WindowedRelationQuery::new(&world, 0, 10);
///
/// // 只查询窗口内的数据
/// for change in windowed_query {
///     println!("tick: {}", change.tick());
/// }
/// ```
pub struct WindowedRelationQuery<'a, P: RelationQuery> {
    query: &'a P,
    window_start: u64,
    window_size: u64,
}

impl<'a, P: RelationQuery> WindowedRelationQuery<'a, P> {
    /// 创建新的窗口化关系查询
    ///
    /// # 参数
    ///
    /// * `query` - 关系查询
    /// * `window_start` - 窗口起始时间刻
    /// * `window_size` - 窗口大小
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化关系查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedRelationQuery::new(&world, 0, 10);
    /// ```
    pub fn new(query: &'a P, window_start: u64, window_size: u64) -> Self {
        Self {
            query,
            window_start,
            window_size,
        }
    }

    /// 滚动窗口
    ///
    /// # 参数
    ///
    /// * `offset` - 滚动偏移量（正数向后滚动，负数向前滚动）
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化关系查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedRelationQuery::new(&world, 0, 10)
    ///     .scroll(5);
    /// ```
    pub fn scroll(self, offset: i64) -> Self {
        let new_start = if offset >= 0 {
            self.window_start + offset as u64
        } else {
            self.window_start.saturating_sub((-offset) as u64)
        };

        Self {
            window_start: new_start,
            ..self
        }
    }

    /// 调整窗口大小
    ///
    /// # 参数
    ///
    /// * `new_size` - 新的窗口大小
    ///
    /// # 返回值
    ///
    /// 返回新的窗口化关系查询
    ///
    /// # 示例
    ///
    /// ```text
    /// let world = BasicWorld::new();
    /// let windowed_query = WindowedRelationQuery::new(&world, 0, 10)
    ///     .resize(20);
    /// ```
    pub fn resize(self, new_size: u64) -> Self {
        Self {
            window_size: new_size,
            ..self
        }
    }
}

impl<'a, P: RelationQuery> Iterator for WindowedRelationQuery<'a, P> {
    type Item = &'a RelationChange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.window_size == 0 {
            return None;
        }

        let change = self.query.get_relation_at(Tick::new(self.window_start))?;
        self.window_start += 1;
        self.window_size -= 1;
        Some(change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::BasicWorld;

    #[test]
    fn test_lazy_query_iterator() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let lazy_query = LazyQueryIterator::new(&world, 0, 5);
        let count = lazy_query.count();
        assert_eq!(count, 6);
    }

    #[test]
    fn test_lazy_query_iterator_filter_range() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let lazy_query = LazyQueryIterator::new(&world, 0, 10)
            .filter_range(3, 7);
        let count = lazy_query.count();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_lazy_relation_query_iterator() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let lazy_query = LazyRelationQueryIterator::new(&world, 0, 5);
        let count = lazy_query.count();
        // BasicWorld 的 RelationStore 中没有关系变化，所以结果为空
        assert_eq!(count, 0);
    }

    #[test]
    fn test_lazy_relation_query_iterator_filter_range() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            let _ = world.step_world().unwrap();
        }

        let lazy_query = LazyRelationQueryIterator::new(&world, 0, 10)
            .filter_range(3, 7);
        let count = lazy_query.count();
        // BasicWorld 的 RelationStore 中没有关系变化，所以结果为空
        assert_eq!(count, 0);
    }

    #[test]
    fn test_windowed_query() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let windowed_query = WindowedQuery::new(&world, 0, 5);
        let count = windowed_query.count();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_windowed_query_scroll() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let windowed_query = WindowedQuery::new(&world, 0, 5)
            .scroll(3);
        let count = windowed_query.count();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_windowed_query_resize() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let windowed_query = WindowedQuery::new(&world, 0, 5)
            .resize(3);
        let count = windowed_query.count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_windowed_relation_query() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let windowed_query = WindowedRelationQuery::new(&world, 0, 5);
        let count = windowed_query.count();
        // BasicWorld 的 RelationStore 中没有关系变化，所以结果为空
        assert_eq!(count, 0);
    }

    #[test]
    fn test_windowed_relation_query_scroll() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }

        let windowed_query = WindowedRelationQuery::new(&world, 0, 5)
            .scroll(3);
        let count = windowed_query.count();
        // BasicWorld 的 RelationStore 中没有关系变化，所以结果为空
        assert_eq!(count, 0);
    }

    #[test]
    fn test_windowed_relation_query_resize() {
        let mut world = BasicWorld::new();

        for _ in 0..10 {
            world.step_world().unwrap();
        }
        
        let windowed_query = WindowedRelationQuery::new(&world, 0, 5)
            .resize(3);
        let count = windowed_query.count();
        // BasicWorld 的 RelationStore 中没有关系变化，所以结果为空
        assert_eq!(count, 0);
    }
}
