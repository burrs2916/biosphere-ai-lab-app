# Perception v0 宪法

## 核心原则

Perception 层是 Manifest 和 UI 之间的桥梁，负责将树形结构的 Manifest 展开为线性感知序列。

## 不可违背的约束

### 1. Perception 是线性感知序列

- ✅ **必须**：Perception 是线性序列，不是树形结构
- ✅ **必须**：Perception 是 Observation Trace，不是 View Tree
- ❌ **禁止**：引入 Widget、Layout、Container、Box、Rect 等 UI 概念
- ❌ **禁止**：在 Perception 中包含任何渲染语义

### 2. PerceptionEntry 是不可变的

- ✅ **必须**：PerceptionEntry 是不可变的数据结构
- ✅ **必须**：PerceptionEntry 只包含路径、深度、类型、值信息
- ❌ **禁止**：在 PerceptionEntry 中添加状态、事件、交互等 UI 语义
- ❌ **禁止**：PerceptionEntry 包含任何布局或渲染信息

### 3. ManifestPath 是纯定位工具

- ✅ **必须**：ManifestPath 是索引序列，用于定位节点
- ✅ **必须**：ManifestPath 从根节点开始，使用零基索引
- ❌ **禁止**：在 ManifestPath 中添加任何 UI 语义
- ❌ **禁止**：ManifestPath 包含任何渲染或布局信息

### 4. PerceptionBuilder 是感知策略

- ✅ **必须**：PerceptionBuilder 只定义"如何被看见"的策略
- ✅ **必须**：PerceptionBuilder 是纯函数，无副作用
- ❌ **禁止**：PerceptionBuilder 包含任何渲染逻辑
- ❌ **禁止**：PerceptionBuilder 依赖任何 UI 框架

### 5. 严格的时间绑定

- ✅ **必须**：Perception 绑定到特定时间点
- ✅ **必须**：Perception 是时间点的快照，不是时间段的流
- ❌ **禁止**：Perception 包含时间连续性语义
- ❌ **禁止**：Perception 包含动画、过渡等时间相关 UI 概念

## 边界定义

### Manifest → Perception

```
Manifest ——(如何被看见)——> Perception
```

- Manifest：世界是什么
- Perception：世界如何被看到
- 转换：感知策略，不是渲染策略

### Perception → Renderer

```
Perception ——(如何被呈现)——> Renderer
```

- Perception：感知结果
- Renderer：如何被呈现
- 呈现：渲染策略，不是 UI 框架

## 永久禁止

以下概念永远不允许出现在 Perception 层中：

1. **UI 控件**：Button、Input、List 等
2. **布局系统**：Flex、Grid、Stack 等
3. **事件系统**：Click、Hover、Focus 等
4. **样式系统**：Color、Font、Size 等
5. **动画系统**：Transition、Animation 等
6. **状态管理**：State、Store、Reducer 等

## 修改流程

任何对 Perception v0 宪法的修改必须：

1. 通过全体核心开发者讨论
2. 确保不违背核心原则
3. 更新本宪法文档
4. 更新所有相关文档和示例

## 版本历史

- **v0.0**：初始版本，确立核心原则和约束
- 维护者：Biosphere 核心团队
- 最后更新：2026-02-01

---

**记住：Perception 是 Observation Trace，不是 UI。**