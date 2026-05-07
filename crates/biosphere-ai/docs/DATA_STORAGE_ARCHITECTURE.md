# 数据存储架构设计

## 概述

本文档描述 Biosphere AI 应用的数据存储架构，包括数据库选型、存储策略和实现方案。

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                     数据存储架构                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   SQLite    │  │   LanceDB   │  │    sled     │         │
│  │  (关系型)   │  │  (向量型)   │  │  (键值型)   │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         │                │                │                 │
│         ▼                ▼                ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ • 分析记录  │  │ • 图像特征  │  │ • 热数据    │         │
│  │ • 用户配置  │  │ • 嵌入向量  │  │ • 会话缓存  │         │
│  │ • 生成历史  │  │ • 相似度    │  │ • 临时数据  │         │
│  │ • 元数据    │  │ • AI 模型   │  │ • 计数器    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│                                                             │
│  ┌─────────────────────────────────────────────────┐       │
│  │              文件系统存储                        │       │
│  │  • 原始图像缓存                                  │       │
│  │  • 生成的粒子效果                                │       │
│  │  • 导出文件                                      │       │
│  └─────────────────────────────────────────────────┘       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 数据库选型

### 1. SQLite - 关系型数据库

**用途**：
- 图像分析记录存储
- 用户配置管理
- 生成历史追踪
- 元数据管理

**特点**：
| 特性 | 说明 |
|------|------|
| 嵌入式 | 无需单独服务器，零配置 |
| 单文件 | 易于备份、迁移 |
| 高性能 | 读操作极快，适合分析型查询 |
| Rust 支持 | rusqlite 库成熟稳定 |
| 跨平台 | Windows/macOS/Linux 通用 |
| 商业友好 | 公有领域，无许可证问题 |

**表结构设计**：

```sql
-- 图像分析记录
CREATE TABLE image_analysis (
    id TEXT PRIMARY KEY,           -- UUID
    file_name TEXT NOT NULL,
    file_hash TEXT,                -- SHA256，用于去重
    file_size INTEGER,
    width INTEGER,
    height INTEGER,
    format TEXT,
    analysis_data TEXT,            -- JSON: 完整分析结果
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 粒子生成配置
CREATE TABLE particle_configs (
    id TEXT PRIMARY KEY,
    name TEXT,
    config_data TEXT,              -- JSON: 配置参数
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 生成历史
CREATE TABLE generation_history (
    id TEXT PRIMARY KEY,
    image_id TEXT,
    config_id TEXT,
    output_path TEXT,
    duration_ms INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (image_id) REFERENCES image_analysis(id),
    FOREIGN KEY (config_id) REFERENCES particle_configs(id)
);

-- 用户标签
CREATE TABLE image_tags (
    id TEXT PRIMARY KEY,
    image_id TEXT,
    tag TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (image_id) REFERENCES image_analysis(id)
);

-- 创建索引
CREATE INDEX idx_image_hash ON image_analysis(file_hash);
CREATE INDEX idx_image_created ON image_analysis(created_at);
CREATE INDEX idx_history_image ON generation_history(image_id);
```

---

### 2. LanceDB - 向量数据库

**用途**：
- 图像特征向量存储
- 相似图片搜索
- 以图搜图功能
- AI 模型嵌入存储

**特点**：
| 特性 | 说明 |
|------|------|
| Rust 原生 | 无 C 依赖，编译简单 |
| 无服务器 | 嵌入式运行，无需外部服务 |
| 多模态 | 支持图像、文本、向量 |
| 高性能 | 列式存储，查询快速 |
| 开源 | Apache 2.0 许可证 |

**数据结构**：

```rust
// 图像特征向量表
struct ImageEmbedding {
    id: String,           // 图像 ID
    embedding: Vec<f32>,  // 特征向量 (512维或更高)
    model_name: String,   // 使用的模型名称
    created_at: i64,      // 创建时间
}

// 向量搜索示例
// 查找与给定图像最相似的 10 张图片
fn search_similar_images(query_vector: Vec<f32>, limit: usize) -> Vec<String>;
```

---

### 3. sled - 键值存储

**用途**：
- 分析结果缓存
- 会话数据存储
- 临时数据管理
- 计数器和统计

**特点**：
| 特性 | 说明 |
|------|------|
| 纯 Rust | 无外部依赖 |
| ACID | 支持事务 |
| 高性能 | 内存映射，读写极快 |
| 简单 API | 类似 HashMap 的使用方式 |
| 线程安全 | 支持并发访问 |

**使用场景**：

```rust
// 缓存键命名规范
// cache:analysis:{hash}     - 分析结果缓存
// cache:thumbnail:{id}      - 缩略图缓存
// session:{session_id}      - 会话数据
// counter:total_analyses    - 统计计数器
// temp:{uuid}               - 临时数据
```

---

## 文件系统存储

### 目录结构

```
~/.biosphere/
├── config/
│   └── settings.json          // 应用配置
├── data/
│   ├── biosphere.db           // SQLite 数据库
│   ├── vectors/               // LanceDB 向量数据
│   │   └── images.lance/
│   └── cache/                 // sled 缓存数据
│       └── sled/
├── images/
│   ├── original/              // 原始图像缓存
│   │   └── {hash}.png
│   └── generated/             // 生成的粒子效果
│       └── {timestamp}/
├── exports/                   // 导出文件
│   └── {timestamp}/
└── logs/
    ├── app.log                // 应用日志
    └── error.log              // 错误日志
```

---

## 依赖配置

### Cargo.toml

```toml
[dependencies]
# 关系型数据库
rusqlite = { version = "0.31", features = ["bundled"] }

# 向量数据库
lancedb = "0.4"

# 键值存储
sled = "0.34"

# 工具库
uuid = { version = "1.8", features = ["v4"] }
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }

# 异步运行时 (可选，用于 LanceDB)
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

---

## 项目结构

```
src-tauri/
├── src/
│   ├── domain/              // 数据模型
│   │   ├── mod.rs
│   │   ├── image.rs
│   │   ├── color.rs
│   │   └── ...
│   │
│   ├── services/            // 业务逻辑
│   │   ├── mod.rs
│   │   ├── analyzers/
│   │   └── image_service.rs
│   │
│   ├── infrastructure/      // 基础设施
│   │   ├── mod.rs
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs      // 数据库连接管理
│   │   │   ├── migrations.rs      // 迁移脚本
│   │   │   └── repositories/
│   │   │       ├── mod.rs
│   │   │       ├── image_repo.rs  // 图像仓库
│   │   │       ├── config_repo.rs // 配置仓库
│   │   │       └── vector_repo.rs // 向量仓库
│   │   │
│   │   ├── cache/
│   │   │   ├── mod.rs
│   │   │   └── sled_cache.rs      // sled 缓存
│   │   │
│   │   └── storage/
│   │       ├── mod.rs
│   │       └── file_storage.rs    // 文件存储
│   │
│   └── commands/            // Tauri 命令
│       └── image_commands.rs
│
└── migrations/              // 数据库迁移文件
    ├── 001_initial.sql
    └── 002_add_tags.sql
```

---

## 数据流设计

### 图像分析流程

```
┌─────────────────────────────────────────────────────────────┐
│                    图像分析数据流                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  前端上传图像                                                │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────┐                                                │
│  │ 检查缓存 │ ──→ sled: cache:analysis:{hash}               │
│  └────┬────┘                                                │
│       │                                                     │
│       ├── 命中 ──→ 直接返回缓存结果                          │
│       │                                                     │
│       └── 未命中                                            │
│            │                                                │
│            ▼                                                │
│       ┌──────────┐                                          │
│       │ 图像分析 │ ──→ 颜色、边缘、区域、统计                 │
│       └────┬─────┘                                          │
│            │                                                │
│            ▼                                                │
│       ┌──────────┐                                          │
│       │ 存储结果 │                                          │
│       └────┬─────┘                                          │
│            │                                                │
│            ├──→ SQLite: image_analysis 表                   │
│            ├──→ sled: cache:analysis:{hash}                 │
│            └──→ LanceDB: 图像特征向量 (可选)                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 实现优先级

### 阶段 1：核心存储（必须）

| 组件 | 功能 | 优先级 |
|------|------|--------|
| SQLite | 图像分析记录 | ⭐⭐⭐ |
| sled | 分析结果缓存 | ⭐⭐⭐ |
| 文件存储 | 图像缓存 | ⭐⭐⭐ |

### 阶段 2：扩展功能（重要）

| 组件 | 功能 | 优先级 |
|------|------|--------|
| SQLite | 用户配置、生成历史 | ⭐⭐ |
| LanceDB | 图像特征向量 | ⭐⭐ |

### 阶段 3：高级功能（未来）

| 组件 | 功能 | 优先级 |
|------|------|--------|
| LanceDB | 以图搜图 | ⭐ |
| SQLite FTS5 | 全文搜索 | ⭐ |

---

## 性能考虑

### 缓存策略

```
┌─────────────────────────────────────────┐
│              缓存层级                    │
├─────────────────────────────────────────┤
│                                         │
│  L1: 内存缓存 (HashMap)                 │
│      └── 热点数据，毫秒级访问            │
│                                         │
│  L2: sled 键值存储                      │
│      └── 分析结果缓存，10ms 级访问       │
│                                         │
│  L3: SQLite 数据库                      │
│      └── 持久化存储，100ms 级访问        │
│                                         │
│  L4: 文件系统                           │
│      └── 原始图像，秒级访问              │
│                                         │
└─────────────────────────────────────────┘
```

### 缓存过期策略

```rust
// 缓存配置
struct CacheConfig {
    max_memory_items: usize,    // 内存最大缓存数: 100
    max_sled_size: u64,         // sled 最大大小: 1GB
    default_ttl: Duration,      // 默认过期时间: 24小时
    cleanup_interval: Duration, // 清理间隔: 1小时
}
```

---

## 备份与恢复

### 备份策略

```bash
# 备份脚本
~/.biosphere/backup/
├── daily/                    # 每日备份
│   └── {date}.tar.gz
├── weekly/                   # 每周备份
│   └── {week}.tar.gz
└── manual/                   # 手动备份
    └── {timestamp}.tar.gz
```

### 恢复流程

1. 停止应用
2. 解压备份文件到 `~/.biosphere/`
3. 重启应用

---

## 安全考虑

### 数据加密

- 敏感配置使用加密存储
- 数据库文件权限控制 (600)
- 用户数据隔离

### 数据清理

```rust
// 清理策略
struct CleanupPolicy {
    delete_original_after_days: u32,  // 原始图像保留天数: 30
    delete_cache_after_hours: u32,    // 缓存保留小时: 24
    max_storage_size_gb: u32,         // 最大存储大小: 10GB
}
```

---

## 总结

本架构设计提供了：

1. **完整的数据存储方案** - 关系型 + 向量型 + 键值型
2. **嵌入式运行** - 无需外部服务，单机运行
3. **高性能** - 多级缓存，快速访问
4. **可扩展** - 为 AI 功能预留接口
5. **商业友好** - 开源许可证宽松

---

## 参考资料

- [SQLite 官方文档](https://www.sqlite.org/docs.html)
- [rusqlite GitHub](https://github.com/rusqlite/rusqlite)
- [LanceDB 官方文档](https://lancedb.github.io/lancedb/)
- [sled GitHub](https://github.com/spacejam/sled)
