//! 拓扑模块
//!
//! 拓扑模块提供了世界事实平面的实现，负责管理当前被承认的关系事实。
//!
//! # 设计哲学
//!
//! Topology 与 Temporal 的关系是：
//! - Temporal 决定"发生过什么"
//! - Topology 决定"世界承认什么"
//!
//! 这意味着：
//! - Topology 是裁决后的结果，不是历史记录
//! - Topology ≠ RelationHistory
//! - Topology 是"无时间、无历史、永恒当下"的事实集合
//!
//! # 模块结构
//!
//! - [`StableTopology`]: 稳定拓扑，世界的唯一事实源

pub mod stable_topology;

pub use stable_topology::StableTopology;
