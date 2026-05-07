# Invariants 模块文档

## 概述

invariants 模块提供了 Biosphere 不变量约束的实现。WorldAxioms 定义了任何世界都不能违反的存在论约束，它们不是规则、不是策略，而是"存在成立的前提"。

### 核心概念

invariants 模块的核心概念是：

1. **WorldAxioms（世界公理）**：不可破坏的基础公理集合
2. **AxiomConfig（公理配置）**：公理的配置选项
3. **WorldAxiomViolation（世界公理违反错误）**：公理违反的错误类型

### 设计原则

- **基础公理**：定义最基本的存在论约束
- **不可违反**：任何 World / Executor / Clock 都必须遵守
- **可配置**：支持灵活的公理配置
- **可扩展**：应用层可以扩展公理实现

## 文件结构

```
invariants/
└── world_axioms.rs    # 世界公理实现
```

## 文件详解

### world_axioms.rs

**文件路径**：`src/invariants/world_axioms.rs`

**说明**：实现了世界公理，定义了任何世界都不能违反的存在论约束。

#### 主要类型

##### WorldAxiomViolation

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct WorldAxiomViolation {
    pub axiom: &'static str,
    pub reason: &'static str,
}
```

**字段说明**：
- `axiom: &'static str`：违反的公理名称
- `reason: &'static str`：违反原因

**实现方式**：
- 公共字段，便于访问
- 实现了 `Display` 和 `Error` trait

**优点**：
- 清晰的错误信息
- 便于调试和错误处理

---

##### AxiomConfig

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct AxiomConfig {
    pub allow_self_reference: bool,
    pub allow_cycles: bool,
}
```

**字段说明**：
- `allow_self_reference: bool`：是否允许自指关系
- `allow_cycles: bool`：是否允许循环关系

**实现方式**：
- 公共字段，便于配置
- 提供默认实现

**默认实现**：
```rust
impl Default for AxiomConfig {
    fn default() -> Self {
        Self {
            allow_self_reference: false,
            allow_cycles: false,
        }
    }
}
```

**优点**：
- 灵活的公理配置
- 合理的默认值
- 应用层可以自定义配置

---

##### WorldAxioms

**定义**：
```rust
pub struct WorldAxioms;
```

**实现方式**：
- 不可实例化的结构体
- 只包含静态方法

**公共方法**：

1. **`assert_time_irreversible(previous_tick: u64, next_tick: u64)`** - 公理一：世界不可回滚
   ```rust
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
   ```
   - 检查时间是否倒流
   - 如果时间倒流，返回错误
   - 这是不可违反的基础公理，不支持配置

2. **`assert_relation_integrity(relation: &RelationFact, existing_facts: &[RelationFact], config: &AxiomConfig)`** - 公理三：关系完整性
   ```rust
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
   ```
   - 检查关系是否自指（除非配置允许）
   - 检查关系是否重复
   - 支持配置：通过 AxiomConfig 控制是否允许自指

3. **`assert_topology_invariant(facts: &[RelationFact], config: &AxiomConfig)`** - 公理四：拓扑不变性
   ```rust
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
   ```
   - 检查拓扑是否包含循环（除非配置允许）
   - 支持配置：通过 AxiomConfig 控制是否允许循环

4. **`assert_existence_uniqueness(entity_id: EntityId, existing_ids: &[EntityId])`** - 公理五：存在唯一性
   ```rust
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
   ```
   - 检查实体 ID 是否唯一
   - 如果 ID 已存在，返回错误
   - 这是不可违反的基础公理，不支持配置

5. **`assert_causality(cause_tick: u64, effect_tick: u64)`** - 公理六：因果关系
   ```rust
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
   ```
   - 检查因果关系的时间顺序
   - 如果结果发生在原因之前，返回错误
   - 这是不可违反的基础公理，不支持配置

**私有方法**：

1. **`has_cycle(facts: &[RelationFact])`** - 检查是否存在循环关系
   ```rust
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
   ```
   - 构建邻接表表示的有向图
   - 使用深度优先搜索检测循环
   - 返回 true 表示存在循环

2. **`has_cycle_dfs(node, graph, visited, recursion_stack)`** - 深度优先搜索检测循环
   ```rust
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
   ```
   - 递归访问邻接节点
   - 使用递归栈检测回边
   - 返回 true 表示存在循环

## 设计优点

### 1. 基础公理

WorldAxioms 定义了最基本的存在论约束：
```rust
pub struct WorldAxioms;
```

### 2. 不可违反

任何 World / Executor / Clock 都必须遵守：
```rust
pub fn assert_time_irreversible(
    previous_tick: u64,
    next_tick: u64,
) -> Result<(), WorldAxiomViolation>
```

### 3. 可配置

支持灵活的公理配置：
```rust
pub struct AxiomConfig {
    pub allow_self_reference: bool,
    pub allow_cycles: bool,
}
```

### 4. 清晰的错误信息

提供了清晰的错误信息：
```rust
#[derive(Debug, Clone)]
pub struct WorldAxiomViolation {
    pub axiom: &'static str,
    pub reason: &'static str,
}
```

### 5. 合理的默认值

提供了合理的默认配置：
```rust
impl Default for AxiomConfig {
    fn default() -> Self {
        Self {
            allow_self_reference: false,
            allow_cycles: false,
        }
    }
}
```

### 6. 循环检测

实现了高效的循环检测算法：
```rust
fn has_cycle(facts: &[RelationFact]) -> bool {
    // 构建邻接表
    // 使用深度优先搜索检测循环
}
```

### 7. 类型安全

充分利用 Rust 的类型系统，使用强类型而不是字符串。

## 使用示例

### 验证时间不可逆

```rust
use biosphere_foundation::invariants::WorldAxioms;

// 验证时间不可逆
let result = WorldAxioms::assert_time_irreversible(0, 1);
assert!(result.is_ok());

let result = WorldAxioms::assert_time_irreversible(1, 0);
assert!(result.is_err());
```

### 验证关系完整性

```rust
use biosphere_foundation::invariants::{WorldAxioms, AxiomConfig};
use biosphere_core::{RelationFact, ExistentialRelationKind, EntityId};

let relation = RelationFact {
    subject_id: EntityId::new(1),
    object_id: EntityId::new(2),
    kind: ExistentialRelationKind::EmbodimentInField,
};
let config = AxiomConfig::default();

// 验证关系完整性
let result = WorldAxioms::assert_relation_integrity(&relation, &[], &config);
assert!(result.is_ok());
```

### 验证拓扑不变性

```rust
use biosphere_foundation::invariants::{WorldAxioms, AxiomConfig};

let facts = vec![
    // 关系事实
];
let config = AxiomConfig::default();

// 验证拓扑不变性
let result = WorldAxioms::assert_topology_invariant(&facts, &config);
assert!(result.is_ok());
```

### 自定义公理配置

```rust
use biosphere_foundation::invariants::AxiomConfig;

// 允许自指关系和循环
let config = AxiomConfig {
    allow_self_reference: true,
    allow_cycles: true,
};
```

## 总结

invariants 模块提供了 Biosphere 不变量约束的实现。WorldAxioms 定义了任何世界都不能违反的存在论约束，它们不是规则、不是策略，而是"存在成立的前提"。

模块设计遵循以下原则：
- 基础公理：定义最基本的存在论约束
- 不可违反：任何 World / Executor / Clock 都必须遵守
- 可配置：支持灵活的公理配置
- 可扩展：应用层可以扩展公理实现

WorldAxioms 提供了多个公理验证方法，包括时间不可逆、关系完整性、拓扑不变性、存在唯一性和因果关系。AxiomConfig 允许应用层自定义公理配置，WorldAxiomViolation 提供了清晰的错误信息。
