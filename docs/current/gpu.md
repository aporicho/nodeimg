# GPU 子系统

定义 GPU 加速的架构设计、执行流程和着色器规范。

---

## 技术栈

- **wgpu 27**（WebGPU API 的 Rust 实现）
- **WGSL**（WebGPU Shading Language）
- 通过 eframe 的 wgpu 后端共享 device 和 queue，无需独立初始化

---

## 组件结构

| 组件 | 文件 | 职责 |
|------|------|------|
| GpuContext | `gpu/mod.rs` | 持有 wgpu device/queue，管理 pipeline 缓存 |
| GpuTexture | `gpu/texture.rs` | GPU 纹理封装，CPU↔GPU 数据传输 |
| pipeline 辅助函数 | `gpu/pipeline.rs` | compute pipeline 创建、dispatch、bind group 构建 |
| WGSL shaders | `gpu/shaders/*.wgsl` | 每个 GPU 节点对应一个或多个着色器文件 |

---

## GpuContext

从 eframe 的渲染状态获取 device 和 queue，以 `Arc<GpuContext>` 在全应用共享。

**Pipeline 缓存：** 通过 `pipeline(name, shader_source)` 方法，按名称缓存已编译的 compute pipeline。相同名称直接返回缓存，避免重复编译。

---

## GpuTexture

GPU 端的 RGBA 纹理，格式固定为 `Rgba8Unorm`。

**核心操作：**
- `create_empty(device, width, height)` — 创建空纹理（用于输出）
- `from_dynamic_image(device, queue, img)` — CPU Image 上传到 GPU
- `to_dynamic_image(device, queue)` — GPU 纹理回读到 CPU Image
- `to_color_image(device, queue)` — GPU 纹理回读为 egui ColorImage（用于预览）

---

## 执行流程

EvalEngine 对每个节点按以下优先级选择执行路径：

```
1. GPU 路径（gpu_process 存在 且 GpuContext 可用）
   ├── 输入为 GpuImage → 直接使用
   ├── 输入为 CPU Image → 自动上传到 GPU
   └── 输出为 GpuImage

2. CPU 路径（process 存在）
   ├── 输入为 GpuImage → 自动回读到 CPU Image
   └── 输出为 CPU Image

3. AI 路径（process 和 gpu_process 均为 None）
   └── 序列化子图发送给 Python 后端
```

---

## 着色器规范

### 通用模式

所有 compute shader 使用 16x16 workgroup size：

```wgsl
@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(output);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    // ...
}
```

### Bind Group 布局

**无参数节点（binding 0 + 1）：**
- binding 0: input texture (read)
- binding 1: output texture (read_write)

**有参数节点（binding 0 + 1 + 2）：**
- binding 0: input texture (read)
- binding 1: output texture (read_write)
- binding 2: uniform buffer (参数结构体，通过 bytemuck 序列化)

### Pipeline 辅助函数

| 函数 | 用途 |
|------|------|
| `create_compute_pipeline` | 从 WGSL 源码创建 pipeline（auto layout） |
| `dispatch_compute` | 按 16x16 workgroup 提交计算（ceil-divide 覆盖所有像素） |
| `create_io_bind_group` | 构建输入+输出纹理的 bind group |
| `create_io_params_bind_group` | 构建输入+输出+uniform 参数的 bind group |

---

## 节点 GPU 实现模式

每个 builtin 节点的 `gpu_process` 函数遵循统一模式：

1. 从 inputs 获取输入（接受 `GpuImage` 或 `Image`，后者自动上传）
2. 创建空输出纹理
3. 通过 `gpu.pipeline(name, include_str!(...))` 获取/缓存 pipeline
4. 构建 bind group（有参数则包含 uniform buffer）
5. dispatch compute
6. 输出 `Value::GpuImage`

---

## GPU 加速状态

| 分类 | GPU 加速节点 | CPU-only 节点 |
|------|-------------|--------------|
| 生成型 | solid_color, gradient, noise, checkerboard | — |
| 颜色处理型 | color_adjust, invert, threshold, levels, hue_saturation, color_balance, lut_apply | curves (v2) |
| 空间变换型 | resize, crop, rotate, flip, distort | — |
| 滤镜型 | blur, sharpen, edge_detect, emboss, denoise, pixelate, vignette, film_grain | — |
| 合成型 | blend, mask, channel_merge | — |
| 数据型 | — | load_image, save_image (I/O) |
| 工具型 | — | histogram, preview (分析/展示) |

> resize 的 GPU 路径仅支持 nearest 和 bilinear 插值，bicubic/lanczos3 回退 CPU。
