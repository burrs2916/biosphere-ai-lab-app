//! 存在拓扑的基础定义
//!
//! 这个模块定义了 [`ExistentialTopology`] trait，它记录当前有效的存在关系。
//!
//! # 核心公理
//!
//! 存在拓扑是"世界承认的关系事实"的记录，而不是生命声称的关系。
//!
//! # 设计原则
//!
//! - **事实性**：只记录世界承认的关系事实
//! - **只读性**：外部只能查询，不能修改
//! - **不可伪造**：关系事实只能由 World/Topology 生成
//!
//! # 与其他组件的关系
//!
//! - **ExistentialRelationKind**：使用关系种类来分类关系事实
//! - **Field**：拓扑记录了存在与场域之间的连接
//! - **Environment**：拓扑记录了环境与场域之间的连接
//!
//! # 哲学含义
//!
//! 存在拓扑是"世界的真理"，而不是"生命的视角"。
//!
//! 这意味着：
//! - 任何存在都不能说"我在某个 Field"
//! - 任何代码都不能伪造关系
//! - 只有世界拓扑能说"这是事实"
//!
//! # 示例
//!
//! ```rust
//! use biosphere::ExistentialTopology;
//! use biosphere::ExistentialRelationKind;
//! use biosphere::RelationFact;
//!
//! // 假设有一个具体的 ExistentialTopology 实现
//! struct MyTopology;
//!
//! impl ExistentialTopology for MyTopology {
//!     fn relations(&self) -> Vec<RelationFact> {
//!         // 返回当前成立的关系事实
//!         vec![]
//!     }
//!
//!     fn acknowledges(&self, relation: &RelationFact) -> bool {
//!         // 判断关系是否成立
//!         false
//!     }
//! }
//! ```

use crate::topology::existential_relation::ExistentialRelationKind;

/// 存在拓扑的核心抽象
///
/// [`ExistentialTopology`] 记录"哪些存在承认关系当前有效"。
///
/// 它是"世界的真理"，而不是"生命的视角"。
///
/// # 方法
///
/// - [`relations`](ExistentialTopology::relations)：返回当前成立的关系事实集合
/// - [`acknowledges`](ExistentialTopology::acknowledges)：判断某个关系事实是否成立
///
/// # 设计约束
///
/// - 只读接口：外部只能查询，不能修改
/// - 不可伪造：关系事实只能由 World/Topology 生成
/// - 事实性：只记录世界承认的关系事实
///
/// # 哲学含义
///
/// 存在拓扑是"世界的真理"，而不是"生命的视角"。
///
/// 这意味着：
/// - 任何存在都不能说"我在某个 Field"
/// - 任何代码都不能伪造关系
/// - 只有世界拓扑能说"这是事实"
///
/// # 示例
///
/// ```rust
/// use biosphere::ExistentialTopology;
/// use biosphere::ExistentialRelationKind;
/// use biosphere::RelationFact;
///
/// struct MyTopology;
///
/// impl ExistentialTopology for MyTopology {
///     fn relations(&self) -> Vec<RelationFact> {
///         // 返回当前成立的关系事实
///         vec![]
///     }
///
///     fn acknowledges(&self, relation: &RelationFact) -> bool {
///         // 判断关系是否成立
///         false
///     }
/// }
/// ```
pub trait ExistentialTopology {
    /// 返回当前成立的关系事实集合
    ///
    /// 这个方法返回所有当前世界承认的关系事实。
    ///
    /// # 返回值
    ///
    /// 返回当前成立的关系事实集合
    ///
    /// # 约束
    ///
    /// - 只读：不修改拓扑状态
    /// - 不可伪造：关系事实只能由 World/Topology 生成
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::ExistentialTopology;
    ///
    /// let topology = MyTopology;
    /// let relations = topology.relations();
    /// ```
    fn relations(&self) -> Vec<RelationFact>;

    /// 判断某个关系事实是否成立
    ///
    /// 这个方法判断给定的关系事实是否被世界承认。
    ///
    /// # 参数
    ///
    /// - `relation`：要判断的关系事实
    ///
    /// # 返回值
    ///
    /// 返回 `true` 如果关系事实成立，否则返回 `false`
    ///
    /// # 约束
    ///
    /// - 只读：不修改拓扑状态
    /// - 不可伪造：关系事实只能由 World/Topology 生成
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::ExistentialTopology;
    ///
    /// let topology = MyTopology;
    /// let relation = RelationFact::new(...);
    /// let is_valid = topology.acknowledges(&relation);
    /// ```
    fn acknowledges(&self, relation: &RelationFact) -> bool;
}

/// 关系事实
///
/// [`RelationFact`] 是关系的"事实实例"，只能存在于 biosphere-foundation 的拓扑中。
///
/// 它携带具体的关系数据，包括关系种类、主体和客体。
///
/// # 字段
///
/// - [`kind`](RelationFact::kind)：关系种类
/// - [`subject_id`](RelationFact::subject_id)：主体 ID
/// - [`object_id`](RelationFact::object_id)：客体 ID
///
/// # 设计约束
///
/// - 不能手动构造
/// - 只能由 World/Topology 生成
/// - 不能脱离 World 存在
///
/// # 哲学含义
///
/// 关系事实是"世界的真理"，而不是"生命的视角"。
///
/// 这意味着：
/// - 任何存在都不能说"我在某个 Field"
/// - 任何代码都不能伪造关系
/// - 只有世界拓扑能说"这是事实"
///
/// # 示例
///
/// ```rust
/// use biosphere::ExistentialRelationKind;
///
/// // 关系事实只能由 World/Topology 生成
/// let fact = RelationFact::new(
///     ExistentialRelationKind::EmbodimentInField,
///     EntityId::new(1),
///     EntityId::new(2),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RelationFact {
    /// 关系种类
    pub kind: ExistentialRelationKind,

    /// 主体 ID
    pub subject_id: EntityId,

    /// 客体 ID
    pub object_id: EntityId,
}

impl RelationFact {
    /// 创建新的关系事实
    ///
    /// 这个方法创建一个新的关系事实。
    ///
    /// # 参数
    ///
    /// - `kind`：关系种类
    /// - `subject_id`：主体 ID
    /// - `object_id`：客体 ID
    ///
    /// # 返回值
    ///
    /// 返回新的关系事实
    ///
    /// # 约束
    ///
    /// - 只能由 World/Topology 生成
    /// - 不能脱离 World 存在
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere::ExistentialRelationKind;
    ///
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// ```
    pub fn new(kind: ExistentialRelationKind, subject_id: EntityId, object_id: EntityId) -> Self {
        Self {
            kind,
            subject_id,
            object_id,
        }
    }
}

/// 实体 ID
///
/// [`EntityId`] 用于标识世界中的实体。
///
/// # 设计约束
///
/// - 唯一性：每个实体有唯一的 ID
/// - 不可伪造：只能由 World 生成
///
/// # 示例
///
/// ```rust
/// let id = EntityId::new(1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    /// 创建新的实体 ID
    ///
    /// # 参数
    ///
    /// - `id`：ID 值
    ///
    /// # 返回值
    ///
    /// 返回新的实体 ID
    ///
    /// # 示例
    ///
    /// ```rust
    /// let id = EntityId::new(1);
    /// ```
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}
