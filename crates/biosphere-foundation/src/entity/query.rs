use biosphere_core::EntityId;
use crate::entity::entity::Entity;
use crate::entity::filter::EntityFilter;

/// 实体查询接口
///
/// [`EntityQuery`] 定义了对实体的只读查询操作。
///
/// # 设计约束
///
/// - 只读查询：不提供任何修改接口
/// - 过滤支持：支持复杂的过滤条件
/// - 不可变返回：返回的都是不可变引用
/// - 无副作用：查询操作不会改变状态
///
/// # 哲学含义
///
/// EntityQuery 是"实体的只读观察者"，而不是"实体修改者"。
///
/// 这意味着：
/// - 查询不会改变实体
/// - 查询不会影响世界演化
/// - 查询是安全的、可重复的
/// - 查询是"观察"而非"干预"
pub trait EntityQuery {
    /// 获取实体
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    ///
    /// # 返回值
    ///
    /// 如果实体存在，返回实体的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::EntityQuery;
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// let id = manager.create(EntityKind::Default);
    ///
    /// let entity = manager.get(id);
    /// assert!(entity.is_some());
    /// ```
    fn get(&self, id: EntityId) -> Option<&Entity>;

    /// 查询实体
    ///
    /// # 参数
    ///
    /// * `filter` - 实体过滤器
    ///
    /// # 返回值
    ///
    /// 返回匹配过滤器的实体 ID 列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::EntityQuery;
    /// use biosphere_foundation::entity::entity::EntityKind;
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let mut manager = EntityManager::new();
    /// let id1 = manager.create(EntityKind::Default);
    /// let id2 = manager.create(EntityKind::Default);
    ///
    /// let results = manager.query(EntityFilter::kind(EntityKind::Default));
    /// assert_eq!(results.len(), 2);
    /// ```
    fn query(&self, filter: EntityFilter) -> Vec<EntityId>;

    /// 获取所有实体 ID
    ///
    /// # 返回值
    ///
    /// 返回所有实体的 ID 列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::EntityQuery;
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.create(EntityKind::Default);
    /// manager.create(EntityKind::Default);
    ///
    /// let all_ids = manager.all_ids();
    /// assert_eq!(all_ids.len(), 2);
    /// ```
    fn all_ids(&self) -> Vec<EntityId>;

    /// 获取实体数量
    ///
    /// # 返回值
    ///
    /// 返回实体的数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::EntityQuery;
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// assert_eq!(manager.len(), 0);
    ///
    /// manager.create(EntityKind::Default);
    /// assert_eq!(manager.len(), 1);
    /// ```
    fn len(&self) -> usize;

    /// 检查是否为空
    ///
    /// # 返回值
    ///
    /// 如果没有实体，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::EntityQuery;
    ///
    /// let manager = EntityManager::new();
    /// assert!(manager.is_empty());
    /// ```
    fn is_empty(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::manager::EntityManager;
    use crate::entity::entity::EntityKind;

    #[test]
    fn test_entity_query_get() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let entity = manager.get(id);
        assert!(entity.is_some());
        assert_eq!(entity.unwrap().id(), id);
    }

    #[test]
    fn test_entity_query_get_not_found() {
        let manager = EntityManager::new();
        let id = EntityId::new(999);

        let entity = manager.get(id);
        assert!(entity.is_none());
    }

    #[test]
    fn test_entity_query_query_all() {
        let mut manager = EntityManager::new();
        manager.create(EntityKind::Default);
        manager.create(EntityKind::Default);
        manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::All);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_entity_query_query_kind() {
        let mut manager = EntityManager::new();
        let id1 = manager.create(EntityKind::Default);
        let id2 = manager.create(EntityKind::Default);
        let id3 = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::kind(EntityKind::Default));
        assert_eq!(results.len(), 3);
        assert!(results.contains(&id1));
        assert!(results.contains(&id3));
        assert!(results.contains(&id2));
    }

    #[test]
    fn test_entity_query_all_ids() {
        let mut manager = EntityManager::new();
        let id1 = manager.create(EntityKind::Default);
        let id2 = manager.create(EntityKind::Default);
        let id3 = manager.create(EntityKind::Default);

        let all_ids = manager.all_ids();
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&id1));
        assert!(all_ids.contains(&id2));
        assert!(all_ids.contains(&id3));
    }

    #[test]
    fn test_entity_query_len() {
        let mut manager = EntityManager::new();
        assert_eq!(manager.len(), 0);

        manager.create(EntityKind::Default);
        assert_eq!(manager.len(), 1);

        manager.create(EntityKind::Default);
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_entity_query_is_empty() {
        let mut manager = EntityManager::new();
        assert!(manager.is_empty());

        manager.create(EntityKind::Default);
        assert!(!manager.is_empty());
    }

    #[test]
    fn test_entity_query_read_only() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let entity = manager.get(id);
        assert!(entity.is_some());

        let results = manager.query(EntityFilter::All);
        assert_eq!(results.len(), 1);

        let all_ids = manager.all_ids();
        assert_eq!(all_ids.len(), 1);

        assert_eq!(manager.len(), 1);
        assert!(!manager.is_empty());
    }
}
