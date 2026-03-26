# Move NodeInstance to nodeimg-types

**Issue:** #72
**Date:** 2026-03-26
**Status:** Design Approved

## Background

NodeInstance 是纯数据结构（`type_id: String` + `params: HashMap<String, Value>`），当前定义在 `nodeimg-engine` 的 `internal/registry.rs` 中。它的唯一依赖 `Value` 已在 `nodeimg-types` 中，但 app 层为了使用这个数据结构不得不依赖整个 engine。

这是 Phase 2（类型层解耦）的第一步，也是 #73（Transport 序列化职责封装）的前置条件。

## Design

### 目标

将 NodeInstance 从 engine 移到 types，engine 不 re-export，所有使用方直接从 `nodeimg_types` 引用。

### 变更清单

1. **新增** `crates/nodeimg-types/src/node_instance.rs` — NodeInstance 定义
2. **更新** `crates/nodeimg-types/src/lib.rs` — 添加 `pub mod node_instance` 并导出
3. **删除** `crates/nodeimg-engine/src/internal/registry.rs` 中的 NodeInstance 定义，改为 `use nodeimg_types::NodeInstance`
4. **删除** `crates/nodeimg-engine/src/lib.rs` 中的 `pub use internal::registry::NodeInstance`
5. **更新** engine 内部引用（eval.rs、backend.rs、local.rs 等）改为从 `nodeimg_types` 引用
6. **更新** app 层引用（app.rs、serial.rs、viewer.rs、node_canvas.rs）改为 `use nodeimg_types::NodeInstance`
7. **更新** engine 集成测试（tests/pipelines.rs）

### 不变的

- NodeInstance 的字段和 API 完全不变
- 不添加 serde derive（序列化走 SerializedNode 桥接，已在 types 层）
- Cargo.toml 无需改动（types 已有所有依赖，app 已依赖 types）

### NodeInstance 定义

```rust
use std::collections::HashMap;
use crate::value::Value;

/// Runtime node instance stored in the graph.
#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub type_id: String,
    pub params: HashMap<String, Value>,
}
```

### 影响的文件

| 文件 | 变更类型 |
|------|---------|
| `crates/nodeimg-types/src/node_instance.rs` | 新增 |
| `crates/nodeimg-types/src/lib.rs` | 添加导出 |
| `crates/nodeimg-engine/src/internal/registry.rs` | 删除定义，添加导入 |
| `crates/nodeimg-engine/src/lib.rs` | 删除 re-export |
| `crates/nodeimg-engine/src/internal/eval.rs` | 改导入路径 |
| `crates/nodeimg-engine/src/internal/backend.rs` | 改导入路径 |
| `crates/nodeimg-engine/src/transport/local.rs` | 改导入路径 |
| `crates/nodeimg-engine/tests/pipelines.rs` | 改导入路径 |
| `crates/nodeimg-app/src/app.rs` | 改导入路径 |
| `crates/nodeimg-app/src/node/serial.rs` | 改导入路径 |
| `crates/nodeimg-app/src/node/viewer.rs` | 改导入路径 |
| `crates/nodeimg-app/src/ui/node_canvas.rs` | 改导入路径 |

### 验证

1. `cargo test --workspace` 通过
2. `cargo build -p nodeimg-app --release` 通过
3. 确认 app 不再通过 engine 路径引用 NodeInstance
