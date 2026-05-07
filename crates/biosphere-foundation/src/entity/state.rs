use biosphere_core::EntityId;
use crate::entity::entity::Entity;
use crate::entity::existence::EntityExistence;
use std::collections::HashMap;

/// 实体身份空间
///
/// [`EntityIdentitySpace`] 负责管理实体身份和 ID 生成。
///
/// # 设计约束
///
/// - ID 唯一性：确保每个实体 ID 都是唯一的
/// - 顺序生成：ID 按顺序生成，便于调试和排序
/// - 无状态：不存储实体状态，只管理身份
/// - 可扩展：支持未来添加更多身份管理功能
///
/// # 哲学含义
///
/// EntityIdentitySpace 是"实体身份的来源"，而不是"实体状态的容器"。
///
/// 这意味着：
/// - EntityIdentitySpace 只负责生成唯一 ID
/// - EntityIdentitySpace 不存储实体状态
/// - EntityIdentitySpace 不关心实体是否存在
/// - EntityIdentitySpace 可以被多个世界共享
#[derive(Debug, Default)]
pub struct EntityIdentitySpace {
    /// 下一个实体 ID
    next_id: u64,
}

impl EntityIdentitySpace {
    /// 创建新的实体身份空间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::identity::EntityIdentitySpace;
    ///
    /// let identity_space = EntityIdentitySpace::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成新的实体 ID
    ///
    /// # 返回值
    ///
    /// 返回新的实体 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::identity::EntityIdentitySpace;
    ///
    /// let mut identity_space = EntityIdentitySpace::new();
    /// let id1 = identity_space.generate_id();
    /// let id2 = identity_space.generate_id();
    /// assert_ne!(id1, id2);
    /// ```
    pub fn generate_id(&mut self) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// 获取下一个将要生成的 ID
    ///
    /// # 返回值
    ///
    /// 返回下一个将要生成的 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::identity::EntityIdentitySpace;
    ///
    /// let identity_space = EntityIdentitySpace::new();
    /// assert_eq!(identity_space.next_id(), 0);
    /// ```
    pub fn next_id(&self) -> u64 {
        self.next_id
    }
}

/// 世界实体状态
///
/// [`WorldEntityState`] 负责管理特定时间点的实体集合。
///
/// # 设计约束
///
/// - 时间感知：支持时间范围内的实体查询
/// - 状态管理：管理实体的当前状态
/// - 存在记录：跟踪实体的生命周期
/// - 可查询：支持多种查询方式
///
/// # 哲学含义
///
/// WorldEntityState 是"特定时间点的实体集合"，而不是"所有实体的集合"。
///
/// 这意味着：
/// - WorldEntityState 管理特定时间点的实体
/// - WorldEntityState 跟踪实体的生命周期
/// - WorldEntityState 支持历史查询
/// - WorldEntityState 可以被多个世界共享
#[derive(Debug, Default)]
pub struct WorldEntityState {
    /// 当前存活的实体
    entities: HashMap<EntityId, Entity>,
    /// 实体存在记录
    existences: HashMap<EntityId, EntityExistence>,
}

impl WorldEntityState {
    /// 创建新的世界实体状态
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    ///
    /// let world_state = WorldEntityState::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新实体
    ///
    /// # 参数
    ///
    /// * `entity` - 实体
    /// * `at` - 创建时间
    ///
    /// # 返回值
    ///
    /// 返回实体 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let id = world_state.create_entity(entity, 0);
    /// assert_eq!(id, EntityId::new(1));
    /// ```
    pub fn create_entity(&mut self, entity: Entity, at: u64) -> EntityId {
        let id = entity.id();
        let existence = EntityExistence::new(id, at);
        
        self.entities.insert(id, entity);
        self.existences.insert(id, existence);
        
        id
    }

    /// 删除实体
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    /// * `at` - 删除时间
    ///
    /// # 返回值
    ///
    /// 如果实体存在，返回 Ok(())，否则返回 Err
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let id = world_state.create_entity(entity, 0);
    ///
    /// let result = world_state.delete_entity(id, 10);
    /// assert!(result.is_ok());
    /// ```
    pub fn delete_entity(&mut self, id: EntityId, at: u64) -> Result<(), String> {
        if let Some(existence) = self.existences.get_mut(&id) {
            existence.die(at);
            self.entities.remove(&id);
            Ok(())
        } else {
            Err(format!("Entity {:?} not found", id))
        }
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
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let id = world_state.create_entity(entity, 0);
    ///
    /// let entity = world_state.get_entity(id);
    /// assert!(entity.is_some());
    /// ```
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// 获取实体存在记录
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    ///
    /// # 返回值
    ///
    /// 如果实体存在记录存在，返回存在记录的引用，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let id = world_state.create_entity(entity, 0);
    ///
    /// let existence = world_state.get_existence(id);
    /// assert!(existence.is_some());
    /// ```
    pub fn get_existence(&self, id: EntityId) -> Option<&EntityExistence> {
        self.existences.get(&id)
    }

    /// 获取在指定时间存活的实体 ID
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    ///
    /// # 返回值
    ///
    /// 返回在指定时间存活的实体 ID 列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity1 = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let entity2 = Entity::new(EntityId::new(2), EntityKind::Default);
    /// world_state.create_entity(entity1, 0);
    /// world_state.create_entity(entity2, 5);
    ///
    /// let alive_at_0 = world_state.alive_entities_at(0);
    /// assert_eq!(alive_at_0.len(), 1);
    ///
    /// let alive_at_5 = world_state.alive_entities_at(5);
    /// assert_eq!(alive_at_5.len(), 2);
    /// ```
    pub fn alive_entities_at(&self, tick: u64) -> Vec<EntityId> {
        self.existences
            .iter()
            .filter(|(_, existence)| existence.is_alive_at(tick))
            .map(|(id, _)| *id)
            .collect()
    }

    /// 获取所有实体 ID
    ///
    /// # 返回值
    ///
    /// 返回所有实体 ID 列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// let entity1 = Entity::new(EntityId::new(1), EntityKind::Default);
    /// let entity2 = Entity::new(EntityId::new(2), EntityKind::Default);
    /// world_state.create_entity(entity1, 0);
    /// world_state.create_entity(entity2, 0);
    ///
    /// let all_ids = world_state.all_entity_ids();
    /// assert_eq!(all_ids.len(), 2);
    /// ```
    pub fn all_entity_ids(&self) -> Vec<EntityId> {
        self.entities.keys().copied().collect()
    }

    /// 获取实体数量
    ///
    /// # 返回值
    ///
    /// 返回实体数量
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::state::WorldEntityState;
    /// use biosphere_foundation::entity::entity::{Entity, EntityKind};
    /// use biosphere_core::EntityId;
    ///
    /// let mut world_state = WorldEntityState::new();
    /// assert_eq!(world_state.len(), 0);
    ///
    /// let entity = Entity::new(EntityId::new(1), EntityKind::Default);
    /// world_state.create_entity(entity, 0);
    /// assert_eq!(world_state.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.entities.len()
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
    /// use biosphere_foundation::entity::state::WorldEntityState;
    ///
    /// let world_state = WorldEntityState::new();
    /// assert!(world_state.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::entity::EntityKind;
    use biosphere_core::EntityId;

    #[test]
    fn test_entity_identity_space_creation() {
        let identity_space = EntityIdentitySpace::new();
        assert_eq!(identity_space.next_id(), 0);
    }

    #[test]
    fn test_entity_identity_space_generate_id() {
        let mut identity_space = EntityIdentitySpace::new();
        let id1 = identity_space.generate_id();
        let id2 = identity_space.generate_id();
        
        assert_eq!(id1, EntityId::new(0));
        assert_eq!(id2, EntityId::new(1));
        assert_eq!(identity_space.next_id(), 2);
    }

    #[test]
    fn test_world_entity_state_creation() {
        let world_state = WorldEntityState::new();
        assert!(world_state.is_empty());
        assert_eq!(world_state.len(), 0);
    }

    #[test]
    fn test_world_entity_state_create_entity() {
        let mut world_state = WorldEntityState::new();
        let entity = Entity::new(EntityId::new(1), EntityKind::Default);
        let id = world_state.create_entity(entity, 0);
        
        assert_eq!(id, EntityId::new(1));
        assert_eq!(world_state.len(), 1);
        assert!(world_state.get_entity(id).is_some());
        assert!(world_state.get_existence(id).is_some());
    }

    #[test]
    fn test_world_entity_state_delete_entity() {
        let mut world_state = WorldEntityState::new();
        let entity = Entity::new(EntityId::new(1), EntityKind::Default);
        let id = world_state.create_entity(entity, 0);
        
        let result = world_state.delete_entity(id, 10);
        assert!(result.is_ok());
        assert_eq!(world_state.len(), 0);
        assert!(world_state.get_entity(id).is_none());
        
        let existence = world_state.get_existence(id).unwrap();
        assert_eq!(existence.died_at(), Some(10));
    }

    #[test]
    fn test_world_entity_state_delete_not_found() {
        let mut world_state = WorldEntityState::new();
        let id = EntityId::new(999);
        
        let result = world_state.delete_entity(id, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_world_entity_state_alive_entities_at() {
        let mut world_state = WorldEntityState::new();
        let entity1 = Entity::new(EntityId::new(1), EntityKind::Default);
        let entity2 = Entity::new(EntityId::new(2), EntityKind::Default);
        let entity3 = Entity::new(EntityId::new(3), EntityKind::Default);
        
        world_state.create_entity(entity1, 0);
        world_state.create_entity(entity2, 5);
        world_state.create_entity(entity3, 10);
        
        let alive_at_0 = world_state.alive_entities_at(0);
        assert_eq!(alive_at_0.len(), 1);
        assert!(alive_at_0.contains(&EntityId::new(1)));
        
        let alive_at_5 = world_state.alive_entities_at(5);
        assert_eq!(alive_at_5.len(), 2);
        assert!(alive_at_5.contains(&EntityId::new(1)));
        assert!(alive_at_5.contains(&EntityId::new(2)));
        
        let alive_at_10 = world_state.alive_entities_at(10);
        assert_eq!(alive_at_10.len(), 3);
        assert!(alive_at_10.contains(&EntityId::new(1)));
        assert!(alive_at_10.contains(&EntityId::new(2)));
        assert!(alive_at_10.contains(&EntityId::new(3)));
    }

    #[test]
    fn test_world_entity_state_alive_entities_at_with_death() {
        let mut world_state = WorldEntityState::new();
        let entity1 = Entity::new(EntityId::new(1), EntityKind::Default);
        let entity2 = Entity::new(EntityId::new(2), EntityKind::Default);
        
        let id1 = world_state.create_entity(entity1, 0);
        let id2 = world_state.create_entity(entity2, 0);
        
        // 在时间0，两个实体都存活
        let alive_at_0 = world_state.alive_entities_at(0);
        assert_eq!(alive_at_0.len(), 2);
        assert!(alive_at_0.contains(&id1));
        assert!(alive_at_0.contains(&id2));
        
        // 删除实体1
        world_state.delete_entity(id1, 5).unwrap();
        
        // 在时间5，只有实体2存活
        let alive_at_5 = world_state.alive_entities_at(5);
        assert_eq!(alive_at_5.len(), 1);
        assert!(alive_at_5.contains(&id2));
        assert!(!alive_at_5.contains(&id1));
    }

    #[test]
    fn test_world_entity_state_all_entity_ids() {
        let mut world_state = WorldEntityState::new();
        let entity1 = Entity::new(EntityId::new(1), EntityKind::Default);
        let entity2 = Entity::new(EntityId::new(2), EntityKind::Default);
        
        world_state.create_entity(entity1, 0);
        world_state.create_entity(entity2, 0);
        
        let all_ids = world_state.all_entity_ids();
        assert_eq!(all_ids.len(), 2);
        assert!(all_ids.contains(&EntityId::new(1)));
        assert!(all_ids.contains(&EntityId::new(2)));
    }
}