# Topology 模块文档

## 概述

topology 模块提供了 Biosphere 拓扑结构的实现。StableTopology 是唯一的事实源，记录当前存在的所有关系事实。

### 核心概念

topology 模块的核心概念是：

1. **StableTopology（稳定拓扑）**：唯一的事实源，记录当前存在的所有关系事实
2. **RelationFact（关系事实）**：描述实体之间的关系

### 设计原则

- **唯一事实源**：系统中只有一个 StableTopology 实例
- **只读性**：外部代码只能读取，不能直接修改
- **不可伪造**：关系事实由应用层生成并注入
- **原子性**：拓扑更新是原子的（由应用层保证）
- **并发安全**：使用 Arc<RwLock> 支持并发访问

## 文件结构

```
topology/
└── stable_topology.rs    # 稳定拓扑实现
```

## 文件详解

### stable_topology.rs

**文件路径**：`src/topology/stable_topology.rs`

**说明**：实现了稳定拓扑，是唯一的事实源，记录当前存在的所有关系事实。

#### 主要类型

##### StableTopology

**定义**：
```rust
#[derive(Clone)]
pub struct StableTopology {
    facts: Arc<RwLock<Vec<RelationFact>>>,
}
```

**字段说明**：
- `facts: Arc<RwLock<Vec<RelationFact>>>`（私有）：关系事实列表

**实现方式**：
- 字段私有，确保封装性
- 使用 Arc<RwLock> 支持并发访问
- 实现了 `ExistentialTopology` trait

**公共方法**：

1. **`new()`** - 创建新的稳定拓扑
   ```rust
   pub fn new() -> Self {
       Self {
           facts: Arc::new(RwLock::new(Vec::new())),
       }
   }
   ```
   - 创建空的关系事实列表
   - 使用 Arc<RwLock> 包装
   - 返回新的拓扑实例

2. **`facts_snapshot()`** - 返回所有关系事实的快照
   ```rust
   pub fn facts_snapshot(&self) -> Vec<RelationFact> {
       let facts = self.facts.read().unwrap();
       facts.clone()
   }
   ```
   - 获取读锁
   - 克隆所有关系事实
   - 返回快照

3. **`facts_by_kind(kind: ExistentialRelationKind)`** - 查询特定关系种类的事实
   ```rust
   pub fn facts_by_kind(&self, kind: ExistentialRelationKind) -> Vec<RelationFact> {
       let facts = self.facts.read().unwrap();
       facts.iter().filter(|f| f.kind == kind).cloned().collect()
   }
   ```
   - 获取读锁
   - 过滤特定种类的关系事实
   - 返回匹配的事实列表

4. **`facts_by_subject(subject_id: EntityId)`** - 查询特定主体的关系事实
   ```rust
   pub fn facts_by_subject(&self, subject_id: EntityId) -> Vec<RelationFact> {
       let facts = self.facts.read().unwrap();
       facts.iter().filter(|f| f.subject_id == subject_id).cloned().collect()
   }
   ```
   - 获取读锁
   - 过滤特定主体的关系事实
   - 返回匹配的事实列表

5. **`facts_by_object(object_id: EntityId)`** - 查询特定客体的关系事实
   ```rust
   pub fn facts_by_object(&self, object_id: EntityId) -> Vec<RelationFact> {
       let facts = self.facts.read().unwrap();
       facts.iter().filter(|f| f.object_id == object_id).cloned().collect()
   }
   ```
   - 获取读锁
   - 过滤特定客体的关系事实
   - 返回匹配的事实列表

6. **`find_by_predicate<F>(predicate: F)`** - 根据谓词查找关系事实
   ```rust
   pub fn find_by_predicate<F>(&self, predicate: F) -> Vec<RelationFact>
   where
       F: Fn(&RelationFact) -> bool,
   {
       let facts = self.facts.read().unwrap();
       facts.iter().filter(|f| predicate(f)).cloned().collect()
   }
   ```
   - 获取读锁
   - 使用谓词函数过滤关系事实
   - 返回匹配的事实列表

7. **`filter_by_kinds(kinds: &[ExistentialRelationKind])`** - 按关系种类过滤关系事实
   ```rust
   pub fn filter_by_kinds(&self, kinds: &[ExistentialRelationKind]) -> Vec<RelationFact> {
       let facts = self.facts.read().unwrap();
       facts.iter()
           .filter(|f| kinds.contains(&f.kind))
           .cloned()
           .collect()
   }
   ```
   - 获取读锁
   - 过滤指定种类的关系事实
   - 返回匹配的事实列表

8. **`has_relation(subject_id, object_id, kind)`** - 检查是否存在某个关系
   ```rust
   pub fn has_relation(
       &self,
       subject_id: EntityId,
       object_id: EntityId,
       kind: ExistentialRelationKind,
   ) -> bool {
       let facts = self.facts.read().unwrap();
       facts.iter().any(|f| {
           f.subject_id == subject_id
               && f.object_id == object_id
               && f.kind == kind
       })
   }
   ```
   - 获取读锁
   - 检查是否存在指定的关系
   - 返回 true 表示存在，false 表示不存在

9. **`subjects()`** - 获取所有主体
   ```rust
   pub fn subjects(&self) -> Vec<EntityId> {
       let facts = self.facts.read().unwrap();
       facts.iter()
           .map(|f| f.subject_id)
           .collect::<std::collections::HashSet<_>>()
           .into_iter()
           .collect()
   }
   ```
   - 获取读锁
   - 提取所有主体 ID
   - 去重后返回

10. **`objects()`** - 获取所有客体
    ```rust
    pub fn objects(&self) -> Vec<EntityId> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .map(|f| f.object_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
    ```
    - 获取读锁
    - 提取所有客体 ID
    - 去重后返回

11. **`kinds()`** - 获取所有关系种类
    ```rust
    pub fn kinds(&self) -> Vec<ExistentialRelationKind> {
        let facts = self.facts.read().unwrap();
        facts.iter()
            .map(|f| f.kind)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
    ```
    - 获取读锁
    - 提取所有关系种类
    - 去重后返回

12. **`count()`** - 计算关系事实的数量
    ```rust
    pub fn count(&self) -> usize {
        let facts = self.facts.read().unwrap();
        facts.len()
    }
    ```
    - 获取读锁
    - 返回关系事实的数量

13. **`is_empty()`** - 检查拓扑是否为空
    ```rust
    pub fn is_empty(&self) -> bool {
        let facts = self.facts.read().unwrap();
        facts.is_empty()
    }
    ```
    - 获取读锁
    - 返回 true 表示为空，false 表示不为空

14. **`validate_consistency()`** - 验证拓扑一致性
    ```rust
    pub fn validate_consistency(&self) -> bool {
        let facts = self.facts.read().unwrap();
        
        let mut seen = std::collections::HashSet::new();
        for fact in facts.iter() {
            if !seen.insert(fact) {
                return false;
            }
        }
        
        true
    }
    ```
    - 获取读锁
    - 检查是否有重复的关系事实
    - 返回 true 表示一致，false 表示不一致

**Trait 实现**：

1. **ExistentialTopology trait**：
   ```rust
   impl ExistentialTopology for StableTopology {
       fn relations(&self) -> Vec<RelationFact> {
           let facts = self.facts.read().unwrap();
           facts.clone()
       }

       fn acknowledges(&self, relation: &RelationFact) -> bool {
           let facts = self.facts.read().unwrap();
           facts.iter().any(|f| f == relation)
       }
   }
   ```
   - `relations()`：返回所有关系事实
   - `acknowledges()`：检查是否承认某个关系

**优点**：
- 唯一的事实源
- 只读访问，确保数据一致性
- 支持并发访问
- 丰富的查询接口
- 一致性验证

## 设计优点

### 1. 唯一事实源

StableTopology 是系统中唯一的事实源，记录当前存在的所有关系事实：
```rust
pub struct StableTopology {
    facts: Arc<RwLock<Vec<RelationFact>>>,
}
```

### 2. 只读访问

外部代码只能读取，不能直接修改拓扑：
```rust
pub fn facts_snapshot(&self) -> Vec<RelationFact> {
    let facts = self.facts.read().unwrap();
    facts.clone()
}
```

### 3. 并发安全

使用 Arc<RwLock> 支持并发访问：
```rust
facts: Arc<RwLock<Vec<RelationFact>>>,
```

### 4. 丰富的查询接口

提供了多种查询方法：
- 按种类查询
- 按主体查询
- 按客体查询
- 按谓词查询
- 按多种种类过滤

### 5. 一致性验证

提供了拓扑一致性验证：
```rust
pub fn validate_consistency(&self) -> bool {
    let facts = self.facts.read().unwrap();
    
    let mut seen = std::collections::HashSet::new();
    for fact in facts.iter() {
        if !seen.insert(fact) {
            return false;
        }
    }
    
    true
}
```

### 6. 良好的封装性

所有字段都是私有的，通过公共方法提供只读访问。

### 7. 去重功能

subjects、objects 和 kinds 方法都提供了去重功能。

## 使用示例

### 创建拓扑

```rust
use biosphere_foundation::StableTopology;

// 创建新的稳定拓扑
let topology = StableTopology::new();

// 检查是否为空
assert!(topology.is_empty());
```

### 查询关系事实

```rust
use biosphere_foundation::StableTopology;
use biosphere_core::ExistentialRelationKind;

let topology = StableTopology::new();

// 获取所有关系事实
let facts = topology.facts_snapshot();

// 按种类查询
let embodiment_facts = topology.facts_by_kind(ExistentialRelationKind::EmbodimentInField);

// 按主体查询
let subject_facts = topology.facts_by_subject(entity_id);

// 按客体查询
let object_facts = topology.facts_by_object(entity_id);
```

### 检查关系

```rust
use biosphere_foundation::StableTopology;
use biosphere_core::ExistentialRelationKind;

let topology = StableTopology::new();

// 检查是否存在某个关系
let exists = topology.has_relation(
    subject_id,
    object_id,
    ExistentialRelationKind::EmbodimentInField,
);
```

### 验证一致性

```rust
use biosphere_foundation::StableTopology;

let topology = StableTopology::new();

// 验证拓扑一致性
let is_consistent = topology.validate_consistency();
assert!(is_consistent);
```

## 总结

topology 模块提供了 Biosphere 拓扑结构的实现。StableTopology 是唯一的事实源，记录当前存在的所有关系事实。

模块设计遵循以下原则：
- 唯一事实源：系统中只有一个 StableTopology 实例
- 只读性：外部代码只能读取，不能直接修改
- 不可伪造：关系事实由应用层生成并注入
- 原子性：拓扑更新是原子的（由应用层保证）
- 并发安全：使用 Arc<RwLock> 支持并发访问

StableTopology 提供了丰富的查询接口，包括按种类、主体、客体、谓词等多种查询方式。同时提供了一致性验证功能，确保拓扑的完整性。
