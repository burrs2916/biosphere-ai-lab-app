# Biosphere AI - 技术设计文档

## 一、项目概述

### 1.1 定位

`biosphere-ai` 是 Biosphere 生命系统的**感知层**，负责将外部信息转化为系统可理解的形式。

```
biosphere-core (生命公理)
      ↓
biosphere-biology (生物模型)
      ↓
biosphere-art (艺术表现)
      ↓
biosphere-ai (感知认知) ← 本模块
      ↓
biosphere-app (应用交互)
```

### 1.2 核心理念

遵循 Biosphere 哲学，`biosphere-ai` 不是一个"工具"，而是一个"感知器官"：

- **不是**：被动执行命令的工具
- **而是**：具有边界的自主感知系统

### 1.3 产品目标

构建一个**单机版 AI 视觉生成工具**：

```
用户上传图像 → AI 感知分析 → 生成粒子配置 → 渲染动画
```

---

## 二、哲学映射

### 2.1 五个存在维度

Biosphere 定义了生命存在的五个最小维度，`biosphere-ai` 将其映射到 AI 系统：

| 维度 | 含义 | AI 映射 | 实现文件 |
|------|------|---------|----------|
| **边界 (Boundary)** | 自我与环境的区别 | 输入/输出的类型边界 | `perception/boundary.rs` |
| **状态 (State)** | 内部配置和条件 | 模型参数、Tensor 状态 | `perception/state.rs` |
| **驱动 (Drive)** | 内部动机和倾向 | 学习目标、优化方向 | `perception/drive.rs` |
| **规则 (Rule)** | 约束和行为 | 神经网络层、变换规则 | `perception/rule.rs` |
| **传播 (Propagation)** | 变化和交互的机制 | 前向传播、信息流动 | `perception/propagation.rs` |

### 2.2 信息流动

Biosphere 的信息流动是单向的：

```
系统 → 环境 → 条件 → 投射 → 观察者
```

在 `biosphere-ai` 中：

```
输入图像 → 感知处理 → 特征提取 → 粒子配置 → 渲染输出
```

### 2.3 时间特性

- **不可逆**：推理只能向前，不能回溯
- **仅追加**：处理历史只能增长
- **不可变**：过去的处理结果不能修改

---

## 三、技术架构

### 3.1 依赖关系

```
biosphere-ai
├── burn (0.20)           # 深度学习框架
│   ├── burn-core         # 核心 Tensor 操作
│   ├── burn-wgpu         # GPU 加速后端
│   └── burn-fusion       # 算子融合优化
├── burn-vision (0.20)    # 图像处理模块
├── serde                 # 序列化
└── thiserror             # 错误处理
```

### 3.2 目录结构

```
biosphere-ai/
├── Cargo.toml
├── docs/
│   └── TECHNICAL_DESIGN.md
├── src/
│   ├── lib.rs                    # 模块入口
│   │
│   ├── perception/               # 感知模块（核心）
│   │   ├── mod.rs
│   │   ├── boundary.rs           # 边界定义
│   │   ├── state.rs              # 状态管理
│   │   ├── drive.rs              # 驱动目标
│   │   ├── rule.rs               # 变换规则
│   │   └── propagation.rs        # 信息传播
│   │
│   ├── image/                    # 图像处理
│   │   ├── mod.rs
│   │   ├── loader.rs             # 图像加载
│   │   ├── color.rs              # 颜色提取
│   │   ├── edge.rs               # 边缘检测
│   │   └── region.rs             # 区域分割
│   │
│   ├── feature/                  # 特征提取
│   │   ├── mod.rs
│   │   ├── color_feature.rs      # 颜色特征
│   │   ├── shape_feature.rs      # 形状特征
│   │   └── texture_feature.rs    # 纹理特征
│   │
│   ├── particle/                 # 粒子配置生成
│   │   ├── mod.rs
│   │   ├── config.rs             # 粒子配置定义
│   │   ├── generator.rs          # 粒子生成器
│   │   └── mapper.rs             # 特征映射
│   │
│   └── model/                    # 神经网络模型（未来）
│       ├── mod.rs
│       └── pretrained/           # 预训练模型
```

### 3.3 模块职责

#### 3.3.1 perception/ - 感知核心

实现 Biosphere 五个存在维度的 AI 映射：

```rust
// boundary.rs - 边界定义
pub struct PerceptionInput {
    pub image_data: Tensor<B, 3>,  // HWC 格式
    pub width: u32,
    pub height: u32,
}

pub struct PerceptionOutput {
    pub particle_config: ParticleConfig,
}

// state.rs - 状态管理
pub struct PerceptionState<B: Backend> {
    pub parameters: Vec<Tensor<B, 2>>,
    pub intermediate: Option<Tensor<B, 3>>,
}

// rule.rs - 变换规则
pub trait PerceptionRule<B: Backend>: Send + Sync {
    fn apply(&self, input: &Tensor<B, 3>) -> Tensor<B, 3>;
}

// propagation.rs - 信息传播
pub fn propagate<B: Backend>(
    input: PerceptionInput,
    rules: &[Box<dyn PerceptionRule<B>>],
) -> PerceptionOutput;
```

#### 3.3.2 image/ - 图像处理

基于 `burn-vision` 实现图像处理功能：

```rust
// loader.rs - 图像加载
pub fn load_image(path: &Path) -> Result<Tensor<B, 3>, ImageError>;

// color.rs - 颜色提取
pub fn extract_dominant_colors(tensor: &Tensor<B, 3>, k: usize) -> Vec<Color>;

// edge.rs - 边缘检测
pub fn detect_edges(tensor: &Tensor<B, 3>) -> Tensor<B, 2>;

// region.rs - 区域分割
pub fn segment_regions(tensor: &Tensor<B, 3>) -> Vec<Region>;
```

#### 3.3.3 feature/ - 特征提取

从图像中提取有意义的特征：

```rust
// color_feature.rs - 颜色特征
pub struct ColorFeature {
    pub dominant_colors: Vec<Color>,
    pub color_distribution: Vec<f32>,
    pub brightness: f32,
    pub saturation: f32,
}

// shape_feature.rs - 形状特征
pub struct ShapeFeature {
    pub contours: Vec<Contour>,
    pub bounding_boxes: Vec<BoundingBox>,
    pub center_of_mass: Point,
}

// texture_feature.rs - 纹理特征
pub struct TextureFeature {
    pub density: f32,
    pub complexity: f32,
    pub pattern_type: PatternType,
}
```

#### 3.3.4 particle/ - 粒子配置

生成 `biosphere-art` 可用的粒子配置：

```rust
// config.rs - 粒子配置定义
pub struct ParticleConfig {
    pub positions: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub sizes: Vec<f32>,
    pub velocities: Vec<[f32; 2]>,
    pub lifetimes: Vec<f32>,
}

// mapper.rs - 特征映射
pub fn map_features_to_particles(
    color_feature: &ColorFeature,
    shape_feature: &ShapeFeature,
    texture_feature: &TextureFeature,
) -> ParticleConfig;
```

---

## 四、数据流

### 4.1 主数据流

```
┌─────────────┐
│  用户图像    │
└──────┬──────┘
       ↓
┌─────────────┐
│ image/      │
│ loader.rs   │ → Tensor<B, 3>
└──────┬──────┘
       ↓
┌─────────────┐
│ image/      │
│ color.rs    │ → ColorFeature
│ edge.rs     │ → ShapeFeature
│ region.rs   │ → TextureFeature
└──────┬──────┘
       ↓
┌─────────────┐
│ particle/   │
│ mapper.rs   │ → ParticleConfig
└──────┬──────┘
       ↓
┌─────────────┐
│ biosphere-  │
│ art         │ → 渲染动画
└─────────────┘
```

### 4.2 Tensor 维度约定

| 阶段 | 维度 | 说明 |
|------|------|------|
| 输入图像 | `[H, W, C]` | 高度、宽度、通道 |
| 灰度图 | `[H, W]` | 单通道 |
| 特征图 | `[H, W, F]` | F 为特征数量 |
| 粒子数据 | `[N, D]` | N 为粒子数，D 为属性维度 |

---

## 五、技术实现

### 5.1 Burn 框架使用

#### 5.1.1 Backend 选择

```rust
use burn::backend::Wgpu;

pub type AIBackend = Wgpu;
```

WGPU 后端优势：
- 跨平台（Windows/Mac/Linux）
- 使用本地 GPU（无需 CUDA）
- 支持核显

#### 5.1.2 Tensor 操作

```rust
use burn::tensor::Tensor;

// 创建 Tensor
let tensor = Tensor::<AIBackend, 3>::from_data(data, &device);

// 基本操作
let normalized = (tensor - tensor.min()) / (tensor.max() - tensor.min());
let reshaped = tensor.reshape([new_shape]);
let transposed = tensor.transpose(0, 1);
```

#### 5.1.3 图像处理

```rust
use burn_vision::ops::*;

// 连通区域分析
let components = connected_components(&binary_image);

// 形态学操作
let eroded = morph(&image, &kernel, MorphOp::Erode);
let dilated = morph(&image, &kernel, MorphOp::Dilate);
```

### 5.2 颜色提取算法

使用 K-Means 聚类提取主色调：

```rust
pub fn extract_dominant_colors<B: Backend>(
    tensor: &Tensor<B, 3>,
    k: usize,
) -> Vec<Color> {
    // 1. 将图像像素展平
    let pixels = tensor.reshape([height * width, 3]);
    
    // 2. K-Means 聚类
    let centroids = kmeans(&pixels, k, 100);
    
    // 3. 转换为颜色
    centroids.iter().map(|c| Color::from_rgb(c)).collect()
}
```

### 5.3 边缘检测

使用 Sobel 算子：

```rust
pub fn detect_edges<B: Backend>(tensor: &Tensor<B, 3>) -> Tensor<B, 2> {
    // 1. 转灰度
    let gray = to_grayscale(tensor);
    
    // 2. Sobel 算子
    let sobel_x = Tensor::from_data([[[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]]]);
    let sobel_y = Tensor::from_data([[[-1, -2, -1], [0, 0, 0], [1, 2, 1]]]);
    
    // 3. 卷积
    let gx = conv2d(&gray, &sobel_x);
    let gy = conv2d(&gray, &sobel_y);
    
    // 4. 梯度幅值
    (gx.powi(2) + gy.powi(2)).sqrt()
}
```

### 5.4 粒子配置生成

```rust
pub fn generate_particles(
    colors: &[Color],
    edges: &Tensor<B, 2>,
    regions: &[Region],
) -> ParticleConfig {
    let mut positions = Vec::new();
    let mut particle_colors = Vec::new();
    
    // 沿边缘放置粒子
    for (y, x) in edge_points(edges) {
        positions.push([x as f32, y as f32]);
        particle_colors.push(sample_color(colors, x, y));
    }
    
    // 在区域内填充粒子
    for region in regions {
        for _ in 0..region.area / 100 {
            let (x, y) = region.random_point();
            positions.push([x, y]);
            particle_colors.push(region.avg_color);
        }
    }
    
    ParticleConfig {
        positions,
        colors: particle_colors,
        sizes: vec![2.0; positions.len()],
        velocities: vec![[0.0, 0.0]; positions.len()],
        lifetimes: vec![5.0; positions.len()],
    }
}
```

---

## 六、与 biosphere-art 集成

### 6.1 接口定义

```rust
// biosphere-ai 输出
pub struct ParticleConfig {
    pub positions: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub sizes: Vec<f32>,
    pub velocities: Vec<[f32; 2]>,
    pub lifetimes: Vec<f32>,
}

// biosphere-art 接收
impl ArtWorld {
    pub fn load_from_config(&mut self, config: ParticleConfig) {
        for i in 0..config.positions.len() {
            self.spawn_particle(Particle {
                position: config.positions[i],
                color: config.colors[i],
                size: config.sizes[i],
                velocity: config.velocities[i],
                lifetime: config.lifetimes[i],
            });
        }
    }
}
```

### 6.2 数据传递

```
biosphere-ai                    biosphere-art
     │                               │
     │  ParticleConfig               │
     ├──────────────────────────────►│
     │                               │
     │                    渲染动画    │
     │                               │
```

---

## 七、未来发展

### 7.1 阶段一：传统算法（当前）

- 颜色提取：K-Means
- 边缘检测：Sobel
- 区域分割：连通区域

### 7.2 阶段二：小型神经网络

- 导入预训练 ONNX 模型
- 图像语义分割
- 风格识别

### 7.3 阶段三：端到端模型

- 训练专用小模型
- 图像 → 粒子配置
- 模型大小 < 50MB

---

## 八、性能要求

### 8.1 目标

| 指标 | 目标值 |
|------|--------|
| 图像加载 | < 100ms |
| 特征提取 | < 500ms |
| 粒子生成 | < 100ms |
| 总处理时间 | < 1s |

### 8.2 硬件要求

| 配置 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 4核 | 8核+ |
| 内存 | 8GB | 16GB+ |
| GPU | 核显 | 独立显卡 |
| 存储 | 100MB | 500MB |

---

## 九、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v0.1.0 | 2026-03-17 | 初始设计文档 |

---

*文档维护者：Biosphere 开发团队*
*最后更新：2026-03-17*
