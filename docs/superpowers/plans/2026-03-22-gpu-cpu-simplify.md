# GPU/CPU 双实现精简 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 去掉 27 个节点的重复 CPU 算法实现，每个算法只保留 GPU shader 一份，同时补全 resize shader 的 bicubic/lanczos3 支持。

**Architecture:** 自底向上：先修复 AI 节点识别逻辑（避免误判），再补全 resize shader，然后逐模块删除 CPU 重复代码，最后清理 processing crate。

**Tech Stack:** Rust, WGSL compute shaders, wgpu

**Spec:** `docs/superpowers/specs/2026-03-22-gpu-cpu-simplify-design.md`

---

## Task 1: 修复 AI 节点识别逻辑

**目标**：将 AI 节点判据从 `process.is_none()` 改为 `process.is_none() && gpu_process.is_none()`，避免去掉 CPU process 后 GPU 节点被误判为 AI 节点。

**Files:**
- Modify: `crates/nodeimg-engine/src/eval.rs`
- Modify: `crates/nodeimg-engine/src/backend.rs`

- [ ] **Step 1: 修改 eval.rs 中的 pending_ai_execution()**

在 `pending_ai_execution()` 函数中（约 line 113），将：
```rust
if def.process.is_none() {
```
改为：
```rust
if def.process.is_none() && def.gpu_process.is_none() {
```

- [ ] **Step 2: 修改 backend.rs 中的 collect_ai_node_ids()**

在 `collect_ai_node_ids()` 函数中（约 line 275），将：
```rust
if def.process.is_none() {
```
改为：
```rust
if def.process.is_none() && def.gpu_process.is_none() {
```

- [ ] **Step 3: 修改 backend.rs 中的 serialize_ai_subgraph()**

在 `serialize_ai_subgraph()` 中有两处检查：

第一处（约 line 127），验证输出节点是 AI 节点：
```rust
if output_def.process.is_some() {
    return None;
}
```
改为：
```rust
if output_def.process.is_some() || output_def.gpu_process.is_some() {
    return None;
}
```

第二处（约 line 145），BFS 只包含 AI 上游节点：
```rust
if def.process.is_none() {
```
改为：
```rust
if def.process.is_none() && def.gpu_process.is_none() {
```

- [ ] **Step 4: 修改 eval.rs 执行逻辑，GPU 节点无 GPU 时报错**

在 `evaluate()` 函数的执行分支后（约 line 230），添加对 GPU-only 节点的检查：
```rust
// 在现有的 AI 节点分支之前，添加：
if def.gpu_process.is_some() && gpu_ctx.is_none() && def.process.is_none() {
    return Err(format!("Node '{}' requires GPU but no GPU context available", def.title));
}
```

- [ ] **Step 5: 验证编译和测试**

```bash
cargo build --workspace && cargo test --workspace
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "refactor: 修复 AI 节点识别逻辑，支持 GPU-only 节点 #59"
```

---

## Task 2: 补全 resize shader（bicubic + lanczos3）

**目标**：在 resize.wgsl 中实现 bicubic 和 lanczos3 插值，使 resize 节点不再需要 CPU 回退。同时关闭 #6。

**Files:**
- Modify: `crates/nodeimg-gpu/src/shaders/resize.wgsl`
- Modify: `crates/nodeimg-gpu/src/shaders.rs`（如果 shader 拆分）
- Modify: `crates/nodeimg-engine/src/builtins/resize.rs`（去掉 GPU→CPU 回退逻辑）

- [ ] **Step 1: 实现 bicubic 插值**

在 resize.wgsl 中添加 bicubic 插值函数。使用 Catmull-Rom 样条（4x4 采样核）：

```wgsl
fn cubic_weight(x: f32) -> f32 {
    let ax = abs(x);
    if (ax <= 1.0) {
        return (1.5 * ax * ax * ax) - (2.5 * ax * ax) + 1.0;
    } else if (ax < 2.0) {
        return (-0.5 * ax * ax * ax) + (2.5 * ax * ax) - (4.0 * ax) + 2.0;
    }
    return 0.0;
}
```

在 main 函数中根据 method 参数分支，method==2 时使用 4x4 采样核的 bicubic。

- [ ] **Step 2: 实现 lanczos3 插值**

添加 lanczos3 函数（6x6 采样核）：

```wgsl
fn sinc(x: f32) -> f32 {
    if (abs(x) < 0.0001) { return 1.0; }
    let px = 3.14159265 * x;
    return sin(px) / px;
}

fn lanczos3_weight(x: f32) -> f32 {
    if (abs(x) >= 3.0) { return 0.0; }
    return sinc(x) * sinc(x / 3.0);
}
```

method==3 时使用 6x6 采样核。

- [ ] **Step 3: 更新 resize.rs 去掉 CPU 回退**

当前 `gpu_process()` 在 bicubic/lanczos3 时返回空 HashMap 触发 CPU 回退。删除这个逻辑，让所有 method 都走 GPU：

删除 resize.rs 中 gpu_process 函数里类似这样的代码：
```rust
// 删除：对不支持的 method 返回空 map
if method >= 2 {
    return HashMap::new();
}
```

- [ ] **Step 4: 验证编译和测试**

```bash
cargo build --workspace && cargo test --workspace
```

- [ ] **Step 5: 手动验证**

```bash
cargo run -p nodeimg-app --release
```

创建节点图：Load Image → Resize（选 bicubic）→ Preview，确认图片正确缩放。
再测试 lanczos3。

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat: GPU resize 支持 bicubic/lanczos3 插值 #6"
```

---

## Task 3: 删除 27 个 builtin 节点的 CPU process

**目标**：将 27 个纯 GPU 节点的 `process` 设为 `None`，删除对应的 CPU process 函数。

**Files:**
- Modify: `crates/nodeimg-engine/src/builtins/` — 27 个文件

**需要修改的节点文件（全部在 `crates/nodeimg-engine/src/builtins/`）：**
blur.rs, sharpen.rs, crop.rs, resize.rs, rotate.rs, flip.rs, distort.rs, denoise.rs, edge_detect.rs, emboss.rs, pixelate.rs, vignette.rs, film_grain.rs, invert.rs, threshold.rs, levels.rs, color_adjust.rs, color_balance.rs, hue_saturation.rs, blend.rs, mask.rs, channel_merge.rs, checkerboard.rs, gradient.rs, noise.rs, solid_color.rs, lut_apply.rs

**不改的节点：** load_image.rs, save_image.rs, histogram.rs, preview.rs, curves.rs

- [ ] **Step 1: 对每个文件执行相同操作**

对上述 27 个文件：
1. 将 `process: Some(Box::new(process))` 改为 `process: None`
2. 删除 `fn process(...)` 函数体
3. 删除不再需要的 `use nodeimg_processing::*` import
4. 保留 `gpu_process` 和相关 import 不变

特殊情况：
- **lut_apply.rs**：`gpu_process` 内部调用了 `nodeimg_processing::color::parse_cube_lut()`，这个 import 要保留。只删除 CPU 的 `process` 函数和 `lut_apply()` 调用。
- **resize.rs**：CPU process 函数在 Task 2 中已不再被回退使用，直接删除。

- [ ] **Step 2: 验证编译**

```bash
cargo build --workspace
```

预期：编译通过。processing crate 中部分函数会出现 unused warnings（下一步清理）。

- [ ] **Step 3: 验证测试**

```bash
cargo test --workspace
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "refactor: 27 个节点移除 CPU process，仅保留 GPU 路径 #59"
```

---

## Task 4: 精简 nodeimg-processing crate

**目标**：删除不再被调用的 CPU 算法函数，只保留仍有用途的函数。

**Files:**
- Modify: `crates/nodeimg-processing/src/color.rs` — 保留 compute_histogram, render_histogram_image, parse_cube_lut；删除其余
- Delete: `crates/nodeimg-processing/src/filter.rs` — 全部删除
- Delete: `crates/nodeimg-processing/src/transform.rs` — 全部删除
- Delete: `crates/nodeimg-processing/src/composite.rs` — 全部删除
- Delete: `crates/nodeimg-processing/src/generate.rs` — 全部删除
- Modify: `crates/nodeimg-processing/src/lib.rs` — 移除已删除模块声明

- [ ] **Step 1: 精简 color.rs**

保留以下函数（及其辅助类型/函数）：
- `compute_histogram()` — histogram 节点用
- `render_histogram_image()` — histogram 节点用
- `parse_cube_lut()` — lut_apply 节点的 GPU process 用
- `rgb_to_hsl()` / `hsl_to_rgb()` — 如果 compute_histogram 或 render_histogram_image 用到就保留

删除以下函数：
- `invert()`
- `threshold()`
- `levels()`
- `hsl_adjust()`
- `color_balance()`
- `color_adjust()`
- `lut_apply()` — GPU shader 替代

- [ ] **Step 2: 删除整个文件**

```bash
rm -f crates/nodeimg-processing/src/filter.rs
rm -f crates/nodeimg-processing/src/transform.rs
rm -f crates/nodeimg-processing/src/composite.rs
rm -f crates/nodeimg-processing/src/generate.rs
```

- [ ] **Step 3: 更新 lib.rs**

```rust
// crates/nodeimg-processing/src/lib.rs
pub mod color;
// filter, transform, composite, generate 已删除
```

- [ ] **Step 4: 修复编译错误**

删除模块后，如果有测试引用了已删除的函数，需要一并删除。

```bash
cargo build --workspace
```

逐一修复编译错误。

- [ ] **Step 5: 运行测试**

```bash
cargo test --workspace
```

部分 processing 测试会因为函数删除而需要移除。保留 color.rs 中相关函数的测试。

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "refactor: 精简 nodeimg-processing，删除 GPU shader 已覆盖的 CPU 算法 #59"
```

---

## Task 5: 最终验证与清理

**目标**：确认所有功能正常，更新 nodeimg-engine 的 Cargo.toml（如果 processing 依赖可以减少），推送并创建 PR。

- [ ] **Step 1: 完整构建和测试**

```bash
cargo build --workspace && cargo test --workspace
```

- [ ] **Step 2: 运行验证**

```bash
cargo run -p nodeimg-app --release
```

手动测试：
- 每个分类至少测一个节点：blur（滤镜）、crop（变换）、blend（合成）、gradient（生成）、color_adjust（颜色）
- 测试 resize 的 bicubic 和 lanczos3 模式
- 测试 lut_apply（确认 CPU 解析 + GPU 应用仍然工作）
- 测试 histogram（确认纯 CPU 节点不受影响）
- 测试 load_image → 多节点链 → preview

- [ ] **Step 3: 检查 nodeimg-engine 的 processing 依赖**

nodeimg-engine 的 Cargo.toml 仍然依赖 nodeimg-processing。检查 builtins 中是否还有文件 import nodeimg_processing。如果只剩 lut_apply 和 histogram，依赖仍然需要保留。

- [ ] **Step 4: 提交最终清理**

```bash
git add -A
git commit -m "refactor: GPU/CPU 精简完成，最终清理 #59"
```

- [ ] **Step 5: 推送并创建 PR**

```bash
git push origin HEAD
gh pr create --title "refactor: GPU/CPU 双实现精简 #59" --body "..."
```

PR 描述中包含：`Closes #59, Closes #6`
