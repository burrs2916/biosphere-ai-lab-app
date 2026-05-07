use biosphere_core::EntityId;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}