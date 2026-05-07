//! 最小世界实现
//!
//! 这个模块提供了 [`BasicWorld`] 的实现，它是第一个可运行的世界实现。
//!
//! # 设计理念
//!
//! [`BasicWorld`] 是"被时间驱动、受公理约束的变化载体"。
//! 它遵循以下核心原则：
//!
//! - **时间驱动**：世界的演化只能通过时间推进触发
//! - **公理约束**：世界的变化必须遵守世界公理
//! - **单向信息流**：世界只提供条件，不观察外部
//! - **最小化实现**：只包含世界存在的最小必要条件
//!
//! # 架构定位
//!
//! [`BasicWorld`] 位于 biosphere-foundation 层，是 biosphere-core 中定义的
//! [`Environment`] trait 的具体实现。它作为基础实现层的一部分，
//! 为更高级的应用层提供可用的世界抽象。
//!
//! # 使用示例
//!
//! ```rust
//! use biosphere_foundation::BasicWorld;
//! use biosphere_core::Environment;
//!
//! // 创建一个新世界
//! let mut world = BasicWorld::new();
//!
//! // 推进世界
//! world.step();
//!
//! // 获取当前条件
//! let conditions = world.conditions();
//! ```
//!
//! # 与其他组件的关系
//!
//! - [`BasicEnvironment`]: 世界内部的环境实现
//! - [`SensedConditions`]: 世界对外暴露的条件

use biosphere_core::{Environment, TemporalEnvironment, RelationFact};
use crate::ontology::BasicEnvironment;
use crate::temporal::state::{StateStore, StateSnapshot, StatePayload, StateProvider, StateHistory, StateQuery};
use crate::temporal::relations::{RelationStore, RelationChange, RelationQuery};
use crate::temporal::Tick;
use crate::invariants::{WorldAxioms, AxiomConfig, WorldAxiomViolation};

/// 最小世界
///
/// [`BasicWorld`] 是第一个可运行的世界实现。
///
/// 它是"被时间驱动、受公理约束的变化载体"。
///
/// # 设计约束
///
/// - 世界拥有时间
/// - 世界不能修改时间规则
/// - 世界变化只能发生在时间推进之后
/// - 世界不能"偷偷变化"
/// - 世界对外只暴露条件
/// - 世界不关心 UI / Projection / Observer
///
/// # 哲学含义
///
/// BasicWorld 是"世界的具体实现"，是所有存在的容器。
///
/// 这意味着：
/// - 世界 = 被时间驱动的存在过程
/// - 世界不解释自己
/// - 世界只做一件事：在合法的时间点，承载一次变化
/// - 世界提供条件，但不"观察"
///
/// # 字段
///
/// - `environment`: 内部环境实现，负责时间管理和条件生成
/// - `state_store`: 世界状态存储器（时间记忆）
/// - `relation_store`: 世界关系存储器（时间记忆）
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::BasicWorld;
///
/// let world = BasicWorld::new();
/// ```
#[derive(Debug)]
pub struct BasicWorld {
    environment: BasicEnvironment,
    state_store: StateStore,
    relation_store: RelationStore,
    axioms: AxiomConfig, // immutable after creation
}

impl BasicWorld {
    /// 创建一个新世界
    ///
    /// 这个方法初始化一个新的 [`BasicWorld`] 实例，包含一个全新的
    /// [`BasicEnvironment`]。新世界的时间从 0 开始。
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 [`BasicWorld`] 实例，其内部环境的时间计数器
    /// 初始化为 0。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_foundation::temporal::Tick;
    ///
    /// let world = BasicWorld::new();
    /// assert_eq!(world.environment().current_tick(), Tick::new(0));
    /// ```
    ///
    /// # 注意
    ///
    /// 新创建的世界不包含任何存在关系或条件信号，除了时间信号。
    /// 要添加这些，需要通过其他机制（如 [`RelationStore`]）。
    pub fn new() -> Self {
        let environment = BasicEnvironment::new();
        let state_store = StateStore::new();
        let relation_store = RelationStore::new();
        let axioms = AxiomConfig::default();

        let mut world = Self {
            environment,
            state_store,
            relation_store,
            axioms,
        };

        // 记录初始状态（tick 0）
        world.commit_state(Tick::new(0));

        world
    }

    /// 使用自定义公理配置创建新世界
    ///
    /// 这个方法允许使用自定义的公理配置创建世界。
    /// 公理配置在创建后将被冻结，不能修改。
    ///
    /// # 参数
    ///
    /// * `axioms` - 公理配置
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 [`BasicWorld`] 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::{BasicWorld, AxiomConfig};
    ///
    /// let axioms = AxiomConfig {
    ///     allow_self_reference: true,
    ///     allow_cycles: false,
    /// };
    /// let world = BasicWorld::with_axioms(axioms);
    /// ```
    pub fn with_axioms(axioms: AxiomConfig) -> Self {
        let environment = BasicEnvironment::new();
        let state_store = StateStore::new();
        let relation_store = RelationStore::new();

        let mut world = Self {
            environment,
            state_store,
            relation_store,
            axioms,
        };

        // 记录初始状态（tick 0）
        world.commit_state(Tick::new(0));

        world
    }

    /// 推进一步世界
    ///
    /// 这是世界唯一允许的演化入口。所有世界状态的变化都必须通过
    /// 这个方法触发。
    ///
    /// # 行为
    ///
    /// 1. 推进内部环境的时间
    /// 2. 验证时间不可逆公理
    /// 3. 更新条件快照
    /// 4. 记录世界状态（时间之后）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    ///
    /// let mut world = BasicWorld::new();
    ///
    /// // 推进世界 5 步
    /// for _ in 0..5 {
    ///     world.step_world();
    /// }
    /// ```
    ///
    /// # 注意
    ///
    /// - 这是世界唯一允许的演化入口
    /// - 不要直接修改内部状态
    /// - 世界变化只能发生在时间推进之后
    /// - 状态永远在时间推进之后产生
    /// - 时间推进必须通过公理验证
    pub fn step_world(&mut self) -> Result<(), crate::invariants::WorldAxiomViolation> {
        let previous_tick = self.environment.current_tick();
        
        // 推进时间
        self.environment.advance();
        let new_tick = self.environment.current_tick();
        
        // 验证时间不可逆公理
        WorldAxioms::assert_time_irreversible(previous_tick.value(), new_tick.value())?;
        
        // 记录状态
        self.commit_state(new_tick);
        
        Ok(())
    }

    /// 获取当前环境状态
    ///
    /// 返回对内部环境的不可变引用，允许外部代码查询环境的状态
    /// 但不允许修改。
    ///
    /// # 返回值
    ///
    /// 返回对 [`BasicEnvironment`] 的不可变引用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    ///
    /// let world = BasicWorld::new();
    /// let env = world.environment();
    /// println!("Current tick: {}", env.current_tick());
    /// ```
    ///
    /// # 注意
    ///
    /// - 返回的是不可变引用，不能修改环境状态
    /// - 要修改环境状态，必须通过 [`step()`] 方法
    pub fn environment(&self) -> &BasicEnvironment {
        &self.environment
    }

    /// 记录世界状态
    ///
    /// 这个方法在时间推进之后记录世界状态快照。
    ///
    /// # 参数
    ///
    /// * `tick` - 当前时间刻
    ///
    /// # 哲学含义
    ///
    /// 状态只记录"结果"，不记录"过程"。过程属于 world 内部逻辑。
    ///
    /// 状态永远在时间推进之后产生，状态 ≠ 变化前。
    fn commit_state(&mut self, tick: Tick) {
        let world_state = WorldState {
            tick_seen: tick.value(),
        };

        let snapshot = StateSnapshot::new(
            tick,
            StatePayload::new(world_state),
        );

        self.state_store.commit(snapshot);
    }

    /// 获取最新的时间刻
    ///
    /// 返回世界最新的时间刻。
    ///
    /// # 返回值
    ///
    /// 返回最新的时间刻。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    ///
    /// let world = BasicWorld::new();
    /// let latest_tick = world.latest_tick();
    /// ```
    pub fn latest_tick(&self) -> u64 {
        self.environment.current_tick().value()
    }

    /// 提交关系变更
    ///
    /// 这是世界唯一允许的关系变更入口。所有关系变更都必须通过
    /// 这个方法触发，并经过公理验证。
    ///
    /// # 参数
    ///
    /// * `change` - 关系变更
    ///
    /// # 返回值
    ///
    /// 如果变更违反公理，返回错误，否则返回 Ok(())
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_foundation::temporal::relations::{RelationChange, RelationChangeKind};
    /// use biosphere_foundation::temporal::Tick;
    /// use biosphere_core::{ExistentialRelationKind, RelationFact, EntityId};
    ///
    /// let mut world = BasicWorld::new();
    /// let fact = RelationFact::new(
    ///     ExistentialRelationKind::EmbodimentInField,
    ///     EntityId::new(1),
    ///     EntityId::new(2),
    /// );
    /// let change = RelationChange::new(Tick::new(1), RelationChangeKind::Added, fact);
    /// let result = world.commit_relation_change(change);
    /// ```
    ///
    /// # 注意
    ///
    /// - 这是世界唯一允许的关系变更入口
    /// - 所有关系变更都必须经过公理验证
    /// - 不能绕过此方法直接修改关系
    pub fn commit_relation_change(&mut self, change: RelationChange) -> Result<(), WorldAxiomViolation> {
        // 验证关系完整性公理
        let relation = change.fact();
        let existing_facts: Vec<RelationFact> = self.relation_store.history()
            .iter()
            .filter_map(|c| Some(c.fact().clone()))
            .collect();
        
        WorldAxioms::assert_relation_integrity(relation, &existing_facts, &self.axioms)?;

        // 验证拓扑不变性公理
        let mut all_facts = existing_facts.clone();
        all_facts.push(relation.clone());
        
        WorldAxioms::assert_topology_invariant(&all_facts, &self.axioms)?;

        // 提交变更
        self.relation_store.commit(change);

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorldState {
    tick_seen: u64,
}

impl Environment for BasicWorld {
    type State = crate::ontology::BasicEnvironmentState;
    type Conditions = crate::conditions::SensedConditions;

    /// 推进世界
    ///
    /// 这是 [`Environment`] trait 的实现。它调用内部的 [`step_world()`] 方法
    /// 来推进世界。
    ///
    /// # 行为
    ///
    /// 这个方法会忽略任何错误，因为 [`Environment`] trait 的 `step()` 方法
    /// 不返回错误。如果需要处理错误，应该直接调用 [`step_world()`] 方法。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_core::Environment;
    ///
    /// let mut world = BasicWorld::new();
    /// world.step(); // 忽略错误
    /// ```
    fn step(&mut self) {
        let _ = self.step_world();
    }

    /// 获取当前条件
    ///
    /// 返回对当前条件的不可变引用。条件是世界对外暴露的唯一接口，
    /// 外部代码可以通过条件观察世界状态，但不能修改世界。
    ///
    /// # 返回值
    ///
    /// 返回对 [`SensedConditions`] 的不可变引用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_core::Environment;
    ///
    /// let world = BasicWorld::new();
    /// let conditions = world.conditions();
    /// let snapshot = conditions.snapshot();
    /// ```
    ///
    /// # 注意
    ///
    /// - 返回的是不可变引用，不能修改条件
    /// - 条件是只读的，世界不"观察"外部
    /// - 条件是单向信息流：World → Conditions → Projection → Observer
    fn conditions(&self) -> &Self::Conditions {
        self.environment.conditions()
    }
}

impl TemporalEnvironment for BasicWorld {
    /// 推进时间
    ///
    /// 这是 [`TemporalEnvironment`] trait 的实现。它推进内部环境的时间。
    ///
    /// # 行为
    ///
    /// 这个方法会推进内部环境的时间计数器，并更新条件快照。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_core::TemporalEnvironment;
    ///
    /// let mut world = BasicWorld::new();
    /// world.advance();
    /// ```
    ///
    /// # 注意
    ///
    /// - 这个方法不返回错误，时间推进总是成功的
    /// - 要处理可能的错误，应该使用 [`step()`] 方法
    fn advance(&mut self) {
        self.environment.advance();
    }
}

impl StateProvider for BasicWorld {
    fn current_state(&self) -> Option<&StateSnapshot> {
        self.state_store.history().latest()
    }

    fn state_history(&self) -> &StateHistory {
        self.state_store.history()
    }
}

impl StateQuery for BasicWorld {
    fn get_at(&self, tick: Tick) -> Option<&StateSnapshot> {
        self.state_store.history().get_at(tick)
    }

    fn query_range(&self, start: Tick, end: Tick) -> Vec<&StateSnapshot> {
        self.state_store.history().query_range(start, end)
    }

    fn latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.state_store.history().latest()
    }
}

impl RelationQuery for BasicWorld {
    fn get_relation_at(&self, tick: Tick) -> Option<&RelationChange> {
        self.relation_store.history().get_at(tick)
    }

    fn query_relations_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange> {
        self.relation_store.history()
            .iter()
            .filter(|c| c.tick() >= start && c.tick() <= end)
            .collect()
    }

    fn latest_relation_change(&self) -> Option<&RelationChange> {
        self.relation_store.history().latest()
    }
}

impl Default for BasicWorld {
    /// 创建默认的世界实例
    ///
    /// 这个方法使用 [`new()`] 创建一个新世界。
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 [`BasicWorld`] 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::BasicWorld;
    /// use biosphere_foundation::temporal::Tick;
    /// use std::default::Default;
    ///
    /// let world: BasicWorld = Default::default();
    /// assert_eq!(world.environment().current_tick(), Tick::new(0));
    /// ```
    fn default() -> Self {
        Self::new()
    }
}
