# Input 模块文档

## 概述

input 模块提供了 Biosphere 输入系统的实现。它定义了条件输入的抽象接口和输入管理器。

### 核心概念

input 模块的核心概念是：

1. **ConditionInput（条件输入）**：生成条件信号的抽象接口
2. **InputManager（输入管理器）**：管理多个条件输入源
3. **Command（命令）**：定义命令的接口

### 设计原则

- **中立接口**：不包含任何设备语义
- **不指定来源**：只定义如何生成信号
- **可验证**：提供信号验证能力
- **单向执行**：只提供 `apply` 方法，不提供 `undo` 和 `redo` 方法
- **时间驱动**：Command 只负责产生新的 StateSnapshot / RelationChange

## 文件结构

```
input/
├── mod.rs          # 模块入口，导出所有公共类型
├── input.rs        # 条件输入接口
├── manager.rs      # 输入管理器
└── command.rs      # 命令接口
```

## 文件详解

### mod.rs

**文件路径**：`src/input/mod.rs`

**说明**：input 模块的入口文件，导出所有公共类型。

**导出内容**：
- `ConditionInput`：条件输入 trait
- `InputManager`：输入管理器
- `Command`：命令 trait

**实现方式**：
```rust
pub mod input;
pub mod manager;
pub mod command;

pub use input::ConditionInput;
pub use manager::InputManager;
pub use command::Command;
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### input.rs

**文件路径**：`src/input/input.rs`

**说明**：定义了 ConditionInput trait，条件输入接口。

#### 主要 Trait

##### ConditionInput

**定义**：
```rust
pub trait ConditionInput {
    fn generate(&self) -> Vec<ConditionSignal>;
    fn validate(&self, signal: &ConditionSignal) -> bool;
}
```

**方法**：

1. **`generate()`** - 生成条件信号
   ```rust
   fn generate(&self) -> Vec<ConditionSignal>;
   ```
   - 返回生成的条件信号列表
   - 不指定信号来源

2. **`validate(signal: &ConditionSignal)`** - 验证条件信号
   ```rust
   fn validate(&self, signal: &ConditionSignal) -> bool;
   ```
   - 接受要验证的条件信号作为参数
   - 如果信号有效，返回 true，否则返回 false

**设计约束**：
- 中立接口：不包含任何设备语义
- 不指定来源：只定义如何生成信号
- 可验证：提供信号验证能力

**优点**：
- 清晰的抽象接口
- 中立性
- 可扩展性

**注意**：
- Foundation 不知道信号来自哪里
- Foundation 只知道"有人给了我一些 ConditionSignal"
- 具体的输入源在 UI 层实现

---

### manager.rs

**文件路径**：`src/input/manager.rs`

**说明**：实现了 InputManager，管理多个条件输入源。

#### 主要类型

##### InputManager

**定义**：
```rust
pub struct InputManager {
    sources: Vec<Box<dyn ConditionInput>>,
    queue: VecDeque<ConditionSignal>,
}
```

**字段说明**：
- `sources: Vec<Box<dyn ConditionInput>>`（私有）：输入源列表
- `queue: VecDeque<ConditionSignal>`（私有）：信号队列

**实现方式**：
- 字段私有，确保封装性
- 支持多个输入源

**公共方法**：

1. **`new()`** - 创建新的输入管理器
   ```rust
   pub fn new() -> Self {
       Self {
           sources: Vec::new(),
           queue: VecDeque::new(),
       }
   }
   ```
   - 创建空的管理器

2. **`add_source(source: Box<dyn ConditionInput>)`** - 添加输入源
   ```rust
   pub fn add_source(&mut self, source: Box<dyn ConditionInput>) {
       self.sources.push(source);
   }
   ```
   - 接受输入源作为参数
   - 添加到输入源列表

3. **`process()`** - 处理所有输入源
   ```rust
   pub fn process(&mut self) -> Vec<ConditionSignal> {
       let mut valid_signals = Vec::new();

       for source in &self.sources {
           for signal in source.generate() {
               if self.sources.iter().any(|s| s.validate(&signal)) {
                   self.queue.push_back(signal.clone());
                   valid_signals.push(signal);
               }
           }
       }

       valid_signals
   }
   ```
   - 收集所有输入源的信号
   - 验证信号有效性
   - 将有效信号加入队列
   - 返回生成的有效信号列表

4. **`next()`** - 获取队列中的下一个信号
   ```rust
   pub fn next(&mut self) -> Option<ConditionSignal> {
       self.queue.pop_front()
   }
   ```
   - 返回下一个信号
   - 如果队列为空，返回 None

5. **`peek_all()`** - 获取队列中的所有信号
   ```rust
   pub fn peek_all(&self) -> Vec<&ConditionSignal> {
       self.queue.iter().collect()
   }
   ```
   - 返回所有信号的引用
   - 不修改队列

6. **`is_empty()`** - 检查队列是否为空
   ```rust
   pub fn is_empty(&self) -> bool {
       self.queue.is_empty()
   }
   ```
   - 如果队列为空，返回 true

**设计约束**：
- 中立管理：不知道具体输入源是什么
- 信号验证：只接受有效的信号
- 不包含 UI 语义：不关心输入设备类型
- 不包含优先级逻辑：优先级处理由应用层负责

**优点**：
- 清晰的管理接口
- 中立性
- 可扩展性
- 信号验证

---

### command.rs

**文件路径**：`src/input/command.rs`

**说明**：定义了 Command trait，命令接口。

#### 主要 Trait

##### Command

**定义**：
```rust
pub trait Command {
    fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String>;
}
```

**方法**：

1. **`apply(runtime: &mut BasicWorld)`** - 应用命令
   ```rust
   fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String>;
   ```
   - 接受世界运行时作为参数
   - 修改世界状态
   - 产生新的 StateSnapshot
   - 产生新的 RelationChange（如果需要）
   - 如果成功，返回 Ok(())

**设计约束**：
- 单向执行：只提供 `apply` 方法，不提供 `undo` 和 `redo` 方法
- 时间驱动：Command 只负责产生新的 StateSnapshot / RelationChange
- 业务承载：Command 承担业务爆炸，Intent 保持稳定

**为什么不包含 undo 和 redo？**

在传统的命令模式中，Command 通常包含 `execute`、`undo` 和 `redo` 方法。但在这个时间驱动的系统中，这种设计是错误的。

原因：
1. **时间不可逆**：世界历史是 append-only 的，不能被修改
2. **时间游标**：undo/redo 是 WorldRuntime 的时间游标行为
3. **单向执行**：Command 只负责产生新的 StateSnapshot / RelationChange

正确的做法：
- Command 只包含 `apply` 方法
- WorldRuntime 管理时间游标
- undo/redo 通过改变观察 tick 实现

**优点**：
- 清晰的抽象接口
- 单向执行
- 时间驱动
- 业务承载

**注意**：
- undo/redo 是 WorldRuntime 的时间游标行为，不是 command 的职责
- 不要在 Command trait 中添加 undo 和 redo 方法

## 设计优点

### 1. 中立性

ConditionInput 是中立的输入接口，不包含任何设备语义：
```rust
pub trait ConditionInput {
    fn generate(&self) -> Vec<ConditionSignal>;
    fn validate(&self, signal: &ConditionSignal) -> bool;
}
```

### 2. 可扩展性

InputManager 支持多个输入源：
```rust
pub struct InputManager {
    sources: Vec<Box<dyn ConditionInput>>,
    queue: VecDeque<ConditionSignal>,
}
```

### 3. 信号验证

InputManager 提供信号验证能力：
```rust
for source in &self.sources {
    for signal in source.generate() {
        if self.sources.iter().any(|s| s.validate(&signal)) {
            self.queue.push_back(signal.clone());
            valid_signals.push(signal);
        }
    }
}
```

### 4. 单向执行

Command 只提供 `apply` 方法：
```rust
pub trait Command {
    fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String>;
}
```

### 5. 时间驱动

Command 只负责产生新的 StateSnapshot / RelationChange：
- Command 不修改世界历史
- Command 不实现 undo 逻辑
- undo/redo 是 WorldRuntime 的时间游标行为

### 6. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问。

## 使用示例

### 创建条件输入

```rust
use biosphere_foundation::input::ConditionInput;
use biosphere_core::ConditionSignal;

struct MyInput;

impl ConditionInput for MyInput {
    fn generate(&self) -> Vec<ConditionSignal> {
        vec![
            ConditionSignal {
                kind: "key",
                intensity: 1.0,
            },
        ]
    }

    fn validate(&self, _signal: &ConditionSignal) -> bool {
        true
    }
}
```

### 使用输入管理器

```rust
use biosphere_foundation::input::InputManager;

let mut manager = InputManager::new();
let input = MyInput;
manager.add_source(Box::new(input));

let signals = manager.process();
```

### 创建命令

```rust
use biosphere_foundation::input::Command;
use biosphere_foundation::BasicWorld;

struct MyCommand;

impl Command for MyCommand {
    fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String> {
        runtime.step_world();
        Ok(())
    }
}

let mut world = BasicWorld::new();
let command = MyCommand;

command.apply(&mut world)?;
```

## 总结

input 模块提供了 Biosphere 输入系统的实现。它定义了条件输入的抽象接口和输入管理器。

模块设计遵循以下原则：
- 中立接口：不包含任何设备语义
- 不指定来源：只定义如何生成信号
- 可验证：提供信号验证能力
- 单向执行：只提供 `apply` 方法，不提供 `undo` 和 `redo` 方法
- 时间驱动：Command 只负责产生新的 StateSnapshot / RelationChange

所有类型都提供了良好的封装性，通过公共方法提供只读访问。InputManager 支持多个输入源，Command 是时间驱动的单向执行接口。
