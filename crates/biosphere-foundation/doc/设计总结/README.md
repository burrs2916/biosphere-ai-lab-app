# Biosphere Foundation 设计总结

## 概述

Biosphere Foundation 是 Biosphere 项目的核心基础层，提供了世界存在论的基本抽象和实现。它位于 biosphere-core（核心抽象层）和 biosphere-ui（应用层）之间，是整个系统的基石。

### 核心设计理念

Biosphere Foundation 遵循以下核心设计原则：

1. **分层架构**：严格遵循分层原则，Foundation 层只提供抽象接口和数据结构，不包含 UI 相关逻辑
2. **单向信息流**：World → Conditions → Projection → Observer，信息只能单向流动
3. **封装性**：所有结构体的字段都是私有的，通过公共方法提供只读访问
4. **不可变性**：世界状态一旦记录就不可修改，只能通过时间推进产生新状态
5. **类型安全**：充分利用 Rust 的类型系统，使用强类型而不是字符串
6. **可扩展性**：提供灵活的配置和扩展点，应用层可以自定义实现

### 模块组织

Foundation 层包含以下主要模块：

- **ontology**：存在论基础实现，包括环境、感知、表示等
- **world**：世界实现，包括 BasicWorld、WorldClock、WorldRules
- **topology**：拓扑结构，包括 StableTopology
- **conditions**：条件系统，包括 SensedConditions
- **projection**：投射系统，包括 Projection、AsciiView、TimelineView、SceneGraphView
- **invariants**：不变量约束，包括 WorldAxioms
- **temporal**：时间系统，包括状态历史、关系历史、查询接口
- **input**：输入系统，包括 ConditionInput、InputManager、Command
- **entity**：实体系统，包括 Entity、EntityManager、EntityQuery
- **runtime**：运行时系统，包括 WorldRuntime
- **error**：错误处理，包括 FoundationError
- **recovery**：错误恢复，包括 RecoveryStrategy、RecoveryResult

## 目录树

```
biosphere-foundation/
├── doc/
│   └── 设计总结/
│       ├── README.md                          # 本文档
│       ├── ontology.md                        # ontology 模块文档
│       ├── world.md                           # world 模块文档
│       ├── topology.md                        # topology 模块文档
│       ├── conditions.md                      # conditions 模块文档
│       ├── projection.md                      # projection 模块文档
│       ├── invariants.md                      # invariants 模块文档
│       ├── temporal.md                        # temporal 模块文档
│       ├── input.md                           # input 模块文档
│       ├── entity.md                          # entity 模块文档
│       ├── runtime.md                         # runtime 模块文档
│       ├── error.md                           # error 模块文档
│       └── recovery.md                        # recovery 模块文档
├── src/
│   ├── ontology/                              # 存在论模块
│   │   ├── mod.rs
│   │   ├── basic_environment.rs
│   │   ├── basic_embodiment.rs
│   │   ├── basic_existence.rs
│   │   ├── basic_perception.rs
│   │   └── basic_representation.rs
│   ├── world/                                 # 世界模块
│   │   ├── mod.rs
│   │   ├── basic_world.rs
│   │   ├── world_clock.rs
│   │   └── world_rules.rs
│   ├── topology/                              # 拓扑模块
│   │   ├── mod.rs
│   │   └── stable_topology.rs
│   ├── conditions/                            # 条件模块
│   │   ├── mod.rs
│   │   └── sensed_conditions.rs
│   ├── projection/                            # 投射模块
│   │   ├── mod.rs
│   │   ├── projection.rs
│   │   ├── ascii_view.rs
│   │   ├── timeline_view.rs
│   │   ├── properties.rs
│   │   └── scene_graph.rs
│   ├── invariants/                            # 不变量模块
│   │   ├── mod.rs
│   │   └── world_axioms.rs
│   ├── temporal/                              # 时间模块
│   │   ├── mod.rs
│   │   ├── state/
│   │   │   ├── mod.rs
│   │   │   ├── history.rs
│   │   │   ├── payload.rs
│   │   │   ├── provider.rs
│   │   │   ├── query.rs
│   │   │   ├── snapshot.rs
│   │   │   └── store.rs
│   │   ├── relations/
│   │   │   ├── mod.rs
│   │   │   ├── change.rs
│   │   │   ├── history.rs
│   │   │   ├── query.rs
│   │   │   └── store.rs
│   │   └── query.rs
│   ├── input/                                 # 输入模块
│   │   ├── mod.rs
│   │   ├── input.rs
│   │   ├── manager.rs
│   │   └── command.rs
│   ├── entity/                                # 实体模块
│   │   ├── mod.rs
│   │   ├── entity.rs
│   │   ├── filter.rs
│   │   ├── manager.rs
│   │   └── query.rs
│   ├── runtime/                               # 运行时模块
│   │   ├── mod.rs
│   │   └── runtime.rs
│   ├── error.rs                               # 错误处理
│   ├── recovery.rs                            # 错误恢复
│   └── lib.rs                                 # 库入口
├── Cargo.toml
└── README.md
```

## 文件树

### 核心文件

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/lib.rs` | 库入口文件，导出所有公共 API | 模块导出、类型别名 |
| `src/error.rs` | 错误类型定义 | FoundationError, FoundationResult |
| `src/recovery.rs` | 错误恢复策略 | RecoveryStrategy, RecoveryResult |

### ontology 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/ontology/mod.rs` | ontology 模块入口 | 模块导出 |
| `src/ontology/basic_environment.rs` | 基础环境实现 | BasicEnvironment, BasicEnvironmentState |
| `src/ontology/basic_embodiment.rs` | 基础具身实现 | BasicEmbodiment |
| `src/ontology/basic_existence.rs` | 基础存在实现 | BasicExistence |
| `src/ontology/basic_perception.rs` | 基础感知实现 | BasicPerception |
| `src/ontology/basic_representation.rs` | 基础表示实现 | BasicRepresentation, BasicRepresentationData |

### world 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/world/mod.rs` | world 模块入口 | 模块导出 |
| `src/world/basic_world.rs` | 基础世界实现 | BasicWorld, WorldState |
| `src/world/world_clock.rs` | 世界时钟 | WorldClock |
| `src/world/world_rules.rs` | 世界规则 | WorldRules |

### topology 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/topology/mod.rs` | topology 模块入口 | 模块导出 |
| `src/topology/stable_topology.rs` | 稳定拓扑实现 | StableTopology |

### conditions 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/conditions/mod.rs` | conditions 模块入口 | 模块导出 |
| `src/conditions/sensed_conditions.rs` | 感知条件实现 | SensedConditions |

### projection 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/projection/mod.rs` | projection 模块入口 | 模块导出 |
| `src/projection/projection.rs` | 投射 trait 定义 | Projection trait |
| `src/projection/ascii_view.rs` | ASCII 视图 trait | AsciiView trait |
| `src/projection/timeline_view.rs` | 时间线视图实现 | TimelineView, TimelineViewModel |
| `src/projection/properties.rs` | 属性视图 trait | PropertiesView trait |
| `src/projection/scene_graph.rs` | 场景图视图实现 | SceneGraphView, SceneGraphViewModel, SceneGraphNode, SceneGraphEdge |

### invariants 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/invariants/mod.rs` | invariants 模块入口 | 模块导出 |
| `src/invariants/world_axioms.rs` | 世界公理实现 | WorldAxioms, AxiomConfig, WorldAxiomViolation |

### temporal 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/temporal/mod.rs` | temporal 模块入口 | 模块导出 |
| `src/temporal/state/mod.rs` | 状态子模块入口 | 模块导出 |
| `src/temporal/state/history.rs` | 状态历史 | StateHistory |
| `src/temporal/state/payload.rs` | 状态载荷 | StatePayload |
| `src/temporal/state/provider.rs` | 状态提供者 | StateProvider trait |
| `src/temporal/state/query.rs` | 状态查询 | StateQuery trait |
| `src/temporal/state/snapshot.rs` | 状态快照 | StateSnapshot |
| `src/temporal/state/store.rs` | 状态存储 | StateStore |
| `src/temporal/relations/mod.rs` | 关系子模块入口 | 模块导出 |
| `src/temporal/relations/change.rs` | 关系变化 | RelationChange, RelationChangeKind |
| `src/temporal/relations/history.rs` | 关系历史 | RelationHistory |
| `src/temporal/relations/query.rs` | 关系查询 | RelationQuery trait |
| `src/temporal/relations/store.rs` | 关系存储 | RelationStore |
| `src/temporal/query.rs` | 查询工具 | LazyQueryIterator, LazyRelationQueryIterator, WindowedQuery, WindowedRelationQuery |

### input 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/input/mod.rs` | input 模块入口 | 模块导出 |
| `src/input/input.rs` | 条件输入接口 | ConditionInput trait |
| `src/input/manager.rs` | 输入管理器 | InputManager |
| `src/input/command.rs` | 命令接口 | Command trait |

### entity 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/entity/mod.rs` | entity 模块入口 | 模块导出、trait 实现 |
| `src/entity/entity.rs` | 实体定义 | Entity, EntityKind |
| `src/entity/filter.rs` | 实体过滤器 | EntityFilter |
| `src/entity/manager.rs` | 实体管理器 | EntityManager |
| `src/entity/query.rs` | 实体查询 | EntityQuery trait |

### runtime 模块

| 文件路径 | 说明 | 主要类型/结构 |
|---------|------|--------------|
| `src/runtime/mod.rs` | runtime 模块入口 | 模块导出 |
| `src/runtime/runtime.rs` | 世界运行时 | WorldRuntime |

## 设计优点

### 1. 清晰的分层架构

Foundation 层严格遵循分层原则，与 biosphere-core 和 biosphere-ui 层保持清晰的边界：

- **不依赖 UI 框架**：所有 Projection 只返回 ViewModel，不进行渲染
- **不包含 UI 特定概念**：不包含 undo/redo 逻辑（UI 特定概念）
- **只提供抽象接口**：Foundation 层只提供接口定义，具体实现由应用层负责

### 2. 单向信息流

信息只能单向流动：World → Conditions → Projection → Observer

- **SensedConditions** 是"单向闸门"，只允许信息从世界流向生命
- **Projection** 只读访问 Conditions，不修改任何状态
- **无法反推**：无法从 Projection 反推世界的完整状态

### 3. 良好的封装性

所有结构体的字段都是私有的，通过公共方法提供只读访问：

```rust
pub struct Entity {
    id: EntityId,      // 私有字段
    kind: EntityKind,  // 私有字段
}

impl Entity {
    pub fn id(&self) -> EntityId {      // 只读访问方法
        self.id
    }
    
    pub fn kind(&self) -> EntityKind {  // 只读访问方法
        self.kind
    }
}
```

### 4. 不可变性

世界状态一旦记录就不可修改，只能通过时间推进产生新状态：

- **StateSnapshot** 是不可变的
- **StateHistory** 只能添加新状态，不能修改已有状态
- **时间不可逆**：WorldAxioms::assert_time_irreversible 确保时间不能倒流

### 5. 类型安全

充分利用 Rust 的类型系统，使用强类型而不是字符串：

```rust
pub enum FoundationError {
    TemporalViolation { message: String },
    StateError { message: String },
    RelationError { message: String },
    // ...
}
```

### 6. 灵活的配置

提供灵活的配置和扩展点，应用层可以自定义实现：

```rust
pub struct AxiomConfig {
    pub allow_self_reference: bool,
    pub allow_cycles: bool,
}
```

### 7. 性能优化

- **惰性查询**：LazyQueryIterator 只在需要时才计算结果
- **窗口化查询**：WindowedQuery 支持大范围数据的分页显示
- **并发访问**：使用 Arc<RwLock> 支持并发访问

### 8. 完整的测试覆盖

所有模块都有完整的测试，测试代码质量高：

- 使用常量替代魔法数字
- 测试覆盖所有公共 API
- 包含边界条件和错误情况测试

### 9. 清晰的文档

所有公共 API 都有详细的文档：

- 包含设计约束说明
- 包含哲学含义解释
- 提供清晰的示例代码

### 10. 错误处理

- **统一的错误类型**：FoundationError 定义了所有错误类型
- **可恢复的错误**：所有错误都是可恢复的
- **错误恢复策略**：RecoveryStrategy 提供多种恢复策略

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      biosphere-ui                           │
│                    (应用层 - UI 特定)                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  biosphere-foundation                        │
│                  (基础实现层 - 抽象接口)                      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   ontology   │  │    world     │  │   topology   │      │
│  │  (存在论)    │  │   (世界)     │  │   (拓扑)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  conditions  │  │  projection  │  │  invariants  │      │
│  │   (条件)     │  │   (投射)     │  │  (不变量)    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   temporal   │  │    input     │  │    entity    │      │
│  │   (时间)     │  │   (输入)     │  │   (实体)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │   runtime    │  │   error      │                        │
│  │  (运行时)    │  │  (错误)      │                        │
│  └──────────────┘  └──────────────┘                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   biosphere-core                             │
│                  (核心抽象层 - trait 定义)                    │
└─────────────────────────────────────────────────────────────┘
```

## 信息流图

```
┌─────────────────────────────────────────────────────────────┐
│                         World                                │
│                    (BasicWorld)                              │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Environment  │  │ StateStore   │  │RelationStore │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Conditions                              │
│                    (SensedConditions)                        │
│                                                              │
│                    单向闸门 (只读)                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Projection                              │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ AsciiView    │  │ TimelineView │  │SceneGraphView│      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Observer                               │
│                    (UI 层 - 渲染和交互)                       │
└─────────────────────────────────────────────────────────────┘
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

### 使用投射

```rust
use biosphere_foundation::projection::TimelineView;

// 创建时间线视图
let timeline = TimelineView::unlimited();

// 渲染视图模型
let model = timeline.render(&world, 0, u64::MAX);

// 访问视图数据
println!("Ticks: {:?}", model.ticks());
```

### 查询历史

```rust
use biosphere_foundation::temporal::StateQuery;

// 查询特定时间刻的状态
let snapshot = world.get_at(10);

// 查询时间范围
let snapshots = world.query_range(0, 100);

// 获取最新状态
let latest = world.latest_snapshot();
```

## 总结

Biosphere Foundation 是一个设计精良、实现严谨的核心基础层。它严格遵循分层架构原则，提供了清晰的抽象接口和灵活的扩展点。通过单向信息流、良好的封装性、不可变性和类型安全等设计原则，确保了系统的稳定性、可维护性和可扩展性。

所有模块都有完整的测试覆盖和详细的文档，代码质量高，易于理解和使用。Foundation 层为上层应用提供了坚实的基础，是整个 Biosphere 项目的核心支柱。
