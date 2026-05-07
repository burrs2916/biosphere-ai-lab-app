# Manifest 设计文档

## 概述

Manifest 是系统对外部世界的最小可感知声明。它是 UI 渲染的唯一输入，也是系统在没有 UI 的情况下仍然存在的证据。

## 核心概念

### Manifest 的定位

**Manifest 不是 UI 的最小感知单元，而是系统对外部世界的最小可感知声明。**

这个定位决定了：
- Manifest 属于 foundation 层，而不是 UI 层
- UI 只是 Manifest 的一种消费者
- 系统在没有 UI 的情况下仍然完整
- Manifest 可以被多个消费者使用（Desktop UI、CLI、AI Observer、Audit Log 等）

### 依赖关系

```
foundation → UI
```

UI 依赖 foundation，foundation 不依赖 UI。Manifest 在 foundation 中，意味着：
- 世界模型不包含 UI 语义
- UI 只是 Manifest 的一种消费者
- 世界模型保持纯净

## 核心公式

```
Snapshot --Derivation--> Manifest --Renderer--> View
```

这个公式成为整个系统的物理定律。

## 设计约束

### Manifest v0 必须满足

- 不可变
- 纯数据
- 时间绑定
- 可比较
- 可组合
- 可被多个 Renderer 消费
- 不含 UI 概念

## 核心类型

### Manifest 本体

```rust
pub struct Manifest {
    pub time: u64,                // 来自 Snapshot
    pub root: ManifestNode,       // 感知结构根
}
```

### ManifestNode（最小感知原子）

```rust
pub struct ManifestNode {
    pub kind: NodeKind,
    pub value: Value,
    pub children: Vec<ManifestNode>,
}

pub enum NodeKind {
    Scalar,     // 单一感知
    Group,      // 组合感知
    Sequence,   // 有序感知
}
```

### Value（可感知值的封闭集合）

```rust
pub enum Value {
    Number(f64),
    Text(String),
    Boolean(bool),
    Tuple(Vec<Value>),
    Map(Vec<(String, Value)>),
    Opaque(String),
}
```

## NodeKind 的设计

这三个类型覆盖了所有可能的感知结构：
- **Scalar**：单一值（如数字、文本）
- **Group**：无序组合（如属性集合）
- **Sequence**：有序组合（如列表、树）

简单、完整、无冗余。

## Value 的设计

这个设计：
- 覆盖了常见数据类型
- 支持嵌套结构（Tuple、Map）
- 提供了扩展点（Opaque）
- 便于序列化和传输

## Derivation Trait

```rust
pub trait Derivation {
    fn derive(&self, snapshot: &StateSnapshot) -> Manifest;
}
```

Derivation 是从 Snapshot 到 Manifest 的转换逻辑。每个 Derivation 实现定义了一种"如何从世界状态中提取可感知信息"的方式。

## Diff 接口（v0 只立接口）

```rust
pub trait ManifestDiff {
    fn diff(&self, other: &Manifest) -> Vec<DiffOperation>;
}

pub enum DiffOperation {
    Insert { path: Vec<usize>, node: ManifestNode },
    Update { path: Vec<usize>, old_value: Value, new_value: Value },
    Delete { path: Vec<usize>, node: ManifestNode },
    Move { from_path: Vec<usize>, to_path: Vec<usize>, node: ManifestNode },
}
```

Diff 是宪法级预留，不是现在实现。它为未来的增量更新提供了接口。

## 工程判据

**如果某个概念在没有 UI 的情况下仍然成立，它就不属于 UI 层**

来套一下：

| 概念 | 没 UI 还成立吗？ |
|------|----------------|
| Widget | ❌ |
| Layout | ❌ |
| Event | ❌ |
| Focus | ❌ |
| View | ❌ |
| Manifest | ✅ |
| Snapshot | ✅ |
| Transform | ✅ |

这就是答案。

## 多消费者宇宙

Manifest 支持多种消费者：
- Desktop UI
- CLI / TUI
- Remote Debugger
- AI Observer
- Audit Log
- Replay System
- Headless Verification

这些消费者需要一种公共语言，不是 UI、不是事件、不是像素，而是 Manifest。

## 示例

### 创建简单的 Manifest

```rust
use biosphere_foundation::manifest::{Manifest, ManifestNode, NodeKind, Value};

let root = ManifestNode::scalar(Value::text("Hello, World!"));
let manifest = Manifest::new(42, root);
```

### 创建复杂的 Manifest

```rust
let root = ManifestNode::group(
    Value::text("WorldState"),
    vec![
        ManifestNode::group(
            Value::text("Entities"),
            vec![
                ManifestNode::scalar(Value::text("Entity #1")),
                ManifestNode::scalar(Value::text("Entity #2")),
            ],
        ),
        ManifestNode::group(
            Value::text("Environment"),
            vec![
                ManifestNode::scalar(Value::text("Temperature: 25.0")),
                ManifestNode::scalar(Value::text("Humidity: 60.0")),
            ],
        ),
    ],
);

let manifest = Manifest::new(100, root);
```

### 从 Snapshot 创建 Manifest

```rust
use biosphere_foundation::manifest::{Manifest, Derivation};

struct DebugDerivation;

impl Derivation for DebugDerivation {
    fn derive(&self, snapshot: &StateSnapshot) -> Manifest {
        let tick = snapshot.tick().value();
        
        let root = ManifestNode::group(
            Value::text("DebugView"),
            vec![
                ManifestNode::scalar(Value::text(format!("Tick: {}", tick))),
            ],
        );
        
        Manifest::new(tick, root)
    }
}

let derivation = DebugDerivation;
let manifest = derivation.derive(&snapshot);
```

## 架构意义

Manifest 的引入解决了以下问题：

1. **防止 UI 语义反向污染世界模型**
   - Manifest 在 foundation 中，UI 只是消费者
   - 世界模型保持纯净

2. **支持多消费者宇宙**
   - 多种消费者可以共享同一 Manifest
   - 不需要为每种消费者重复实现

3. **解耦 UI 和世界模型**
   - UI 不直接访问世界模型
   - 通过 Manifest 作为中间层

4. **提供清晰的架构边界**
   - foundation 负责状态和感知
   - UI 负责渲染和交互

## 未来扩展

### Value 的扩展

如果需要新的 Value 类型，可以添加到 Value 枚举中。但要确保：
- 类型是纯数据
- 类型可以被序列化和传输
- 类型不包含 UI 概念

### NodeKind 的扩展

如果需要新的 NodeKind，可以添加到 NodeKind 枚举中。但要确保：
- 新的 Kind 能够覆盖新的感知结构
- 新的 Kind 与现有的 Kind 不重复

### Diff 的实现

未来可以实现 ManifestDiff trait，提供增量更新能力。但要确保：
- Diff 操作是幂等的
- Diff 操作是可逆的
- Diff 操作不破坏不可变性

## 总结

Manifest 是系统对外部世界的最小可感知声明。它在 foundation 中，UI 只是它的一种消费者。这个设计：
- ✅ 依赖方向正确
- ✅ 可替换性验证通过
- ✅ 支持多消费者宇宙
- ✅ 防止 UI 语义反向污染
- ✅ Manifest v0 设计简洁、完整、无冗余
