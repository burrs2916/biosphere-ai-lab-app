//! Biosphere - 生命系统的基础库
//!
//! 这个库定义了生命系统的最原始抽象，提供了 [`ExistenceCore`] trait 的纯净定义。
//!
//! # 核心理念
//!
//! Biosphere 不是一个应用、框架或平台，而是一个生命系统可以存在、
//! 持续演化而没有预定义目的的领域。
//!
//! 它定义了**生命可能存在的地方**，而不是**生命必须做什么**。
//!
//! # 非目标
//!
//! Biosphere 永远不会：
//! - 定义业务逻辑或应用功能
//! - 强加目标、目的或优化目标给生命型系统
//! - 将生命视为资源、任务或服务
//! - 假设智能、有用性或成功是默认结果
//!
//! # 与 ExistenceCore 的关系
//!
//! [`ExistenceCore`] 定义了生命得以可能的最小公理集。
//!
//! Biosphere 不修改、扩展或重新解释 ExistenceCore。
//! 它只提供一个领域，让符合 ExistenceCore 的系统可以存在、交互和分化。
//!
//! # 与 Perception 和 Representation 的关系
//!
//! [`Perception`] 定义了"什么是可被感知的"。
//! [`Representation`] 定义了"什么是可被呈现的"。
//!
//! Biosphere 不定义具体的感知或呈现技术（UI、Canvas 等）。
//! 它只定义"可被感知"和"可被呈现"的抽象条件。

// 导出存在论层（生命成立的存在论前提）
pub mod ontology;

// 导出拓扑层（世界承认的事实关系）
pub mod topology;

// 导出可能性层（迁移）
pub mod possibility;

// 导出主体层（主体资格）
pub mod subject;

// 导出协议层（世界如何允许改变）
pub mod protocol;

// 导出现实层（现实承认）
pub mod reality;

// 重新导出核心 trait 和类型
pub use ontology::existence_core::ExistenceCore;
pub use ontology::environment::{Environment, WorldState};
pub use ontology::conditions::{Conditions, ConditionSnapshot, ConditionSignal};
pub use ontology::temporal_environment::TemporalEnvironment;
pub use ontology::embodiment::Embodiment;
pub use ontology::field::Field;
pub use ontology::perception::Perception;
pub use ontology::representation::{Representation, RepresentationTopology, RepresentationDimension};
pub use topology::existential_topology::{ExistentialTopology, RelationFact, EntityId};
pub use topology::existential_relation::ExistentialRelationKind;
pub use possibility::migration::Migration;
pub use subject::agency::Agency;
pub use subject::embodied_agency::EmbodiedAgency;
pub use reality::executor::Executor;
pub use protocol::arbiter::Arbiter;
pub use protocol::world_step::WorldStep;
pub use protocol::world_transition::WorldTransition;