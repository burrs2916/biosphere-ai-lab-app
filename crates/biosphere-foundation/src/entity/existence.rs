use biosphere_core::EntityId;

/// 实体存在记录
///
/// [`EntityExistence`] 描述实体在时间轴上的存在范围。
///
/// # 设计约束
///
/// - 时间绑定：每个实体存在记录都与时间轴绑定
/// - 不可变性：一旦创建就不可修改（除了标记死亡）
/// - 类型安全：所有操作都是类型安全的
/// - 可查询：支持时间范围内的查询
///
/// # 哲学含义
///
/// EntityExistence 是"实体在时间轴上的存在"，而不是"实体本身"。
///
/// 这意味着：
/// - 实体存在记录描述实体何时存在
/// - 实体存在记录不包含实体的具体属性
/// - 实体存在记录可以被查询和过滤
/// - 实体存在记录支持历史回溯
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityExistence {
    /// 实体 ID
    entity_id: EntityId,
    /// 出生时间
    born_at: u64,
    /// 死亡时间（None 表示仍然存活）
    died_at: Option<u64>,
}

impl EntityExistence {
    /// 创建新的实体存在记录
    ///
    /// # 参数
    ///
    /// * `entity_id` - 实体 ID
    /// * `born_at` - 出生时间
    ///
    /// # 返回值
    ///
    /// 返回新的实体存在记录
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let existence = EntityExistence::new(EntityId::new(1), 0);
    /// ```
    pub fn new(entity_id: EntityId, born_at: u64) -> Self {
        Self {
            entity_id,
            born_at,
            died_at: None,
        }
    }

    /// 获取实体 ID
    ///
    /// # 返回值
    ///
    /// 返回实体 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let existence = EntityExistence::new(EntityId::new(1), 0);
    /// assert_eq!(existence.entity_id(), EntityId::new(1));
    /// ```
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    /// 获取出生时间
    ///
    /// # 返回值
    ///
    /// 返回出生时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let existence = EntityExistence::new(EntityId::new(1), 0);
    /// assert_eq!(existence.born_at(), 0);
    /// ```
    pub fn born_at(&self) -> u64 {
        self.born_at
    }

    /// 获取死亡时间
    ///
    /// # 返回值
    ///
    /// 返回死亡时间（None 表示仍然存活）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let existence = EntityExistence::new(EntityId::new(1), 0);
    /// assert_eq!(existence.died_at(), None);
    /// ```
    pub fn died_at(&self) -> Option<u64> {
        self.died_at
    }

    /// 标记实体死亡
    ///
    /// # 参数
    ///
    /// * `at` - 死亡时间
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let mut existence = EntityExistence::new(EntityId::new(1), 0);
    /// existence.die(10);
    /// assert_eq!(existence.died_at(), Some(10));
    /// ```
    pub fn die(&mut self, at: u64) {
        self.died_at = Some(at);
    }

    /// 检查实体在指定时间是否存活
    ///
    /// # 参数
    ///
    /// * `tick` - 时间刻
    ///
    /// # 返回值
    ///
    /// 如果实体在指定时间存活，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let mut existence = EntityExistence::new(EntityId::new(1), 0);
    /// assert!(existence.is_alive_at(0));
    /// assert!(existence.is_alive_at(5));
    ///
    /// existence.die(10);
    /// assert!(existence.is_alive_at(9));
    /// assert!(!existence.is_alive_at(10));
    /// ```
    pub fn is_alive_at(&self, tick: u64) -> bool {
        tick >= self.born_at && self.died_at.map_or(true, |death| tick < death)
    }

    /// 获取存在时间范围
    ///
    /// # 返回值
    ///
    /// 返回存在时间范围 (born_at, died_at)
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::entity::existence::EntityExistence;
    /// use biosphere_core::EntityId;
    ///
    /// let existence = EntityExistence::new(EntityId::new(1), 0);
    /// assert_eq!(existence.lifespan(), (0, None));
    ///
    /// let mut existence = EntityExistence::new(EntityId::new(1), 0);
    /// existence.die(10);
    /// assert_eq!(existence.lifespan(), (0, Some(10)));
    /// ```
    pub fn lifespan(&self) -> (u64, Option<u64>) {
        (self.born_at, self.died_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_existence_creation() {
        let existence = EntityExistence::new(EntityId::new(1), 0);
        assert_eq!(existence.entity_id(), EntityId::new(1));
        assert_eq!(existence.born_at(), 0);
        assert_eq!(existence.died_at(), None);
    }

    #[test]
    fn test_entity_existence_die() {
        let mut existence = EntityExistence::new(EntityId::new(1), 0);
        existence.die(10);
        assert_eq!(existence.died_at(), Some(10));
    }

    #[test]
    fn test_entity_existence_is_alive_at() {
        let mut existence = EntityExistence::new(EntityId::new(1), 10);
        
        // 出生前
        assert!(!existence.is_alive_at(5));
        assert!(!existence.is_alive_at(9));
        
        // 出生后
        assert!(existence.is_alive_at(10));
        assert!(existence.is_alive_at(15));
        
        // 死亡后
        existence.die(20);
        assert!(existence.is_alive_at(19));
        assert!(!existence.is_alive_at(20));
        assert!(!existence.is_alive_at(25));
    }

    #[test]
    fn test_entity_existence_lifespan() {
        let existence = EntityExistence::new(EntityId::new(1), 0);
        assert_eq!(existence.lifespan(), (0, None));
        
        let mut existence = EntityExistence::new(EntityId::new(1), 0);
        existence.die(10);
        assert_eq!(existence.lifespan(), (0, Some(10)));
    }

    #[test]
    fn test_entity_existence_clone() {
        let existence = EntityExistence::new(EntityId::new(1), 0);
        let cloned = existence.clone();
        assert_eq!(existence, cloned);
    }

    #[test]
    fn test_entity_existence_equality() {
        let existence1 = EntityExistence::new(EntityId::new(1), 0);
        let existence2 = EntityExistence::new(EntityId::new(1), 0);
        assert_eq!(existence1, existence2);
        
        let existence3 = EntityExistence::new(EntityId::new(2), 0);
        assert_ne!(existence1, existence3);
        
        let existence4 = EntityExistence::new(EntityId::new(1), 1);
        assert_ne!(existence1, existence4);
    }
}