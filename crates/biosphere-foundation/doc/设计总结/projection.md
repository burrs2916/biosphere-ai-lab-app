# Projection 模块文档

## 概述

projection 模块提供了 Biosphere 投射系统的实现。Projection 描述如何将 Conditions 映射为视图模型，是 Conditions 的视图，可以被复制、投射。

### 核心概念

projection 模块的核心概念是：

1. **Projection（投射）**：将 Conditions 映射为视图模型的抽象
2. **AsciiView（ASCII 视图）**：ASCII 视图的抽象
3. **TimelineView（时间线视图）**：时间线视图的实现
4. **PropertiesView（属性视图）**：属性视图的抽象
5. **SceneGraphView（场景图视图）**：场景图视图的抽象

### 设计原则

- **可复制**：可以被复制
- **可投射**：可以被 Representation 映射和呈现
- **不可反推**：无法从 Projection 反推世界的完整状态
- **不包含渲染逻辑**：只返回视图模型，不进行渲染
- **只读访问**：只通过 Conditions 访问状态
- **不修改状态**：不提供任何修改接口
- **不持有状态**：不包含任何状态
- **不依赖 UI 框架**：不依赖具体的 UI 框架

## 文件结构

```
projection/
├── mod.rs              # 模块入口，导出所有公共类型
├── projection.rs       # 投射 trait 定义
├── ascii_view.rs       # ASCII 视图 trait
├── timeline_view.rs    # 时间线视图实现
├── properties.rs       # 属性视图 trait
└── scene_graph.rs      # 场景图视图实现
```

## 文件详解

### mod.rs

**文件路径**：`src/projection/mod.rs`

**说明**：projection 模块的入口文件，导出所有公共类型。

**导出内容**：
- `Projection`：投射 trait
- `AsciiView`：ASCII 视图 trait
- `TimelineView`：时间线视图
- `TimelineViewModel`：时间线视图模型
- `PropertiesView`：属性视图 trait
- `SceneGraphView`：场景图视图 trait
- `SceneGraphViewModel`：场景图视图模型
- `SceneGraphNode`：场景图节点
- `SceneGraphEdge`：场景图边

**实现方式**：
```rust
pub mod projection;
pub mod ascii_view;
pub mod timeline_view;
pub mod properties;
pub mod scene_graph;

pub use projection::Projection;
pub use ascii_view::AsciiView;
pub use timeline_view::{TimelineView, TimelineViewModel};
pub use properties::PropertiesView;
pub use scene_graph::{SceneGraphView, SceneGraphViewModel, SceneGraphNode, SceneGraphEdge};
```

**优点**：
- 清晰的模块组织
- 统一的导出接口
- 易于使用和维护

---

### projection.rs

**文件路径**：`src/projection/projection.rs`

**说明**：定义了 Projection trait，描述如何将 Conditions 映射为视图模型。

#### 主要 Trait

##### Projection

**定义**：
```rust
pub trait Projection {
    type ViewModel;

    fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
}
```

**关联类型**：
- `ViewModel`：视图模型类型

**方法**：

1. **`render(conditions: &dyn Conditions)`** - 返回投射的视图模型
   ```rust
   fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
   ```
   - 接受条件引用作为参数
   - 返回投射的视图模型
   - 只返回视图模型，不进行渲染
   - 渲染逻辑由 UI 层负责

**设计约束**：
- 只返回视图模型，不进行渲染
- 渲染逻辑由 UI 层负责
- Foundation 层只提供数据接口

**优点**：
- 清晰的抽象接口
- 可复制：可以被复制
- 可投射：可以被 Representation 映射和呈现
- 不可反推：无法从 Projection 反推世界的完整状态

---

### ascii_view.rs

**文件路径**：`src/projection/ascii_view.rs`

**说明**：定义了 AsciiView trait，描述 ASCII 视图的接口。

#### 主要 Trait

##### AsciiView

**定义**：
```rust
pub trait AsciiView {
    type ViewModel;

    fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
}
```

**关联类型**：
- `ViewModel`：视图模型类型

**方法**：

1. **`render(conditions: &dyn Conditions)`** - 渲染 ASCII 视图
   ```rust
   fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
   ```
   - 接受条件引用作为参数
   - 返回 ASCII 视图的视图模型

**设计约束**：
- 只读访问：只通过 Conditions 访问状态
- 不修改状态：不提供任何修改接口
- 不持有状态：不包含任何状态
- 不依赖 UI 框架：不依赖具体的 UI 框架
- 不包含渲染逻辑：不包含具体的渲染逻辑

**优点**：
- 清晰的抽象接口
- 只读访问
- 不依赖 UI 框架
- 不包含渲染逻辑

---

### timeline_view.rs

**文件路径**：`src/projection/timeline_view.rs`

**说明**：实现了时间线视图，描述时间轴视图的接口。

#### 主要类型

##### TimelineViewModel

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct TimelineViewModel {
    ticks: Vec<u64>,
}
```

**字段说明**：
- `ticks: Vec<u64>`（私有）：时间点列表

**实现方式**：
- 字段私有，确保封装性
- 只包含只读数据
- 不包含任何状态
- 可以序列化和反序列化

**公共方法**：

1. **`new(ticks: Vec<u64>)`** - 创建新的时间轴视图模型
   ```rust
   pub fn new(ticks: Vec<u64>) -> Self {
       Self { ticks }
   }
   ```
   - 接受时间点列表作为参数
   - 返回新的时间轴视图模型
   - 只读构造函数，时间点列表在构造时确定，之后不可修改

2. **`tick_count()`** - 获取时间点数量
   ```rust
   pub fn tick_count(&self) -> usize {
       self.ticks.len()
   }
   ```
   - 返回时间点的数量

3. **`is_empty()`** - 检查是否为空
   ```rust
   pub fn is_empty(&self) -> bool {
       self.ticks.is_empty()
   }
   ```
   - 如果没有时间点，返回 true，否则返回 false

4. **`ticks()`** - 获取所有时间点
   ```rust
   pub fn ticks(&self) -> &[u64] {
       &self.ticks
   }
   ```
   - 返回所有时间点的列表

---

##### TimelineView

**定义**：
```rust
pub struct TimelineView {
    limit: Option<usize>,
}
```

**字段说明**：
- `limit: Option<usize>`（私有）：可选的显示限制

**实现方式**：
- 字段私有，确保封装性
- 不持有状态
- 不依赖 UI 框架

**公共方法**：

1. **`new(limit: Option<usize>)`** - 创建新的时间轴视图
   ```rust
   pub fn new(limit: Option<usize>) -> Self {
       Self { limit }
   }
   ```
   - 接受可选的显示限制作为参数
   - None 表示显示全部历史
   - 返回新的时间轴视图

2. **`unlimited()`** - 创建无限制的时间轴视图
   ```rust
   pub fn unlimited() -> Self {
       Self::new(None)
   }
   ```
   - 创建无限制的时间轴视图
   - 显示全部历史

3. **`limited(n: usize)`** - 创建有限制的时间轴视图
   ```rust
   pub fn limited(n: usize) -> Self {
       Self::new(Some(n))
   }
   ```
   - 创建有限制的时间轴视图
   - 只显示最近 n 个时间点

4. **`render<P: StateProvider + StateQuery>(provider: &P, start: u64, end: u64)`** - 渲染时间轴
   ```rust
   pub fn render<P: StateProvider + StateQuery>(
       &self,
       provider: &P,
       start: u64,
       end: u64
   ) -> TimelineViewModel {
       let snapshots = provider.query_range(start, end);
       
       let ticks: Vec<u64> = snapshots.iter().map(|s| s.tick()).collect();
       
       let ticks = if let Some(limit) = self.limit {
           if ticks.len() > limit {
               ticks[ticks.len() - limit..].to_vec()
           } else {
               ticks
           }
       } else {
           ticks
       };
       
       TimelineViewModel { ticks }
   }
   ```
   - 查询时间范围内的状态快照
   - 提取时间点
   - 如果有限制，只保留最近的时间点
   - 返回时间轴视图模型

**优点**：
- 清晰的接口设计
- 支持限制显示数量
- 良好的封装性
- 不依赖 UI 框架

---

### properties.rs

**文件路径**：`src/projection/properties.rs`

**说明**：定义了 PropertiesView trait，描述属性视图的接口。

#### 主要 Trait

##### PropertiesView

**定义**：
```rust
pub trait PropertiesView<P: StateQuery + RelationQuery> {
    type ViewModel;

    fn render(&self, query: &P, entity_id: EntityId) -> Self::ViewModel;
}
```

**关联类型**：
- `ViewModel`：视图模型类型

**类型参数**：
- `P: StateQuery + RelationQuery`：必须实现 StateQuery 和 RelationQuery trait

**方法**：

1. **`render(query: &P, entity_id: EntityId)`** - 渲染属性视图
   ```rust
   fn render(&self, query: &P, entity_id: EntityId) -> Self::ViewModel;
   ```
   - 接受状态查询和实体 ID 作为参数
   - 返回属性视图的视图模型

**设计约束**：
- 只读访问：只通过 StateQuery 和 RelationQuery 访问状态
- 不修改状态：不提供任何修改接口
- 不持有状态：不包含任何状态
- 不依赖 UI 框架：不依赖具体的 UI 框架

**优点**：
- 清晰的抽象接口
- 只读访问
- 不依赖 UI 框架
- 支持实体级别的属性查询

---

### scene_graph.rs

**文件路径**：`src/projection/scene_graph.rs`

**说明**：实现了场景图视图，描述场景图视图的接口。

#### 主要类型

##### SceneGraphNode

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct SceneGraphNode {
    id: EntityId,
}
```

**字段说明**：
- `id: EntityId`（私有）：实体 ID

**实现方式**：
- 字段私有，确保封装性
- 表示场景图的节点

**公共方法**：

1. **`new(id: EntityId)`** - 创建新的场景图节点
   ```rust
   pub fn new(id: EntityId) -> Self {
       Self { id }
   }
   ```
   - 接受实体 ID 作为参数
   - 返回新的场景图节点

2. **`id()`** - 获取实体 ID
   ```rust
   pub fn id(&self) -> EntityId {
       self.id
   }
   ```
   - 返回实体 ID

---

##### SceneGraphEdge

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct SceneGraphEdge {
    source: EntityId,
    target: EntityId,
}
```

**字段说明**：
- `source: EntityId`（私有）：源实体 ID
- `target: EntityId`（私有）：目标实体 ID

**实现方式**：
- 字段私有，确保封装性
- 表示场景图的边

**公共方法**：

1. **`new(source: EntityId, target: EntityId)`** - 创建新的场景图边
   ```rust
   pub fn new(source: EntityId, target: EntityId) -> Self {
       Self { source, target }
   }
   ```
   - 接受源实体 ID 和目标实体 ID 作为参数
   - 返回新的场景图边

2. **`source()`** - 获取源实体 ID
   ```rust
   pub fn source(&self) -> EntityId {
       self.source
   }
   ```
   - 返回源实体 ID

3. **`target()`** - 获取目标实体 ID
   ```rust
   pub fn target(&self) -> EntityId {
       self.target
   }
   ```
   - 返回目标实体 ID

---

##### SceneGraphViewModel

**定义**：
```rust
#[derive(Debug, Clone)]
pub struct SceneGraphViewModel {
    nodes: Vec<SceneGraphNode>,
    edges: Vec<SceneGraphEdge>,
}
```

**字段说明**：
- `nodes: Vec<SceneGraphNode>`（私有）：节点列表
- `edges: Vec<SceneGraphEdge>`（私有）：边列表

**实现方式**：
- 字段私有，确保封装性
- 只包含只读数据
- 不包含任何状态
- 可以序列化和反序列化

**公共方法**：

1. **`new(nodes: Vec<SceneGraphNode>, edges: Vec<SceneGraphEdge>)`** - 创建新的场景图视图模型
   ```rust
   pub fn new(nodes: Vec<SceneGraphNode>, edges: Vec<SceneGraphEdge>) -> Self {
       Self { nodes, edges }
   }
   ```
   - 接受节点列表和边列表作为参数
   - 返回新的场景图视图模型
   - 只读构造函数，节点和边列表在构造时确定，之后不可修改

2. **`nodes()`** - 获取节点列表
   ```rust
   pub fn nodes(&self) -> &[SceneGraphNode] {
       &self.nodes
   }
   ```
   - 返回节点列表的引用

3. **`edges()`** - 获取边列表
   ```rust
   pub fn edges(&self) -> &[SceneGraphEdge] {
       &self.edges
   }
   ```
   - 返回边列表的引用

4. **`node_count()`** - 获取节点数量
   ```rust
   pub fn node_count(&self) -> usize {
       self.nodes.len()
   }
   ```
   - 返回节点的数量

5. **`edge_count()`** - 获取边数量
   ```rust
   pub fn edge_count(&self) -> usize {
       self.edges.len()
   }
   ```
   - 返回边的数量

---

##### SceneGraphView

**定义**：
```rust
pub trait SceneGraphView<P: StateQuery + RelationQuery> {
    fn render(&self, query: &P) -> SceneGraphViewModel;
}
```

**类型参数**：
- `P: StateQuery + RelationQuery`：必须实现 StateQuery 和 RelationQuery trait

**方法**：

1. **`render(query: &P)`** - 渲染场景图视图
   ```rust
   fn render(&self, query: &P) -> SceneGraphViewModel;
   ```
   - 接受状态查询作为参数
   - 返回场景图视图的视图模型

**设计约束**：
- 只读访问：只通过 StateQuery 和 RelationQuery 访问状态
- 不修改状态：不提供任何修改接口
- 不持有状态：不包含任何状态
- 不依赖 UI 框架：不依赖具体的 UI 框架

**优点**：
- 清晰的抽象接口
- 只读访问
- 不依赖 UI 框架
- 支持节点和边的查询

## 设计优点

### 1. 清晰的抽象接口

所有 Projection trait 都提供了清晰的抽象接口：
- `Projection`：投射的抽象
- `AsciiView`：ASCII 视图的抽象
- `TimelineView`：时间线视图的实现
- `PropertiesView`：属性视图的抽象
- `SceneGraphView`：场景图视图的抽象

### 2. 可复制和可投射

Projection 可以被复制和投射：
```rust
pub trait Projection {
    type ViewModel;
    fn render(&self, conditions: &dyn Conditions) -> Self::ViewModel;
}
```

### 3. 不可反推

无法从 Projection 反推世界的完整状态：
- Projection 只提供数据接口
- 不包含世界的完整信息

### 4. 不包含渲染逻辑

所有 Projection 都不包含渲染逻辑：
- 只返回视图模型
- 渲染逻辑由 UI 层负责

### 5. 只读访问

所有 Projection 都只读访问状态：
- 不修改状态
- 不提供任何修改接口

### 6. 不依赖 UI 框架

所有 Projection 都不依赖具体的 UI 框架：
- Foundation 层只提供数据接口
- UI 层负责渲染

### 7. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问：
```rust
pub struct TimelineViewModel {
    ticks: Vec<u64>,  // 私有字段
}
```

### 8. 支持限制显示

TimelineView 支持限制显示数量：
```rust
pub fn limited(n: usize) -> Self {
    Self::new(Some(n))
}
```

## 使用示例

### 使用 Projection

```rust
use biosphere_foundation::projection::Projection;
use biosphere_core::Conditions;

struct MyViewModel {
    data: String,
}

struct MyProjection;

impl Projection for MyProjection {
    type ViewModel = MyViewModel;

    fn render(&self, _conditions: &dyn Conditions) -> Self::ViewModel {
        MyViewModel { data: String::new() }
    }
}
```

### 使用 TimelineView

```rust
use biosphere_foundation::{BasicWorld, TimelineView};

let mut world = BasicWorld::new();

// 推进世界
for _ in 0..10 {
    world.step_world();
}

// 创建时间线视图
let timeline = TimelineView::unlimited();

// 渲染视图模型
let model = timeline.render(&world, 0, u64::MAX);

// 访问视图数据
println!("Ticks: {:?}", model.ticks());
println!("Tick count: {}", model.tick_count());
```

### 使用 SceneGraphView

```rust
use biosphere_foundation::projection::{SceneGraphView, SceneGraphViewModel, SceneGraphNode, SceneGraphEdge};
use biosphere_core::EntityId;

// 创建视图模型
let nodes = vec![
    SceneGraphNode::new(EntityId::new(1)),
    SceneGraphNode::new(EntityId::new(2)),
];
let edges = vec![
    SceneGraphEdge::new(EntityId::new(1), EntityId::new(2)),
];
let model = SceneGraphViewModel::new(nodes, edges);

// 访问视图数据
println!("Nodes: {:?}", model.nodes());
println!("Edges: {:?}", model.edges());
println!("Node count: {}", model.node_count());
println!("Edge count: {}", model.edge_count());
```

## 总结

projection 模块提供了 Biosphere 投射系统的实现。Projection 描述如何将 Conditions 映射为视图模型，是 Conditions 的视图，可以被复制、投射。

模块设计遵循以下原则：
- 可复制：可以被复制
- 可投射：可以被 Representation 映射和呈现
- 不可反推：无法从 Projection 反推世界的完整状态
- 不包含渲染逻辑：只返回视图模型，不进行渲染
- 只读访问：只通过 Conditions 访问状态
- 不修改状态：不提供任何修改接口
- 不持有状态：不包含任何状态
- 不依赖 UI 框架：不依赖具体的 UI 框架

所有类型都提供了良好的封装性，通过公共方法提供只读访问。TimelineView 支持限制显示数量，SceneGraphView 支持节点和边的查询。
