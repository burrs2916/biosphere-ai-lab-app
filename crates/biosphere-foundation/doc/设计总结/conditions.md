# Conditions 模块文档

## 概述

conditions 模块提供了 Biosphere 条件系统的实现。SensedConditions 是 Conditions trait 的唯一实现，描述世界允许被感知的那一部分，是"世界 → 生命"的单向闸门。

### 核心概念

conditions 模块的核心概念是：

1. **SensedConditions（感知条件）**：世界允许被感知的那一部分
2. **ConditionSnapshot（条件快照）**：条件的瞬时快照
3. **ConditionSignal（条件信号）**：条件中的信号

### 设计原则

- **不可构造**：外部代码无法构造 SensedConditions（只能通过 new）
- **不可修改**：Conditions 是不可变的
- **不可反推**：无法从 Conditions 反推世界的完整状态
- **单向性**：世界 → 生命，生命无法反向影响世界
- **中立容器**：不包含任何业务逻辑

## 文件结构

```
conditions/
├── mod.rs                  # 模块入口，导出所有公共类型
└── sensed_conditions.rs    # 感知条件实现
```

## 文件详解

### mod.rs

**文件路径**：`src/conditions/mod.rs`

**说明**：conditions 模块的入口文件，导出所有公共类型。

**导出内容**：
- `SensedConditions`：感知条件

**实现方式**：
```rust
pub mod sensed_conditions;

pub use sensed_conditions::SensedConditions;
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### sensed_conditions.rs

**文件路径**：`src/conditions/sensed_conditions.rs`

**说明**：实现了 Conditions trait，是 Conditions trait 的唯一实现。

#### 主要类型

##### SensedConditions

**定义**：
```rust
#[derive(Clone, Debug)]
pub struct SensedConditions {
    snapshot: Arc<ConditionSnapshot>,
}
```

**字段说明**：
- `snapshot: Arc<ConditionSnapshot>`（私有）：条件快照

**实现方式**：
- 字段私有，确保封装性
- 使用 Arc 包装快照，支持共享
- 实现了 `Conditions` trait

**公共方法**：

1. **`new(snapshot: ConditionSnapshot)`** - 创建感知条件
   ```rust
   pub fn new(snapshot: ConditionSnapshot) -> Self {
       Self {
           snapshot: Arc::new(snapshot),
       }
   }
   ```
   - 接受条件快照作为参数
   - 使用 Arc 包装快照
   - 返回新的感知条件实例

2. **`update(snapshot: ConditionSnapshot)`** - 更新条件快照
   ```rust
   pub fn update(&mut self, snapshot: ConditionSnapshot) {
       self.snapshot = Arc::new(snapshot);
   }
   ```
   - 接受新的条件快照作为参数
   - 使用 Arc 包装新快照
   - 替换旧快照

3. **`snapshot()`** - 返回条件快照
   ```rust
   pub fn snapshot(&self) -> ConditionSnapshot {
       (*self.snapshot).clone()
   }
   ```
   - 克隆条件快照
   - 返回快照的副本

**Trait 实现**：

1. **Conditions trait**：
   ```rust
   impl Conditions for SensedConditions {
       fn snapshot(&self) -> ConditionSnapshot {
           self.snapshot()
       }
   }
   ```
   - `snapshot()`：返回条件快照

**优点**：
- 单向信息流
- 不可修改的条件
- 不可反推
- 中立容器
- 支持共享（Arc）

## 设计优点

### 1. 单向信息流

SensedConditions 是"单向闸门"，只允许信息从世界流向生命：
```rust
pub struct SensedConditions {
    snapshot: Arc<ConditionSnapshot>,
}
```

### 2. 不可修改

Conditions 是不可变的，外部代码无法修改：
```rust
pub fn snapshot(&self) -> ConditionSnapshot {
    (*self.snapshot).clone()
}
```

### 3. 不可反推

无法从 Conditions 反推世界的完整状态：
- Conditions 只包含世界允许被感知的那一部分
- 不包含世界的完整信息

### 4. 中立容器

SensedConditions 是中立容器，不解释信号含义：
- 不创建信号
- 不解释信号含义
- 信号创建和解释由应用层负责

### 5. 支持共享

使用 Arc 包装快照，支持多个引用共享：
```rust
snapshot: Arc<ConditionSnapshot>,
```

### 6. 良好的封装性

所有字段都是私有的，通过公共方法提供只读访问。

### 7. 实现了 Conditions trait

SensedConditions 是 Conditions trait 的唯一实现。

## 使用示例

### 创建感知条件

```rust
use biosphere_foundation::SensedConditions;
use biosphere_core::ConditionSnapshot;

// 创建条件快照
let snapshot = ConditionSnapshot {
    signals: vec![],
};

// 创建感知条件
let conditions = SensedConditions::new(snapshot);
```

### 更新条件快照

```rust
use biosphere_foundation::SensedConditions;
use biosphere_core::ConditionSnapshot;

let mut conditions = SensedConditions::new(snapshot1);

// 更新条件快照
let new_snapshot = ConditionSnapshot {
    signals: vec![signal1, signal2],
};
conditions.update(new_snapshot);
```

### 获取条件快照

```rust
use biosphere_foundation::SensedConditions;

let conditions = SensedConditions::new(snapshot);

// 获取条件快照
let snapshot = conditions.snapshot();
```

### 与 Environment 配合使用

```rust
use biosphere_foundation::BasicWorld;
use biosphere_core::Environment;

let mut world = BasicWorld::new();

// 推进世界
world.step();

// 获取当前条件
let conditions = world.conditions();

// 获取条件快照
let snapshot = conditions.snapshot();
```

## 总结

conditions 模块提供了 Biosphere 条件系统的实现。SensedConditions 是 Conditions trait 的唯一实现，描述世界允许被感知的那一部分，是"世界 → 生命"的单向闸门。

模块设计遵循以下原则：
- 不可构造：外部代码无法构造 SensedConditions（只能通过 new）
- 不可修改：Conditions 是不可变的
- 不可反推：无法从 Conditions 反推世界的完整状态
- 单向性：世界 → 生命，生命无法反向影响世界
- 中立容器：不包含任何业务逻辑

SensedConditions 使用 Arc 包装快照，支持多个引用共享。所有字段都是私有的，通过公共方法提供只读访问。
