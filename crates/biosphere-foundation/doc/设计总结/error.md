# Error 模块文档

## 概述

error 模块提供了 Biosphere Foundation 层的错误处理系统的实现。它定义了 Foundation 层的所有错误类型和结果类型。

### 核心概念

error 模块的核心概念是：

1. **FoundationError**：Foundation 层的错误类型
2. **FoundationResult**：Foundation 层的结果类型

### 设计原则

- **类型安全**：使用枚举而不是字符串
- **可恢复**：所有错误都应该可以恢复
- **可追溯**：错误应该包含足够的上下文信息

## 文件结构

```
error.rs          # 错误定义
```

## 文件详解

### error.rs

**文件路径**：`src/error.rs`

**说明**：定义了 FoundationError 和 FoundationResult。

#### 主要类型

##### FoundationError

**定义**：
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundationError {
    TemporalViolation {
        message: String,
    },
    StateError {
        message: String,
    },
    RelationError {
        message: String,
    },
    CommandError {
        message: String,
    },
    QueryError {
        message: String,
    },
    CursorError {
        message: String,
    },
}
```

**字段说明**：
- `TemporalViolation`：时间违规错误
  - `message: String`：错误消息
- `StateError`：状态错误
  - `message: String`：错误消息
- `RelationError`：关系错误
  - `message: String`：错误消息
- `CommandError`：命令错误
  - `message: String`：错误消息
- `QueryError`：查询错误
  - `message: String`：错误消息
- `CursorError`：游标错误
  - `message: String`：错误消息

**实现方式**：
- 枚举类型，提供类型安全的错误处理
- 每个错误变体都包含错误消息
- 实现了 Debug、Clone、PartialEq、Eq trait

**公共方法**：

1. **`temporal_violation(message: impl Into<String>)`** - 创建时间违规错误
   ```rust
   pub fn temporal_violation(message: impl Into<String>) -> Self {
       Self::TemporalViolation {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的时间违规错误

2. **`state_error(message: impl Into<String>)`** - 创建状态错误
   ```rust
   pub fn state_error(message: impl Into<String>) -> Self {
       Self::StateError {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的状态错误

3. **`relation_error(message: impl Into<String>)`** - 创建关系错误
   ```rust
   pub fn relation_error(message: impl Into<String>) -> Self {
       Self::RelationError {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的关系错误

4. **`command_error(message: impl Into<String>)`** - 创建命令错误
   ```rust
   pub fn command_error(message: impl Into<String>) -> Self {
       Self::CommandError {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的命令错误

5. **`query_error(message: impl Into<String>)`** - 创建查询错误
   ```rust
   pub fn query_error(message: impl Into<String>) -> Self {
       Self::QueryError {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的查询错误

6. **`cursor_error(message: impl Into<String>)`** - 创建游标错误
   ```rust
   pub fn cursor_error(message: impl Into<String>) -> Self {
       Self::CursorError {
           message: message.into(),
       }
   }
   ```
   - 接受错误消息作为参数
   - 返回新的游标错误

7. **`is_recoverable()`** - 检查错误是否可恢复
   ```rust
   pub fn is_recoverable(&self) -> bool {
       match self {
           Self::TemporalViolation { .. } => true,
           Self::StateError { .. } => true,
           Self::RelationError { .. } => true,
           Self::CommandError { .. } => true,
           Self::QueryError { .. } => true,
           Self::CursorError { .. } => true,
       }
   }
   ```
   - 返回 true 表示错误可恢复，false 表示不可恢复
   - 所有 Foundation 层错误都是可恢复的

8. **`message()`** - 获取错误消息
   ```rust
   pub fn message(&self) -> &str {
       match self {
           Self::TemporalViolation { message } => message,
           Self::StateError { message } => message,
           Self::RelationError { message } => message,
           Self::CommandError { message } => message,
           Self::QueryError { message } => message,
           Self::CursorError { message } => message,
       }
   }
   ```
   - 返回错误消息

**Trait 实现**：

1. **Display trait**：
   ```rust
   impl std::fmt::Display for FoundationError {
       fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
           match self {
               Self::TemporalViolation { message } => write!(f, "Temporal violation: {}", message),
               Self::StateError { message } => write!(f, "State error: {}", message),
               Self::RelationError { message } => write!(f, "Relation error: {}", message),
               Self::CommandError { message } => write!(f, "Command error: {}", message),
               Self::QueryError { message } => write!(f, "Query error: {}", message),
               Self::CursorError { message } => write!(f, "Cursor error: {}", message),
           }
       }
   }
   ```
   - 实现了 Display trait
   - 提供用户友好的错误显示

2. **Error trait**：
   ```rust
   impl std::error::Error for FoundationError {}
   ```
   - 实现了 Error trait
   - 使 FoundationError 可以作为标准错误类型

---

##### FoundationResult

**定义**：
```rust
pub type FoundationResult<T> = Result<T, FoundationError>;
```

**说明**：
- Foundation 层的标准结果类型
- 封装了 FoundationError 作为错误类型

## 设计优点

### 1. 类型安全

FoundationError 使用枚举而不是字符串：
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundationError {
    TemporalViolation { message: String },
    StateError { message: String },
    RelationError { message: String },
    CommandError { message: String },
    QueryError { message: String },
    CursorError { message: String },
}
```

### 2. 可恢复性

所有错误都是可恢复的：
```rust
pub fn is_recoverable(&self) -> bool {
    match self {
        Self::TemporalViolation { .. } => true,
        Self::StateError { .. } => true,
        Self::RelationError { .. } => true,
        Self::CommandError { .. } => true,
        Self::QueryError { .. } => true,
        Self::CursorError { .. } => true,
    }
}
```

### 3. 可追溯性

错误包含足够的上下文信息：
```rust
pub fn message(&self) -> &str {
    match self {
        Self::TemporalViolation { message } => message,
        Self::StateError { message } => message,
        Self::RelationError { message } => message,
        Self::CommandError { message } => message,
        Self::QueryError { message } => message,
        Self::CursorError { message } => message,
    }
}
```

### 4. 错误分类

错误按类别分类：
- **时间错误**：时间相关的错误
- **状态错误**：状态相关的错误
- **关系错误**：关系相关的错误
- **命令错误**：命令相关的错误
- **查询错误**：查询相关的错误
- **游标错误**：游标相关的错误

### 5. 标准结果类型

FoundationResult 提供了标准的结果类型：
```rust
pub type FoundationResult<T> = Result<T, FoundationError>;
```

### 6. 良好的错误处理

实现了标准的错误处理 trait：
- Debug：用于调试
- Clone：支持错误复制
- PartialEq、Eq：支持错误比较
- Display：用于用户友好的错误显示
- Error：标准错误 trait

## 使用示例

### 创建错误

```rust
use biosphere_foundation::error::FoundationError;

let error = FoundationError::temporal_violation("Time cannot go backwards");
```

### 检查错误

```rust
use biosphere_foundation::error::FoundationError;

let error = FoundationError::temporal_violation("Time cannot go backwards");
assert!(error.is_recoverable());
assert_eq!(error.message(), "Time cannot go backwards");
```

### 处理错误

```rust
use biosphere_foundation::error::{FoundationError, FoundationResult};

fn do_something() -> FoundationResult<()> {
    Err(FoundationError::temporal_violation("Time cannot go backwards"))
}

match do_something() {
    Ok(_) => println!("Success"),
    Err(FoundationError::TemporalViolation { message }) => {
        println!("Temporal violation: {}", message);
    }
    Err(e) => println!("Error: {:?}", e),
}
```

### 使用 FoundationResult

```rust
use biosphere_foundation::error::{FoundationError, FoundationResult};

fn do_something() -> FoundationResult<()> {
    // 成功情况
    Ok(())
}

fn do_something_else() -> FoundationResult<()> {
    // 失败情况
    Err(FoundationError::state_error("State not found"))
}
```

## 总结

error 模块提供了 Biosphere Foundation 层的错误处理系统的实现。它定义了 Foundation 层的所有错误类型和结果类型。

模块设计遵循以下原则：
- 类型安全：使用枚举而不是字符串
- 可恢复：所有错误都应该可以恢复
- 可追溯：错误应该包含足够的上下文信息

FoundationError 提供了分类的错误处理，FoundationResult 提供了标准的结果类型，两者共同构成了 Foundation 层的错误处理系统。
