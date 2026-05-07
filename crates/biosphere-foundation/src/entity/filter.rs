use biosphere_core::EntityId;
use crate::entity::entity::EntityKind;

/// 实体过滤器
///
/// [`EntityFilter`] 定义了实体的过滤条件。
///
/// # 设计约束
///
/// - 组合过滤：支持多个过滤条件的组合
/// - 类型安全：所有过滤条件都是类型安全的
/// - 可扩展：可以添加新的过滤条件
#[derive(Debug, Clone)]
pub enum EntityFilter {
    /// 匹配所有实体
    All,
    /// 匹配指定 ID 的实体
    Id(EntityId),
    /// 匹配指定种类的实体
    Kind(EntityKind),
    /// 匹配多个 ID 的实体
    Ids(Vec<EntityId>),
    /// 匹配多个种类的实体
    Kinds(Vec<EntityKind>),
    /// 逻辑与：同时满足多个过滤条件
    And(Vec<EntityFilter>),
    /// 逻辑或：满足任一过滤条件
    Or(Vec<EntityFilter>),
    /// 逻辑非：不满足过滤条件
    Not(Box<EntityFilter>),
}

impl EntityFilter {
    /// 创建 ID 过滤器
    ///
    /// # 参数
    ///
    /// * `id` - 实体 ID
    ///
    /// # 返回值
    ///
    /// 返回 ID 过滤器
    pub fn id(id: EntityId) -> Self {
        EntityFilter::Id(id)
    }

    /// 创建种类过滤器
    ///
    /// # 参数
    ///
    /// * `kind` - 实体种类
    ///
    /// # 返回值
    ///
    /// 返回种类过滤器
    pub fn kind(kind: EntityKind) -> Self {
        EntityFilter::Kind(kind)
    }

    /// 创建多个 ID 过滤器
    ///
    /// # 参数
    ///
    /// * `ids` - 实体 ID 列表
    ///
    /// # 返回值
    ///
    /// 返回多个 ID 过滤器
    pub fn ids(ids: Vec<EntityId>) -> Self {
        EntityFilter::Ids(ids)
    }

    /// 创建多个种类过滤器
    ///
    /// # 参数
    ///
    /// * `kinds` - 实体种类列表
    ///
    /// # 返回值
    ///
    /// 返回多个种类过滤器
    pub fn kinds(kinds: Vec<EntityKind>) -> Self {
        EntityFilter::Kinds(kinds)
    }

    /// 创建逻辑与过滤器
    ///
    /// # 参数
    ///
    /// * `filters` - 过滤器列表
    ///
    /// # 返回值
    ///
    /// 返回逻辑与过滤器
    pub fn and(filters: Vec<EntityFilter>) -> Self {
        EntityFilter::And(filters)
    }

    /// 创建逻辑或过滤器
    ///
    /// # 参数
    ///
    /// * `filters` - 过滤器列表
    ///
    /// # 返回值
    ///
    /// 返回逻辑或过滤器
    pub fn or(filters: Vec<EntityFilter>) -> Self {
        EntityFilter::Or(filters)
    }

    /// 创建逻辑非过滤器
    ///
    /// # 参数
    ///
    /// * `filter` - 过滤器
    ///
    /// # 返回值
    ///
    /// 返回逻辑非过滤器
    pub fn not(filter: EntityFilter) -> Self {
        EntityFilter::Not(Box::new(filter))
    }
}

impl Default for EntityFilter {
    fn default() -> Self {
        EntityFilter::All
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_filter_id() {
        let filter = EntityFilter::id(EntityId::new(1));
        match filter {
            EntityFilter::Id(id) => {
                assert_eq!(id, EntityId::new(1));
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_kind() {
        let filter = EntityFilter::kind(EntityKind::Default);
        match filter {
            EntityFilter::Kind(kind) => {
                assert_eq!(kind, EntityKind::Default);
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_ids() {
        let filter = EntityFilter::ids(vec![
            EntityId::new(1),
            EntityId::new(2),
            EntityId::new(3),
        ]);
        match filter {
            EntityFilter::Ids(ids) => {
                assert_eq!(ids.len(), 3);
                assert_eq!(ids[0], EntityId::new(1));
                assert_eq!(ids[1], EntityId::new(2));
                assert_eq!(ids[2], EntityId::new(3));
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_kinds() {
        let filter = EntityFilter::kinds(vec![
            EntityKind::Default,
        ]);
        match filter {
            EntityFilter::Kinds(kinds) => {
                assert_eq!(kinds.len(), 1);
                assert_eq!(kinds[0], EntityKind::Default);
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_and() {
        let filter = EntityFilter::and(vec![
            EntityFilter::id(EntityId::new(1)),
            EntityFilter::kind(EntityKind::Default),
        ]);
        match filter {
            EntityFilter::And(filters) => {
                assert_eq!(filters.len(), 2);
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_or() {
        let filter = EntityFilter::or(vec![
            EntityFilter::kind(EntityKind::Default),
        ]);
        match filter {
            EntityFilter::Or(filters) => {
                assert_eq!(filters.len(), 1);
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_not() {
        let filter = EntityFilter::not(EntityFilter::kind(EntityKind::Default));
        match filter {
            EntityFilter::Not(inner) => {
                match *inner {
                    EntityFilter::Kind(kind) => {
                        assert_eq!(kind, EntityKind::Default);
                    }
                    _ => panic!("Unexpected inner filter type"),
                }
            }
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_default() {
        let filter = EntityFilter::default();
        match filter {
            EntityFilter::All => {}
            _ => panic!("Unexpected filter type"),
        }
    }

    #[test]
    fn test_entity_filter_clone() {
        let filter = EntityFilter::id(EntityId::new(1));
        let cloned = filter.clone();
        match cloned {
            EntityFilter::Id(id) => {
                assert_eq!(id, EntityId::new(1));
            }
            _ => panic!("Unexpected filter type"),
        }
    }
}
