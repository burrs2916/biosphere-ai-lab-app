use biosphere_core::EntityId;

/// 实体种类
///
/// [`EntityKind`] 定义了实体的种类。
///
/// # 设计约束
///
/// - 基础抽象：提供基础的实体分类
/// - 可扩展：应用层可以定义更多具体的种类
/// - 类型安全：所有种类都是类型安全的
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// 默认实体
    Default,
    /// 未知实体
    Unknown,
    /// 任意实体（用于过滤）
    Any,
}

impl Default for EntityKind {
    fn default() -> Self {
        EntityKind::Default
    }
}

/// 实体
///
/// [`Entity`] 表示世界中的一个实体。
///
/// # 设计约束
///
/// - 值对象：不包含任何世界引用
/// - 不可变：一旦创建就不可修改
/// - 类型绑定：每个实体都与唯一的类型绑定
///
/// # 哲学含义
///
/// Entity 是"世界中的一个存在"，而不是"可修改的对象"。
///
/// 这意味着：
/// - 实体是值对象，不包含任何世界引用
/// - 实体一旦创建就不可修改
/// - 实体与唯一的类型绑定
#[derive(Debug, Clone)]
pub struct Entity {
    id: EntityId,
    kind: EntityKind,
}

impl Entity {
    /// 创建新实体
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    /// * `kind` - 实体种类
    pub fn new(id: EntityId, kind: EntityKind) -> Self {
        Self { id, kind }
    }

    /// 获取实体 ID
    ///
    /// # 返回值
    ///
    /// 返回实体 ID
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// 获取实体种类
    ///
    /// # 返回值
    ///
    /// 返回实体种类
    pub fn kind(&self) -> EntityKind {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(EntityId::new(1), EntityKind::Default);
        assert_eq!(entity.id(), EntityId::new(1));
        assert_eq!(entity.kind(), EntityKind::Default);
    }

    #[test]
    fn test_entity_clone() {
        let entity = Entity::new(EntityId::new(1), EntityKind::Default);
        let cloned = entity.clone();
        assert_eq!(cloned.id(), entity.id());
        assert_eq!(cloned.kind(), entity.kind());
    }

    #[test]
    fn test_entity_kind_default() {
        let kind = EntityKind::default();
        assert_eq!(kind, EntityKind::Default);
    }

    #[test]
    fn test_entity_kind_unknown() {
        let entity = Entity::new(EntityId::new(1), EntityKind::Unknown);
        assert_eq!(entity.kind(), EntityKind::Unknown);
    }

    #[test]
    fn test_entity_kind_any() {
        let entity = Entity::new(EntityId::new(1), EntityKind::Any);
        assert_eq!(entity.kind(), EntityKind::Any);
    }

    #[test]
    fn test_entity_kind_equality() {
        assert_eq!(EntityKind::Default, EntityKind::Default);
        assert_eq!(EntityKind::Unknown, EntityKind::Unknown);
        assert_eq!(EntityKind::Any, EntityKind::Any);
        assert_ne!(EntityKind::Default, EntityKind::Unknown);
        assert_ne!(EntityKind::Default, EntityKind::Any);
        assert_ne!(EntityKind::Unknown, EntityKind::Any);
    }

    #[test]
    fn test_entity_kind_hash() {
        use std::collections::HashSet;
        
        let mut set = HashSet::new();
        set.insert(EntityKind::Default);
        set.insert(EntityKind::Unknown);
        set.insert(EntityKind::Any);
        
        assert_eq!(set.len(), 3);
        assert!(set.contains(&EntityKind::Default));
        assert!(set.contains(&EntityKind::Unknown));
        assert!(set.contains(&EntityKind::Any));
    }

    #[test]
    fn test_entity_kind_copy() {
        let kind1 = EntityKind::Default;
        let kind2 = kind1;
        assert_eq!(kind1, kind2);
    }
}
