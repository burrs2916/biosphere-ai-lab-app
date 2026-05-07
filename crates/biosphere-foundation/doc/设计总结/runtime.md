# Runtime 模块文档

## 概述

runtime 模块提供了 Biosphere 运行时系统的实现。它管理世界的运行时状态，包括时间游标和命令执行。

### 核心概念

runtime 模块的核心概念是：

1. **WorldRuntime（世界运行时）**：管理世界的运行时状态
2. **时间游标**：管理当前观察的时间刻
3. **命令执行**：提供命令执行接口

### 设计原则

- **时间游标**：管理当前观察的时间刻
- **命令执行**：提供命令执行接口
- **无 undo/redo**：不包含 undo/redo 逻辑（UI 特定概念）
- **无命令队列**：不包含命令队列，命令队列由 UI 层管理

## 文件结构

```
runtime/
├── mod.rs          # 模块入口，导出所有公共类型
└── runtime.rs      # 世界运行时
```

## 文件详解

### mod.rs

**文件路径**：`src/runtime/mod.rs`

**说明**：runtime 模块的入口文件，导出所有公共类型。

**导出内容**：
- `WorldRuntime`：世界运行时

**实现方式**：
```rust
pub mod runtime;

pub use runtime::WorldRuntime;
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### runtime.rs

**文件路径**：`src/runtime/runtime.rs`

**说明**：实现了 WorldRuntime，管理世界的运行时状态。

#### 主要类型

##### WorldRuntime

**定义**：
```rust
pub struct WorldRuntime {
    world: BasicWorld,
    cursor: u64,
}
```

**字段说明**：
- `world: BasicWorld`（私有）：世界实例
- `cursor: u64`（私有）：当前观察的时间刻

**实现方式**：
- 字段私有，确保封装性
- 管理时间游标
- 提供命令执行接口

**公共方法**：

1. **`new()`** - 创建新的世界运行时
   ```rust
   pub fn new() -> Self {
       let world = BasicWorld::new();
       let cursor = 0;

       Self { world, cursor }
   }
   ```
   - 创建空的世界
   - 初始化游标为 0

2. **`cursor()`** - 获取当前观察的时间刻
   ```rust
   pub fn cursor(&self) -> u64 {
       self.cursor
   }
   ```
   - 返回当前观察的时间刻

3. **`set_cursor(tick: u64)`** - 设置观察的时间刻
   ```rust
   pub fn set_cursor(&mut self, tick: u64) -> Result<(), String> {
       let latest_tick = self.world.latest_tick();

       if tick > latest_tick {
           return Err(format!(
               "Cannot set cursor to tick {}: exceeds latest tick {}",
               tick, latest_tick
           ));
       }

       self.cursor = tick;
       Ok(())
   }
   ```
   - 接受时间刻作为参数
   - 如果时间刻超出历史范围，返回错误
   - 否则设置游标

4. **`execute_command(command: &dyn Command)`** - 执行命令
   ```rust
   pub fn execute_command(&mut self, command: &dyn Command) -> Result<(), String> {
       command.apply(&mut self.world)?;
       
       let latest_tick = self.world.latest_tick();
       self.cursor = latest_tick;

       Ok(())
   }
   ```
   - 接受命令作为参数
   - 执行命令
   - 推进世界时间
   - 将游标移动到最新时间刻

5. **`current_snapshot()`** - 获取当前观察的状态快照
   ```rust
   pub fn current_snapshot(&self) -> Option<&StateSnapshot> {
       self.world.get_at(self.cursor)
   }
   ```
   - 返回当前观察的时间刻的状态快照

6. **`world()`** - 获取世界引用
   ```rust
   pub fn world(&self) -> &BasicWorld {
       &self.world
   }
   ```
   - 返回世界的引用

**Trait 实现**：

1. **StateQuery trait**：
   ```rust
   impl StateQuery for WorldRuntime {
       fn get_at(&self, tick: u64) -> Option<&StateSnapshot> {
           self.world.get_at(tick)
       }

       fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot> {
           self.world.query_range(start, end)
       }

       fn latest_snapshot(&self) -> Option<&StateSnapshot> {
           self.world.latest_snapshot()
       }
   }
   ```
   - 实现了 StateQuery trait
   - 提供状态查询接口

2. **RelationQuery trait**：
   ```rust
   impl RelationQuery for WorldRuntime {
       fn get_relation_at(&self, tick: u64) -> Option<&RelationChange> {
           self.world.get_relation_at(tick)
       }

       fn query_relations_range(&self, start: u64, end: u64) -> Vec<&RelationChange> {
           self.world.query_relations_range(start, end)
       }

       fn latest_relation_change(&self) -> Option<&RelationChange> {
           self.world.latest_relation_change()
       }
   }
   ```
   - 实现了 RelationQuery trait
   - 提供关系查询接口

3. **Default trait**：
   ```rust
   impl Default for WorldRuntime {
       fn default() -> Self {
           Self::new()
       }
   }
   ```
   - 提供默认实现

**设计约束**：
- 时间游标：管理当前观察的时间刻
- 命令执行：提供命令执行接口
- 无 undo/redo：不包含 undo/redo 逻辑（UI 特定概念）
- 无命令队列：不包含命令队列，命令队列由 UI 层管理

**优点**：
- 时间游标管理
- 命令执行接口
- 实现了 StateQuery 和 RelationQuery trait
- 不依赖 UI 框架

## 设计优点

### 1. 时间游标管理

WorldRuntime 管理当前观察的时间刻：
```rust
pub struct WorldRuntime {
    world: BasicWorld,
    cursor: u64,
}
```

### 2. 命令执行接口

WorldRuntime 提供命令执行接口：
```rust
pub fn execute_command(&mut self, command: &dyn Command) -> Result<(), String> {
    command.apply(&mut self.world)?;
    
    let latest_tick = self.world.latest_tick();
    self.cursor = latest_tick;

    Ok(())
}
```

### 3. 无 undo/redo 逻辑

WorldRuntime 不包含 undo/redo 逻辑（UI 特定概念）：
- undo/redo 通过 set_cursor 实现
- UI 层管理命令队列
- WorldRuntime 不关心命令的来源

### 4. 无命令队列

WorldRuntime 不包含命令队列，命令队列由 UI 层管理：
- UI 层管理命令队列
- UI 层调用 WorldRuntime::execute_command 执行命令
- UI 层通过 set_cursor 实现 undo/redo

### 5. 实现了 StateQuery 和 RelationQuery trait

WorldRuntime 实现了 StateQuery 和 RelationQuery trait：
```rust
impl StateQuery for WorldRuntime {
    fn get_at(&self, tick: u64) -> Option<&StateSnapshot> {
        self.world.get_at(tick)
    }

    fn query_range(&self, start: u64, end: u64) -> Vec<&StateSnapshot> {
        self.world.query_range(start, end)
    }

    fn latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.world.latest_snapshot()
    }
}
```

### 6. 不依赖 UI 框架

WorldRuntime 不依赖 UI 框架：
- WorldRuntime 管理时间游标，不关心 UI
- WorldRuntime 提供命令执行接口，不管理命令队列
- WorldRuntime 不包含 undo/redo 逻辑（UI 特定概念）

### 7. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问。

## 使用示例

### 创建世界运行时

```rust
use biosphere_foundation::runtime::WorldRuntime;

let runtime = WorldRuntime::new();
```

### 执行命令

```rust
use biosphere_foundation::runtime::WorldRuntime;
use biosphere_foundation::input::Command;
use biosphere_foundation::world::BasicWorld;

struct TestCommand;

impl Command for TestCommand {
    fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String> {
        runtime.step_world();
        Ok(())
    }
}

let mut runtime = WorldRuntime::new();
let command = TestCommand;

runtime.execute_command(&command)?;
```

### 设置时间游标

```rust
use biosphere_foundation::runtime::WorldRuntime;

let mut runtime = WorldRuntime::new();

runtime.set_cursor(0)?;
```

### 查询状态

```rust
use biosphere_foundation::runtime::WorldRuntime;

let runtime = WorldRuntime::new();

let snapshot = runtime.current_snapshot();
let latest = runtime.latest_snapshot();
```

## 总结

runtime 模块提供了 Biosphere 运行时系统的实现。它管理世界的运行时状态，包括时间游标和命令执行。

模块设计遵循以下原则：
- 时间游标：管理当前观察的时间刻
- 命令执行：提供命令执行接口
- 无 undo/redo：不包含 undo/redo 逻辑（UI 特定概念）
- 无命令队列：不包含命令队列，命令队列由 UI 层管理

WorldRuntime 提供了时间游标管理和命令执行接口，实现了 StateQuery 和 RelationQuery trait，不依赖 UI 框架。
