# World 模块文档

## 概述

world 模块提供了 Biosphere 世界的核心实现。它是第一个可运行的世界实现，是"被时间驱动、受公理约束的变化载体"。

### 核心概念

world 模块的核心概念是：

1. **World（世界）**：所有存在的容器，被时间驱动的存在过程
2. **WorldClock（世界时钟）**：管理世界的时间推进，不可逆
3. **WorldRules（世界规则）**：定义世界的约束，验证迁移的有效性

### 设计原则

- **时间驱动**：世界的演化只能通过时间推进触发
- **公理约束**：世界的变化必须遵守世界公理
- **单向信息流**：世界只提供条件，不观察外部
- **最小化实现**：只包含世界存在的最小必要条件
- **不可变性**：世界状态一旦记录就不可修改

## 文件结构

```
world/
├── mod.rs              # 模块入口，导出所有公共类型
├── basic_world.rs      # 基础世界实现
├── world_clock.rs      # 世界时钟
└── world_rules.rs      # 世界规则
```

## 文件详解

### mod.rs

**文件路径**：`src/world/mod.rs`

**说明**：world 模块的入口文件，导出所有公共类型。

**导出内容**：
- `BasicWorld`：基础世界
- `WorldClock`：世界时钟
- `WorldRules`：世界规则

**实现方式**：
```rust
pub mod basic_world;
pub mod world_clock;
pub mod world_rules;

pub use basic_world::BasicWorld;
pub use world_clock::WorldClock;
pub use world_rules::WorldRules;
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### basic_world.rs

**文件路径**：`src/world/basic_world.rs`

**说明**：实现了第一个可运行的世界实现。它是"被时间驱动、受公理约束的变化载体"。

#### 主要类型

##### BasicWorld

**定义**：
```rust
#[derive(Debug)]
pub struct BasicWorld {
    environment: BasicEnvironment,
    state_store: StateStore,
    relation_store: RelationStore,
}
```

**字段说明**：
- `environment: BasicEnvironment`（私有）：内部环境实现，负责时间管理和条件生成
- `state_store: StateStore`（私有）：世界状态存储器（时间记忆）
- `relation_store: RelationStore`（私有）：世界关系存储器（时间记忆）

**实现方式**：
- 所有字段都是私有的，确保封装性
- 组合了环境、状态存储和关系存储
- 实现了 `Environment`、`TemporalEnvironment`、`StateProvider`、`StateQuery` 和 `RelationQuery` trait

**公共方法**：

1. **`new()`** - 创建一个新世界
   ```rust
   pub fn new() -> Self {
       let environment = BasicEnvironment::new();
       let state_store = StateStore::new();
       let relation_store = RelationStore::new();

       let mut world = Self {
           environment,
           state_store,
           relation_store,
       };

       // 记录初始状态（tick 0）
       world.commit_state(0);

       world
   }
   ```
   - 初始化环境、状态存储和关系存储
   - 记录初始状态（tick 0）
   - 返回新的世界实例

2. **`step_world()`** - 推进一步世界
   ```rust
   pub fn step_world(&mut self) {
       self.environment.advance();
       let new_tick = self.environment.current_tick();
       self.commit_state(new_tick);
   }
   ```
   - 推进内部环境的时间
   - 更新条件快照
   - 记录世界状态（时间之后）
   - 这是世界唯一允许的演化入口

3. **`environment()`** - 获取当前环境状态
   ```rust
   pub fn environment(&self) -> &BasicEnvironment {
       &self.environment
   }
   ```
   - 返回对内部环境的不可变引用
   - 只读访问，不修改环境状态

4. **`latest_tick()`** - 获取最新的时间刻
   ```rust
   pub fn latest_tick(&self) -> u64 {
       self.environment.current_tick()
   }
   ```
   - 返回世界最新的时间刻
   - 只读访问，不修改状态

**私有方法**：

1. **`commit_state(tick: u64)`** - 记录世界状态
   ```rust
   fn commit_state(&mut self, tick: u64) {
       let world_state = WorldState {
           tick_seen: tick,
       };

       let snapshot = StateSnapshot::new(
           tick,
           StatePayload::new(world_state),
       );

       self.state_store.commit(snapshot);
   }
   ```
   - 在时间推进之后记录世界状态快照
   - 状态只记录"结果"，不记录"过程"
   - 状态永远在时间推进之后产生

**Trait 实现**：

1. **Environment trait**：
   ```rust
   impl Environment for BasicWorld {
       type State = crate::ontology::BasicEnvironmentState;
       type Conditions = crate::conditions::SensedConditions;

       fn step(&mut self) {
           let _ = self.step_world();
       }

       fn conditions(&self) -> &Self::Conditions {
           self.environment.conditions()
       }
   }
   ```
   - `step()`：推进世界，忽略任何错误
   - `conditions()`：返回当前条件的不可变引用

2. **TemporalEnvironment trait**：
   ```rust
   impl TemporalEnvironment for BasicWorld {
       fn advance(&mut self) {
           self.environment.advance();
       }
   }
   ```
   - `advance()`：推进内部环境的时间

3. **StateProvider trait**：
   ```rust
   impl StateProvider for BasicWorld {
       fn current_state(&self) -> Option<&StateSnapshot> {
           self.state_store.history().latest()
       }

       fn state_history(&self) -> &StateHistory {
           self.state_store.history()
       }
   }
   ```
   - `current_state()`：返回当前状态
   - `state_history()`：返回状态历史

4. **StateQuery trait**：
   ```rust
   impl StateQuery for BasicWorld {
       fn get_at(&self, tick: u64) -> Option<&StateSnapshot> {
           self.state_store.history().get_at(tick)
       }

       fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot> {
           self.state_store.history().query_range(start, end)
       }

       fn latest_snapshot(&self) -> Option<&StateSnapshot> {
           self.state_store.history().latest()
       }
   }
   ```
   - `get_at()`：查询特定时间刻的状态
   - `query_range()`：查询时间范围内的状态
   - `latest_snapshot()`：获取最新状态快照

5. **RelationQuery trait**：
   ```rust
   impl RelationQuery for BasicWorld {
       fn get_relation_at(&self, tick: u64) -> Option<&RelationChange> {
           self.relation_store.history().get_at(tick)
       }

       fn query_relations_range(&self, start: u64, end: u64) -> Vec<&RelationChange> {
           self.relation_store.history()
               .iter()
               .filter(|c| c.tick() >= start && c.tick() <= end)
               .collect()
       }

       fn latest_relation_change(&self) -> Option<&RelationChange> {
           self.relation_store.history().latest()
       }
   }
   ```
   - `get_relation_at()`：查询特定时间刻的关系变化
   - `query_relations_range()`：查询时间范围内的关系变化
   - `latest_relation_change()`：获取最新关系变化

**优点**：
- 清晰的世界抽象
- 良好的封装性
- 实现了多个核心 trait
- 支持状态和关系查询
- 时间驱动的演化机制

---

##### WorldState

**定义**：
```rust
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorldState {
    tick_seen: u64,
}
```

**字段说明**：
- `tick_seen: u64`：已经看到的时间刻

**实现方式**：
- 私有结构体，只在 BasicWorld 内部使用
- 记录世界看到的时间刻

**优点**：
- 简单的状态表示
- 封装在 BasicWorld 内部

---

### world_clock.rs

**文件路径**：`src/world/world_clock.rs`

**说明**：实现了世界时钟，描述世界的时间推进，是不可逆的。

#### 主要类型

##### WorldClock

**定义**：
```rust
#[derive(Debug, PartialEq, Eq)]
pub struct WorldClock {
    tick: u64,
}
```

**字段说明**：
- `tick: u64`（私有）：当前时间刻

**实现方式**：
- 字段私有，确保封装性
- 时间只能向前推进，不能倒流
- 时间戳永远递增

**公共方法**：

1. **`new()`** - 创建新的世界时钟
   ```rust
   pub fn new() -> Self {
       Self { tick: 0 }
   }
   ```
   - 从 0 开始
   - 世界不存在"带历史的出生"

2. **`current_tick()`** - 获取当前时间刻
   ```rust
   pub fn current_tick(&self) -> u64 {
       self.tick
   }
   ```
   - 返回当前的时间刻
   - 只读访问，不修改状态

3. **`advance()`** - 推进世界时间
   ```rust
   pub fn advance(&mut self) -> Result<u64, WorldAxiomViolation> {
       let next_tick = self.tick + 1;

       WorldAxioms::assert_time_irreversible(self.tick, next_tick)?;

       self.tick = next_tick;
       Ok(self.tick)
   }
   ```
   - 计算下一个时间刻
   - 通过 WorldAxioms 验证时间不可逆
   - 更新时间刻
   - 返回新的时间刻
   - 这是唯一允许推进时间的方式

**优点**：
- 不可逆的时间推进
- 单调递增的时间戳
- 原子性的时间推进
- 通过公理验证

---

### world_rules.rs

**文件路径**：`src/world/world_rules.rs`

**说明**：实现了世界规则，描述世界的约束，调用公理来验证迁移的有效性。

#### 主要类型

##### WorldRules

**定义**：
```rust
#[derive(Clone)]
pub struct WorldRules<T>
where
    T: ExistentialTopology,
{
    _phantom: std::marker::PhantomData<T>,
}
```

**类型参数**：
- `T: ExistentialTopology`：存在拓扑类型

**字段说明**：
- `_phantom: PhantomData<T>`：类型标记，不占用内存

**实现方式**：
- 使用 PhantomData 作为类型标记
- 泛型参数支持不同的拓扑类型
- 零成本抽象

**公共方法**：

1. **`new()`** - 创建新的世界规则
   ```rust
   pub fn new() -> Self {
       Self {
           _phantom: std::marker::PhantomData,
       }
   }
   ```
   - 返回新的规则实例
   - 不需要任何参数

2. **`validate(current: &T, target: &T)`** - 验证迁移是否有效
   ```rust
   pub fn validate(&self, _current: &T, _target: &T) -> bool {
       true
   }
   ```
   - 接受当前拓扑和目标拓扑作为参数
   - 返回 true 表示迁移有效，false 表示无效
   - 这是基础实现，总是返回 true
   - 应用层可以覆盖此方法来实现自定义规则

**优点**：
- 灵活的规则验证
- 支持泛型拓扑类型
- 零成本抽象
- 应用层可以自定义规则

**注意**：
- 这是基础实现，总是返回 true
- 应用层应该覆盖此方法来实现具体的规则逻辑
- Foundation 层不包含具体的规则逻辑

---

## 设计优点

### 1. 清晰的世界抽象

BasicWorld 提供了清晰的世界抽象：
- 世界 = 被时间驱动的存在过程
- 世界不解释自己
- 世界只做一件事：在合法的时间点，承载一次变化
- 世界提供条件，但不"观察"

### 2. 时间驱动的演化

世界的演化只能通过时间推进触发：
```rust
pub fn step_world(&mut self) {
    self.environment.advance();
    let new_tick = self.environment.current_tick();
    self.commit_state(new_tick);
}
```

### 3. 不可逆的时间

WorldClock 确保时间不可逆：
```rust
pub fn advance(&mut self) -> Result<u64, WorldAxiomViolation> {
    let next_tick = self.tick + 1;
    WorldAxioms::assert_time_irreversible(self.tick, next_tick)?;
    self.tick = next_tick;
    Ok(self.tick)
}
```

### 4. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问：
```rust
pub struct BasicWorld {
    environment: BasicEnvironment,  // 私有字段
    state_store: StateStore,       // 私有字段
    relation_store: RelationStore, // 私有字段
}
```

### 5. 多 Trait 实现

BasicWorld 实现了多个核心 trait：
- `Environment`：环境 trait
- `TemporalEnvironment`：时间环境 trait
- `StateProvider`：状态提供者 trait
- `StateQuery`：状态查询 trait
- `RelationQuery`：关系查询 trait

### 6. 灵活的规则验证

WorldRules 支持灵活的规则验证：
```rust
pub struct WorldRules<T>
where
    T: ExistentialTopology,
{
    _phantom: std::marker::PhantomData<T>,
}
```

### 7. 状态和关系查询

BasicWorld 支持状态和关系查询：
- 查询特定时间刻的状态
- 查询时间范围内的状态
- 查询最新状态
- 查询特定时间刻的关系变化
- 查询时间范围内的关系变化
- 查询最新关系变化

### 8. 零成本抽象

使用 PhantomData 等技术实现零成本抽象：
```rust
pub struct WorldRules<T>
where
    T: ExistentialTopology,
{
    _phantom: std::marker::PhantomData<T>,  // 不占用内存
}
```

## 使用示例

### 创建世界

```rust
use biosphere_foundation::BasicWorld;
use biosphere_core::Environment;

// 创建一个新世界
let mut world = BasicWorld::new();

// 推进世界
world.step();

// 获取当前条件
let conditions = world.conditions();
```

### 查询状态

```rust
use biosphere_foundation::BasicWorld;
use biosphere_foundation::temporal::StateQuery;

let mut world = BasicWorld::new();

// 推进世界
for _ in 0..10 {
    world.step_world();
}

// 查询特定时间刻的状态
let snapshot = world.get_at(5);

// 查询时间范围
let snapshots = world.query_range(0, 10);

// 获取最新状态
let latest = world.latest_snapshot();
```

### 使用世界时钟

```rust
use biosphere_foundation::WorldClock;

let mut clock = WorldClock::new();

// 获取当前时间刻
let tick = clock.current_tick();

// 推进时间
let new_tick = clock.advance().unwrap();
```

### 使用世界规则

```rust
use biosphere_foundation::WorldRules;
use biosphere_core::ExistentialTopology;

let rules = WorldRules::<MyTopology>::new();

// 验证迁移
let is_valid = rules.validate(&current_topology, &target_topology);
```

## 总结

world 模块提供了 Biosphere 世界的核心实现。它是第一个可运行的世界实现，是"被时间驱动、受公理约束的变化载体"。

模块设计遵循以下原则：
- 时间驱动：世界的演化只能通过时间推进触发
- 公理约束：世界的变化必须遵守世界公理
- 单向信息流：世界只提供条件，不观察外部
- 最小化实现：只包含世界存在的最小必要条件
- 不可变性：世界状态一旦记录就不可修改

所有类型都提供了良好的封装性，通过公共方法提供只读访问。BasicWorld 实现了多个核心 trait，支持状态和关系查询。WorldClock 确保时间不可逆，WorldRules 支持灵活的规则验证。
