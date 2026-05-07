//! 世界公理（不可破坏）
//!
//! WorldAxioms 定义了任何世界都不能违反的存在论约束。
//! 它们不是规则、不是策略，而是"存在成立的前提"。
//!
//! # 设计约束
//!
//! - 基础公理：定义最基本的存在论约束
//! - 可配置：支持灵活的公理配置
//! - 可扩展：应用层可以扩展公理实现

use std::fmt;
use biosphere_core::{RelationFact, EntityId};

/// 世界公理违反错误
#[derive(Debug, Clone)]
pub struct WorldAxiomViolation {
    pub axiom: &'static str,
    pub reason: &'static str,
}

impl fmt::Display for WorldAxiomViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "World axiom violated: {} — {}",
            self.axiom, self.reason
        )
    }
}

impl std::error::Error for WorldAxiomViolation {}

/// 公理配置
///
/// [`AxiomConfig`] 定义了公理的配置选项。
///
/// # 设计约束
///
/// - 灵活配置：支持不同的公理配置
/// - 默认值：提供合理的默认配置
/// - 应用层控制：应用层可以自定义配置
#[derive(Debug, Clone)]
pub struct AxiomConfig {
    /// 是否允许自指关系
    pub allow_self_reference: bool,
    /// 是否允许循环关系
    pub allow_cycles: bool,
}

impl Default for AxiomConfig {
    fn default() -> Self {
        Self {
            allow_self_reference: false,
            allow_cycles: false,
        }
    }
}

/// 世界公理集合（不可实例化）
///
/// ❗任何 World / Executor / Clock 都必须遵守
pub struct WorldAxioms;

impl WorldAxioms {
    /// 公理一：世界不可回滚
    ///
    /// 一旦世界时间推进，就不能回到过去的状态。
    /// 任何试图减少时间、恢复旧状态的行为都是非法的。
    ///
    /// # 设计约束
    ///
    /// - 这是不可违反的基础公理
    /// - 不支持配置，时间不可逆是核心原则
    pub fn assert_time_irreversible(
        previous_tick: u64,
        next_tick: u64,
    ) -> Result<(), WorldAxiomViolation> {
        if next_tick < previous_tick {
            Err(WorldAxiomViolation {
                axiom: "TimeIsIrreversible",
                reason: "world time attempted to move backwards",
            })
        } else {
            Ok(())
        }
    }

    /// 公理三：关系完整性
    ///
    /// 关系必须满足完整性约束：
    /// - 关系的主体和客体必须存在
    /// - 关系不能重复
    /// - 关系不能自指（除非配置允许）
    ///
    /// # 设计约束
    ///
    /// - 支持配置：通过 AxiomConfig 控制是否允许自指
    /// - 应用层控制：应用层可以自定义配置
    pub fn assert_relation_integrity(
        relation: &RelationFact,
        existing_facts: &[RelationFact],
        config: &AxiomConfig,
    ) -> Result<(), WorldAxiomViolation> {
        if !config.allow_self_reference && relation.subject_id == relation.object_id {
            Err(WorldAxiomViolation {
                axiom: "RelationIntegrity",
                reason: "relation cannot refer to itself",
            })
        } else if existing_facts.contains(relation) {
            Err(WorldAxiomViolation {
                axiom: "RelationIntegrity",
                reason: "relation already exists",
            })
        } else {
            Ok(())
        }
    }

    /// 公理四：拓扑不变性
    ///
    /// 拓扑结构必须满足不变性约束：
    /// - 不能出现循环依赖（除非配置允许）
    /// - 拓扑更新必须是原子的
    ///
    /// # 设计约束
    ///
    /// - 支持配置：通过 AxiomConfig 控制是否允许循环
    /// - 应用层控制：应用层可以自定义配置
    pub fn assert_topology_invariant(
        facts: &[RelationFact],
        config: &AxiomConfig,
    ) -> Result<(), WorldAxiomViolation> {
        if !config.allow_cycles && Self::has_cycle(facts) {
            Err(WorldAxiomViolation {
                axiom: "TopologyInvariant",
                reason: "topology contains cycles",
            })
        } else {
            Ok(())
        }
    }

    /// 公理五：存在唯一性
    ///
    /// 每个实体必须具有唯一的标识符。
    /// 不能存在两个具有相同标识符的不同实体。
    ///
    /// # 设计约束
    ///
    /// - 这是不可违反的基础公理
    /// - 不支持配置，实体唯一性是核心原则
    pub fn assert_existence_uniqueness(
        entity_id: EntityId,
        existing_ids: &[EntityId],
    ) -> Result<(), WorldAxiomViolation> {
        if existing_ids.contains(&entity_id) {
            Err(WorldAxiomViolation {
                axiom: "ExistenceUniqueness",
                reason: "entity with this ID already exists",
            })
        } else {
            Ok(())
        }
    }

    /// 公理六：因果关系
    ///
    /// 因果关系必须满足时间顺序：
    /// - 原因必须在结果之前
    /// - 不能出现因果循环
    ///
    /// # 设计约束
    ///
    /// - 这是不可违反的基础公理
    /// - 不支持配置，因果关系是核心原则
    pub fn assert_causality(
        cause_tick: u64,
        effect_tick: u64,
    ) -> Result<(), WorldAxiomViolation> {
        if effect_tick < cause_tick {
            Err(WorldAxiomViolation {
                axiom: "Causality",
                reason: "effect cannot occur before its cause",
            })
        } else {
            Ok(())
        }
    }

    /// 检查是否存在循环关系
    ///
    /// # 设计约束
    ///
    /// - 这是辅助方法，用于检测循环
    /// - 应用层可以覆盖此方法来实现自定义的循环检测逻辑
    fn has_cycle(facts: &[RelationFact]) -> bool {
        let mut graph: std::collections::HashMap<EntityId, Vec<EntityId>> = std::collections::HashMap::new();
        for fact in facts {
            graph.entry(fact.subject_id)
                .or_insert_with(Vec::new)
                .push(fact.object_id);
        }

        let mut visited = std::collections::HashSet::new();
        let mut recursion_stack = std::collections::HashSet::new();

        for &node in graph.keys() {
            if Self::has_cycle_dfs(node, &graph, &mut visited, &mut recursion_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycle_dfs(
        node: EntityId,
        graph: &std::collections::HashMap<EntityId, Vec<EntityId>>,
        visited: &mut std::collections::HashSet<EntityId>,
        recursion_stack: &mut std::collections::HashSet<EntityId>,
    ) -> bool {
        visited.insert(node);
        recursion_stack.insert(node);

        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if Self::has_cycle_dfs(neighbor, graph, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(&neighbor) {
                    return true;
                }
            }
        }

        recursion_stack.remove(&node);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biosphere_core::ExistentialRelationKind;

    #[test]
    fn test_time_irreversible_valid() {
        let result = WorldAxioms::assert_time_irreversible(0, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_time_irreversible_invalid() {
        let result = WorldAxioms::assert_time_irreversible(1, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "TimeIsIrreversible");
    }

    #[test]
    fn test_relation_integrity_self_reference() {
        let relation = RelationFact {
            subject_id: EntityId::new(1),
            object_id: EntityId::new(1),
            kind: ExistentialRelationKind::EmbodimentInField,
        };
        let config = AxiomConfig::default();
        let result = WorldAxioms::assert_relation_integrity(&relation, &[], &config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "RelationIntegrity");
    }

    #[test]
    fn test_relation_integrity_duplicate() {
        let relation = RelationFact {
            subject_id: EntityId::new(1),
            object_id: EntityId::new(2),
            kind: ExistentialRelationKind::EmbodimentInField,
        };
        let existing_facts = vec![relation.clone()];
        let config = AxiomConfig::default();
        let result = WorldAxioms::assert_relation_integrity(&relation, &existing_facts, &config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "RelationIntegrity");
    }

    #[test]
    fn test_relation_integrity_valid() {
        let relation = RelationFact {
            subject_id: EntityId::new(1),
            object_id: EntityId::new(2),
            kind: ExistentialRelationKind::EmbodimentInField,
        };
        let config = AxiomConfig::default();
        let result = WorldAxioms::assert_relation_integrity(&relation, &[], &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_relation_integrity_allow_self_reference() {
        let relation = RelationFact {
            subject_id: EntityId::new(1),
            object_id: EntityId::new(1),
            kind: ExistentialRelationKind::EmbodimentInField,
        };
        let config = AxiomConfig {
            allow_self_reference: true,
            ..Default::default()
        };
        let result = WorldAxioms::assert_relation_integrity(&relation, &[], &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_topology_invariant_no_cycle() {
        let facts = vec![
            RelationFact {
                subject_id: EntityId::new(1),
                object_id: EntityId::new(2),
                kind: ExistentialRelationKind::EmbodimentInField,
            },
            RelationFact {
                subject_id: EntityId::new(2),
                object_id: EntityId::new(3),
                kind: ExistentialRelationKind::EnvironmentInField,
            },
        ];
        let config = AxiomConfig::default();
        let result = WorldAxioms::assert_topology_invariant(&facts, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_topology_invariant_with_cycle() {
        let facts = vec![
            RelationFact {
                subject_id: EntityId::new(1),
                object_id: EntityId::new(2),
                kind: ExistentialRelationKind::EmbodimentInField,
            },
            RelationFact {
                subject_id: EntityId::new(2),
                object_id: EntityId::new(3),
                kind: ExistentialRelationKind::EnvironmentInField,
            },
            RelationFact {
                subject_id: EntityId::new(3),
                object_id: EntityId::new(1),
                kind: ExistentialRelationKind::EmbodimentInField,
            },
        ];
        let config = AxiomConfig::default();
        let result = WorldAxioms::assert_topology_invariant(&facts, &config);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "TopologyInvariant");
    }

    #[test]
    fn test_topology_invariant_allow_cycles() {
        let facts = vec![
            RelationFact {
                subject_id: EntityId::new(1),
                object_id: EntityId::new(2),
                kind: ExistentialRelationKind::EmbodimentInField,
            },
            RelationFact {
                subject_id: EntityId::new(2),
                object_id: EntityId::new(3),
                kind: ExistentialRelationKind::EnvironmentInField,
            },
            RelationFact {
                subject_id: EntityId::new(3),
                object_id: EntityId::new(1),
                kind: ExistentialRelationKind::EmbodimentInField,
            },
        ];
        let config = AxiomConfig {
            allow_cycles: true,
            ..Default::default()
        };
        let result = WorldAxioms::assert_topology_invariant(&facts, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_existence_uniqueness_valid() {
        let entity_id = EntityId::new(1);
        let existing_ids = vec![EntityId::new(2), EntityId::new(3)];
        let result = WorldAxioms::assert_existence_uniqueness(entity_id, &existing_ids);
        assert!(result.is_ok());
    }

    #[test]
    fn test_existence_uniqueness_duplicate() {
        let entity_id = EntityId::new(1);
        let existing_ids = vec![EntityId::new(1), EntityId::new(2)];
        let result = WorldAxioms::assert_existence_uniqueness(entity_id, &existing_ids);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "ExistenceUniqueness");
    }

    #[test]
    fn test_causality_valid() {
        let result = WorldAxioms::assert_causality(0, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_causality_invalid() {
        let result = WorldAxioms::assert_causality(1, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().axiom, "Causality");
    }
}
