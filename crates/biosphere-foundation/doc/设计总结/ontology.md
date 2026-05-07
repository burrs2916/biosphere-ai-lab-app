# Ontology 模块文档

## 概述

ontology 模块提供了 Biosphere 存在论的基础实现。它实现了 biosphere-core 中定义的核心 trait，包括 Environment、Embodiment、Existence、Perception 和 Representation。

### 核心概念

ontology 模块的核心概念是：

1. **Environment（环境）**：生命得以存在和演化的外部条件
2. **Embodiment（具身）**：生命在环境中的具体化
3. **Existence（存在）**：生命的基本存在形式
4. **Perception（感知）**：生命对环境的感知能力
5. **Representation（表示）**：生命对环境的内部表示

### 设计原则

- **基础实现**：提供默认的实现，应用层可以覆盖
- **中立容器**：不包含具体的业务逻辑
- **可扩展**：应用层可以自定义实现
- **类型安全**：充分利用 Rust 的类型系统

## 文件结构

```
ontology/
├── mod.rs                      # 模块入口，导出所有公共类型
├── basic_environment.rs        # 基础环境实现
├── basic_embodiment.rs         # 基础具身实现
├── basic_existence.rs          # 基础存在实现
├── basic_perception.rs         # 基础感知实现
└── basic_representation.rs     # 基础表示实现
```

## 文件详解

### mod.rs

**文件路径**：`src/ontology/mod.rs`

**说明**：ontology 模块的入口文件，导出所有公共类型。

**导出内容**：
- `BasicEnvironment`：基础环境
- `BasicEnvironmentState`：基础环境状态
- `BasicEmbodiment`：基础具身
- `BasicPerception`：基础感知
- `BasicRepresentation`：基础表示
- `BasicExistence`：基础存在

**实现方式**：
```rust
pub mod basic_environment;
pub mod basic_embodiment;
pub mod basic_perception;
pub mod basic_representation;
pub mod basic_existence;

pub use basic_environment::{BasicEnvironment, BasicEnvironmentState};
pub use basic_embodiment::BasicEmbodiment;
pub use basic_perception::BasicPerception;
pub use basic_representation::BasicRepresentation;
pub use basic_existence::BasicExistence;
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### basic_environment.rs

**文件路径**：`src/ontology/basic_environment.rs`

**说明**：实现了 biosphere-core 中定义的 Environment trait，提供基础环境实现。

#### 主要类型

##### BasicEnvironment

**定义**：
```rust
pub struct BasicEnvironment {
    clock: WorldClock,
    conditions: SensedConditions,
}
```

**字段说明**：
- `clock: WorldClock`（私有）：世界时钟，管理时间推进
- `conditions: SensedConditions`（私有）：感知条件，暴露给生命的条件

**实现方式**：
- 所有字段都是私有的，确保封装性
- 通过公共方法提供只读访问
- 实现了 `Environment` 和 `TemporalEnvironment` trait

**公共方法**：

1. **`new()`** - 创建新的基础环境
   ```rust
   pub fn new() -> Self {
       let clock = WorldClock::new();
       let snapshot = ConditionSnapshot { signals: Vec::new() };
       let conditions = SensedConditions::new(snapshot);

       Self { clock, conditions }
   }
   ```
   - 初始化世界时钟
   - 创建空的条件快照
   - 返回新的环境实例

2. **`advance()`** - 推进时间
   ```rust
   pub fn advance(&mut self) -> Option<u64> {
       self.clock.advance()
   }
   ```
   - 调用时钟的 advance 方法
   - 返回新的时间刻
   - 如果时钟达到上限，返回 None

3. **`current_tick()`** - 获取当前时间刻
   ```rust
   pub fn current_tick(&self) -> u64 {
       self.clock.current_tick()
   }
   ```
   - 返回当前的时间刻
   - 只读访问，不修改状态

4. **`conditions()`** - 获取当前条件
   ```rust
   pub fn conditions(&self) -> &SensedConditions {
       &self.conditions
   }
   ```
   - 返回条件的不可变引用
   - 只读访问，不修改状态

**Trait 实现**：

1. **Environment trait**：
   ```rust
   impl Environment for BasicEnvironment {
       type State = BasicEnvironmentState;
       type Conditions = SensedConditions;

       fn step(&mut self) {
           let _new_tick = self.clock.advance().unwrap();
           let snapshot = ConditionSnapshot { signals: Vec::new() };
           self.conditions.update(snapshot);
       }

       fn conditions(&self) -> &Self::Conditions {
           &self.conditions
       }
   }
   ```
   - `step()`：推进环境，更新条件
   - `conditions()`：返回当前条件

2. **TemporalEnvironment trait**：
   ```rust
   impl TemporalEnvironment for BasicEnvironment {
       fn advance(&mut self) {
           self.clock.advance();
       }
   }
   ```
   - `advance()`：推进时间

**优点**：
- 良好的封装性，所有字段都是私有的
- 清晰的接口设计
- 实现了核心 trait
- 支持时间推进和条件更新

---

##### BasicEnvironmentState

**定义**：
```rust
pub struct BasicEnvironmentState {
    tick: u64,
}
```

**字段说明**：
- `tick: u64`（私有）：当前时间刻

**实现方式**：
- 字段私有，确保封装性
- 通过公共方法提供只读访问
- 实现了 `WorldState` trait

**公共方法**：

1. **`new(tick: u64)`** - 创建新的环境状态
   ```rust
   pub fn new(tick: u64) -> Self {
       Self { tick }
   }
   ```
   - 接受时间刻参数
   - 返回新的状态实例

2. **`tick()`** - 获取时间刻
   ```rust
   pub fn tick(&self) -> u64 {
       self.tick
   }
   ```
   - 返回当前时间刻
   - 只读访问，不修改状态

**Trait 实现**：

1. **WorldState trait**：
   ```rust
   impl WorldState for BasicEnvironmentState {}
   ```
   - 标记 trait，表示这是世界状态类型

**优点**：
- 简单清晰的状态表示
- 良好的封装性
- 类型安全

---

### basic_embodiment.rs

**文件路径**：`src/ontology/basic_embodiment.rs`

**说明**：实现了 biosphere-core 中定义的 Embodiment trait，提供基础具身实现。

#### 主要类型

##### BasicEmbodiment

**定义**：
```rust
pub struct BasicEmbodiment {
    existence: BasicExistence,
    perception: BasicPerception,
    representation: BasicRepresentation,
}
```

**字段说明**：
- `existence: BasicExistence`（私有）：生命的基本存在形式
- `perception: BasicPerception`（私有）：生命对环境的感知能力
- `representation: BasicRepresentation`（私有）：生命对环境的内部表示

**实现方式**：
- 所有字段都是私有的，确保封装性
- 组合了存在、感知和表示三个核心概念
- 实现了 `Embodiment` trait

**公共方法**：

1. **`new(existence, perception, representation)`** - 创建新的具身
   ```rust
   pub fn new(
       existence: BasicExistence,
       perception: BasicPerception,
       representation: BasicRepresentation,
   ) -> Self {
       Self {
           existence,
           perception,
           representation,
       }
   }
   ```
   - 接受存在、感知和表示三个参数
   - 返回新的具身实例

**Trait 实现**：

1. **Embodiment trait**：
   ```rust
   impl Embodiment for BasicEmbodiment {
       type Existence = BasicExistence;
       type PerceptionImpl = BasicPerception;
       type RepresentationImpl = BasicRepresentation;

       fn existence(&self) -> &Self::Existence {
           &self.existence
       }

       fn perception(&self) -> &Self::PerceptionImpl {
           &self.perception
       }

       fn representation(&self) -> &Self::RepresentationImpl {
           &self.representation
       }
   }
   ```
   - `existence()`：返回存在的引用
   - `perception()`：返回感知的引用
   - `representation()`：返回表示的引用

**优点**：
- 清晰的组合结构
- 良好的封装性
- 实现了核心 trait
- 易于扩展

---

### basic_existence.rs

**文件路径**：`src/ontology/basic_existence.rs`

**说明**：实现了 biosphere-core 中定义的 ExistenceCore trait，提供基础存在实现。

#### 主要类型

##### BasicExistence

**定义**：
```rust
#[allow(dead_code)]
pub struct BasicExistence {
    _marker: std::marker::PhantomData<()>,
}
```

**字段说明**：
- `_marker: PhantomData<()>`（私有）：类型标记，不占用内存

**实现方式**：
- 使用 PhantomData 作为占位符
- 类型参数是占位符，应用层应该提供具体实现
- 实现了 `ExistenceCore` trait

**公共方法**：

1. **`new()`** - 创建新的基础存在
   ```rust
   pub fn new() -> Self {
       Self {
           _marker: std::marker::PhantomData,
       }
   }
   ```
   - 返回新的存在实例
   - 不需要任何参数

**Trait 实现**：

1. **ExistenceCore trait**：
   ```rust
   impl ExistenceCore for BasicExistence {
       type Boundary = ();
       type State = ();
       type Drive = ();
       type Rules = ();
       type Propagation = ();
   }
   ```
   - 所有类型参数都是占位符 `()`
   - 应用层应该提供具体的类型实现

**优点**：
- 提供了基础的 trait 实现
- 灵活的类型参数，应用层可以自定义
- 零成本抽象（PhantomData 不占用内存）

**注意**：
- 这是一个占位符实现
- 应用层应该提供具体的类型实现
- 类型参数应该被替换为具体的类型

---

### basic_perception.rs

**文件路径**：`src/ontology/basic_perception.rs`

**说明**：实现了 biosphere-core 中定义的 Perception trait，提供基础感知实现。

#### 主要类型

##### BasicPerception

**定义**：
```rust
pub struct BasicPerception {
    snapshot: ConditionSnapshot,
}
```

**字段说明**：
- `snapshot: ConditionSnapshot`（私有）：条件快照，包含所有可感知的信号

**实现方式**：
- 字段私有，确保封装性
- 持有条件快照，提供感知能力
- 实现了 `Perception` trait

**公共方法**：

1. **`new(conditions: &dyn Conditions)`** - 创建新的基础感知
   ```rust
   pub fn new(conditions: &dyn Conditions) -> Self {
       Self {
           snapshot: conditions.snapshot(),
       }
   }
   ```
   - 接受条件引用作为参数
   - 从条件中获取快照
   - 返回新的感知实例

2. **`snapshot()`** - 获取条件快照
   ```rust
   pub fn snapshot(&self) -> &ConditionSnapshot {
       &self.snapshot
   }
   ```
   - 返回条件快照的引用
   - 只读访问，不修改状态

**Trait 实现**：

1. **Perception trait**：
   ```rust
   impl Perception for BasicPerception {
       type Signal = ConditionSignal;

       fn signal(&self) -> Self::Signal {
           if let Some(signal) = self.snapshot.signals.first() {
               signal.clone()
           } else {
               ConditionSignal {
                   kind: "",
                   intensity: 0,
               }
           }
       }

       fn distinguish(&self, a: &Self::Signal, b: &Self::Signal) -> bool {
           a.kind == b.kind && a.intensity == b.intensity
       }
   }
   ```
   - `signal()`：返回第一个信号，如果没有信号则返回默认信号
   - `distinguish()`：区分两个信号是否相同

**优点**：
- 清晰的感知接口
- 良好的封装性
- 实现了核心 trait
- 提供了默认的感知行为

**注意**：
- 这是默认实现
- 应用层可以覆盖此方法以提供自定义的感知行为

---

### basic_representation.rs

**文件路径**：`src/ontology/basic_representation.rs`

**说明**：实现了 biosphere-core 中定义的 Representation trait，提供基础表示实现。

#### 主要类型

##### BasicRepresentation

**定义**：
```rust
pub struct BasicRepresentation {
    data: BasicRepresentationData,
}
```

**字段说明**：
- `data: BasicRepresentationData`（私有）：表示数据

**实现方式**：
- 字段私有，确保封装性
- 持有表示数据
- 实现了 `Representation` trait

**公共方法**：

1. **`new(data: BasicRepresentationData)`** - 创建新的基础表示
   ```rust
   pub fn new(data: BasicRepresentationData) -> Self {
       Self { data }
   }
   ```
   - 接受表示数据作为参数
   - 返回新的表示实例

2. **`data()`** - 获取表示数据
   ```rust
   pub fn data(&self) -> &BasicRepresentationData {
       &self.data
   }
   ```
   - 返回表示数据的引用
   - 只读访问，不修改状态

**Trait 实现**：

1. **Representation trait**：
   ```rust
   impl Representation for BasicRepresentation {
       type Data = BasicRepresentationData;

       fn data(&self) -> &Self::Data {
           &self.data
       }
   }
   ```
   - `data()`：返回表示数据的引用

**优点**：
- 清晰的表示接口
- 良好的封装性
- 实现了核心 trait
- 易于扩展

---

##### BasicRepresentationData

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct BasicRepresentationData {
    signal: biosphere_core::ConditionSignal,
}
```

**字段说明**：
- `signal: ConditionSignal`（私有）：条件信号

**实现方式**：
- 字段私有，确保封装性
- 持有条件信号作为表示数据

**公共方法**：

1. **`new(signal: ConditionSignal)`** - 创建新的表示数据
   ```rust
   pub fn new(signal: biosphere_core::ConditionSignal) -> Self {
       Self { signal }
   }
   ```
   - 接受条件信号作为参数
   - 返回新的表示数据实例

2. **`signal()`** - 获取条件信号
   ```rust
   pub fn signal(&self) -> &biosphere_core::ConditionSignal {
       &self.signal
   }
   ```
   - 返回条件信号的引用
   - 只读访问，不修改状态

**优点**：
- 简单清晰的数据结构
- 良好的封装性
- 类型安全

---

## 设计优点

### 1. 清晰的模块组织

ontology 模块按照存在论的核心概念组织，每个文件对应一个核心概念：
- basic_environment.rs：环境
- basic_embodiment.rs：具身
- basic_existence.rs：存在
- basic_perception.rs：感知
- basic_representation.rs：表示

### 2. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问：
```rust
pub struct BasicEnvironment {
    clock: WorldClock,      // 私有字段
    conditions: SensedConditions,  // 私有字段
}
```

### 3. Trait 实现

所有类型都实现了 biosphere-core 中定义的核心 trait：
- `BasicEnvironment` 实现了 `Environment` 和 `TemporalEnvironment`
- `BasicEmbodiment` 实现了 `Embodiment`
- `BasicExistence` 实现了 `ExistenceCore`
- `BasicPerception` 实现了 `Perception`
- `BasicRepresentation` 实现了 `Representation`

### 4. 灵活的扩展

应用层可以覆盖默认实现，提供自定义的行为：
```rust
impl Perception for MyCustomPerception {
    type Signal = MySignal;

    fn signal(&self) -> Self::Signal {
        // 自定义实现
    }
}
```

### 5. 类型安全

充分利用 Rust 的类型系统，使用强类型而不是字符串：
```rust
pub struct BasicRepresentationData {
    signal: biosphere_core::ConditionSignal,  // 强类型
}
```

### 6. 零成本抽象

使用 PhantomData 等技术实现零成本抽象：
```rust
pub struct BasicExistence {
    _marker: std::marker::PhantomData<()>,  // 不占用内存
}
```

## 使用示例

### 创建环境

```rust
use biosphere_foundation::BasicEnvironment;

// 创建新的环境
let environment = BasicEnvironment::new();

// 推进时间
environment.advance();

// 获取当前时间刻
let tick = environment.current_tick();

// 获取当前条件
let conditions = environment.conditions();
```

### 创建具身

```rust
use biosphere_foundation::{
    BasicEmbodiment,
    BasicExistence,
    BasicPerception,
    BasicRepresentation,
};

// 创建具身
let existence = BasicExistence::new();
let perception = BasicPerception::new(&conditions);
let representation = BasicRepresentation::new(data);

let embodiment = BasicEmbodiment::new(existence, perception, representation);

// 访问组件
let existence_ref = embodiment.existence();
let perception_ref = embodiment.perception();
let representation_ref = embodiment.representation();
```

### 创建感知

```rust
use biosphere_foundation::BasicPerception;

// 创建感知
let perception = BasicPerception::new(&conditions);

// 获取信号
let signal = perception.signal();

// 区分信号
let is_same = perception.distinguish(&signal1, &signal2);
```

## 总结

ontology 模块提供了 Biosphere 存在论的基础实现。它实现了 biosphere-core 中定义的核心 trait，包括 Environment、Embodiment、Existence、Perception 和 Representation。

模块设计遵循以下原则：
- 清晰的模块组织
- 良好的封装性
- Trait 实现
- 灵活的扩展
- 类型安全
- 零成本抽象

所有类型都提供了良好的封装性，通过公共方法提供只读访问。应用层可以覆盖默认实现，提供自定义的行为。
