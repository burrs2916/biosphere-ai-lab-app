# biosphere-foundation

> **Constitutional Layer of the Biosphere Architecture**  
> *Time-driven world, immutable history, one-way observability.*

---

## 1. Purpose / 目的

**biosphere-foundation** is the *constitutional layer* of the Biosphere system.

It defines **what a world is allowed to be**, **how time exists**, and **what can be observed** —
but **never** how it is rendered, interacted with, or controlled.

> Foundation answers *"What exists and what is allowed"*,
> not *"What users do"* or *"What UI looks like"*.

中文解释：
> foundation 是整个系统的“宪法层”。
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
> 不存在“偷偷修改状态”的可能。

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
> 世界不会“看见”观察者。

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

- A **time-driven world model**
- A **state history system**
- A **conditions exposure mechanism**
- A **projection-neutral observation base**

中文解释：
> foundation 提供“世界如何存在”的最低完备实现。

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

```
biosphere-foundation/
├── ontology/        # What exists
├── world/           # Time-driven world container
├── temporal/
│   └── state/       # Immutable state history
├── conditions/      # World → observer gate
├── topology/        # Structural invariants
├── invariants/      # World axioms
└── projection/      # Read-only projections
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

中文解释：
> StateHistory 是“时间记忆”。

---

### 6.3 StateQuery

> Constitutional query interface over state history.

- Query by tick
- Query by time range
- Query latest state

中文解释：
> StateQuery 是唯一合法的历史观察方式。

---

## 7. World Model / 世界模型

### BasicWorld

> The minimal runnable world implementation.

- Owns time
- Owns state history
- Advances only through `step()`

中文解释：
> BasicWorld 是世界的最小可运行形态。

---

## 8. Projection / 投射

> Projections transform observable data into representations.

- Pure functions
- No side effects
- No knowledge of world internals

Examples:

- `AsciiView` — current state projection
- `TimelineView` — historical time-axis projection

中文解释：
> 投射只是“呈现”，不是“解释”。

---

## 9. Boundary Rules / 边界规则（必须遵守）

### MUST

- MUST be time-driven
- MUST be append-only
- MUST be immutable to observers

### MUST NOT

- MUST NOT expose mutable state
- MUST NOT depend on UI or input
- MUST NOT observe observers

中文解释：
> 这些规则是不可违反的系统宪法。

---

## 10. Extension Policy / 扩展策略

Allowed future extensions:

- `input/` — neutral condition input abstractions
- `temporal/relations/` — relationship history

Constraints:

- Must obey one-way flow
- Must remain UI-agnostic

中文解释：
> 扩展必须服从宪法，而不是反过来。

---

## 11. Philosophy Summary / 哲学总结

> **A world does not explain itself.**  
> **A world only exists, changes with time, and leaves traces.**

中文解释：
> 世界不解释自己，
> 世界只存在、演化、并留下痕迹。

---

## 12. Status

- Boundary-audited
- Minimal-complete
- Ready for higher layers

---

*This document is constitutional. Violating it means breaking the system.*
