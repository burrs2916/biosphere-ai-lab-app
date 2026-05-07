# biosphere-foundation

> **Constitutional Layer of Biosphere Architecture**  
> *Time-driven world, immutable history, one-way observability.*

---

## 1. Purpose / 目的

**biosphere-foundation** is the *constitutional layer* of the Biosphere system.

It defines **what a world is allowed to be**, **how time exists**, and **what can be observed** —
but **never** how it is rendered, interacted with, or controlled.

> Foundation answers *"What exists and what is allowed"*,  
> not *"What users do"* or *"What UI looks like"*.

中文解释：
> foundation 是整个系统的"宪法层"。
> 它只定义世界的存在方式、时间规则和可被观察的边界，
> **不关心 UI、不关心输入设备、不关心交互形式**。

---

## 2. Core Principles / 核心原则

### 2.1 Time Is Fundamental

> **Nothing changes without time advancing.**

- All state changes are triggered by time progression
- Time is monotonic and irreversible
- There is no concept of "instant mutation"

中文解释：
> 世界只能在时间推进之后发生变化。
> 不存在"偷偷修改状态"的可能。

---

### 2.2 Append-Only History

> **History never changes, only grows.**

- All world states are recorded as immutable snapshots
- Past states are never modified or deleted
- Observation does not affect history

中文解释：
> 历史是追加的，不可回写、不可撤销。
> 观察历史不会改变历史。

---

### 2.3 One-Way Information Flow

> **World → Conditions → Projection → Observer**

- Information flows outward only
- No external system can directly mutate the world
- Foundation never observes the observer

中文解释：
> 信息只能单向流动。
> 世界不会"看见"观察者。

---

### 2.4 Read-Only Observation

> **Observation is not interaction.**

- All exposed data is immutable
- Observers cannot influence evolution
- Queries have no side effects

中文解释：
> 查询不是干预。
> 所有观察都是只读的。

---

## 3. What Foundation IS / 它是什么

Foundation provides:

- A **time-driven world model** ([`BasicWorld`])
- A **state history system** ([`StateHistory`], [`StateStore`])
- A **constitutional query interface** ([`StateQuery`])
- A **conditions exposure mechanism** ([`SensedConditions`])
- A **projection-neutral observation base** ([`Projection`])

中文解释：
> foundation 提供"世界如何存在"的最低完备实现。

---

## 4. What Foundation IS NOT / 它不是什么

Foundation explicitly does **NOT** include:

- ❌ UI components
- ❌ Input devices (keyboard, mouse, touch, network)
- ❌ Event systems tied to users
- ❌ Layout, widgets, or interaction models
- ❌ Application logic

中文解释：
> foundation 绝不包含任何 UI 或交互语义。

---

## 5. Module Overview / 模块总览

```text
biosphere-foundation/
├── ontology/        # What exists
│   ├── basic_existence.rs
│   ├── basic_environment.rs
│   ├── basic_perception.rs
│   ├── basic_representation.rs
│   ├── basic_embodiment.rs
│   └── basic_field.rs
├── world/           # Time-driven world container
│   ├── basic_world.rs
│   ├── world_clock.rs
│   └── world_rules.rs
├── temporal/
│   ├── state/       # Immutable state history
│   │   ├── snapshot.rs
│   │   ├── payload.rs
│   │   ├── history.rs
│   │   ├── store.rs
│   │   ├── provider.rs
│   │   └── query.rs
│   └── relations/   # Immutable relationship history
│       ├── change.rs
│       ├── history.rs
│       ├── store.rs
│       └── query.rs
├── conditions/      # World → observer gate
│   └── sensed_conditions.rs
├── topology/        # Structural invariants
│   └── stable_topology.rs
├── invariants/      # World axioms
│   └── world_axioms.rs
└── projection/      # Read-only projections
    ├── projection.rs
    ├── ascii_view.rs
    └── timeline_view.rs
```

中文解释：
> 每个模块都有严格的职责边界，禁止跨界引用。

---

## 6. Temporal State Model / 时间状态模型

### 6.1 StateSnapshot

> An immutable record of world state at a specific tick.

- Identified by `tick`
- Contains opaque payload
- Never modified

中文解释：
> StateSnapshot 是时间点上的世界证据。

---

### 6.2 StateHistory

> Append-only ordered collection of StateSnapshots.

- Stores snapshots in time order
- Provides read-only iteration
- Implements [`StateQuery`]
- **Enforces temporal monotonicity at runtime**

中文解释：
> StateHistory 是"时间记忆"。
> 运行时保证时间单调性。

---

### 6.3 StateQuery

> Constitutional query interface over state history.

- Query by tick
- Query by time range
- Query latest state
- **The only legal way to observe history**

中文解释：
> StateQuery 是唯一合法的历史观察方式。

---

### 6.4 StateStore

> World-side state storage.

- Only [`BasicWorld`] can hold
- Provides commit interface only
- No mutation of past states

中文解释：
> StateStore 是"世界侧"的状态存储。

---

### 6.5 StateProvider

> Read-only state access interface.

- Provides current state
- Provides state history
- No write operations

中文解释：
> StateProvider 是只读的状态访问接口。

---

### 6.6 StatePayload

> Opaque container for state content.

- Foundation does not interpret payload meaning
- Type-safe through `Arc<dyn Any + Send + Sync>`
- **Opaque by design — semantic correctness is projection-layer concern**

中文解释：
> StatePayload 是"不透明的状态容器"。
> Foundation 不解释载荷的语义含义。

---

## 7. Temporal Constitutional Guarantees / Temporal 宪法保证

### 7.1 Foundation Does Not Guarantee Semantic Correctness

> **Foundation guarantees structural integrity, not semantic meaning.**

Foundation enforces:

- ✅ Time monotonicity
- ✅ Append-only history
- ✅ Immutable snapshots
- ✅ Read-only observation

Foundation does NOT guarantee:

- ❌ Payload semantic correctness
- ❌ Business logic validity
- ❌ Application-level invariants

中文解释：
> Foundation 保证结构完整性，不保证语义正确性。
> 语义正确性是 Projection 层的责任。

---

### 7.2 Payload Is Opaque By Design

> **StatePayload is intentionally opaque to Foundation.**

- Foundation stores payload without interpretation
- Foundation does not validate payload content
- Foundation does not enforce payload semantics

**Misuse of `StatePayload::downcast_ref` is a projection-layer fault, not a Temporal-layer concern.**

中文解释：
> StatePayload 对 Foundation 来说是黑盒。
> 滥用 downcast_ref 是 Projection 层的过错，不是 Temporal 层的问题。

---

### 7.3 Temporal Violations Are Detected At Runtime

> **Temporal monotonicity is enforced via `debug_assert!`.**

- `StateHistory::record` checks time monotonicity
- `RelationHistory::record` checks time monotonicity
- Violations panic in debug builds
- Release builds may skip checks for performance

中文解释：
> 时间单调性通过 debug_assert! 强制执行。
> 调试构建会 panic，发布构建可能跳过检查以提升性能。

---

## 8. World Model / 世界模型

### BasicWorld

> The minimal runnable world implementation.

- Owns time
- Owns state history
- Advances only through [`step_world()`]
- Implements [`StateProvider`] and [`StateQuery`]

中文解释：
> BasicWorld 是世界的最小可运行形态。

---

### WorldClock

> Monotonic, irreversible time progression.

- Starts at 0
- Advances by 1 tick per step
- Enforces [`WorldAxioms`]

中文解释：
> WorldClock 是"世界的时间"。

---

### WorldAxioms

> Constitutional constraints on world evolution.

- Time irreversibility
- No instant mutation
- Append-only history

中文解释：
> WorldAxioms 是"世界的宪法"。

---

## 9. Ontology / 存在论

### BasicExistence

> The five minimal dimensions for life existence.

- Boundary
- State
- Drive
- Rules
- Propagation

中文解释：
> BasicExistence 是"存在的五个最小维度"。

---

### BasicEnvironment

> The environment in which existence occurs.

- Manages time
- Generates conditions
- Enforces axioms

中文解释：
> BasicEnvironment 是"存在的环境"。

---

### BasicPerception, BasicRepresentation, BasicEmbodiment, BasicField

> The perceptual, representational, embodied, and field aspects of existence.

中文解释：
> 这些是存在的感知、表征、具身和场维度。

---

## 10. Conditions / 条件

### SensedConditions

> The only interface through which the world exposes its state.

- Read-only
- Time-stamped
- Immutable to observers

中文解释：
> SensedConditions 是世界对外暴露的唯一接口。

---

## 11. Projection / 投射

> Projections transform observable data into representations.

- Pure functions
- No side effects
- No knowledge of world internals
- Only use [`StateQuery`] to access history

Examples:

- [`AsciiView`] — current state projection
- [`TimelineView`] — historical time-axis projection

中文解释：
> 投射只是"呈现"，不是"解释"。

---

## 12. Boundary Rules / 边界规则（必须遵守）

### MUST

- MUST be time-driven
- MUST be append-only
- MUST be immutable to observers
- MUST use [`StateQuery`] to access history

### MUST NOT

- MUST NOT expose mutable state
- MUST NOT depend on UI or input
- MUST NOT observe observers
- MUST NOT access [`StateHistory`] internals directly

中文解释：
> 这些规则是不可违反的系统宪法。

---

## 13. Extension Policy / 扩展策略

Allowed future extensions:

- `input/` — neutral condition input abstractions
- `temporal/relations/` — relationship history

Constraints:

- Must obey one-way flow
- Must remain UI-agnostic
- Must use [`StateQuery`] for history access

中文解释：
> 扩展必须服从宪法，而不是反过来。

---

## 14. Philosophy Summary / 哲学总结

> **A world does not explain itself.**  
> **A world only exists, changes with time, and leaves traces.**

中文解释：
> 世界不解释自己，
> 世界只存在、演化、并留下痕迹。

---

## 15. Status / 状态

- ✅ Boundary-audited
- ✅ Minimal-complete
- ✅ [`StateQuery`] constitutionalized
- ✅ Temporal monotonicity enforced
- ✅ Snapshot/Change fields privatized
- ✅ Ready for higher layers

---

*This document is constitutional. Violating it means breaking the system.*
