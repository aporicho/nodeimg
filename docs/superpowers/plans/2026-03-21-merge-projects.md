# 三项目合并实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 node-image-studio、node-image-studio-builtin-nodes、node-image-studio-ai-debug 合并为全新项目 nodeimg。

**Architecture:** 全新 cargo 项目，从两个源项目 cherry-pick 最优代码。6 个核心文件需手动合并（types.rs, registry.rs, eval.rs, viewer.rs, backend.rs, app.rs），其余直接复制。项目无测试套件，验证手段为 `cargo build` 和 `cargo run`。

**Tech Stack:** Rust (eframe/egui, egui-snarl, wgpu, reqwest), Python (FastAPI, torch, diffusers)

**源项目路径（文档简写）：**
- `NIS` = `/Users/aporicho/Desktop/nodestudio/node-image-studio`（AI 后端、文件管理）
- `BN` = `/Users/aporicho/Desktop/nodestudio/node-image-studio-builtin-nodes`（30+ 节点、GPU）
- `TARGET` = `/Users/aporicho/Desktop/nodestudio/nodeimg`（目标项目）

**重要说明：**
- 以下 bash 命令中的 `$NIS`、`$BN`、`$TARGET` 是 shell 环境变量，需在 Task 1 Step 0 中导出后才能使用
- AI 节点（load_checkpoint 等 5 个）通过 Python 后端动态注册，无静态 Rust 文件。设计规格文件结构树中列出它们仅表示注册后的逻辑存在
- NIS 的 `processing.rs` 是单文件（59 行），BN 的 `processing/` 是 5 文件模块化版本。本计划使用 BN 版本整体替换

---

### Task 1: 项目骨架

**Files:**
- Create: `TARGET/Cargo.toml`
- Create: `TARGET/src/main.rs`
- Create: `TARGET/src/lib.rs`

- [ ] **Step 0: 导出环境变量（后续所有 Task 的 bash 命令依赖这些变量）**

```bash
export NIS=/Users/aporicho/Desktop/nodestudio/node-image-studio
export BN=/Users/aporicho/Desktop/nodestudio/node-image-studio-builtin-nodes
export TARGET=/Users/aporicho/Desktop/nodestudio/nodeimg
```

- [ ] **Step 1: 创建 cargo 项目 + .gitignore**

```bash
cd /Users/aporicho/Desktop/nodestudio
cargo init nodeimg
cat > $TARGET/.gitignore << 'EOF'
/target
python/.venv/
__pycache__/
*.pyc
.DS_Store
autosave.nis
EOF
```

- [ ] **Step 2: 写入 Cargo.toml**

覆盖 `TARGET/Cargo.toml`，合并两个项目的依赖：

```toml
[package]
name = "nodeimg"
version = "0.1.0"
edition = "2021"

[dependencies]
# GUI
eframe = { version = "0.33", features = ["wgpu"] }
egui_extras = { version = "0.33", features = ["image", "file"] }
egui-snarl = { version = "0.9", features = ["serde"] }

# 图像
image = "0.25"

# GPU
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }

# 文件对话框
rfd = "0.17"

# HTTP（AI 后端通信）
reqwest = { version = "0.12", features = ["blocking", "json"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Base64（AI 节点图像传输）
base64 = "0.22"
```

- [ ] **Step 3: 写入 main.rs**

```rust
use eframe::egui;
use nodeimg::app;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Node Image Studio"),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "Node Image Studio",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
```

- [ ] **Step 4: 写入 lib.rs（暂时只声明将逐步填充的模块）**

```rust
pub mod app;
pub mod gpu;
pub mod node;
pub mod processing;
pub mod theme;
pub mod ui;
```

- [ ] **Step 5: Commit**

```bash
cd TARGET
git init
git add Cargo.toml src/main.rs src/lib.rs
git commit -m "feat: init nodeimg project skeleton"
```

**注意：** 此时 `cargo build` 会失败（模块声明了但文件不存在），这是预期的。后续 task 逐步填充。

---

### Task 2: 节点系统基础（types.rs + constraint.rs + category.rs）

**Files:**
- Create: `TARGET/src/node/mod.rs`
- Copy: `BN/src/node/types.rs` → `TARGET/src/node/types.rs`（合并基础）
- Copy: `BN/src/node/constraint.rs` → `TARGET/src/node/constraint.rs`
- Copy: `BN/src/node/category.rs` → `TARGET/src/node/category.rs`

- [ ] **Step 1: 创建目录结构**

```bash
mkdir -p TARGET/src/node
```

- [ ] **Step 2: 复制 types.rs 从 builtin-nodes（已含 GpuImage 变体）**

```bash
cp BN/src/node/types.rs TARGET/src/node/types.rs
```

检查：文件应包含 `Value::GpuImage(Arc<GpuTexture>)` 变体。如果 import 路径引用 `crate::gpu::GpuTexture`，保持不变（gpu 模块将在 Task 7 搬入）。

- [ ] **Step 3: 复制 constraint.rs 和 category.rs**

```bash
cp BN/src/node/constraint.rs TARGET/src/node/constraint.rs
cp BN/src/node/category.rs TARGET/src/node/category.rs
```

- [ ] **Step 4: 创建 node/mod.rs（渐进式，只声明当前已有的模块）**

先只声明 Task 2 中已搬入的 3 个模块。后续 Task 会逐步添加更多 `pub mod` 声明。

```rust
pub mod category;
pub mod constraint;
pub mod types;

pub use category::{CategoryId, CategoryInfo, CategoryRegistry};
pub use constraint::{Constraint, ConstraintType};
pub use types::{DataTypeId, DataTypeInfo, DataTypeRegistry, Value};
```

**重要**：后续每个 Task 搬入新文件时，必须在 node/mod.rs 中添加对应的 `pub mod` 声明。完整的最终 mod.rs �� Task 14。

- [ ] **Step 5: Commit**

```bash
git add src/node/
git commit -m "feat: add node system foundation (types, constraint, category)"
```

---

### Task 3: Widget 系统

**Files:**
- Copy: `BN/src/node/widget/` → `TARGET/src/node/widget/`（7 个文件）
- Copy: `NIS/src/node/widget/text_input.rs` → `TARGET/src/node/widget/text_input.rs`

- [ ] **Step 1: 复制整个 widget 目录从 builtin-nodes**

```bash
cp -r BN/src/node/widget TARGET/src/node/
```

- [ ] **Step 2: 复制 text_input.rs 从 node-image-studio（NIS 独有控件）**

```bash
cp NIS/src/node/widget/text_input.rs TARGET/src/node/widget/text_input.rs
```

- [ ] **Step 3: 更新 widget/mod.rs 添加 text_input 模块声明**

在 `TARGET/src/node/widget/mod.rs` 中添加：
```rust
pub mod text_input;
```

确保与已有的模块声明并列。

- [ ] **Step 4: Commit**

```bash
git add src/node/widget/
git commit -m "feat: add widget system (8 controls + text_input)"
```

---

### Task 4: NodeRegistry（registry.rs）

**Files:**
- Copy: `BN/src/node/registry.rs` → `TARGET/src/node/registry.rs`（合并基础，已含 gpu_process）

- [ ] **Step 1: 复制 registry.rs 从 builtin-nodes**

```bash
cp BN/src/node/registry.rs TARGET/src/node/registry.rs
```

此文件已包含 `gpu_process: Option<GpuProcessFn>` 字段。确认 `GpuProcessFn` 的类型定义存在且引用 `crate::gpu::GpuContext`。

- [ ] **Step 2: Commit**

```bash
git add src/node/registry.rs
git commit -m "feat: add NodeRegistry with gpu_process support"
```

---

### Task 5: EvalEngine + Cache + Serializer + Menu

**Files:**
- Copy: `NIS/src/node/eval.rs` → `TARGET/src/node/eval.rs`（合并基础）
- Copy: `BN/src/node/cache.rs` → `TARGET/src/node/cache.rs`
- Copy: `BN/src/node/serial.rs` → `TARGET/src/node/serial.rs`
- Copy: `BN/src/node/menu.rs` → `TARGET/src/node/menu.rs`

- [ ] **Step 1: 复制 cache.rs, serial.rs, menu.rs（两项目一致）**

```bash
cp BN/src/node/cache.rs TARGET/src/node/cache.rs
cp BN/src/node/serial.rs TARGET/src/node/serial.rs
cp BN/src/node/menu.rs TARGET/src/node/menu.rs
```

- [ ] **Step 2: 复制 eval.rs 从 NIS（合并基础，含 AI 远程执行）**

```bash
cp NIS/src/node/eval.rs TARGET/src/node/eval.rs
```

- [ ] **Step 3: 合并 GPU 执行路径到 eval.rs**

打开 `BN/src/node/eval.rs` 和 `TARGET/src/node/eval.rs`。在 eval.rs 的节点执行函数中，找到调用 `process` 函数的位置，在其**前面**添加 GPU 路径检查：

```rust
// GPU 路径（优先）
if let Some(ref gpu_process) = node_def.gpu_process {
    if let Some(ref gpu_ctx) = gpu_ctx {
        let outputs = gpu_process(gpu_ctx, &inputs, &params);
        // 缓存 outputs...
        continue; // 或 return
    }
}
// CPU 路径（原有逻辑）
if let Some(ref process) = node_def.process {
    // ... 原有代码 ...
}
// AI 远程路径（原有逻辑）
// ... 原有的 AI 子图序列化代码 ...
```

具体代码需参考 BN 的 eval.rs 中 `gpu_process` 的调用方式，确保签名匹配。evaluate 函数签名需要接受 `gpu_ctx: Option<&Arc<GpuContext>>` 参数。

- [ ] **Step 4: Commit**

```bash
git add src/node/eval.rs src/node/cache.rs src/node/serial.rs src/node/menu.rs
git commit -m "feat: add eval engine (GPU/CPU/AI paths), cache, serializer, menu"
```

---

### Task 6: Processing 模块

**Files:**
- Copy: `BN/src/processing/` → `TARGET/src/processing/`（模块化版本，5 个文件）

- [ ] **Step 1: 复制整个 processing 目录**

```bash
cp -r BN/src/processing TARGET/src/
```

- [ ] **Step 2: 确认模块结构**

检查 `TARGET/src/processing/mod.rs` 包含：
```rust
pub mod color;
pub mod composite;
pub mod filter;
pub mod generate;
pub mod transform;
```

- [ ] **Step 3: Commit**

```bash
git add src/processing/
git commit -m "feat: add modular processing library (color, filter, transform, composite, generate)"
```

---

### Task 7: GPU 模块

**Files:**
- Copy: `BN/src/gpu/` → `TARGET/src/gpu/`（mod.rs + pipeline.rs + texture.rs + 21 shaders）

- [ ] **Step 1: 复制整个 gpu 目录**

```bash
cp -r BN/src/gpu TARGET/src/
```

- [ ] **Step 2: 确认文件完整性**

```bash
ls TARGET/src/gpu/shaders/*.wgsl | wc -l
# 预期输出: 21
```

- [ ] **Step 3: Commit**

```bash
git add src/gpu/
git commit -m "feat: add GPU module (GpuContext, pipeline, texture, 21 WGSL shaders)"
```

---

### Task 8: BackendClient + AiExecutor

**Files:**
- Copy: `BN/src/node/backend.rs` → `TARGET/src/node/backend.rs`（合并基础）
- Copy: `NIS/src/node/ai_task.rs` → `TARGET/src/node/ai_task.rs`

- [ ] **Step 1: 复制 backend.rs 从 NIS（合并基础 — NIS 是 BN 的严格超集）**

```bash
cp $NIS/src/node/backend.rs $TARGET/src/node/backend.rs
```

NIS 的 backend.rs 已包含 BN 的全部方法（`fetch_node_types`、`health_check`、`register_remote_nodes` 等），加上 NIS 独有的 AI 方法（`serialize_ai_subgraph`、`collect_ai_node_ids`、`parse_backend_response`、`decode_base64_image`、`value_to_json`）。

- [ ] **Step 2: 确保 register_remote_nodes 中 NodeDef 包含 gpu_process 字段**

打开 `$TARGET/src/node/backend.rs`，找到 `register_remote_nodes()` 方法中创建 `NodeDef` 的位置。确保 struct 初始化包含 `gpu_process: None`。如果 NIS 版本没有此字段（因为 NIS 的 NodeDef 不含 gpu_process），需要手动添加。

- [ ] **Step 3: 复制 ai_task.rs 从 NIS**

```bash
cp NIS/src/node/ai_task.rs TARGET/src/node/ai_task.rs
```

- [ ] **Step 4: Commit**

```bash
git add src/node/backend.rs src/node/ai_task.rs
git commit -m "feat: add BackendClient (merged) + AiExecutor"
```

---

### Task 9: Viewer（viewer.rs）

**Files:**
- Copy: `NIS/src/node/viewer.rs` → `TARGET/src/node/viewer.rs`（合并基础）

这是最复杂的合并。NIS 版本包含 AI 相关字段和 UI 修复，BN 版本包含 GPU 上下文。

- [ ] **Step 1: 复制 viewer.rs 从 NIS（合并基础）**

```bash
cp NIS/src/node/viewer.rs TARGET/src/node/viewer.rs
```

- [ ] **Step 2: 添加 GPU 字段到 NodeViewer struct**

在 `NodeViewer` struct 定义中添加：
```rust
pub gpu_ctx: Option<Arc<GpuContext>>,
```

添加 import：
```rust
use crate::gpu::GpuContext;
```

- [ ] **Step 3: 更新 NodeViewer::new() 接受 gpu_ctx 参数**

修改构造函数签名：
```rust
pub fn new(theme: Arc<dyn Theme>, gpu_ctx: Option<Arc<GpuContext>>) -> Self {
```

在 struct 初始化中添加 `gpu_ctx` 字段。

- [ ] **Step 4: 更新 evaluate 调用传递 gpu_ctx**

在 viewer.rs 中调用 eval engine 的地方，传递 `self.gpu_ctx.as_ref()`：

```rust
// 找到调用 evaluate 的地方，添加 gpu_ctx 参数
eval::evaluate(&self.node_registry, ..., self.gpu_ctx.as_ref(), ...);
```

- [ ] **Step 5: Commit**

```bash
git add src/node/viewer.rs
git commit -m "feat: add merged viewer (GPU context + AI executor + UI fixes)"
```

---

### Task 10: Theme + UI

**Files:**
- Copy: `BN/src/theme/` → `TARGET/src/theme/`
- Copy: `BN/src/ui/` → `TARGET/src/ui/`

- [ ] **Step 1: 复制 theme 和 ui 目录**

```bash
cp -r BN/src/theme TARGET/src/
cp -r BN/src/ui TARGET/src/
```

两个项目的 theme/ 和 ui/ 基本一致，使用 BN 版本。

- [ ] **Step 2: Commit**

```bash
git add src/theme/ src/ui/
git commit -m "feat: add theme system and UI components"
```

---

### Task 11: App（app.rs）

**Files:**
- Copy: `NIS/src/app.rs` → `TARGET/src/app.rs`（合并基础）

- [ ] **Step 1: 复制 app.rs 从 NIS（合并基础）**

```bash
cp NIS/src/app.rs TARGET/src/app.rs
```

NIS 版本包含完整的 Python 后端启动、文件管理、NIS_BACKEND_URL、auto_save 和 Drop 实现。

- [ ] **Step 2: 添加 GPU 初始化逻辑**

参考 `BN/src/app.rs`，在 `App::new(cc)` 中添加 GpuContext 初始化：

```rust
use crate::gpu::GpuContext;

// 在 App::new(cc) 的开头添加：
let gpu_ctx = GpuContext::from_eframe(cc);
if gpu_ctx.is_some() {
    eprintln!("[gpu] GPU context initialized successfully");
} else {
    eprintln!("[gpu] GPU not available, using CPU fallback");
}
```

- [ ] **Step 3: 更新 NodeViewer::new() 调用传递 gpu_ctx**

```rust
let mut viewer = NodeViewer::new(Arc::clone(&theme), gpu_ctx);
```

- [ ] **Step 4: 修复 crate 引用路径**

将所有 `use node_image_studio::` 改为 `use crate::`（或确保 lib.rs 中的模块导出正确）。main.rs 中的 `use node_image_studio::app` 已改为 `use nodeimg::app`。

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: add merged app (GPU init + AI backend + file management)"
```

---

### Task 12: Builtin 节点

**Files:**
- Copy: `BN/src/node/builtins/` → `TARGET/src/node/builtins/`（30 个文件）
- Create: `TARGET/src/node/builtins/curves.rs`（v2 占位）

- [ ] **Step 1: 复制整个 builtins 目录从 BN**

```bash
cp -r $BN/src/node/builtins $TARGET/src/node/
```

BN 的 builtins/ 包含 31 个节点 .rs 文件 + mod.rs（共 32 个文件）。注意：5 个 AI 节点（load_checkpoint 等）不存在静态 Rust 文件 — 它们通过 `BackendClient::register_remote_nodes()` 从 Python 后端动态注册。

- [ ] **Step 2: 检查共有节点是否需要更新**

BN 和 NIS 共有 4 个节点文件（load_image.rs、color_adjust.rs、preview.rs、save_image.rs）。BN 版本已兼容 gpu_process，保留即可。

- [ ] **Step 3: 创建 curves.rs 占位文件**

```rust
// v2 占位 — 需要 Curve 约束和 CurveEditor 控件后实现
// 见 catalog.md 中 Curves 节点的完整规格

use crate::node::registry::{NodeDef, NodeRegistry};

pub fn register(_registry: &mut NodeRegistry) {
    // TODO: v2 实现
    // 需要先注册 Curve 约束和 CurveEditor 控件
}
```

- [ ] **Step 4: 确保 builtins/mod.rs 调用 curves::register**

在 `TARGET/src/node/builtins/mod.rs` 的 `register_all()` 中添加：
```rust
pub mod curves;

// 在 register_all() 函数中添加：
curves::register(registry);
```

- [ ] **Step 5: 更新 node/mod.rs 添加 builtins 模块**

在 `$TARGET/src/node/mod.rs` 中确认 `pub mod builtins;` 已声明。

- [ ] **Step 6: Commit**

```bash
git add src/node/builtins/
git commit -m "feat: add 32 builtin node definitions + curves v2 placeholder"
```

---

### Task 13: Python 后端

**Files:**
- Copy: `NIS/python/` → `TARGET/python/`

- [ ] **Step 1: 复制 python 目录并清理生成物**

```bash
cp -r $NIS/python $TARGET/
rm -rf $TARGET/python/.venv $TARGET/python/__pycache__ $TARGET/python/nodes/__pycache__
```

- [ ] **Step 2: 确认文件完整性**

```bash
ls $TARGET/python/
# 预期: server.py registry.py executor.py device.py download_models.py requirements.txt nodes/
ls $TARGET/python/nodes/
# 预期: __init__.py load_checkpoint.py clip_text_encode.py empty_latent_image.py ksampler.py vae_decode.py
```

- [ ] **Step 3: Commit**

```bash
git add python/
git commit -m "feat: add Python AI backend (FastAPI + SDXL nodes)"
```

---

### Task 14: 编译验证与修复

**Files:**
- Modify: 多个文件的 import 路径修复

- [ ] **Step 1: 首次 cargo build**

```bash
cd TARGET
cargo build 2>&1 | head -100
```

预期会有编译错误，主要是 import 路径问题。

- [ ] **Step 2: 修复 crate 路径引用**

常见问题：
1. 源文件中 `use node_image_studio::` → 改为 `use crate::`
2. 源文件中 `use node_image_studio::node::` → 改为 `use crate::node::`
3. main.rs 中 `use node_image_studio::app` → 已改为 `use nodeimg::app`

批量查找替换：
```bash
# 在 src/ 下查找所有引用旧 crate 名的地方
grep -r "node_image_studio" TARGET/src/
# 逐一替换为 nodeimg 或 crate
```

- [ ] **Step 3: 修复缺失的 import**

合并文件可能引用了未 import 的类型。逐个修复 `cargo build` 报告的错误。

常见缺失 import：
- `use crate::gpu::GpuContext;` — 在 viewer.rs, eval.rs, registry.rs 中
- `use crate::node::ai_task::AiExecutor;` — 在 viewer.rs 中
- `use crate::node::backend::BackendClient;` — 在 viewer.rs, eval.rs 中

- [ ] **Step 4: 反复 cargo build 直到无错误**

```bash
cargo build 2>&1
# 预期输出末尾: Finished `dev` profile [unoptimized + debuginfo] target(s)
```

- [ ] **Step 5: 最终化 node/mod.rs — 确认包含全部 13 个模块声明**

```rust
pub mod ai_task;
pub mod backend;
pub mod builtins;
pub mod cache;
pub mod category;
pub mod constraint;
pub mod eval;
pub mod menu;
pub mod registry;
pub mod serial;
pub mod types;
pub mod viewer;
pub mod widget;

pub use category::{CategoryId, CategoryInfo, CategoryRegistry};
pub use constraint::{Constraint, ConstraintType};
pub use registry::{NodeDef, NodeInstance, NodeRegistry, ParamDef, PinDef, ProcessFn, GpuProcessFn};
pub use types::{DataTypeId, DataTypeInfo, DataTypeRegistry, Value};
```

- [ ] **Step 6: cargo run 验证**

```bash
cargo run
```

预期：窗口启动，显示空节点编辑器。右键菜单应显示 7 个分类和 32 个静态注册的节点。额外 5 个 AI 节点需要 Python 后端运行后才会出现。

- [ ] **Step 7: Commit**

```bash
git add src/ Cargo.toml
git commit -m "fix: resolve all import paths and compilation errors"
```

---

### Task 15: 删除旧项目

**Files:**
- Delete: `nodestudio/node-image-studio-builtin-nodes/`
- Delete: `nodestudio/node-image-studio-ai-debug/`

- [ ] **Step 1: 确认 nodeimg 可以正常 cargo build 和 cargo run**

```bash
cd TARGET
cargo build --release
cargo run --release
```

- [ ] **Step 2: 删除旧项目目录**

```bash
rm -rf /Users/aporicho/Desktop/nodestudio/node-image-studio-builtin-nodes
rm -rf /Users/aporicho/Desktop/nodestudio/node-image-studio-ai-debug
```

- [ ] **Step 3: 确认最终目录结构**

```bash
ls /Users/aporicho/Desktop/nodestudio/
# 预期: node-image-studio  nodeimg
```

node-image-studio 保留（有 git 历史），nodeimg 是合并后的新项目。

- [ ] **Step 4: Final commit**

```bash
cd TARGET
git add -A
git commit -m "chore: project merge complete - nodeimg is the unified project"
```
