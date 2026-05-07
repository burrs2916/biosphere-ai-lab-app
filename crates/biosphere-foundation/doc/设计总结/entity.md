# Entity 模块文档

## 概述

entity 模块提供了 Biosphere 实体系统的实现。它定义了实体的抽象接口、实体管理器和实体过滤器。

### 核心概念

entity 模块的核心概念是：

1. **Entity（实体）**：世界中的一个存在
2. **EntityKind（实体种类）**：实体的种类
3. **EntityFilter（实体过滤器）**：实体的过滤条件
4. **EntityManager（实体管理器）**：管理世界中的所有实体
5. **EntityQuery（实体查询）**：实体查询 trait
6. **EntityExistence（实体存在记录）**：实体在时间轴上的存在范围
7. **EntityIdentitySpace（实体身份空间）**：负责管理实体身份和 ID 生成
8. **WorldEntityState（世界实体状态）**：负责管理特定时间点的实体集合
9. **EntityQueryModel（实体查询模型）**：扩展 Filter，添加时间范围和查询上下文

### 设计原则

- **值对象**：不包含任何世界引用
- **不可变**：一旦创建就不可修改
- **类型绑定**：每个实体都与唯一的类型绑定
- **组合过滤**：支持多个过滤条件的组合
- **类型安全**：所有类型都是类型安全的
- **时间感知**：支持时间范围内的查询
- **身份分离**：实体身份管理与世界状态管理分离
- **查询抽象**：支持复杂的查询场景

## 文件结构

```
entity/
├── mod.rs          # 模块入口，导出所有公共类型
├── entity.rs       # 实体定义
├── filter.rs       # 实体过滤器
├── manager.rs      # 实体管理器
├── query.rs        # 实体查询 trait
├── existence.rs    # 实体存在记录
├── identity.rs     # 实体身份空间
├── state.rs       # 世界实体状态
└── query_model.rs # 实体查询模型
```

## 文件详解

### mod.rs

**文件路径**：`src/entity/mod.rs`

**说明**：entity 模块的入口文件，导出所有公共类型。

**导出内容**：
- `Entity`：实体
- `EntityKind`：实体种类
- `EntityFilter`：实体过滤器
- `EntityManager`：实体管理器
- `EntityQuery`：实体查询 trait
- `EntityExistence`：实体存在记录
- `EntityIdentitySpace`：实体身份空间
- `WorldEntityState`：世界实体状态
- `EntityQueryModel`：实体查询模型
- `TimeRange`：时间范围
- `QueryContext`：查询上下文

**实现方式**：
```rust
pub mod entity;
pub mod filter;
pub mod manager;
pub mod query;
pub mod existence;
pub mod identity;
pub mod state;
pub mod query_model;

pub use entity::{Entity, EntityKind};
pub use filter::EntityFilter;
pub use manager::EntityManager;
pub use query::EntityQuery;
pub use existence::EntityExistence;
pub use identity::EntityIdentitySpace;
pub use state::WorldEntityState;
pub use query_model::{EntityQuery as EntityQueryModel, TimeRange, QueryContext};

impl EntityQuery for EntityManager {
    fn get(&self, id: biosphere_core::EntityId) -> Option<&Entity> {
        manager::EntityManager::get(self, id)
    }

    fn query(&self, filter: EntityFilter) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::query(self, filter)
    }

    fn all_ids(&self) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::all_ids(self)
    }

    fn len(&self) -> usize {
        manager::EntityManager::len(self)
    }

    fn is_empty(&self) -> bool {
        manager::EntityManager::is_empty(self)
    }
}
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 自动实现 EntityQuery trait

**实现方式**：
```rust
pub mod entity;
pub mod filter;
pub mod manager;
pub mod query;

pub use entity::{Entity, EntityKind};
pub use filter::EntityFilter;
pub use manager::EntityManager;
pub use query::EntityQuery;

impl EntityQuery for EntityManager {
    fn get(&self, id: biosphere_core::EntityId) -> Option<&Entity> {
        manager::EntityManager::get(self, id)
    }

    fn query(&self, filter: EntityFilter) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::query(self, filter)
    }

    fn all_ids(&self) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::all_ids(self)
    }

    fn len(&self) -> usize {
        manager::EntityManager::len(self)
    }

    fn is_empty(&self) -> bool {
        manager::EntityManager::is_empty(self)
    }
}
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 自动实现 EntityQuery trait

---

### entity.rs

**文件路径**：`src/entity/entity.rs`

**说明**：定义了 Entity 和 EntityKind。

#### 主要类型

##### EntityKind

**定义**：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Default,
    Unknown,
    Any,
}
```

**字段说明**：
- `Default`：默认实体
- `Unknown`：未知实体
- `Any`：任意实体（用于过滤）

**实现方式**：
- 枚举类型，提供基础的实体分类
- 实现了 Copy、Clone、PartialEq、Eq、Hash trait

**Default 实现**：
```rust
impl Default for EntityKind {
    fn default() -> Self {
        EntityKind::Default
    }
}
```

**优点**：
- 基础抽象
- 可扩展
- 类型安全

---

##### Entity

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct Entity {
    id: EntityId,
    kind: EntityKind,
}
```

**字段说明**：
- `id: EntityId`（私有）：实体 ID
- `kind: EntityKind`（私有）：实体种类

**实现方式**：
- 字段私有，确保封装性
- 值对象，不包含任何世界引用
- 不可变，一旦创建就不可修改

**公共方法**：

1. **`new(id: EntityId, kind: EntityKind)`** - 创建新实体
   ```rust
   pub fn new(id: EntityId, kind: EntityKind) -> Self {
       Self { id, kind }
   }
   ```
   - 接受实体 ID 和种类作为参数
   - 返回新的实体

2. **`id()`** - 获取实体 ID
   ```rust
   pub fn id(&self) -> EntityId {
       self.id
   }
   ```
   - 返回实体 ID

3. **`kind()`** - 获取实体种类
   ```rust
   pub fn kind(&self) -> EntityKind {
       self.kind
   }
   ```
   - 返回实体种类

**优点**：
- 值对象设计
- 不可变性
- 良好的封装性
- 类型绑定

---

### filter.rs

**文件路径**：`src/entity/filter.rs`

**说明**：定义了 EntityFilter，实体的过滤条件。

#### 主要类型

##### EntityFilter

**定义**：
```rust
#[derive(Debug, Clone)]
pub enum EntityFilter {
    All,
    Id(EntityId),
    Kind(EntityKind),
    Ids(Vec<EntityId>),
    Kinds(Vec<EntityKind>),
    And(Vec<EntityFilter>),
    Or(Vec<EntityFilter>),
    Not(Box<EntityFilter>),
}
```

**字段说明**：
- `All`：匹配所有实体
- `Id(EntityId)`：匹配指定 ID 的实体
- `Kind(EntityKind)`：匹配指定种类的实体
- `Ids(Vec<EntityId>)`：匹配多个 ID 的实体
- `Kinds(Vec<EntityKind>)`：匹配多个种类的实体
- `And(Vec<EntityFilter>)`：逻辑与，同时满足多个过滤条件
- `Or(Vec<EntityFilter>)`：逻辑或，满足任一过滤条件
- `Not(Box<EntityFilter>)`：逻辑非，不满足过滤条件

**实现方式**：
- 枚举类型，支持多种过滤条件
- 支持逻辑组合（与、或、非）

**公共方法**：

1. **`id(id: EntityId)`** - 创建 ID 过滤器
   ```rust
   pub fn id(id: EntityId) -> Self {
       EntityFilter::Id(id)
   }
   ```
   - 接受实体 ID 作为参数
   - 返回 ID 过滤器

2. **`kind(kind: EntityKind)`** - 创建种类过滤器
   ```rust
   pub fn kind(kind: EntityKind) -> Self {
       EntityFilter::Kind(kind)
   }
   ```
   - 接受实体种类作为参数
   - 返回种类过滤器

3. **`ids(ids: Vec<EntityId>)`** - 创建多个 ID 过滤器
   ```rust
   pub fn ids(ids: Vec<EntityId>) -> Self {
       EntityFilter::Ids(ids)
   }
   ```
   - 接受实体 ID 列表作为参数
   - 返回多个 ID 过滤器

4. **`kinds(kinds: Vec<EntityKind>)`** - 创建多个种类过滤器
   ```rust
   pub fn kinds(kinds: Vec<EntityKind>) -> Self {
       EntityFilter::Kinds(kinds)
   }
   ```
   - 接受实体种类列表作为参数
   - 返回多个种类过滤器

5. **`and(filters: Vec<EntityFilter>)`** - 创建逻辑与过滤器
   ```rust
   pub fn and(filters: Vec<EntityFilter>) -> Self {
       EntityFilter::And(filters)
   }
   ```
   - 接受过滤器列表作为参数
   - 返回逻辑与过滤器

6. **`or(filters: Vec<EntityFilter>)`** - 创建逻辑或过滤器
   ```rust
   pub fn or(filters: Vec<EntityFilter>) -> Self {
       EntityFilter::Or(filters)
   }
   ```
   - 接受过滤器列表作为参数
   - 返回逻辑或过滤器

7. **`not(filter: EntityFilter)`** - 创建逻辑非过滤器
   ```rust
   pub fn not(filter: EntityFilter) -> Self {
       EntityFilter::Not(Box::new(filter))
   }
   ```
   - 接受过滤器作为参数
   - 返回逻辑非过滤器

**Default 实现**：
```rust
impl Default for EntityFilter {
    fn default() -> Self {
        EntityFilter::All
    }
}
```

**优点**：
- 组合过滤
- 类型安全
- 可扩展

---

### manager.rs

**文件路径**：`src/entity/manager.rs`

**说明**：实现了 EntityManager，管理世界中的所有实体。

#### 主要类型

##### EntityManager

**定义**：
```rust
#[derive(Debug, Default)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: u64,
}
```

**字段说明**：
- `entities: HashMap<EntityId, Entity>`（私有）：实体映射
- `next_id: u64`（私有）：下一个实体 ID

**实现方式**：
- 字段私有，确保封装性
- 使用 HashMap 存储实体
- 自动生成实体 ID

**公共方法**：

1. **`new()`** - 创建新的实体管理器
   ```rust
   pub fn new() -> Self {
       Self::default()
   }
   ```
   - 创建空的管理器

2. **`create(kind: EntityKind)`** - 创建新实体
   ```rust
   pub fn create(&mut self, kind: EntityKind) -> EntityId {
       let id = EntityId::new(self.next_id);
       self.next_id += 1;

       let entity = Entity::new(id, kind);
       self.entities.insert(id, entity);

       id
   }
   ```
   - 接受实体种类作为参数
   - 自动生成实体 ID
   - 返回新创建的实体 ID

3. **`delete(id: EntityId)`** - 删除实体
   ```rust
   pub fn delete(&mut self, id: EntityId) -> Result<(), String> {
       if self.entities.remove(&id).is_some() {
           Ok(())
       } else {
           Err(format!("Entity {:?} not found", id))
       }
   }
   ```
   - 接受实体 ID 作为参数
   - 如果实体存在，返回 Ok(())
   - 如果实体不存在，返回 Err

4. **`get(id: EntityId)`** - 获取实体
   ```rust
   pub fn get(&self, id: EntityId) -> Option<&Entity> {
       self.entities.get(&id)
   }
   ```
   - 接受实体 ID 作为参数
   - 返回实体的引用

5. **`query(filter: EntityFilter)`** - 查询实体
   ```rust
   pub fn query(&self, filter: EntityFilter) -> Vec<EntityId> {
       self.entities
           .iter()
           .filter(|(_, entity)| self.matches_filter(entity, &filter))
           .map(|(id, _)| *id)
           .collect()
   }
   ```
   - 接受实体过滤器作为参数
   - 返回匹配过滤器的实体 ID 列表

6. **`all_ids()`** - 获取所有实体 ID
   ```rust
   pub fn all_ids(&self) -> Vec<EntityId> {
       self.entities.keys().copied().collect()
   }
   ```
   - 返回所有实体的 ID 列表

7. **`len()`** - 获取实体数量
   ```rust
   pub fn len(&self) -> usize {
       self.entities.len()
   }
   ```
   - 返回实体的数量

8. **`is_empty()`** - 检查是否为空
   ```rust
   pub fn is_empty(&self) -> bool {
       self.entities.is_empty()
   }
   ```
   - 如果没有实体，返回 true

**私有方法**：

1. **`matches_filter(entity: &Entity, filter: &EntityFilter)`** - 检查实体是否匹配过滤器
   ```rust
   fn matches_filter(&self, entity: &Entity, filter: &EntityFilter) -> bool {
       match filter {
           EntityFilter::All => true,
           EntityFilter::Id(id) => entity.id() == *id,
           EntityFilter::Kind(kind) => entity.kind() == *kind,
           EntityFilter::Ids(ids) => ids.contains(&entity.id()),
           EntityFilter::Kinds(kinds) => kinds.contains(&entity.kind()),
           EntityFilter::And(filters) => filters.iter().all(|f| self.matches_filter(entity, f)),
           EntityFilter::Or(filters) => filters.iter().any(|f| self.matches_filter(entity, f)),
           EntityFilter::Not(inner) => !self.matches_filter(entity, inner),
       }
   }
   ```
   - 接受实体和过滤器作为参数
   - 如果实体匹配过滤器，返回 true

**优点**：
- 实体创建：只能创建新实体，不能修改已有实体
- 实体删除：可以删除实体
- 实体查询：支持多种查询方式
- 实体过滤：支持复杂的过滤条件

---

### query.rs

**文件路径**：`src/entity/query.rs`

**说明**：定义了 EntityQuery trait，实体查询接口。

#### 主要 Trait

##### EntityQuery

**定义**：
```rust
pub trait EntityQuery {
    fn get(&self, id: biosphere_core::EntityId) -> Option<&Entity>;
    fn query(&self, filter: EntityFilter) -> Vec<biosphere_core::EntityId>;
    fn all_ids(&self) -> Vec<biosphere_core::EntityId>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}
```

**方法**：

1. **`get(id: biosphere_core::EntityId)`** - 获取实体
   ```rust
   fn get(&self, id: biosphere_core::EntityId) -> Option<&Entity>;
   ```

2. **`query(filter: EntityFilter)`** - 查询实体
   ```rust
   fn query(&self, filter: EntityFilter) -> Vec<biosphere_core::EntityId>;
   ```

3. **`all_ids()`** - 获取所有实体 ID
   ```rust
   fn all_ids(&self) -> Vec<biosphere_core::EntityId>;
   ```

4. **`len()`** - 获取实体数量
   ```rust
   fn len(&self) -> usize;
   ```

5. **`is_empty()`** - 检查是否为空
   ```rust
   fn is_empty(&self) -> bool;
   ```

**优点**：
- 清晰的抽象接口
- 只读访问
- 不可变性

## 设计优点

### 1. 值对象

Entity 是值对象，不包含任何世界引用：
```rust
#[derive(Debug, Clone)]
pub struct Entity {
    id: EntityId,
    kind: EntityKind,
}
```

### 2. 不可变性

Entity 一旦创建就不可修改：
- 字段私有
- 只提供 getter 方法
- 不提供 setter 方法

### 3. 类型绑定

每个实体都与唯一的类型绑定：
```rust
pub struct Entity {
    id: EntityId,
    kind: EntityKind,
}
```

### 4. 组合过滤

EntityFilter 支持多个过滤条件的组合：
```rust
pub enum EntityFilter {
    All,
    Id(EntityId),
    Kind(EntityKind),
    Ids(Vec<EntityId>),
    Kinds(Vec<EntityKind>),
    And(Vec<EntityFilter>),
    Or(Vec<EntityFilter>),
    Not(Box<EntityFilter>),
}
```

### 5. 类型安全

所有类型都是类型安全的：
- EntityKind 是枚举类型
- EntityFilter 是枚举类型
- 使用强类型而不是字符串

### 6. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问。

### 7. 自动 ID 生成

EntityManager 自动生成实体 ID：
```rust
pub fn create(&mut self, kind: EntityKind) -> EntityId {
    let id = EntityId::new(self.next_id);
    self.next_id += 1;

    let entity = Entity::new(id, kind);
    self.entities.insert(id, entity);

    id
}
```

## 使用示例

### 创建实体

```rust
use biosphere_foundation::entity::EntityManager;
use biosphere_foundation::entity::entity::EntityKind;

let mut manager = EntityManager::new();
let id = manager.create(EntityKind::Default);
```

### 查询实体

```rust
use biosphere_foundation::entity::EntityManager;
use biosphere_foundation::EntityFilter;

let mut manager = EntityManager::new();
let id = manager.create(EntityKind::Default);

let entity = manager.get(id);
```

### 使用过滤器

```rust
use biosphere_foundation::entity::EntityManager;
use biosphere_foundation::EntityFilter;

let mut manager = EntityManager::new();
let id1 = manager.create(EntityKind::Default);
let id2 = manager.create(EntityKind::Default);

let results = manager.query(EntityFilter::kind(EntityKind::Default));
assert_eq!(results.len(), 2);
```

### 组合过滤器

```rust
use biosphere_foundation::entity::EntityManager;
use biosphere_foundation::EntityFilter;

let mut manager = EntityManager::new();
let id = manager.create(EntityKind::Default);

let results = manager.query(EntityFilter::and(vec![
    EntityFilter::id(id),
    EntityFilter::kind(EntityKind::Default),
]));
```

## 总结

entity 模块提供了 Biosphere 实体系统的实现。它定义了实体的抽象接口、实体管理器和实体过滤器。

模块设计遵循以下原则：
- 值对象：不包含任何世界引用
- 不可变：一旦创建就不可修改
- 类型绑定：每个实体都与唯一的类型绑定
- 组合过滤：支持多个过滤条件的组合
- 类型安全：所有类型都是类型安全的

所有类型都提供了良好的封装性，通过公共方法提供只读访问。EntityFilter 支持复杂的过滤条件，EntityManager 提供了完整的实体管理功能。

---

### existence.rs

**文件路径**：`src/entity/existence.rs`

**说明**：定义了 EntityExistence，实体在时间轴上的存在范围。

#### 主要类型

##### EntityExistence

**定义**：
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityExistence {
    entity_id: EntityId,
    born_at: u64,
    died_at: Option<u64>,
}
```

**字段说明**：
- `entity_id: EntityId`（私有）：实体 ID
- `born_at: u64`（私有）：出生时间
- `died_at: Option<u64>`（私有）：死亡时间（None 表示仍然存活）

**实现方式**：
- 字段私有，确保封装性
- 支持时间范围内的查询
- 支持生命周期管理

**公共方法**：

1. **`new(entity_id: EntityId, born_at: u64)`** - 创建新的实体存在记录
   ```rust
   pub fn new(entity_id: EntityId, born_at: u64) -> Self {
       Self {
           entity_id,
           born_at,
           died_at: None,
       }
   }
   ```
   - 接受实体 ID 和出生时间作为参数
   - 返回新的实体存在记录

2. **`entity_id()`** - 获取实体 ID
   ```rust
   pub fn entity_id(&self) -> EntityId {
       self.entity_id
   }
   ```
   - 返回实体 ID

3. **`born_at()`** - 获取出生时间
   ```rust
   pub fn born_at(&self) -> u64 {
       self.born_at
   }
   ```
   - 返回出生时间

4. **`died_at()`** - 获取死亡时间
   ```rust
   pub fn died_at(&self) -> Option<u64> {
       self.died_at
   }
   ```
   - 返回死亡时间

5. **`die(at: u64)`** - 标记实体死亡
   ```rust
   pub fn die(&mut self, at: u64) {
       self.died_at = Some(at);
   }
   ```
   - 接受死亡时间作为参数
   - 标记实体死亡

6. **`is_alive_at(tick: u64)`** - 检查实体在指定时间是否存活
   ```rust
   pub fn is_alive_at(&self, tick: u64) -> bool {
       tick >= self.born_at && self.died_at.map_or(true, |death| tick < death)
   }
   ```
   - 接受时间刻作为参数
   - 如果实体在指定时间存活，返回 true

7. **`lifespan()`** - 获取存在时间范围
   ```rust
   pub fn lifespan(&self) -> (u64, Option<u64>) {
       (self.born_at, self.died_at)
   }
   ```
   - 返回存在时间范围

**优点**：
- 时间感知：支持时间范围内的查询
- 生命周期管理：跟踪实体的生命周期
- 类型安全：所有操作都是类型安全的

---

### identity.rs

**文件路径**：`src/entity/identity.rs`

**说明**：定义了 EntityIdentitySpace，负责管理实体身份和 ID 生成。

#### 主要类型

##### EntityIdentitySpace

**定义**：
```rust
#[derive(Debug, Default)]
pub struct EntityIdentitySpace {
    next_id: u64,
}
```

**字段说明**：
- `next_id: u64`（私有）：下一个实体 ID

**实现方式**：
- 字段私有，确保封装性
- 顺序生成 ID
- 支持多个世界共享

**公共方法**：

1. **`new()`** - 创建新的实体身份空间
   ```rust
   pub fn new() -> Self {
       Self::default()
   }
   ```
   - 创建空的身份空间

2. **`generate_id()`** - 生成新的实体 ID
   ```rust
   pub fn generate_id(&mut self) -> EntityId {
       let id = EntityId::new(self.next_id);
       self.next_id += 1;
       id
   }
   ```
   - 生成新的实体 ID
   - 返回新的实体 ID

3. **`next_id()`** - 获取下一个将要生成的 ID
   ```rust
   pub fn next_id(&self) -> u64 {
       self.next_id
   }
   ```
   - 返回下一个将要生成的 ID

**优点**：
- ID 唯一性：确保每个实体 ID 都是唯一的
- 顺序生成：ID 按顺序生成，便于调试和排序
- 无状态：不存储实体状态，只管理身份

---

### state.rs

**文件路径**：`src/entity/state.rs`

**说明**：定义了 WorldEntityState，负责管理特定时间点的实体集合。

#### 主要类型

##### WorldEntityState

**定义**：
```rust
#[derive(Debug, Default)]
pub struct WorldEntityState {
    entities: HashMap<EntityId, Entity>,
    existences: HashMap<EntityId, EntityExistence>,
}
```

**字段说明**：
- `entities: HashMap<EntityId, Entity>`（私有）：当前存活的实体
- `existences: HashMap<EntityId, EntityExistence>`（私有）：实体存在记录

**实现方式**：
- 字段私有，确保封装性
- 支持时间范围内的实体查询
- 支持生命周期管理

**公共方法**：

1. **`new()`** - 创建新的世界实体状态
   ```rust
   pub fn new() -> Self {
       Self::default()
   }
   ```
   - 创建空的世界实体状态

2. **`create_entity(entity: Entity, at: u64)`** - 创建新实体
   ```rust
   pub fn create_entity(&mut self, entity: Entity, at: u64) -> EntityId {
       let id = entity.id();
       let existence = EntityExistence::new(id, at);
       
       self.entities.insert(id, entity);
       self.existences.insert(id, existence);
       
       id
   }
   ```
   - 接受实体和创建时间作为参数
   - 创建实体并记录存在信息

3. **`delete_entity(id: EntityId, at: u64)`** - 删除实体
   ```rust
   pub fn delete_entity(&mut self, id: EntityId, at: u64) -> Result<(), String> {
       if let Some(existence) = self.existences.get_mut(&id) {
           existence.die(at);
           self.entities.remove(&id);
           Ok(())
       } else {
           Err(format!("Entity {:?} not found", id))
       }
   }
   ```
   - 接受实体 ID 和删除时间作为参数
   - 标记实体死亡并从当前实体集合中移除

4. **`get_entity(id: EntityId)`** - 获取实体
   ```rust
   pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
       self.entities.get(&id)
   }
   ```
   - 接受实体 ID 作为参数
   - 返回实体的引用

5. **`get_existence(id: EntityId)`** - 获取实体存在记录
   ```rust
   pub fn get_existence(&self, id: EntityId) -> Option<&EntityExistence> {
       self.existences.get(&id)
   }
   ```
   - 接受实体 ID 作为参数
   - 返回实体存在记录的引用

6. **`alive_entities_at(tick: u64)`** - 获取在指定时间存活的实体 ID
   ```rust
   pub fn alive_entities_at(&self, tick: u64) -> Vec<EntityId> {
       self.existences
           .iter()
           .filter(|(_, existence)| existence.is_alive_at(tick))
           .map(|(id, _)| *id)
           .collect()
   }
   ```
   - 接受时间刻作为参数
   - 返回在指定时间存活的实体 ID 列表

7. **`all_entity_ids()`** - 获取所有实体 ID
   ```rust
   pub fn all_entity_ids(&self) -> Vec<EntityId> {
       self.entities.keys().copied().collect()
   }
   ```
   - 返回所有实体 ID 列表

8. **`len()`** - 获取实体数量
   ```rust
   pub fn len(&self) -> usize {
       self.entities.len()
   }
   ```
   - 返回实体数量

9. **`is_empty()`** - 检查是否为空
   ```rust
   pub fn is_empty(&self) -> bool {
       self.entities.is_empty()
   }
   ```
   - 如果没有实体，返回 true

**优点**：
- 时间感知：支持时间范围内的实体查询
- 状态管理：管理实体的当前状态
- 存在记录：跟踪实体的生命周期
- 可查询：支持多种查询方式

---

### query_model.rs

**文件路径**：`src/entity/query_model.rs`

**说明**：定义了 EntityQuery、TimeRange 和 QueryContext，扩展 Filter，添加时间范围和查询上下文。

#### 主要类型

##### EntityQuery

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct EntityQuery {
    filter: EntityFilter,
    time_range: Option<TimeRange>,
    context: QueryContext,
}
```

**字段说明**：
- `filter: EntityFilter`（私有）：过滤条件
- `time_range: Option<TimeRange>`（私有）：时间范围
- `context: QueryContext`（私有）：查询上下文

**实现方式**：
- 字段私有，确保封装性
- 支持时间范围内的查询
- 支持不同的查询上下文

**公共方法**：

1. **`immediate(filter: EntityFilter)`** - 创建即时查询
   ```rust
   pub fn immediate(filter: EntityFilter) -> Self {
       Self {
           filter,
           time_range: None,
           context: QueryContext::Immediate,
       }
   }
   ```
   - 接受过滤条件作为参数
   - 返回即时查询

2. **`time_range(filter: EntityFilter, start: u64, end: u64)`** - 创建时间范围查询
   ```rust
   pub fn time_range(filter: EntityFilter, start: u64, end: u64) -> Self {
       Self {
           filter,
           time_range: Some(TimeRange::new(start, end)),
           context: QueryContext::Historical,
       }
   }
   ```
   - 接受过滤条件、开始时间和结束时间作为参数
   - 返回时间范围查询

3. **`subscription(filter: EntityFilter)`** - 创建订阅查询
   ```rust
   pub fn subscription(filter: EntityFilter) -> Self {
       Self {
           filter,
           time_range: None,
           context: QueryContext::Subscription,
       }
   }
   ```
   - 接受过滤条件作为参数
   - 返回订阅查询

4. **`with_time_range(time_range: TimeRange)`** - 设置时间范围
   ```rust
   pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
       self.time_range = Some(time_range);
       self
   }
   ```
   - 接受时间范围作为参数
   - 返回修改后的查询

5. **`with_context(context: QueryContext)`** - 设置查询上下文
   ```rust
   pub fn with_context(mut self, context: QueryContext) -> Self {
       self.context = context;
       self
   }
   ```
   - 接受查询上下文作为参数
   - 返回修改后的查询

**优点**：
- 时间感知：支持时间范围内的查询
- 上下文感知：支持不同的查询上下文
- 可组合：支持多个查询条件的组合
- 类型安全：所有查询都是类型安全的

##### TimeRange

**定义**：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    start: u64,
    end: u64,
}
```

**字段说明**：
- `start: u64`（私有）：开始时间
- `end: u64`（私有）：结束时间

**公共方法**：

1. **`new(start: u64, end: u64)`** - 创建新的时间范围
   ```rust
   pub fn new(start: u64, end: u64) -> Self {
       assert!(start <= end, "Start time must be less than or equal to end time");
       Self { start, end }
   }
   ```
   - 接受开始时间和结束时间作为参数
   - 返回新的时间范围

2. **`contains(time: u64)`** - 检查时间是否在范围内
   ```rust
   pub fn contains(&self, time: u64) -> bool {
       time >= self.start && time <= self.end
   }
   ```
   - 接受时间作为参数
   - 如果时间在范围内，返回 true

3. **`duration()`** - 获取时间范围的持续时间
   ```rust
   pub fn duration(&self) -> u64 {
       self.end - self.start
   }
   ```
   - 返回时间范围的持续时间

**优点**：
- 包含性：时间范围包含开始和结束时间
- 有效性：开始时间不大于结束时间
- 可选性：时间范围可以是可选的

##### QueryContext

**定义**：
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryContext {
    Immediate,
    Subscription,
    Historical,
}
```

**字段说明**：
- `Immediate`：即时查询
- `Subscription`：持续订阅
- `Historical`：历史查询

**优点**：
- 语义明确：每个上下文都有明确的语义
- 可扩展：支持添加新的查询上下文
- 类型安全：所有上下文都是类型安全的

## 总结

entity 模块提供了 Biosphere 实体系统的实现。它定义了实体的抽象接口、实体管理器和实体过滤器。

模块设计遵循以下原则：
- 值对象：不包含任何世界引用
- 不可变：一旦创建就不可修改
- 类型绑定：每个实体都与唯一的类型绑定
- 组合过滤：支持多个过滤条件的组合
- 类型安全：所有类型都是类型安全的

所有类型都提供了良好的封装性，通过公共方法提供只读访问。EntityFilter 支持复杂的过滤条件，EntityManager 提供了完整的实体管理功能。
