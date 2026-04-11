# Artifact 模块

管理节点执行后形成的**正式持久化输出快照**。

这里的“产物”不是普通执行结果，也不是缓存值，而是：

- 来源于一次节点执行输出；
- 已持久化到项目目录；
- 带最小可追溯元数据；
- 可被后续再次查询、选用、回源、校验和清理。

## 当前状态

该模块的**内核能力已完成**，但**尚未接入 engine 执行流**。

当前已具备：

- 创建产物
- 查询历史版本
- 切换当前选用版本
- 自动回源判定
- 产物文件读写
- `artifacts.json` 保存 / 加载
- 索引与磁盘一致性校验
- 按策略清理旧版本 / 孤立产物
- handler 注册机制

当前限制：

- 当前只实现 `Image` handler
- `exporter.rs` 仍是占位
- `CleanupPolicy.max_total_bytes` 仅预留，未实现
- 尚未接入缓存管理器、调度器、项目管理器和节点执行完成后的自动登记流程

## 模块结构

```text
artifact/
├── README.md
├── mod.rs
├── manager.rs
│
├── model/
│   ├── mod.rs
│   ├── record.rs
│   ├── request.rs
│   ├── stats.rs
│   └── error.rs
│
├── persistence/
│   ├── mod.rs
│   ├── store.rs
│   ├── index.rs
│   └── index_file.rs
│
├── lifecycle/
│   ├── mod.rs
│   ├── selector.rs
│   ├── restorer.rs
│   ├── validator.rs
│   ├── cleaner.rs
│   └── exporter.rs
│
└── handler/
    ├── mod.rs
    ├── handler.rs
    ├── registry.rs
    └── image.rs
```

### 职责分层

- `manager.rs`
  - 对外编排入口
  - 组合 index / store / selector / restorer / validator / cleaner / registry

- `model/`
  - 纯数据结构
  - 定义 `ArtifactRecord`、请求参数、统计结果、错误类型

- `persistence/`
  - 负责持久化
  - `store.rs` 管文件系统
  - `index.rs` 管内存索引
  - `index_file.rs` 管 `artifacts.json` 的磁盘格式

- `lifecycle/`
  - 负责历史版本生命周期规则
  - `selector.rs`：手动选用
  - `restorer.rs`：自动回源判定
  - `validator.rs`：一致性校验
  - `cleaner.rs`：清理策略

- `handler/`
  - 负责数据类型扩展
  - `ArtifactHandler` 定义序列化 / 反序列化接口
  - `ArtifactRegistry` 维护 `DataType -> handler` 映射

## 核心概念

### ArtifactRecord

一条产物记录至少包含：

- `artifact_id`
- `node_id`
- `output_key`
- `version`
- `created_at`
- `data_type`
- `format`
- `path`
- `param_signature`
- `input_signature`
- `kind`
- `orphaned`

### selected artifact

每个 `(node_id, output_key)` 维护一个“当前选用”版本：

- 手动选用：由用户或上层调用明确切换
- 自动回源：只在签名一致时允许恢复

### kind

- `Restorable`
  - 可参与项目内回源
- `Exported`
  - 已预留语义，但当前尚未真正导出实现

## 文件布局

项目目录内当前采用：

```text
<project_root>/
├── artifacts.json
└── artifacts/
    └── {node_id}/
        └── {output_key}/
            └── 000001.png
```

说明：

- 二进制产物文件在 `artifacts/` 下
- 元数据索引在 `artifacts.json` 中
- 当前 `Image` 产物使用 PNG

## 典型流程

### 1. 创建产物

`ArtifactManager::create_artifact()`：

1. 根据 `DataType` 找到 handler
2. 从索引分配下一个 `version`
3. 生成磁盘路径
4. 通过 `store.write()` 写入文件
5. 更新内存索引
6. 设为当前选用

### 2. 手动选用

`ArtifactManager::select_artifact()`：

- 切换当前 selected 版本
- 当前实现只更新索引，不负责写缓存和通知调度器

### 3. 自动回源判定

`ArtifactManager::resolve_for_restore()`：

- 无 selected -> `NoSelection`
- `kind != Restorable` -> `NotRestorableKind`
- `orphaned == true` -> `Orphaned`
- `param_signature` / `input_signature` 不一致 -> `SignatureMismatch`
- 否则 -> `Restorable`

注意：

- **手动选用** 和 **自动回源** 是两套不同语义
- 手动选用不要求签名一致
- 自动回源必须严格签名匹配

### 4. 校验

`ArtifactManager::validate_store()`：

- 索引有、文件无 -> `MissingFile`
- 文件有、索引无 -> `OrphanFile`
- 路径非法 / 空文件 / 非普通文件 -> `CorruptedRecord`

### 5. 清理

`ArtifactManager::cleanup()`：

- 按策略计算删除候选
- 永远跳过当前 selected
- 删除成功后更新索引
- 自动保存 `artifacts.json`

## 对外 API（当前实现）

`ArtifactManager` 当前提供：

- `create_artifact`
- `list_artifacts`
- `get_artifact`
- `get_selected_artifact`
- `select_artifact`
- `read_artifact`
- `resolve_for_restore`
- `delete_artifact`
- `save_index`
- `load_index`
- `validate_store`
- `plan_cleanup`
- `cleanup`

## 设计边界

这个模块当前**不负责**：

- 节点执行后自动创建产物
- cache miss 时自动发起回源并回填缓存
- 选用后通知调度器标脏下游
- 项目打开 / 保存流程中的统一调度
- UI 事件分发

这些属于上层 engine / project / runtime 集成工作。

## 测试

当前 artifact 相关测试覆盖：

- index 内存索引与 `artifacts.json` roundtrip
- selector 规则
- restorer 规则
- image handler roundtrip
- store 写 / 读 / 删
- validator 一致性校验
- cleaner 规划与执行
- manager 编排与持久化闭环

运行方式：

```bash
cargo test -p engine --lib
```

## 后续工作

优先级较高的后续接入：

1. 接入 engine 执行流，在节点执行完成后自动登记 artifact
2. 接入缓存/调度器，在 cache miss 时使用 `resolve_for_restore()`
3. 接入项目管理器，在打开/保存项目时统一调度 `load_index()` / `save_index()`

功能扩展类工作：

1. 实现 `exporter.rs`
2. 增加更多 handler（如 `Text` / `JSON`）
3. 实现按总磁盘配额清理 (`max_total_bytes`)
