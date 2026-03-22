# Workspace 拆分实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 nodeimg 从单 crate 重构为 6-crate Rust workspace（types/gpu/processing/engine/app/server），UI 与计算逻辑编译期隔离，功能不变。

**Architecture:** 自底向上逐层提取：先把现有代码整体移入 nodeimg-app，然后从底层（types）到高层（engine）逐个提取独立 crate。每次提取后 `cargo build` 必须通过。

**Tech Stack:** Rust workspace, wgpu, eframe/egui, egui-snarl, image crate

**Spec:** `docs/superpowers/specs/2026-03-22-workspace-split-design.md`

---

## Task 1: Workspace 骨架搭建

**目标**：将现有代码移入 `crates/nodeimg-app/`，根 Cargo.toml 变成 workspace 声明。编译通过，行为不变。

**Files:**
- Modify: `Cargo.toml` → workspace root
- Move: `src/` → `crates/nodeimg-app/src/`
- Create: `crates/nodeimg-app/Cargo.toml`

- [ ] **Step 1: 创建 crate 目录**

```bash
mkdir -p crates/nodeimg-app
```

- [ ] **Step 2: 移动 src/ 到 crate 内**

```bash
mv src crates/nodeimg-app/src
```

- [ ] **Step 3: 创建 nodeimg-app 的 Cargo.toml**

从根 Cargo.toml 复制依赖部分，`[package]` name 改为 `nodeimg-app`���保留所有现有依赖不变。

```toml
[package]
name = "nodeimg-app"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "nodeimg"
path = "src/main.rs"

[dependencies]
eframe = { version = "0.33", features = ["wgpu"] }
egui_extras = { version = "0.33", features = ["image", "file"] }
egui-snarl = { version = "0.9", features = ["serde"] }
image = "0.25"
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"
reqwest = { version = "0.12", features = ["blocking", "json"] }
rfd = "0.17"
```

注意：`[[bin]]` name 保持 `nodeimg`，这样 `cargo run` 还是启动同一个二进制。

- [ ] **Step 4: 改根 Cargo.toml 为 workspace**

```toml
[workspace]
members = [
    "crates/nodeimg-app",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
reqwest = { version = "0.12", features = ["blocking", "json"] }
base64 = "0.22"
```

删除原来的 `[package]`、`[dependencies]` 等所有内容。workspace.dependencies 统一管理版本，各 crate 用 `dep.workspace = true` 引用。

- [ ] **Step 5: 移动 autosave.nis 到新位置（如果需要）**

autosave.nis 使用相对路径，检查是否需要调整。通常自动保存文件在工作目录下，不需要移动。

- [ ] **Step 6: 验证编译**

```bash
cargo build
cargo run -p nodeimg-app
```

Expected: 编译通过，程序启动正常。

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "refactor: 转换为 workspace，现有代码移入 nodeimg-app #13"
```

---

## Task 2: 创建 nodeimg-types（基础层）

**目标**：提取所有纯数据类型到 nodeimg-types crate。这是最底层，���依赖任何内部 crate。

**Files:**
- Create: `crates/nodeimg-types/Cargo.toml`
- Create: `crates/nodeimg-types/src/lib.rs`
- Create: `crates/nodeimg-types/src/value.rs` ← 从 `node/types.rs` 提取 Value 枚举
- Create: `crates/nodeimg-types/src/data_type.rs` ← 从 `node/types.rs` 提取 DataType 系统
- Create: `crates/nodeimg-types/src/node_def.rs` ← 从 `node/registry.rs` 提取 NodeDef/PinDef/ParamDef
- Create: `crates/nodeimg-types/src/constraint.rs` ← 从 `node/constraint.rs` 搬迁
- Create: `crates/nodeimg-types/src/category.rs` ← 从 `node/category.rs` 搬迁
- Create: `crates/nodeimg-types/src/gpu_texture.rs` ← 从 `gpu/texture.rs` 搬迁
- Create: `crates/nodeimg-types/src/widget_id.rs` ← 从 `node/widget/mod.rs` 提取
- Create: `crates/nodeimg-types/src/serial_data.rs` ← 从 `node/serial.rs` 提取纯数据结构
- Modify: `crates/nodeimg-app/src/node/types.rs` → 改为 re-export nodeimg-types
- Modify: `crates/nodeimg-app/src/node/constraint.rs` → 改为 re-export
- Modify: `crates/nodeimg-app/src/node/category.rs` → 改为 re-export
- Modify: `crates/nodeimg-app/src/node/registry.rs` → 删除已移走的定义
- Modify: `crates/nodeimg-app/src/gpu/texture.rs` → 改为 re-export + 保留 egui 方法
- Modify: `crates/nodeimg-app/Cargo.toml` → 添加 nodeimg-types 依赖

- [ ] **Step 1: 创建 nodeimg-types Cargo.toml**

```toml
[package]
name = "nodeimg-types"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
```

- [ ] **Step 2: 将 workspace root 添加新成员**

在根 Cargo.toml 的 members 中添加 `"crates/nodeimg-types"`。

- [ ] **Step 3: 搬迁 constraint.rs**

原样复制 `crates/nodeimg-app/src/node/constraint.rs` → `crates/nodeimg-types/src/constraint.rs`。

将 `crate::` 引用替换为 `crate::` 或移除（constraint.rs 不引用其他内部模块，只用 serde）。

- [ ] **Step 4: 搬迁 category.rs**

原样复制 `crates/nodeimg-app/src/node/category.rs` → `crates/nodeimg-types/src/category.rs`。

修改内部 `use crate::node::constraint::ConstraintType` 等引用为 `use crate::constraint::ConstraintType`（如果有的话）。category.rs 应该是独立的，只用 serde。

- [ ] **Step 5: 提取 widget_id.rs**

从 `crates/nodeimg-app/src/node/widget/mod.rs` 中提取 `WidgetId` 定义：

```rust
// crates/nodeimg-types/src/widget_id.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WidgetId(pub String);

impl WidgetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}
```

- [ ] **Step 6: 提取 value.rs 和 data_type.rs**

从 `crates/nodeimg-app/src/node/types.rs` 拆分：

`value.rs`：Value 枚举及其方法（type_name, pin_color 等）。将 `use crate::gpu::texture::GpuTexture` 改为 `use crate::gpu_texture::GpuTexture`。

`data_type.rs`：DataTypeId, DataTypeInfo, DataTypeRegistry 及其方法。将 Value 引用改为 `use crate::value::Value`。

- [ ] **Step 7: 搬迁 gpu_texture.rs**

复制 `crates/nodeimg-app/src/gpu/texture.rs` → `crates/nodeimg-types/src/gpu_texture.rs`。

**关键修改**：移除 `to_color_image()` 方法（它返回 `egui::ColorImage`，依赖 egui）。这个方法以 extension trait 形式留在 nodeimg-app 中。

同时移除 `from_dynamic_image` 和 `to_dynamic_image` 中对 `crate::processing` 的引用（如果有的话），改为直接使用 `image` crate。

- [ ] **Step 8: 提取 node_def.rs**

从 `crates/nodeimg-app/src/node/registry.rs` 提取 `NodeDef`、`PinDef`、`ParamDef` 结构体定义到 `crates/nodeimg-types/src/node_def.rs`。

**不包含**：`ProcessFn`、`GpuProcessFn`、`NodeRegistry`、`NodeInstance`（这些留给 engine 层）。

NodeDef 中原本包含 `process: Option<ProcessFn>` 和 `gpu_process: Option<GpuProcessFn>` 字段，这些字段在 types 层的 NodeDef 中需要移除。types 层的 NodeDef 只保留纯数据字段（type_id, title, category, inputs, outputs, params, has_preview）。

- [ ] **Step 9: 提取 serial_data.rs**

从 `crates/nodeimg-app/src/node/serial.rs` 提取纯数据结构：

```rust
// crates/nodeimg-types/src/serial_data.rs
// SerializedGraph, SerializedNode, SerializedConnection, SerializedValue
// 只要 serde derive 的结构体，不包含 snapshot/restore 函数
```

- [ ] **Step 10: 编写 lib.rs**

```rust
// crates/nodeimg-types/src/lib.rs
pub mod value;
pub mod data_type;
pub mod node_def;
pub mod constraint;
pub mod category;
pub mod gpu_texture;
pub mod widget_id;
pub mod serial_data;
```

- [ ] **Step 11: nodeimg-app 添加依赖并更新引用**

在 `crates/nodeimg-app/Cargo.toml` 添加：
```toml
nodeimg-types = { path = "../nodeimg-types" }
```

更新 nodeimg-app 中所有引用已搬走类型的文件。策略：在原文件中用 re-export 保持 `crate::node::types::Value` 等路径可用，减少一次性改动量：

```rust
// crates/nodeimg-app/src/node/types.rs（改为 re-export）
pub use nodeimg_types::value::*;
pub use nodeimg_types::data_type::*;
```

```rust
// crates/nodeimg-app/src/node/constraint.rs（改为 re-export）
pub use nodeimg_types::constraint::*;
```

对 category.rs、widget/mod.rs 中的 WidgetId 做类似处理。

- [ ] **Step 12: 验证编译**

```bash
cargo build
```

Expected: 编译通过。types crate 独立编译，app 通过 re-export 保持所有路径有效。

- [ ] **Step 13: 提交**

```bash
git add -A
git commit -m "refactor: 提取 nodeimg-types crate（基础层）#13"
```

---

## Task 3: 创建 nodeimg-gpu（GPU 计算层）

**目标**：提取 GPU 上下文、计算管线和着色器到独立 crate。

**Files:**
- Create: `crates/nodeimg-gpu/Cargo.toml`
- Create: `crates/nodeimg-gpu/src/lib.rs`
- Create: `crates/nodeimg-gpu/src/context.rs` ← 从 `gpu/mod.rs` 提取 GpuContext
- Create: `crates/nodeimg-gpu/src/pipeline.rs` ← 搬迁
- Modify: `crates/nodeimg-app/src/gpu/mod.rs` → 改为 re-export + 保留 from_eframe
- Modify: `crates/nodeimg-app/Cargo.toml` → 添加 nodeimg-gpu 依赖

- [ ] **Step 1: 创建 nodeimg-gpu Cargo.toml**

```toml
[package]
name = "nodeimg-gpu"
version = "0.1.0"
edition = "2021"

[dependencies]
nodeimg-types = { path = "../nodeimg-types" }
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
image = "0.25"
```

- [ ] **Step 2: 添加到 workspace members**

- [ ] **Step 3: 搬迁 GpuContext**

从 `crates/nodeimg-app/src/gpu/mod.rs` 提取 `GpuContext` 结构体和 `pipeline()` 方法到 `crates/nodeimg-gpu/src/context.rs`。

**关键修改**：移除 `from_eframe(cc: &eframe::CreationContext)` 方法。这个方法留在 nodeimg-app 的 `gpu/mod.rs` 中作为独立函数或 extension trait。

GpuContext 核心只需要 `wgpu::Device` 和 `wgpu::Queue`，提供一个通用 constructor：
```rust
impl GpuContext {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self { ... }
    pub fn pipeline(&self, name: &str, shader_source: &str) -> Arc<wgpu::ComputePipeline> { ... }
}
```

- [ ] **Step 4: 搬迁 pipeline.rs**

原样复制，将 `crate::gpu::GpuContext` 引用改为 `crate::context::GpuContext`。

- [ ] **Step 5: 编写 lib.rs**

```rust
// crates/nodeimg-gpu/src/lib.rs
pub mod context;
pub mod pipeline;

pub use context::GpuContext;
```

- [ ] **Step 6: 更新 nodeimg-app**

在 `crates/nodeimg-app/Cargo.toml` 添加 `nodeimg-gpu` 依赖。

`crates/nodeimg-app/src/gpu/mod.rs` 改为 re-export + 保留 `from_eframe`：
```rust
pub use nodeimg_gpu::*;

// from_eframe 留在 app 层，因为它依赖 eframe
pub fn gpu_context_from_eframe(cc: &eframe::CreationContext) -> Option<Arc<GpuContext>> {
    // 原有 from_eframe 逻辑，改为调用 GpuContext::new()
}
```

**注意**：shaders/ 目录暂不搬迁。shaders 通过 `include_str!` 被 builtins 引用，builtins 在 Task 5 才移到 engine。届时 shaders 一并搬到 nodeimg-gpu，并在 nodeimg-gpu 中导出 shader 常量供 engine/builtins 使用。

- [ ] **Step 7: 验证编译**

```bash
cargo build
```

- [ ] **Step 8: 提交**

```bash
git add -A
git commit -m "refactor: 提取 nodeimg-gpu crate（GPU 计算层）#13"
```

---

## Task 4: 创建 nodeimg-processing（CPU 算法层）

**目标**：提取 CPU 图像处理算法到独立 crate。这是最简单的提取——5 个文件，几乎无内部依赖。

**Files:**
- Create: `crates/nodeimg-processing/Cargo.toml`
- Create: `crates/nodeimg-processing/src/lib.rs`
- Move: `crates/nodeimg-app/src/processing/` → `crates/nodeimg-processing/src/`
- Modify: `crates/nodeimg-app/Cargo.toml` → 添加依赖
- Modify: `crates/nodeimg-app/src/lib.rs` 或 `processing/mod.rs` → re-export

- [ ] **Step 1: 创建 nodeimg-processing Cargo.toml**

```toml
[package]
name = "nodeimg-processing"
version = "0.1.0"
edition = "2021"

[dependencies]
nodeimg-types = { path = "../nodeimg-types" }
image = "0.25"
```

- [ ] **Step 2: 添加到 workspace members**

- [ ] **Step 3: 搬迁所有 processing 文件**

```bash
mkdir -p crates/nodeimg-processing/src
cp crates/nodeimg-app/src/processing/*.rs crates/nodeimg-processing/src/
```

检查每个文件的 `use crate::` 引用。processing/ 模块应该是纯算法，不引用 node/ 或 gpu/。如果有对 `image` crate 类型的使用，保持不变。

- [ ] **Step 4: 编写 lib.rs**

```rust
// crates/nodeimg-processing/src/lib.rs
pub mod color;
pub mod composite;
pub mod filter;
pub mod generate;
pub mod transform;
```

（替代原来的 mod.rs）

- [ ] **Step 5: 更新 nodeimg-app**

在 Cargo.toml 添加 `nodeimg-processing` 依赖。

`crates/nodeimg-app/src/processing/mod.rs` 改为 re-export：
```rust
pub use nodeimg_processing::*;
```

或者更新所有引用 `crate::processing::` 的地方改为 `nodeimg_processing::`。re-export 更安全。

- [ ] **Step 6: 验证编译**

```bash
cargo build
```

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "refactor: 提取 nodeimg-processing crate（CPU 算法层）#13"
```

---

## Task 5: 创建 nodeimg-engine（引擎层）

**目标**：提取执行引擎、节点注册表、内置节点、AI 后端通信到独立 crate。这是最复杂的一步，涉及代码修改。

**Files:**
- Create: `crates/nodeimg-engine/Cargo.toml`
- Create: `crates/nodeimg-engine/src/lib.rs`
- Create: `crates/nodeimg-engine/src/registry.rs` ← 从 `node/registry.rs` 提取
- Move: `crates/nodeimg-app/src/node/eval.rs` → `crates/nodeimg-engine/src/eval.rs`
- Move: `crates/nodeimg-app/src/node/cache.rs` → `crates/nodeimg-engine/src/cache.rs`
- Move: `crates/nodeimg-app/src/node/backend.rs` → `crates/nodeimg-engine/src/backend.rs`
- Move: `crates/nodeimg-app/src/node/ai_task.rs` → `crates/nodeimg-engine/src/ai_task.rs`（需修改）
- Move: `crates/nodeimg-app/src/node/menu.rs` → `crates/nodeimg-engine/src/menu.rs`
- Move: `crates/nodeimg-app/src/node/builtins/` → `crates/nodeimg-engine/src/builtins/`
- Move: `crates/nodeimg-app/src/gpu/shaders/` → `crates/nodeimg-gpu/src/shaders/`（搬到 gpu crate）
- Modify: `crates/nodeimg-app/src/node/` → 精简为只剩 viewer.rs 和 re-exports

- [ ] **Step 1: 创建 nodeimg-engine Cargo.toml**

```toml
[package]
name = "nodeimg-engine"
version = "0.1.0"
edition = "2021"

[dependencies]
nodeimg-types = { path = "../nodeimg-types" }
nodeimg-gpu = { path = "../nodeimg-gpu" }
nodeimg-processing = { path = "../nodeimg-processing" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
base64 = "0.22"
reqwest = { version = "0.12", features = ["blocking", "json"] }
```

- [ ] **Step 2: 添加到 workspace members**

- [ ] **Step 3: 搬迁 cache.rs**

原样搬迁。修改 `use crate::node::types::Value` → `use nodeimg_types::value::Value`。
`NodeId = usize` 类型别名保持不变。

- [ ] **Step 4: 搬迁 eval.rs**

搬迁并更新引用：
- `use crate::gpu::GpuContext` → `use nodeimg_gpu::GpuContext`
- `use crate::node::backend::BackendClient` → `use crate::backend::BackendClient`
- `use crate::node::cache::*` → `use crate::cache::*`
- `use crate::node::registry::*` → `use crate::registry::*`
- `use crate::node::types::*` → `use nodeimg_types::*`

- [ ] **Step 5: 提取 registry.rs**

从 app 的 `node/registry.rs` 中提取 `NodeRegistry`、`NodeInstance`、`ProcessFn`、`GpuProcessFn` 到 `crates/nodeimg-engine/src/registry.rs`。

引用 NodeDef、PinDef、ParamDef 从 `nodeimg_types::node_def`。
引用 GpuContext 从 `nodeimg_gpu::GpuContext`。
引用 Value 从 `nodeimg_types::value::Value`。

- [ ] **Step 6: 搬迁 backend.rs**

更新引用：
- `crate::node::*` → `use nodeimg_types::*` 和 `use crate::*`
- `image::*` 保持不变
- `reqwest` 保持不变

- [ ] **Step 7: 搬迁并修改 ai_task.rs**

**关键修改**：去除 `egui::Context` 依赖。

将 `spawn()` 方法中的 `ctx: egui::Context` 参数改为 `on_complete: Box<dyn Fn() + Send + Sync>`。
在 spawn 的线程中，将 `ctx.request_repaint()` 改为 `on_complete()`。

同时更新所有内部引用路径。

- [ ] **Step 8: 搬迁 menu.rs**

更新引用路径即可。

- [ ] **Step 9: 搬迁 builtins/**

搬迁所有 32 个 builtin 文件和 mod.rs。

每个 builtin 的引用更新：
- `use crate::node::registry::*` → `use crate::registry::*`
- `use crate::node::types::*` → `use nodeimg_types::*`
- `use crate::node::category::*` → `use nodeimg_types::category::*`
- `use crate::node::constraint::*` → `use nodeimg_types::constraint::*`
- `use crate::processing::*` → `use nodeimg_processing::*`
- `use crate::gpu::GpuContext` → `use nodeimg_gpu::GpuContext`

- [ ] **Step 10: 搬迁 shaders 到 nodeimg-gpu 并导出常量**

```bash
mv crates/nodeimg-app/src/gpu/shaders crates/nodeimg-gpu/src/shaders
```

在 `crates/nodeimg-gpu/src/lib.rs` 中为每个 shader 导出常量：
```rust
pub mod shaders {
    pub const GAUSSIAN_BLUR_H: &str = include_str!("shaders/gaussian_blur_h.wgsl");
    pub const GAUSSIAN_BLUR_V: &str = include_str!("shaders/gaussian_blur_v.wgsl");
    // ... 所有 30 个 shader
}
```

更新 builtins 中的 `include_str!("../../gpu/shaders/xxx.wgsl")` 为 `nodeimg_gpu::shaders::XXX`。

**注意**：WidgetRegistry 和 WidgetRenderFn 整体留在 nodeimg-app（它们直接依赖 egui::Ui），engine 层不需要 widget_def.rs。

- [ ] **Step 11: 编写 lib.rs**

```rust
// crates/nodeimg-engine/src/lib.rs
pub mod registry;
pub mod eval;
pub mod cache;
pub mod backend;
pub mod ai_task;
pub mod menu;
pub mod builtins;
```

- [ ] **Step 12: 更新 nodeimg-app**

在 Cargo.toml 添加 `nodeimg-engine` 依赖。

`crates/nodeimg-app/src/node/mod.rs` 精简为只保留：
- viewer.rs（SnarlViewer 实现）
- widget/（UI 渲染）
- re-exports from engine

将 `crates/nodeimg-app/src/node/serial.rs` 中的 `snapshot()` 和 `restore()` 函数保留，它们引用 `nodeimg_engine::registry::NodeRegistry` 等。

更新 viewer.rs 中的引用，使用 `nodeimg_engine::*` 代替 `crate::node::*`。

- [ ] **Step 13: 验证编译**

```bash
cargo build
```

这一步可能有大量编译错误，逐一修复引用路径。

- [ ] **Step 14: 提交**

```bash
git add -A
git commit -m "refactor: 提取 nodeimg-engine crate（引擎层）#13"
```

---

## Task 6: 清理 nodeimg-app

**目标**：清理 app crate，移除已经搬走的文件，确保 app 只剩 UI 相关代码。整理 re-export 和引用。

**Files:**
- Delete: `crates/nodeimg-app/src/node/types.rs`（已由 nodeimg-types re-export）
- Delete: `crates/nodeimg-app/src/node/constraint.rs`
- Delete: `crates/nodeimg-app/src/node/category.rs`
- Delete: `crates/nodeimg-app/src/node/eval.rs`
- Delete: `crates/nodeimg-app/src/node/cache.rs`
- Delete: `crates/nodeimg-app/src/node/backend.rs`
- Delete: `crates/nodeimg-app/src/node/ai_task.rs`
- Delete: `crates/nodeimg-app/src/node/menu.rs`
- Delete: `crates/nodeimg-app/src/node/registry.rs`
- Delete: `crates/nodeimg-app/src/node/builtins/`
- Delete: `crates/nodeimg-app/src/processing/`（如果还剩 re-export 文件）
- Modify: `crates/nodeimg-app/src/node/mod.rs` → 只保留 viewer、widget
- Modify: `crates/nodeimg-app/src/lib.rs` → 移除已搬走模块的声明
- Modify: `crates/nodeimg-app/src/gpu/mod.rs` → 只保留 from_eframe + egui extension
- Modify: 所有 app 文件中的 `crate::node::*` 引用 → 直接用 `nodeimg_engine::*` 或 `nodeimg_types::*`

- [ ] **Step 1: 删除已搬走的源文件**

```bash
rm -f crates/nodeimg-app/src/node/types.rs
rm -f crates/nodeimg-app/src/node/constraint.rs
rm -f crates/nodeimg-app/src/node/category.rs
rm -f crates/nodeimg-app/src/node/eval.rs
rm -f crates/nodeimg-app/src/node/cache.rs
rm -f crates/nodeimg-app/src/node/backend.rs
rm -f crates/nodeimg-app/src/node/ai_task.rs
rm -f crates/nodeimg-app/src/node/menu.rs
rm -f crates/nodeimg-app/src/node/registry.rs
rm -rf crates/nodeimg-app/src/node/builtins/
rm -rf crates/nodeimg-app/src/processing/
```

- [ ] **Step 2: 精简 node/mod.rs**

```rust
// crates/nodeimg-app/src/node/mod.rs
pub mod viewer;
pub mod widget;
pub mod serial;  // snapshot/restore 函数

// 方便 app 内部使用的 re-exports
pub use nodeimg_engine::*;
pub use nodeimg_types::*;
```

- [ ] **Step 3: 精简 gpu/mod.rs**

```rust
// crates/nodeimg-app/src/gpu/mod.rs
pub use nodeimg_gpu::*;

use std::sync::Arc;

/// 从 eframe 创建上下文获取 GPU 设备（app 层专用）
pub fn gpu_context_from_eframe(cc: &eframe::CreationContext) -> Option<Arc<GpuContext>> {
    let wgpu_render_state = cc.wgpu_render_state.as_ref()?;
    let device = wgpu_render_state.device.clone();
    let queue = wgpu_render_state.queue.clone();
    Some(Arc::new(GpuContext::new(device, queue)))
}
```

- [ ] **Step 4: 在 gpu/mod.rs 中添加 GpuTexture 的 egui 扩展方法**

```rust
// 在 crates/nodeimg-app/src/gpu/mod.rs 中添加
use nodeimg_types::gpu_texture::GpuTexture;

pub trait GpuTextureExt {
    fn to_color_image(&self) -> egui::ColorImage;
}

impl GpuTextureExt for GpuTexture {
    fn to_color_image(&self) -> egui::ColorImage {
        // 原 to_color_image 逻辑
    }
}
```

- [ ] **Step 5: 更新 viewer.rs 引用**

将所有 `crate::node::*` 引用改为直接使用 `nodeimg_engine` 和 `nodeimg_types`。
将 `ai_executor.spawn(ctx, ...)` 改为 `ai_executor.spawn(Box::new(move || ctx.request_repaint()), ...)`。

- [ ] **Step 6: 更新 app.rs 引用**

将引用改为使用 `nodeimg_engine` 和 `nodeimg_types`。

- [ ] **Step 7: 更新 serial.rs**

确保 `snapshot()` 和 `restore()` 正确引用 `nodeimg_engine::registry::*` 和 `nodeimg_types::serial_data::*`。

- [ ] **Step 8: 精简 Cargo.toml 依赖**

从 nodeimg-app 的 Cargo.toml 中移除已经由 engine/types 承担的直接依赖：
- 移除 `base64`（已在 engine 中）
- 可能移除 `wgpu`、`bytemuck`（如果 app 不再直接使用）
- 保留 `image`（如果 app 直接使用图片类型）

**注意**：先不激进移除，等编译通过后再精简。

- [ ] **Step 9: 精简 lib.rs**

```rust
// crates/nodeimg-app/src/lib.rs
pub mod app;
pub mod gpu;
pub mod node;
pub mod ui;
pub mod theme;
```

移除 `pub mod processing;`（已搬走）。

- [ ] **Step 10: 验证编译**

```bash
cargo build
```

逐一修复所有编译错误。

- [ ] **Step 11: 验证运行**

```bash
cargo run -p nodeimg-app
```

Expected: 程序启动正常，所有功能与拆分前一致。

- [ ] **Step 12: 提交**

```bash
git add -A
git commit -m "refactor: 清理 nodeimg-app，移除已搬走模块 #13"
```

---

## Task 7: 创建 nodeimg-server（空壳）

**目标**：创建 server crate 占位。

**Files:**
- Create: `crates/nodeimg-server/Cargo.toml`
- Create: `crates/nodeimg-server/src/main.rs`

- [ ] **Step 1: 创建 Cargo.toml**

```toml
[package]
name = "nodeimg-server"
version = "0.1.0"
edition = "2021"

[dependencies]
nodeimg-engine = { path = "../nodeimg-engine" }
```

- [ ] **Step 2: 创建 main.rs**

```rust
fn main() {
    println!("nodeimg-server: not implemented yet");
    println!("See Issue #12 for remote execution support");
}
```

- [ ] **Step 3: 添加到 workspace members**

- [ ] **Step 4: 验证编译**

```bash
cargo build
cargo run -p nodeimg-server
```

Expected: 输出 placeholder 信息。

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "refactor: 创建 nodeimg-server 空壳 #13"
```

---

## Task 8: 最终验证与清理

**目标**：确认所有功能正常，清理残留文件，更新文档。

- [ ] **Step 1: 完整构建**

```bash
cargo build --workspace
cargo test --workspace
```

- [ ] **Step 2: 运行验证**

```bash
cargo run -p nodeimg-app
```

手动测试：
- 创建节点、连线
- 加载图片节点
- 应用滤镜
- 保存/加载项目文件
- 暗色/亮色主题切换

- [ ] **Step 3: 确认 workspace 成员列表**

根 Cargo.toml：
```toml
[workspace]
members = [
    "crates/nodeimg-types",
    "crates/nodeimg-gpu",
    "crates/nodeimg-processing",
    "crates/nodeimg-engine",
    "crates/nodeimg-app",
    "crates/nodeimg-server",
]
resolver = "2"
```

- [ ] **Step 4: 确认依赖方向正确**

验证没有循环依赖：
```bash
cargo build --workspace 2>&1 | grep "cyclic"
```

Expected: 无输出。

- [ ] **Step 5: 更新 CLAUDE.md 项目结构描述**

更新 CLAUDE.md 中的"项目结构"部分，反映新的 workspace 结构。

- [ ] **Step 6: 创建后续 Issue**

用 `gh issue create` 创建：
- Transport trait + LocalTransport（#13 阶段 2）
- nodeimg-server 实现（#13 阶段 3）
- HttpTransport（#13 阶段 4）
- GPU/CPU 双实现精简

- [ ] **Step 7: 最终提交**

```bash
git add -A
git commit -m "refactor: workspace 拆分完成，更新文档 #13"
```

- [ ] **Step 8: 推送并创建 PR**

```bash
git push origin refactor/13-workspace-split
gh pr create --title "refactor: workspace 拆分 + eval engine 后移 #13" --body "..."
```
