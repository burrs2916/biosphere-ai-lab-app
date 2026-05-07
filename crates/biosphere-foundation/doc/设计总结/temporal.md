# Temporal 模块文档

## 概述

temporal 模块提供了 Biosphere 时间系统的实现。它是世界的"时间记忆"，记录世界的状态变化和关系变化。

### 核心概念

temporal 模块的核心概念是：

1. **StateSnapshot（状态快照）**：世界在某一时间点的状态快照
2. **StatePayload（状态载荷）**：状态内容（中立容器）
3. **StateHistory（状态历史）**：状态历史记录
4. **StateStore（状态存储器）**：状态存储器（世界侧）
5. **StateProvider（状态提供者）**：状态提供者 trait
6. **StateQuery（状态查询）**：状态查询 trait
7. **RelationChange（关系变化）**：关系变化
8. **RelationHistory（关系历史）**：关系历史记录
9. **存储器RelationStore（关系）**：关系存储器
10. **RelationQuery（关系查询）**：关系查询 trait
11. **LazyQueryIterator（惰性查询迭代器）**：惰性查询迭代器
12. **WindowedQuery（窗口化查询）**：窗口化查询

### 设计原则

- **追加接口**：只提供 commit 接口，不提供 set_state / update_state / replace 接口
- **不可变性**：状态一旦记录就不可修改
- **中立容器**：Foundation 不理解 payload 的含义
- **只读访问**：只提供查询接口，不提供修改接口
- **惰性求值**：只在需要时才计算结果
- **窗口化**：只查询指定窗口内的数据

## 文件结构

```
temporal/
├── mod.rs                    # 模块入口，导出所有公共类型
├── query.rs                  # 查询工具
├── state/                    # 状态子模块
│   ├── mod.rs                # 状态子模块入口
│   ├── snapshot.rs           # 状态快照
│   ├── payload.rs            # 状态载荷
│   ├── history.rs            # 状态历史
│   ├── store.rs              # 状态存储器
│   ├── provider.rs           # 状态提供者
│   └── query.rs              # 状态查询
└── relations/                # 关系子模块
    ├── mod.rs                # 关系子模块入口
    ├── change.rs             # 关系变化
    ├── history.rs            # 关系历史
    ├── store.rs              # 关系存储器
    └── query.rs              # 关系查询
```

## 文件详解

### mod.rs

**文件路径**：`src/temporal/mod.rs`

**说明**：temporal 模块的入口文件，导出所有公共类型。

**导出内容**：
- `StateSnapshot`：状态快照
- `StatePayload`：状态载荷
- `StateHistory`：状态历史
- `StateStore`：状态存储器
- `StateProvider`：状态提供者 trait
- `StateQuery`：状态查询 trait
- `RelationChange`：关系变化
- `RelationChangeKind`：关系变化种类
- `RelationHistory`：关系历史
- `RelationQuery`：关系查询 trait
- `RelationStore`：关系存储器
- `LazyQueryIterator`：惰性查询迭代器
- `LazyRelationQueryIterator`：惰性关系查询迭代器
- `WindowedQuery`：窗口化查询
- `WindowedRelationQuery`：窗口化关系查询

**实现方式**：
```rust
pub mod state;
pub mod relations;
pub mod query;

pub use state::{
    StateSnapshot,
    StatePayload,
    StateHistory,
    StateStore,
    StateProvider,
    StateQuery,
};
pub use relations::{
    RelationChange,
    RelationChangeKind,
    RelationHistory,
    RelationQuery,
    RelationStore,
};
pub use query::{
    LazyQueryIterator,
    LazyRelationQueryIterator,
    WindowedQuery,
    WindowedRelationQuery,
};
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### state/snapshot.rs

**文件路径**：`src/temporal/state/snapshot.rs`

**说明**：实现了状态快照，世界在某一时间点的状态快照。

#### 主要类型

##### StateSnapshot

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    tick: u64,
    payload: StatePayload,
}
```

**字段说明**：
- `tick: u64`（私有）：世界时间戳
- `payload: StatePayload`（私有）：状态载荷

**实现方式**：
- 字段私有，确保封装性
- 值对象，不包含任何世界引用
- 不可变，一旦创建就不可修改
- 时间绑定，每个快照都与唯一的时间刻绑定

**公共方法**：

1. **`new(tick: u64, payload: StatePayload)`** - 创建新的状态快照
   ```rust
   pub fn new(tick: u64, payload: StatePayload) -> Self {
       Self { tick, payload }
   }
   ```
   - 接受时间戳和状态载荷作为参数
   - 返回新的状态快照

2. **`tick()`** - 获取时间戳
   ```rust
   pub fn tick(&self) -> u64 {
       self.tick
   }
   ```
   - 返回世界时间戳
   - 只读访问，不修改状态

3. **`payload()`** - 获取状态载荷
   ```rust
   pub fn payload(&self) -> &StatePayload {
       &self.payload
   }
   ```
   - 返回状态载荷的引用
   - 只读访问，不修改状态

**优点**：
- 值对象设计
- 不可变性
- 良好的封装性
- 时间绑定

---

### state/payload.rs

**文件路径**：`src/temporal/state/payload.rs`

**说明**：实现了状态载荷，状态内容（中立容器）。

#### 主要类型

##### StatePayload

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct StatePayload {
    inner: Arc<dyn Any + Send + Sync>,
}
```

**字段说明**：
- `inner: Arc<dyn Any + Send + Sync>`（私有）：状态内容

**实现方式**：
- 字段私有，确保封装性
- 使用 Arc 包装，支持共享
- 使用 Any + Send + Sync 保证类型安全

**公共方法**：

1. **`new<T: Any + Send + Sync>(value: T)`** - 创建新的状态载荷
   ```rust
   pub fn new<T: Any + Send + Sync>(value: T) -> Self {
       Self {
           inner: Arc::new(value),
       }
   }
   ```
   - 接受任意类型的状态值
   - 使用 Arc 包装
   - 返回新的状态载荷

2. **`downcast_ref<T: Any>()`** - 尝试将载荷转换为指定类型的引用
   ```rust
   pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
       self.inner.downcast_ref::<T>()
   }
   ```
   - 如果载荷的类型与 T 匹配，返回 Some(&T)
   - 否则返回 None

**优点**：
- 中立容器，Foundation 不理解 payload 的含义
- 类型安全，使用 Any + Send + Sync 保证
- 不可构造，外部代码无法构造 StatePayload（只能通过 new）
- 只读访问，只提供 downcast_ref，不提供 downcast_mut

---

### state/history.rs

**文件路径**：`src/temporal/state/history.rs`

**说明**：实现了状态历史记录，记录世界的状态变化。

#### 主要类型

##### StateHistory

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct StateHistory {
    snapshots: Vec<StateSnapshot>,
}
```

**字段说明**：
- `snapshots: Vec<StateSnapshot>`（私有）：状态快照列表

**实现方式**：
- 字段私有，确保封装性
- 使用 Vec 存储状态快照

**公共方法**：

1. **`new()`** - 创建新的状态历史
   ```rust
   pub fn new() -> Self {
       Self {
           snapshots: Vec::new(),
       }
   }
   ```
   - 创建空的状态历史

2. **`record(snapshot: StateSnapshot)`** - 记录状态快照
   ```rust
   pub fn record(&mut self, snapshot: StateSnapshot) {
       self.snapshots.push(snapshot);
   }
   ```
   - 追加状态快照
   - 只能追加，不能修改或删除

3. **`get_at(tick: u64)`** - 获取特定时间刻的状态
   ```rust
   pub fn get_at(&self, tick: u64) -> Option<&StateSnapshot> {
       self.snapshots.iter().find(|s| s.tick() == tick)
   }
   ```
   - 查找时间戳匹配的状态快照
   - 返回 Option<&StateSnapshot>

4. **`latest()`** - 获取最新的状态
   ```rust
   pub fn latest(&self) -> Option<&StateSnapshot> {
       self.snapshots.last()
   }
   ```
   - 返回最后一个状态快照

5. **`query_range(start: u64, end: u64)`** - 查询时间范围内的状态
   ```rust
   pub fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot> {
       self.snapshots.iter()
           .filter(|s| s.tick() >= start && s.tick() <= end)
           .collect()
   }
   ```
   - 过滤时间范围内的状态快照
   - 返回匹配的快照列表

6. **`len()`** - 获取状态数量
   ```rust
   pub fn len(&self) -> usize {
       self.snapshots.len()
   }
   ```
   - 返回状态快照的数量

7. **`is_empty()`** - 检查是否为空
   ```rust
   pub fn is_empty(&self) -> bool {
       self.snapshots.is_empty()
   }
   ```
   - 如果没有状态快照，返回 true

**优点**：
- 追加接口
- 不可变性
- 丰富的查询接口
- 良好的封装性

---

### state/store.rs

**文件路径**：`src/temporal/state/store.rs`

**说明**：实现了状态存储器，只有 WorldRuntime 可以持有。

#### 主要类型

##### StateStore

**定义**：
```rust
#[derive(Debug)]
pub struct StateStore {
    history: StateHistory,
}
```

**字段说明**：
- `history: StateHistory`（私有）：状态历史

**实现方式**：
- 字段私有，确保封装性
- 世界侧，只有 WorldRuntime 可以持有

**公共方法**：

1. **`new()`** - 创建新的状态存储器
   ```rust
   pub fn new() -> Self {
       Self {
           history: StateHistory::new(),
       }
   }
   ```
   - 创建的状态历史为空

2. **`commit(snapshot: StateSnapshot)`** - 提交状态快照
   ```rust
   pub fn commit(&mut self, snapshot: StateSnapshot) {
       self.history.record(snapshot);
   }
   ```
   - 追加状态快照
   - 这是唯一允许添加状态的方式

3. **`history()`** - 获取历史记录
   ```rust
   pub fn history(&self) -> &StateHistory {
       &self.history
   }
   ```
   - 返回历史记录的引用
   - 只读访问，不修改状态

**Trait 实现**：

1. **StateQuery trait**：
   ```rust
   impl StateQuery for StateStore {
       fn get_at(&self, tick: u64) -> Option<&StateSnapshot> {
           self.history.get_at(tick)
       }

       fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot> {
           self.history.query_range(start, end)
       }

       fn latest_snapshot(&self) -> Option<&StateSnapshot> {
           self.history.latest()
       }
   }
   ```
   - 实现了 StateQuery trait
   - 提供状态查询接口

**优点**：
- 追加接口
- 不可变性
- 世界侧安全
- 实现了 StateQuery trait

---

### state/provider.rs

**文件路径**：`src/temporal/state/provider.rs`

**说明**：定义了状态提供者 trait。

#### 主要 Trait

##### StateProvider

**定义**：
```rust
pub trait StateProvider {
    fn current_state(&self) -> Option<&StateSnapshot>;
    fn state_history(&self) -> &StateHistory;
}
```

**方法**：

1. **`current_state()`** - 返回当前状态
   ```rust
   fn current_state(&self) -> Option<&StateSnapshot>;
   ```

2. **`state_history()`** - 返回状态历史
   ```rust
   fn state_history(&self) -> &StateHistory;
   ```

**优点**：
- 清晰的抽象接口
- 只读访问
- 不可变性

---

### state/query.rs

**文件路径**：`src/temporal/state/query.rs`

**说明**：定义了状态查询 trait。

#### 主要 Trait

##### StateQuery

**定义**：
```rust
pub trait StateQuery {
    fn get_at(&self, tick: u64) -> Option<&StateSnapshot>;
    fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot>;
    fn latest_snapshot(&self) -> Option<&StateSnapshot>;
}
```

**方法**：

1. **`get_at(tick: u64)`** - 查询特定时间刻的状态
   ```rust
   fn get_at(&self, tick: u64) -> Option<&StateSnapshot>;
   ```

2. **`query_range(start: u64, end: u64)`** - 查询时间范围内的状态
   ```rust
   fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot>;
   ```

3. **`latest_snapshot()`** - 获取最新状态快照
   ```rust
   fn latest_snapshot(&self) -> Option<&StateSnapshot>;
   ```

**优点**：
- 清晰的抽象接口
- 只读访问
- 不可变性

---

### relations/change.rs

**文件路径**：`src/temporal/relations/change.rs`

**说明**：实现了关系变化，记录实体之间的关系变化。

#### 主要类型

##### RelationChangeKind

**定义**：
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationChangeKind {
    Added,
    Removed,
}
```

**字段说明**：
- `Added`：关系添加
- `Removed`：关系移除

**优点**：
- 清晰的类型定义
- 不可变性

---

##### RelationChange

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct RelationChange {
    tick: u64,
    kind: RelationChangeKind,
    fact: RelationFact,
}
```

**字段说明**：
- `tick: u64`（私有）：时间刻
- `kind: RelationChangeKind`（私有）：变化种类
- `fact: RelationFact`（私有）：关系事实

**实现方式**：
- 字段私有，确保封装性
- 不可变，一旦创建就不可修改

**公共方法**：

1. **`new(tick: u64, kind: RelationChangeKind, fact: RelationFact)`** - 创建新的关系变化
   ```rust
   pub fn new(tick: u64, kind: RelationChangeKind, fact: RelationFact) -> Self {
       Self { tick, kind, fact }
   }
   ```
   - 接受时间刻、变化种类和关系事实作为参数
   - 返回新的关系变化

2. **`tick()`** - 获取时间刻
   ```rust
   pub fn tick(&self) -> u64 {
       self.tick
   }
   ```
   - 返回时间刻

3. **`kind()`** - 获取变化种类
   ```rust
   pub fn kind(&self) -> &RelationChangeKind {
       &self.kind
   }
   ```
   - 返回变化种类

4. **`fact()`** - 获取关系事实
   ```rust
   pub fn fact(&self) -> &RelationFact {
       &self.fact
   }
   ```
   - 返回关系事实

**优点**：
- 清晰的类型定义
- 不可变性
- 良好的封装性

---

### relations/history.rs

**文件路径**：`src/temporal/relations/history.rs`

**说明**：实现了关系历史记录，记录实体之间的关系变化历史。

#### 主要类型

##### RelationHistory

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct RelationHistory {
    changes: Vec<RelationChange>,
}
```

**字段说明**：
- `changes: Vec<RelationChange>`（私有）：关系变化列表

**实现方式**：
- 字段私有，确保封装性
- 使用 Vec 存储关系变化

**公共方法**：

1. **`new()`** - 创建新的关系历史
   ```rust
   pub fn new() -> Self {
       Self {
           changes: Vec::new(),
       }
   }
   ```
   - 创建空的关系历史

2. **`record(change: RelationChange)`** - 记录关系变化
   ```rust
   pub fn record(&mut self, change: RelationChange) {
       self.changes.push(change);
   }
   ```
   - 追加关系变化
   - 只能追加，不能修改或删除

3. **`get_at(tick: u64)`** - 获取特定时间刻的关系变化
   ```rust
   pub fn get_at(&self, tick: u64) -> Option<&RelationChange> {
       self.changes.iter().find(|c| c.tick() == tick)
   }
   ```
   - 查找时间戳匹配的关系变化
   - 返回 Option<&RelationChange>

4. **`latest()`** - 获取最新的关系变化
   ```rust
   pub fn latest(&self) -> Option<&RelationChange> {
       self.changes.last()
   }
   ```
   - 返回最后一个关系变化

5. **`iter()`** - 获取迭代器
   ```rust
   pub fn iter(&self) -> impl Iterator<Item = &RelationChange> {
       self.changes.iter()
   }
   ```
   - 返回关系变化的迭代器

**优点**：
- 追加接口
- 不可变性
- 良好的封装性

---

### relations/store.rs

**文件路径**：`src/temporal/relations/store.rs`

**说明**：实现了关系存储器。

#### 主要类型

##### RelationStore

**定义**：
```rust
#[derive(Debug)]
pub struct RelationStore {
    history: RelationHistory,
}
```

**字段说明**：
- `history: RelationHistory`（私有）：关系历史

**实现方式**：
- 字段私有，确保封装性

**公共方法**：

1. **`new()`** - 创建新的关系存储器
   ```rust
   pub fn new() -> Self {
       Self {
           history: RelationHistory::new(),
       }
   }
   ```
   - 创建空的关系历史

2. **`commit(change: RelationChange)`** - 提交关系变化
   ```rust
   pub fn commit(&mut self, change: RelationChange) {
       self.history.record(change);
   }
   ```
   - 追加关系变化
   - 这是唯一允许添加关系变化的方式

3. **`history()`** - 获取历史记录
   ```rust
   pub fn history(&self) -> &RelationHistory {
       &self.history
   }
   ```
   - 返回历史记录的引用

**Trait 实现**：

1. **RelationQuery trait**：
   ```rust
   impl RelationQuery for RelationStore {
       fn get_relation_at(&self, tick: u64) -> Option<&RelationChange> {
           self.history.get_at(tick)
       }

       fn query_relations_range(&self, start: u64, end: u64) -> Vec<&RelationChange> {
           self.history.iter()
               .filter(|c| c.tick() >= start && c.tick() <= end)
               .collect()
       }

       fn latest_relation_change(&self) -> Option<&RelationChange> {
           self.history.latest()
       }
   }
   ```
   - 实现了 RelationQuery trait
   - 提供关系查询接口

**优点**：
- 追加接口
- 不可变性
- 实现了 RelationQuery trait

---

### relations/query.rs

**文件路径**：`src/temporal/relations/query.rs`

**说明**：定义了关系查询 trait。

#### 主要 Trait

##### RelationQuery

**定义**：
```rust
pub trait RelationQuery {
    fn get_relation_at(&self, tick: u64) -> Option<&RelationChange>;
    fn query_relations_range(&self, start: u64, end: u64) -> Vec<&RelationChange>;
    fn latest_relation_change(&self) -> Option<&RelationChange>;
}
```

**方法**：

1. **`get_relation_at(tick: u64)`** - 查询特定时间刻的关系变化
   ```rust
   fn get_relation_at(&self, tick: u64) -> Option<&RelationChange>;
   ```

2. **`query_relations_range(start: u64, end: u64)`** - 查询时间范围内的关系变化
   ```rust
   fn query_relations_range(&self, start: u64, end: u64) -> Vec<&RelationChange>;
   ```

3. **`latest_relation_change()`** - 获取最新关系变化
   ```rust
   fn latest_relation_change(&self) -> Option<&RelationChange>;
   ```

**优点**：
- 清晰的抽象接口
- 只读访问
- 不可变性

---

### query.rs

**文件路径**：`src/temporal/query.rs`

**说明**：提供了查询工具，包括惰性查询迭代器和窗口化查询。

#### 主要类型

##### LazyQueryIterator

**定义**：
```rust
pub struct LazyQueryIterator<'a, P: StateQuery> {
    query: &'a P,
    start: u64,
    end: u64,
}
```

**类型参数**：
- `P: StateQuery`：必须实现 StateQuery trait

**实现方式**：
- 惰性求值，只在需要时才计算结果
- 零分配，不预先分配内存

**公共方法**：

1. **`new(query: &'a P, start: u64, end: u64)`** - 创建新的惰性查询迭代器
   ```rust
   pub fn new(query: &'a P, start: u64, end: u64) -> Self {
       Self { query, start, end }
   }
   ```
   - 接受查询对象、起始时间刻和结束时间刻作为参数
   - 返回新的惰性查询迭代器

2. **`filter_range(self, start: u64, end: u64)`** - 过滤时间范围
   ```rust
   pub fn filter_range(self, start: u64, end: u64) -> Self {
       Self {
           start: self.start.max(start),
           end: self.end.min(end),
           ..self
       }
   }
   ```
   - 过滤时间范围
   - 返回新的惰性查询迭代器

**Iterator 实现**：
```rust
impl<'a, P: StateQuery> Iterator for LazyQueryIterator<'a, P> {
    type Item = &'a StateSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let snapshot = self.query.get_at(self.start)?;
        self.start += 1;
        Some(snapshot)
    }
}
```

**优点**：
- 惰性求值
- 零分配
- 链式操作
- 性能优化

---

##### LazyRelationQueryIterator

**定义**：
```rust
pub struct LazyRelationQueryIterator<'a, P: RelationQuery> {
    query: &'a P,
    start: u64,
    end: u64,
}
```

**类型参数**：
- `P: RelationQuery`：必须实现 RelationQuery trait

**实现方式**：
- 惰性求值，只在需要时才计算结果
- 零分配，不预先分配内存

**Iterator 实现**：
```rust
impl<'a, P: RelationQuery> Iterator for LazyRelationQueryIterator<'a, P> {
    type Item = &'a RelationChange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let change = self.query.get_relation_at(self.start)?;
        self.start += 1;
        Some(change)
    }
}
```

**优点**：
- 惰性求值
- 零分配
- 链式操作
- 性能优化

---

##### WindowedQuery

**定义**：
```rust
pub struct WindowedQuery<'a, P: StateQuery> {
    query: &'a P,
    window_start: u64,
    window_size: u64,
}
```

**类型参数**：
- `P: StateQuery`：必须实现 StateQuery trait

**实现方式**：
- 窗口化，只查询指定窗口内的数据
- 滚动支持，支持窗口滚动
- 零分配，不预先分配内存

**公共方法**：

1. **`new(query: &'a P, window_start: u64, window_size: u64)`** - 创建新的窗口化查询
   ```rust
   pub fn new(query: &'a P, window_start: u64, window_size: u64) -> Self {
       Self { query, window_start, window_size }
   }
   ```
   - 接受查询对象、窗口起始时间刻和窗口大小作为参数
   - 返回新的窗口化查询

2. **`scroll(self, offset: i64)`** - 滚动窗口
   ```rust
   pub fn scroll(self, offset: i64) -> Self {
       let new_start = if offset >= 0 {
           self.window_start + offset as u64
       } else {
           self.window_start.saturating_sub((-offset) as u64)
       };

       Self { window_start: new_start, ..self }
   }
   ```
   - 滚动窗口
   - 正数向后滚动，负数向前滚动

3. **`resize(self, new_size: u64)`** - 调整窗口大小
   ```rust
   pub fn resize(self, new_size: u64) -> Self {
       Self { window_size: new_size, ..self }
   }
   ```
   - 调整窗口大小

**Iterator 实现**：
```rust
impl<'a, P: StateQuery> Iterator for WindowedQuery<'a, P> {
    type Item = &'a StateSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.window_size == 0 {
            return None;
        }

        let snapshot = self.query.get_at(self.window_start)?;
        self.window_start += 1;
        self.window_size -= 1;
        Some(snapshot)
    }
}
```

**优点**：
- 窗口化
- 滚动支持
- 零分配
- 性能优化
- 适合 Timeline scrubber 场景

---

##### WindowedRelationQuery

**定义**：
```rust
pub struct WindowedRelationQuery<'a, P: RelationQuery> {
    query: &'a P,
    window_start: u64,
    window_size: u64,
}
```

**类型参数**：
- `P: RelationQuery`：必须实现 RelationQuery trait

**实现方式**：
- 窗口化，只查询指定窗口内的数据
- 滚动支持，支持窗口滚动
- 零分配，不预先分配内存

**Iterator 实现**：
```rust
impl<'a, P: RelationQuery> Iterator for WindowedRelationQuery<'a, P> {
    type Item = &'a RelationChange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.window_size == 0 {
            return None;
        }

        let change = self.query.get_relation_at(self.window_start)?;
        self.window_start += 1;
        self.window_size -= 1;
        Some(change)
    }
}
```

**优点**：
- 窗口化
- 滚动支持
- 零分配
- 性能优化
- 适合 Timeline scrubber 场景

## 设计优点

### 1. 追加接口

所有存储都只提供 commit 接口：
```rust
pub fn commit(&mut self, snapshot: StateSnapshot) {
    self.history.record(snapshot);
}
```

### 2. 不可变性

状态一旦记录就不可修改：
- StateSnapshot 是不可变的
- StateHistory 只能添加新状态
- StateStore 不提供 set_state / update_state / replace 接口

### 3. 中立容器

StatePayload 是中立容器，Foundation 不理解 payload 的含义：
```rust
pub struct StatePayload {
    inner: Arc<dyn Any + Send + Sync>,
}
```

### 4. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问。

### 5. 惰性求值

LazyQueryIterator 只在需要时才计算结果：
```rust
impl<'a, P: StateQuery> Iterator for LazyQueryIterator<'a, P> {
    type Item = &'a StateSnapshot;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            return None;
        }

        let snapshot = self.query.get_at(self.start)?;
        self.start += 1;
        Some(snapshot)
    }
}
```

### 6. 窗口化查询

WindowedQuery 支持大范围数据的分页显示：
```rust
pub struct WindowedQuery<'a, P: StateQuery> {
    query: &'a P,
    window_start: u64,
    window_size: u64,
}
```

### 7. 类型安全

充分利用 Rust 的类型系统，使用强类型而不是字符串。

### 8. 性能优化

- 惰性求值，避免不必要的内存分配
- 窗口化查询，支持大范围数据
- 支持提前终止迭代

## 使用示例

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

### 使用惰性查询

```rust
use biosphere_foundation::BasicWorld;
use biosphere_foundation::temporal::LazyQueryIterator;

let mut world = BasicWorld::new();

for _ in 0..10 {
    world.step_world();
}

// 创建惰性查询
let lazy_query = LazyQueryIterator::new(&world, 0, 5);

// 迭代查询
for snapshot in lazy_query {
    println!("Tick: {}", snapshot.tick());
}
```

### 使用窗口化查询

```rust
use biosphere_foundation::BasicWorld;
use biosphere_foundation::temporal::WindowedQuery;

let mut world = BasicWorld::new();

for _ in 0..10 {
    world.step_world();
}

// 创建窗口化查询
let windowed_query = WindowedQuery::new(&world, 0, 5);

// 迭代查询
for snapshot in windowed_query {
    println!("Tick: {}", snapshot.tick());
}

// 滚动窗口
let windowed_query = windowed_query.scroll(3);

// 调整窗口大小
let windowed_query = windowed_query.resize(3);
```

## 总结

temporal 模块提供了 Biosphere 时间系统的实现。它是世界的"时间记忆"，记录世界的状态变化和关系变化。

模块设计遵循以下原则：
- 追加接口：只提供 commit 接口
- 不可变性：状态一旦记录就不可修改
- 中立容器：Foundation 不理解 payload 的含义
- 只读访问：只提供查询接口
- 惰性求值：只在需要时才计算结果
- 窗口化：只查询指定窗口内的数据

所有类型都提供了良好的封装性，通过公共方法提供只读访问。LazyQueryIterator 和 WindowedQuery 提供了性能优化，支持大范围数据的查询。
