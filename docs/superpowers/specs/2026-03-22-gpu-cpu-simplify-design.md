# GPU/CPU 双实现精简设计

关联 Issue：#59（refactor: GPU/CPU 双实现精简）、#6（feat: GPU Resize 支持 Bicubic/Lanczos3）

## 目标

去掉与 GPU shader 重复的 CPU 算法代码，每个算法只保留一份实现（WGSL shader），降低维护成本。

## 现状

- 27 个节点同时有 CPU（`process`）和 GPU（`gpu_process`）两套实现
- CPU 算法在 `nodeimg-processing/`（~2273 行），GPU shader 在 `nodeimg-gpu/src/shaders/`（~1448 行）
- 两套代码做同一件事，改一个算法要改两份
- eval engine 优先走 GPU，GPU 不可用时走 CPU

## 关键发现

1. **nodeimg 是图形界面应用**：eframe 渲染窗口本身就需要 GPU，所以只要程序能跑起来，GPU 一定可用
2. **wgpu 27 没有 CPU 后端**：不能靠 wgpu 在无 GPU 环境执行 shader
3. **有些节点只能走 CPU**：load_image（文件读取）、save_image（文件写入）、histogram（CPU 分析）、preview（显示）

## 设计

### 节点分类

| 分类 | 节点 | 保留 process | 保留 gpu_process |
|------|------|-------------|-----------------|
| **纯 GPU** | blur, sharpen, crop, resize, rotate, flip, distort, denoise, edge_detect, emboss, pixelate, vignette, film_grain, invert, threshold, levels, color_adjust, color_balance, hue_saturation, blend, mask, channel_merge, checkerboard, gradient, noise, solid_color, lut_apply | 删除 | 保留 |
| **纯 CPU** | load_image, save_image, histogram, preview | 保留 | 无 |
| **未实现** | curves | 保留占位 | 无 |

共 27 个节点从双实现变为 GPU 单实现。

### 精简 nodeimg-processing

删除与 GPU shader 重复的函数。保留：

| 文件 | 保留内容 | 理由 |
|------|---------|------|
| `color.rs` | `compute_histogram()`, `render_histogram_image()` | histogram 节点的 CPU 计算和渲染，无 GPU 替代 |
| `color.rs` | `parse_cube_lut()` | LUT 文件解析，文件 I/O 必须在 CPU |
| `filter.rs` | 全部删除 | shader 完全覆盖 |
| `transform.rs` | 全部删除 | shader 完全覆盖 |
| `composite.rs` | 全部删除 | shader 完全覆盖 |
| `generate.rs` | 全部删除 | shader 完全覆盖 |

### 修改 AI 节点识别方式

当前代码用 `process.is_none()` 来识别 AI 节点（`eval.rs`、`backend.rs` 多处）。去掉 CPU process 后，27 个 GPU 节点也会 `process.is_none()`，会被误判为 AI 节点。

**修复**：AI 节点的正确判据改为 `process.is_none() && gpu_process.is_none()`（既没有 CPU 也没有 GPU 实现的节点才是 AI 节点）。需要修改：
- `eval.rs` 中的 `pending_ai_execution()`
- `backend.rs` 中的 `collect_ai_node_ids()` 和 `serialize_ai_subgraph()`

### 修改 eval engine

当前行为：节点没有可执行路径时**静默跳过**（不输出、不报错）。

改为：如果节点有 `gpu_process` 但 `gpu_ctx` 为 None，**返回错误**而不是静默跳过。

```rust
// GPU 节点在无 GPU 环境下明确报错
if def.gpu_process.is_some() && gpu_ctx.is_none() && def.process.is_none() {
    return Err(format!("Node '{}' requires GPU but no GPU context available", def.title));
}
```

### 修改 builtins

27 个纯 GPU 节点：
- 删除 `process` 字段（设为 `None`）
- 删除节点文件中的 CPU `process` 函数
- 保留 `gpu_process` 不变

5 个纯 CPU 节点（load_image, save_image, histogram, preview, curves）：
- 不改

### 补全 GPU shader 功能

resize.wgsl 目前只支持 bilinear，缺 bicubic 和 lanczos3。需要补全，否则 resize 节点在删除 CPU 后会丢功能。

本次一并完成 #6（GPU Resize 支持 Bicubic/Lanczos3），在 resize.wgsl 中实现 bicubic（4x4 采样核）和 lanczos3（6x6 采样核）插值，确保 resize 节点可以完全去掉 CPU fallback。

## 影响范围

- `crates/nodeimg-processing/src/` — 删除大部分函数（~1800 行）
- `crates/nodeimg-engine/src/builtins/` — 27 个文件删除 process 函数
- `crates/nodeimg-engine/src/eval.rs` — 修改执行逻辑和 AI 节点识别
- `crates/nodeimg-engine/src/backend.rs` — 修改 AI 节点识别判据
- `crates/nodeimg-engine/src/registry.rs` — 无需修改（process 已经是 Option）
- `crates/nodeimg-gpu/src/shaders/resize.wgsl` — 补全 bicubic/lanczos3

## 不做的事

- 不动 GPU shader 代码
- 不改 GPU 上下文初始化方式
- 不处理 headless/无 GPU 场景（那是 server 的事，#58）

## 验证标准

1. `cargo build --workspace` 通过
2. `cargo test --workspace` 通过
3. `cargo run -p nodeimg-app --release` 所有节点功能正常
4. 使用 GPU 加速的节点不再有 CPU 回退路径（验证 process 为 None）
