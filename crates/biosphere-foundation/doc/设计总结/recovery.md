# Recovery 模块文档

## 概述

recovery 模块提供了 Biosphere Foundation 层的错误恢复系统的实现。它定义了错误恢复策略和恢复结果。

### 核心概念

recovery 模块的核心概念是：

1. **RecoveryContext**：恢复上下文，提供世界和时间信息
2. **WorldRecovery**：世界级别的恢复策略
3. **RecoveryStrategy**：操作级别的恢复策略
4. **RecoveryResult**：错误恢复结果，区分操作恢复和世界恢复

### 设计原则

- **世界感知**：恢复操作发生在"世界里"，而不是"函数里"
- **时间感知**：恢复操作与时间相关
- **演化感知**：恢复操作影响世界演化
- **类型安全**：使用强类型而不是字符串
- **可追溯**：包含恢复策略信息
- **可区分**：区分操作恢复和世界恢复

## 核心公式

```
Operation --Error--> Recovery --RecoveryContext--> WorldRecovery / OperationRecovery
```

这个公式成为整个系统的物理定律。

## 文件结构

```
recovery.rs       # 错误恢复定义
```

## 文件详解

### recovery.rs

**文件路径**：`src/recovery.rs`

**说明**：定义了 RecoveryContext、WorldRecovery、RecoveryStrategy 和 RecoveryResult。

#### 主要类型

##### RecoveryContext

**定义**：
```rust
pub trait RecoveryContext {
    fn current_tick(&self) -> Tick;
    fn can_rollback(&self) -> bool;
    fn rollback_to(&mut self, tick: Tick) -> FoundationResult<()>;
}
```

**字段说明**：
- `current_tick(&self) -> Tick`：获取当前时间刻
- `can_rollback(&self) -> bool`：检查是否可以回退
- `rollback_to(&mut self, tick: Tick) -> FoundationResult<()>`：回退到指定时间刻

**设计约束**：
- 世界感知：提供世界状态信息
- 时间感知：提供时间信息
- 状态感知：提供回退能力

**哲学含义**：
RecoveryContext 是"恢复操作的世界上下文"，而不是"函数执行上下文"。

这意味着：
- RecoveryContext 知道世界的存在
- RecoveryContext 知道时间的存在
- RecoveryContext 可以修改世界状态
- RecoveryContext 是世界演化的一部分

---

##### WorldRecovery

**定义**：
```rust
#[derive(Debug, Clone)]
pub enum WorldRecovery {
    RollbackTo(Tick),
    BranchFrom(Tick),
    TerminateWorld,
}
```

**字段说明**：
- `RollbackTo(Tick)`：回退到指定时间刻
- `BranchFrom(Tick)`：从指定时间刻分叉
- `TerminateWorld`：终止世界演化

**设计约束**：
- 世界感知：所有策略都影响世界状态
- 时间感知：所有策略都与时间相关
- 演化感知：所有策略都影响世界演化

**哲学含义**：
WorldRecovery 是"世界演化中的恢复选择"，而不是"函数执行失败后的行为"。

这意味着：
- WorldRecovery 知道世界的存在
- WorldRecovery 知道时间的存在
- WorldRecovery 可以改变世界演化路径
- WorldRecovery 是世界演化的一部分

---

##### RecoveryStrategy

**定义**：
```rust
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Ignore,
    Retry {
        max_attempts: u32,
    },
    Default,
    Terminate,
}
```

**字段说明**：
- `Ignore`：忽略策略，忽略错误，继续执行
- `Retry { max_attempts: u32 }`：重试策略，重试操作，最多重试 N 次
  - `max_attempts: u32`：最大重试次数
- `Default`：默认策略，使用默认值
- `Terminate`：终止策略，终止操作，返回错误

**设计约束**：
- 操作感知：所有策略都是针对操作的
- 可配置：支持不同的恢复策略
- 可组合：支持多个恢复策略的组合
- 抽象定义：不包含具体实现

**哲学含义**：
RecoveryStrategy 是"操作级别的恢复策略"，而不是"世界级别的恢复策略"。

这意味着：
- RecoveryStrategy 只影响单个操作
- RecoveryStrategy 不影响世界状态
- RecoveryStrategy 不影响时间演化
- RecoveryStrategy 是控制流的一部分，不是世界演化的一部分

**注意**：
RecoveryStrategy 不包含回退策略，因为回退是世界级别的操作，应该使用 WorldRecovery 而不是 RecoveryStrategy。

**公共方法**：

1. **`recover<T, F>(&self, ctx: &mut dyn RecoveryContext, operation: F)`** - 执行错误恢复
   ```rust
   pub fn recover<T, F>(&self, _ctx: &mut dyn RecoveryContext, operation: F) -> RecoveryResult<T>
   where
       F: FnOnce() -> FoundationResult<T>,
   {
       match operation() {
           Ok(value) => RecoveryResult::Ok(value),
           Err(error) => RecoveryResult::Failed {
               error,
               strategy: self.clone(),
           },
       }
   }
   ```
   - 接受恢复上下文和操作作为参数
   - 执行操作并返回恢复结果
   - 注意：这是占位实现，总是失败
   - 应用层必须覆盖此方法以提供实际的恢复逻辑

**哲学含义**：
这个方法明确表示：恢复操作发生在"世界里"，而不是"函数里"。

---

##### RecoveryResult

**定义**：
```rust
#[derive(Debug, Clone)]
pub enum RecoveryResult<T> {
    Ok(T),
    OperationRecovered {
        value: T,
        strategy: RecoveryStrategy,
    },
    WorldRecovered {
        recovery: WorldRecovery,
    },
    Failed {
        error: FoundationError,
        strategy: RecoveryStrategy,
    },
}
```

**字段说明**：
- `Ok(T)`：成功，操作成功执行，没有发生任何恢复
- `OperationRecovered { value: T, strategy: RecoveryStrategy }`：操作恢复成功
  - `value: T`：恢复后的值
  - `strategy: RecoveryStrategy`：使用的恢复策略
- `WorldRecovered { recovery: WorldRecovery }`：世界恢复成功
  - `recovery: WorldRecovery`：使用的世界恢复策略
- `Failed { error: FoundationError, strategy: RecoveryStrategy }`：恢复失败
  - `error: FoundationError`：原始错误
  - `strategy: RecoveryStrategy`：使用的最后恢复策略

**设计约束**：
- 类型安全：使用强类型而不是字符串
- 可追溯：包含恢复策略信息
- 演化感知：区分操作恢复和世界恢复

**哲学含义**：
RecoveryResult 是"恢复操作的结果"，而不是"函数执行的结果"。

这意味着：
- RecoveryResult 可以表达世界演化路径的改变
- RecoveryResult 可以区分操作恢复和世界恢复
- RecoveryResult 是世界演化的一部分
- RecoveryResult 可以表达"世界没有按原路径演化，但我们选择了另一条合法路径"

**公共方法**：

1. **`is_ok()`** - 检查结果是否成功
   ```rust
   pub fn is_ok(&self) -> bool {
       matches!(self, Self::Ok(_) | Self::OperationRecovered { .. } | Self::WorldRecovered { .. })
   }
   ```
   - 返回 true 表示成功，false 表示失败

2. **`is_failed()`** - 检查结果是否失败
   ```rust
   pub fn is_failed(&self) -> bool {
       matches!(self, Self::Failed { .. })
   }
   ```
   - 返回 true 表示失败，false 表示成功

3. **`is_operation_recovered()`** - 检查结果是否通过操作恢复成功
   ```rust
   pub fn is_operation_recovered(&self) -> bool {
       matches!(self, Self::OperationRecovered { .. })
   }
   ```
   - 返回 true 表示通过操作恢复成功，false 表示其他情况

4. **`is_world_recovered()`** - 检查结果是否通过世界恢复成功
   ```rust
   pub fn is_world_recovered(&self) -> bool {
       matches!(self, Self::WorldRecovered { .. })
   }
   ```
   - 返回 true 表示通过世界恢复成功，false 表示其他情况

5. **`value()`** - 获取值
   ```rust
   pub fn value(&self) -> Option<&T> {
       match self {
           Self::Ok(value) => Some(value),
           Self::OperationRecovered { value, .. } => Some(value),
           Self::WorldRecovered { .. } => None,
           Self::Failed { .. } => None,
       }
   }
   ```
   - 如果成功，返回值，否则返回 None

6. **`world_recovery()`** - 获取世界恢复策略
   ```rust
   pub fn world_recovery(&self) -> Option<&WorldRecovery> {
       match self {
           Self::WorldRecovered { recovery } => Some(recovery),
           _ => None,
       }
   }
   ```
   - 如果是世界恢复，返回恢复策略，否则返回 None

7. **`to_foundation_result(self)`** - 转换为 FoundationResult
   ```rust
   pub fn to_foundation_result(self) -> FoundationResult<T> {
       match self {
           Self::Ok(value) => Ok(value),
           Self::OperationRecovered { value, .. } => Ok(value),
           Self::WorldRecovered { .. } => Err(FoundationError::temporal_violation("World recovery cannot be converted to a value")),
           Self::Failed { error, .. } => Err(error),
       }
   }
   ```
   - 如果成功，返回 Ok，否则返回 Err

**Default 实现**：
```rust
impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self::Terminate
    }
}
```

## 设计优点

### 1. 世界感知

RecoveryContext 提供世界和时间信息：
```rust
pub trait RecoveryContext {
    fn current_tick(&self) -> Tick;
    fn can_rollback(&self) -> bool;
    fn rollback_to(&mut self, tick: Tick) -> FoundationResult<()>;
}
```

### 2. 时间感知

WorldRecovery 与时间相关：
```rust
pub enum WorldRecovery {
    RollbackTo(Tick),
    BranchFrom(Tick),
    TerminateWorld,
}
```

### 3. 演化感知

RecoveryResult 区分操作恢复和世界恢复：
```rust
pub enum RecoveryResult<T> {
    Ok(T),
    OperationRecovered { value: T, strategy: RecoveryStrategy },
    WorldRecovered { recovery: WorldRecovery },
    Failed { error: FoundationError, strategy: RecoveryStrategy },
}
```

### 4. 类型安全

使用强类型而不是字符串：
```rust
#[derive(Debug, Clone)]
pub enum RecoveryResult<T> {
    Ok(T),
    OperationRecovered { value: T, strategy: RecoveryStrategy },
    WorldRecovered { recovery: WorldRecovery },
    Failed { error: FoundationError, strategy: RecoveryStrategy },
}
```

### 5. 可追溯性

包含恢复策略信息：
```rust
pub enum RecoveryResult<T> {
    Ok(T),
    OperationRecovered { value: T, strategy: RecoveryStrategy },
    WorldRecovered { recovery: WorldRecovery },
    Failed { error: FoundationError, strategy: RecoveryStrategy },
}
```

### 6. 可区分性

区分操作恢复和世界恢复：
```rust
pub enum RecoveryResult<T> {
    Ok(T),
    OperationRecovered { value: T, strategy: RecoveryStrategy },
    WorldRecovered { recovery: WorldRecovery },
    Failed { error: FoundationError, strategy: RecoveryStrategy },
}
```

## 使用示例

### 使用恢复上下文

```rust
use biosphere_foundation::recovery::RecoveryContext;

struct WorldRecoveryContext {
    current_tick: u64,
    history: Vec<u64>,
}

impl RecoveryContext for WorldRecoveryContext {
    fn current_tick(&self) -> Tick {
        Tick::new(self.current_tick)
    }

    fn can_rollback(&self) -> bool {
        self.current_tick > 0
    }

    fn rollback_to(&mut self, tick: Tick) -> FoundationResult<()> {
        let target = tick.value();
        if target >= self.current_tick {
            return Err(FoundationError::temporal_violation(
                format!("Cannot rollback to future tick: {} >= {}", target, self.current_tick)
            ));
        }
        
        if !self.history.contains(&target) {
            return Err(FoundationError::temporal_violation(
                format!("Tick {} not in history", target)
            ));
        }
        
        self.current_tick = target;
        Ok(())
    }
}
```

### 使用恢复策略

```rust
use biosphere_foundation::recovery::{RecoveryStrategy, RecoveryResult};

let strategy = RecoveryStrategy::Retry { max_attempts: 3 };
let mut ctx = WorldRecoveryContext::new();

let result = strategy.recover(&mut ctx, || {
    Ok::<i32, FoundationError>(42)
});

assert!(result.is_ok());
```

### 检查恢复结果

```rust
use biosphere_foundation::recovery::RecoveryResult;

let result: RecoveryResult<i32> = RecoveryResult::Ok(42);
assert!(result.is_ok());

let result: RecoveryResult<i32> = RecoveryResult::Failed {
    error: FoundationError::temporal_violation("Test"),
    strategy: RecoveryStrategy::Terminate,
};
assert!(result.is_failed());
```

### 区分操作恢复和世界恢复

```rust
use biosphere_foundation::recovery::RecoveryResult;

let op_recovered: RecoveryResult<i32> = RecoveryResult::OperationRecovered {
    value: 100,
    strategy: RecoveryStrategy::Default,
};
assert!(op_recovered.is_operation_recovered());
assert!(!op_recovered.is_world_recovered());

let world_recovered: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
    recovery: WorldRecovery::BranchFrom(Tick::new(5)),
};
assert!(!world_recovered.is_operation_recovered());
assert!(world_recovered.is_world_recovered());
```

### 获取恢复值

```rust
use biosphere_foundation::recovery::RecoveryResult;

let result: RecoveryResult<i32> = RecoveryResult::Ok(42);
assert_eq!(result.value(), Some(&42));

let result: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
    recovery: WorldRecovery::BranchFrom(Tick::new(5)),
};
assert_eq!(result.value(), None);
```

### 获取世界恢复策略

```rust
use biosphere_foundation::recovery::RecoveryResult;

let result: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
    recovery: WorldRecovery::BranchFrom(Tick::new(5)),
};
assert!(result.world_recovery().is_some());
```

### 转换为 FoundationResult

```rust
use biosphere_foundation::recovery::RecoveryResult;

let result: RecoveryResult<i32> = RecoveryResult::Ok(42);
let foundation_result = result.to_foundation_result();
assert!(foundation_result.is_ok());

let result: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
    recovery: WorldRecovery::BranchFrom(Tick::new(5)),
};
let foundation_result = result.to_foundation_result();
assert!(foundation_result.is_err());
```

## 注意事项

### 1. 占位实现

RecoveryStrategy::recover 方法是占位实现，总是失败：
```rust
pub fn recover<T, F>(&self, _ctx: &mut dyn RecoveryContext, operation: F) -> RecoveryResult<T>
where
    F: FnOnce() -> FoundationResult<T>,
{
    match operation() {
        Ok(value) => RecoveryResult::Ok(value),
        Err(error) => RecoveryResult::Failed {
            error,
            strategy: self.clone(),
        },
    }
}
```

应用层必须覆盖此方法以提供实际的恢复逻辑。

### 2. 不同的恢复策略

不同的恢复策略（Ignore、Retry、Default）应该有不同的实现。

### 3. 应用层实现

Foundation 层只提供接口定义，具体实现由应用层负责。

### 4. 世界恢复 vs 操作恢复

WorldRecovery 是世界级别的恢复策略，影响世界状态和时间演化。
RecoveryStrategy 是操作级别的恢复策略，只影响单个操作。

### 5. RecoveryContext 的实现

RecoveryContext 的实现必须保证：
- 回退操作必须是原子的
- 回退操作必须保持世界一致性
- 只能回退到之前的时间刻

## 架构意义

Recovery v2 的引入解决了以下问题：

1. **与 Temporal 对齐**
   - Recovery 现在有时间语义
   - Recovery 可以表达回退到指定时间刻
   - Recovery 可以表达从指定时间刻分叉

2. **与 World 对齐**
   - Recovery 现在有世界状态语义
   - Recovery 可以表达世界演化路径的改变
   - Recovery 是世界演化的一部分

3. **区分操作恢复和世界恢复**
   - OperationRecovered：操作级别的恢复
   - WorldRecovered：世界级别的恢复
   - 可以表达"世界没有按原路径演化，但我们选择了另一条合法路径"

4. **防止下沉到"业务层 hack"**
   - Recovery 现在是 Foundation 的一等公民
   - Recovery 不再只是一个提示枚举
   - Recovery 有明确的 ontology 层次

## 总结

recovery 模块提供了 Biosphere Foundation 层的错误恢复系统的实现。它定义了恢复上下文、世界恢复策略、操作恢复策略和恢复结果。

模块设计遵循以下原则：
- 世界感知：恢复操作发生在"世界里"，而不是"函数里"
- 时间感知：恢复操作与时间相关
- 演化感知：恢复操作影响世界演化
- 类型安全：使用强类型而不是字符串
- 可追溯：包含恢复策略信息
- 可区分：区分操作恢复和世界恢复

RecoveryContext 提供世界和时间信息，WorldRecovery 提供世界级别的恢复策略，RecoveryStrategy 提供操作级别的恢复策略，RecoveryResult 提供恢复结果，四者共同构成了 Foundation 层的错误恢复系统。

**Recovery 现在是 World / Temporal 的一等公民，而不是 operation 的伴生物。**
