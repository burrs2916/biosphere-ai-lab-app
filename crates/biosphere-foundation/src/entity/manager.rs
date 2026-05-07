use biosphere_core::EntityId;
use crate::entity::entity::Entity;
use crate::entity::filter::EntityFilter;
use crate::entity::entity::EntityKind;
use crate::entity::identity::EntityIdentitySpace;
use crate::entity::state::WorldEntityState;

/// 实体管理器
///
/// [`EntityManager`] 管理世界中的所有实体。
///
/// # 设计约束
///
/// - 实体创建：只能创建新实体，不能修改已有实体
/// - 实体删除：可以删除实体
/// - 实体查询：支持多种查询方式
/// - 实体过滤：支持复杂的过滤条件
/// - 时间感知：支持时间范围内的查询
///
/// # 哲学含义
///
/// EntityManager 是"实体的容器"，而不是"实体的修改器"。
///
/// 这意味着：
/// - 实体一旦创建就不可修改
/// - 实体可以被删除
/// - 实体可以被查询
/// - 实体可以被过滤
/// - 实体存在有时间维度
#[derive(Debug, Default)]
pub struct EntityManager {
    /// 实体身份空间
    identity_space: EntityIdentitySpace,
    /// 世界实体状态
    world_state: WorldEntityState,
    /// 当前时间刻
    current_tick: u64,
}

impl EntityManager {
    /// 创建新的实体管理器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    ///
    /// let manager = EntityManager::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新实体
    ///
    /// # 参数
    ///
    /// * `kind` - 实体种类
    ///
    /// # 返回值
    ///
    /// 返回新创建的实体 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// let id = manager.create(EntityKind::Default);
    /// ```
    pub fn create(&mut self, kind: EntityKind) -> EntityId {
        let id = self.identity_space.generate_id();
        let entity = Entity::new(id, kind);
        self.world_state.create_entity(entity, self.current_tick)
    }

    /// 删除实体
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    ///
    /// # 返回值
    ///
    /// 如果实体存在，返回 Ok(())，否则返回 Err
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// let id = manager.create(EntityKind::Default);
    ///
    /// let result = manager.delete(id);
    /// assert!(result.is_ok());
    /// ```
    pub fn delete(&mut self, id: EntityId) -> Result<(), String> {
        self.world_state.delete_entity(id, self.current_tick)
    }

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
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// let id = manager.create(EntityKind::Default);
    ///
    /// let entity = manager.get(id);
    /// assert!(entity.is_some());
    /// ```
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.world_state.get_entity(id)
    }

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
    pub fn query(&self, filter: EntityFilter) -> Vec<EntityId> {
        let alive_ids = self.world_state.alive_entities_at(self.current_tick);
        self.filter_entities(alive_ids, filter)
    }

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
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// manager.create(EntityKind::Default);
    /// manager.create(EntityKind::Default);
    ///
    /// let all_ids = manager.all_ids();
    /// assert_eq!(all_ids.len(), 2);
    /// ```
    pub fn all_ids(&self) -> Vec<EntityId> {
        self.world_state.all_entity_ids()
    }

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
    /// use biosphere_foundation::entity::entity::EntityKind;
    ///
    /// let mut manager = EntityManager::new();
    /// assert_eq!(manager.len(), 0);
    ///
    /// manager.create(EntityKind::Default);
    /// assert_eq!(manager.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.world_state.len()
    }

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
    ///
    /// let manager = EntityManager::new();
    /// assert!(manager.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.world_state.is_empty()
    }

    /// 推进时间
    ///
    /// # 返回值
    ///
    /// 返回新的当前时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    ///
    /// let mut manager = EntityManager::new();
    /// let old_tick = manager.current_tick();
    /// let new_tick = manager.advance_time();
    /// assert_eq!(new_tick, old_tick + 1);
    /// ```
    pub fn advance_time(&mut self) -> u64 {
        self.current_tick += 1;
        self.current_tick
    }

    /// 获取当前时间
    ///
    /// # 返回值
    ///
    /// 返回当前时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    ///
    /// let manager = EntityManager::new();
    /// assert_eq!(manager.current_tick(), 0);
    /// ```
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// 查询指定时间存活的实体
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    /// * `filter` - 实体过滤器
    ///
    /// # 返回值
    ///
    /// 返回在指定时间存活的实体 ID 列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::EntityManager;
    /// use biosphere_foundation::entity::entity::EntityKind;
    /// use biosphere_foundation::EntityFilter;
    ///
    /// let mut manager = EntityManager::new();
    /// let id = manager.create(EntityKind::Default);
    ///
    /// let results = manager.query_at(0, EntityFilter::kind(EntityKind::Default));
    /// assert_eq!(results.len(), 1);
    /// ```
    pub fn query_at(&self, tick: u64, filter: EntityFilter) -> Vec<EntityId> {
        let alive_ids = self.world_state.alive_entities_at(tick);
        self.filter_entities(alive_ids, filter)
    }

    /// 根据过滤器过滤实体
    ///
    /// # 参数
    ///
    /// * `ids` - 实体 ID 列表
    /// * `filter` - 实体过滤器
    ///
    /// # 返回值
    ///
    /// 返回匹配过滤器的实体 ID 列表
    fn filter_entities(&self, ids: Vec<EntityId>, filter: EntityFilter) -> Vec<EntityId> {
        ids.into_iter()
            .filter(|&id| {
                if let Some(entity) = self.world_state.get_entity(id) {
                    self.matches_filter(entity, &filter)
                } else {
                    false
                }
            })
            .collect()
    }

    /// 检查实体是否匹配过滤器
    ///
    /// # 参数
    ///
    /// * `entity` - 实体
    /// * `filter` - 实体过滤器
    ///
    /// # 返回值
    ///
    /// 如果实体匹配过滤器，返回 true，否则返回 false
    fn matches_filter(&self, entity: &Entity, filter: &EntityFilter) -> bool {
        match filter {
            EntityFilter::All => true,
            EntityFilter::Id(id) => entity.id() == *id,
            EntityFilter::Kind(kind) => entity.kind() == *kind,
            EntityFilter::Ids(ids) => ids.contains(&entity.id()),
            EntityFilter::Kinds(kinds) => kinds.contains(&entity.kind()),
            EntityFilter::And(filters) => {
                // 优化：如果过滤器列表为空，返回 true
                if filters.is_empty() {
                    true
                } else {
                    filters.iter().all(|f| self.matches_filter(entity, f))
                }
            },
            EntityFilter::Or(filters) => {
                // 优化：如果过滤器列表为空，返回 false
                if filters.is_empty() {
                    false
                } else {
                    filters.iter().any(|f| self.matches_filter(entity, f))
                }
            },
            EntityFilter::Not(inner) => !self.matches_filter(entity, inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_manager_creation() {
        let manager = EntityManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_entity_manager_create() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        assert_eq!(manager.len(), 1);
        assert!(manager.get(id).is_some());
    }

    #[test]
    fn test_entity_manager_delete() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let result = manager.delete(id);
        assert!(result.is_ok());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_entity_manager_delete_not_found() {
        let mut manager = EntityManager::new();
        let id = EntityId::new(999);

        let result = manager.delete(id);
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_manager_get() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let entity = manager.get(id);
        assert!(entity.is_some());
        assert_eq!(entity.unwrap().id(), id);
    }

    #[test]
    fn test_entity_manager_get_not_found() {
        let manager = EntityManager::new();
        let id = EntityId::new(999);

        let entity = manager.get(id);
        assert!(entity.is_none());
    }

    #[test]
    fn test_entity_manager_query_all() {
        let mut manager = EntityManager::new();
        manager.create(EntityKind::Default);
        manager.create(EntityKind::Default);
        manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::All);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_entity_manager_query_id() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::id(id));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id);
    }

    #[test]
    fn test_entity_manager_query_kind() {
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
    fn test_entity_manager_query_ids() {
        let mut manager = EntityManager::new();
        let id1 = manager.create(EntityKind::Default);
        let id2 = manager.create(EntityKind::Default);
        let id3 = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::ids(vec![id1, id3]));
        assert_eq!(results.len(), 2);
        assert!(results.contains(&id1));
        assert!(results.contains(&id3));
        assert!(!results.contains(&id2));
    }

    #[test]
    fn test_entity_manager_query_kinds() {
        let mut manager = EntityManager::new();
        let id1 = manager.create(EntityKind::Default);
        let id2 = manager.create(EntityKind::Default);
        let id3 = manager.create(EntityKind::Default);
        let id4 = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::kinds(vec![EntityKind::Default]));
        assert_eq!(results.len(), 4);
        assert!(results.contains(&id1));
        assert!(results.contains(&id3));
        assert!(results.contains(&id4));
        assert!(results.contains(&id2));
    }

    #[test]
    fn test_entity_manager_query_and() {
        let mut manager = EntityManager::new();
        let id = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::and(vec![
            EntityFilter::id(id),
            EntityFilter::kind(EntityKind::Default),
        ]));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id);
    }

    #[test]
    fn test_entity_manager_query_or() {
        let mut manager = EntityManager::new();
        let id1 = manager.create(EntityKind::Default);
        let id2 = manager.create(EntityKind::Default);
        let id3 = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::or(vec![
            EntityFilter::kind(EntityKind::Default),
        ]));
        assert_eq!(results.len(), 3);
        assert!(results.contains(&id1));
        assert!(results.contains(&id2));
        assert!(results.contains(&id3));
    }

    #[test]
    fn test_entity_manager_query_not() {
        let mut manager = EntityManager::new();
        let _id1 = manager.create(EntityKind::Default);
        let _id2 = manager.create(EntityKind::Default);
        let _id3 = manager.create(EntityKind::Default);

        let results = manager.query(EntityFilter::not(EntityFilter::kind(EntityKind::Default)));
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_entity_manager_all_ids() {
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
    fn test_entity_manager_len() {
        let mut manager = EntityManager::new();
        assert_eq!(manager.len(), 0);

        manager.create(EntityKind::Default);
        assert_eq!(manager.len(), 1);

        manager.create(EntityKind::Default);
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_entity_manager_is_empty() {
        let mut manager = EntityManager::new();
        assert!(manager.is_empty());

        manager.create(EntityKind::Default);
        assert!(!manager.is_empty());
    }
}
