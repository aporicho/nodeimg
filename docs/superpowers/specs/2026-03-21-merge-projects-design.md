# 三项目合并设计方案

## 概述

将 `node-image-studio`、`node-image-studio-builtin-nodes`、`node-image-studio-ai-debug` 三个子项目合并为一个全新项目 `nodeimg`。

### 决策记录

| 决策项 | 选择 | 原因 |
|-------|------|------|
| 合并基础 | 全新项目 cherry-pick | 最干净的代码库 |
| 项目名称 | nodeimg | 用户指定 |
| GPU 加速 | 包含 | 保留全部已实现的 GPU 能力 |
| 调试模式 | 不保留 | 合并后主项目已包含完整 AI 后端 |
| 旧项目 | 合并后删除 | 不保留存档 |

---

## 背景

### 三个项目的关系

| 项目 | 功能 | 独有能力 |
|------|------|---------|
| node-image-studio | 核心项目 | AI 后端（BackendClient + AiExecutor）、文件管理、autosave |
| node-image-studio-builtin-nodes | 扩展项目 | 30+ 节点、GPU 加速（WGPU）、模块化 processing |
| node-image-studio-ai-debug | 调试项目 | 无独有能力，是 AI 后端的精简开发环境 |

### 合并目标

一个项目同时拥有：
- 全部 32 个内置节点（含 v2 Curves 占位）
- GPU 加速（21 个已实现的 WGSL shader + CPU fallback）
- AI 后端集成（动态节点注册、异步推理执行）
- 文件管理（保存/加载/autosave）

---

## 架构设计

### 核心原则

沿用文档 `architecture.md` 的核心原则：**系统不硬编码任何具体类型。所有数据类型、约束、控件、分类、节点都通过注册表注册。**

### 10 个组件

| 组件 | 文件 | 职责 | 来源 |
|------|------|------|------|
| DataTypeRegistry | `node/types.rs` | 数据类型、引脚颜色、兼容规则、转换函数 | 两项目共有 |
| ConstraintRegistry | `node/constraint.rs` | 约束类型、校验函数 | 两项目共有 |
| WidgetRegistry | `node/widget/mod.rs` | 「数据类型 + 约束」→ 控件映射 | 两项目共有 |
| CategoryRegistry | `node/category.rs` | 节点分类、显示名称、排序 | 两项目共有 |
| NodeRegistry | `node/registry.rs` | 节点定义（引脚、参数��处理函数） | **合并**：增加 gpu_process 字段 |
| EvalEngine | `node/eval.rs` | 拓扑排序、求值执行 | **合并**：三条执行路径 |
| Cache | `node/cache.rs` | 节点输出缓存、局部失效 | 两项目共有 |
| Serializer | `node/serial.rs` | 节点图保存与加载 | 两项目共有 |
| Menu | `node/menu.rs` | 分类菜单和搜索 | 两项目共有 |
| BackendClient | `node/backend.rs` | AI 后端 HTTP 通信 | **合并**：两项目都有，需合并 |

### 新增独立模块

| 模块 | 文件 | 职责 | 来源 |
|------|------|------|------|
| GpuContext | `gpu/mod.rs` | wgpu Device/Queue 管理 | builtin-nodes 独有 |
| AiExecutor | `node/ai_task.rs` | AI 推理异步后台执行 | node-image-studio 独有 |

### 依赖关系

```
DataTypeRegistry          ConstraintRegistry          CategoryRegistry
       \                        |                        /
        \                       |                       /
         +-------→ WidgetRegistry ←----+               /
                        |                             /
                        v                            /
                   NodeRegistry ←-------------------+
                   /    |    \    \
                  v     v     v    v
           EvalEngine Cache Serializer Menu

           Viewer（读取以上全部注册表 + Cache）

           BackendClient（独立，被 AI 节点处理函数调用）
           GpuContext（独立，被 GPU 节点处理函数调用）
```

---

## 合并关键点

### 1. Value 类型系统（types.rs）

合并两个项目的 Value 枚举：

```rust
pub enum Value {
    Image(Arc<DynamicImage>),       // CPU 图像
    GpuImage(Arc<GpuTexture>),      // GPU 图像（来自 builtin-nodes）
    Mask(Arc<DynamicImage>),        // 灰度遮罩
    Float(f32),
    Int(i32),
    Color([f32; 4]),
    Boolean(bool),
    String(String),
}
```

兼容矩阵新增 `GpuImage ↔ Image` 自动转换。需要在 `DataTypeRegistry::with_builtins()` 中新增：
- 注册 `gpu_image` 数据类型（引脚颜色与 Image 相同）
- 实现 Image → GpuImage 转换函数（CPU → GPU upload）
- 实现 GpuImage → Image 转换函数（GPU → CPU readback，**注意：这是昂贵操作**，仅在 Save Image 和需要 CPU Image 的下游节点时触发）

### 2. 节点定义（registry.rs）

NodeDef 同时支持 CPU 和 GPU 处理函数：

```rust
pub struct NodeDef {
    pub type_id: String,
    pub title: String,
    pub category: CategoryId,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub has_preview: bool,
    pub process: Option<ProcessFn>,
    pub gpu_process: Option<GpuProcessFn>,  // 来自 builtin-nodes
    // process == None && gpu_process == None → AI 远程节点
}
```

### 3. 求值引擎（eval.rs）

三条执行路径，按优先级选择：

1. **GPU 路径**：`gpu_process.is_some()` 且 GpuContext 可用 → 调用 GPU shader
2. **CPU 路径**：`process.is_some()` → 调用 CPU 处理函数
3. **AI 远程路径**：两者都为 None → 序列化 AI 子图，POST 到 Python 后端

### 4. NodeViewer（viewer.rs）

合并两个版本的字段：

```rust
pub struct NodeViewer {
    // 共有（两项目一致）
    pub type_registry: DataTypeRegistry,
    pub category_registry: CategoryRegistry,
    pub node_registry: NodeRegistry,
    pub widget_registry: WidgetRegistry,
    pub cache: Cache,
    pub textures: HashMap<NodeId, TextureHandle>,
    pub preview_texture: Option<TextureHandle>,
    pub theme: Arc<dyn Theme>,
    // 来自 builtin-nodes
    pub gpu_ctx: Option<Arc<GpuContext>>,
    // 来自 node-image-studio
    pub backend: Option<BackendClient>,
    pub backend_status: Option<String>,
    pub ai_executor: AiExecutor,
    ai_errors: HashMap<NodeId, String>,
    pub graph_dirty: bool,
}
```

### 5. App（app.rs）

合并初始化逻辑：

```rust
pub struct App {
    snarl: Snarl<NodeInstance>,
    viewer: NodeViewer,
    preview_panel: PreviewPanel,
    node_canvas: NodeCanvas,
    theme: Arc<dyn Theme>,
    use_dark: bool,
    backend_process: Option<Child>,
    // 来自 node-image-studio
    current_file: Option<PathBuf>,
    last_saved_json: Option<String>,
    dirty: bool,
}
```

App::new() 中同时初始化 GpuContext（从 eframe CreationContext）和 Python 后端进程。保留 `NIS_BACKEND_URL` 环境变量支持（覆盖默认后端地址，用于连接远程 GPU 服务器）。

App::Drop 需同时包含 auto_save（保存当前图到 autosave.nis）和 backend process cleanup（kill Python 子进程）。

---

## 文件结构

```
nodestudio/nodeimg/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── app.rs
│   ├── gpu/
│   │   ├── mod.rs                  # GpuContext
│   │   ├── pipeline.rs             # ComputePipeline 创建与缓存
│   │   ├── texture.rs              # GpuTexture 封装
│   │   └── shaders/                # WGSL compute shaders（已实现的 21 个）
│   │       ├── color_adjust.wgsl
│   │       ├── invert.wgsl
│   │       ├── threshold.wgsl
│   │       ├── levels.wgsl
│   │       ├── hue_saturation.wgsl
│   │       ├── color_balance.wgsl
│   │       ├── gaussian_blur_h.wgsl
│   │       ├── gaussian_blur_v.wgsl
│   │       ├── box_blur_h.wgsl
│   │       ├── box_blur_v.wgsl
│   │       ├── sharpen.wgsl
│   │       ├── pixelate.wgsl
│   │       ├── vignette.wgsl
│   │       ├── film_grain.wgsl
│   │       ├── blend.wgsl
│   │       ├── mask.wgsl
│   │       ├── channel_merge.wgsl
│   │       ├── solid_color.wgsl
│   │       ├── gradient.wgsl
│   │       ├── checkerboard.wgsl
│   │       └── noise.wgsl
│   ├── node/
│   │   ├── mod.rs
│   │   ├── types.rs              # DataTypeRegistry + 内置数据类型
│   │   ├── constraint.rs         # ConstraintRegistry + 内置约束
│   │   ├── category.rs           # CategoryRegistry + 内置分类（含 AI 型）
│   │   ├── registry.rs           # NodeRegistry（含 gpu_process）
│   │   ├── eval.rs               # EvalEngine（GPU/CPU/AI 三路径）
│   │   ├── cache.rs              # Cache
│   │   ├── serial.rs             # Serializer
│   │   ├── menu.rs               # Menu
│   │   ├── viewer.rs             # SnarlViewer（GPU + AI）
│   │   ├── backend.rs            # BackendClient
│   │   ├── ai_task.rs            # AiExecutor
│   │   ├── widget/
│   │   │   ├── mod.rs            # WidgetRegistry
│   │   │   ├── slider.rs         # Slider + IntSlider
│   │   │   ├── checkbox.rs
│   │   │   ├── color_picker.rs
│   │   │   ├── dropdown.rs
│   │   │   ├── file_picker.rs
│   │   │   ├── text_input.rs     # 文本输入（AI 节点用）
│   │   │   ├── number_input.rs   # 数值输入框
│   │   │   └── radio_group.rs    # 单选按钮组
│   │   └── builtins/
│   │       ├── mod.rs            # 注册全部节点
│   │       ├── load_image.rs
│   │       ├── save_image.rs
│   │       ├── solid_color.rs
│   │       ├── gradient.rs
│   │       ├── noise.rs
│   │       ├── checkerboard.rs
│   │       ├── color_adjust.rs
│   │       ├── curves.rs         # v2 占位
│   │       ├── levels.rs
│   │       ├── hue_saturation.rs
│   │       ├── color_balance.rs
│   │       ├── invert.rs
│   │       ├── threshold.rs
│   │       ├── lut_apply.rs
│   │       ├── resize.rs
│   │       ├── crop.rs
│   │       ├── rotate.rs
│   │       ├── flip.rs
│   │       ├── distort.rs
│   │       ├── blur.rs
│   │       ├── sharpen.rs
│   │       ├── edge_detect.rs
│   │       ├── emboss.rs
│   │       ├── denoise.rs
│   │       ├── pixelate.rs
│   │       ├── vignette.rs
│   │       ├── film_grain.rs
│   │       ├── blend.rs
│   │       ├── mask.rs
│   │       ├── channel_merge.rs
│   │       ├── preview.rs
│   │       ├── histogram.rs
│   │       ├── load_checkpoint.rs
│   │       ├── clip_text_encode.rs
│   │       ├── empty_latent_image.rs
│   │       ├── ksampler.rs
│   │       └── vae_decode.rs
│   ├── processing/
│   │   ├── mod.rs
│   │   ├── color.rs              # color_adjust, hsl_adjust, levels, color_balance, invert, threshold, lut_apply
│   │   ├── filter.rs             # gaussian_blur, box_blur, sharpen, edge_detect, emboss, denoise, pixelate, vignette, film_grain
│   │   ├── transform.rs          # distort（resize/crop/rotate/flip 用 image crate 内置）
│   │   ├── composite.rs          # blend, mask_apply, channel_split, channel_merge
│   │   └── generate.rs           # solid_color, gradient, perlin_noise, checkerboard
│   ├── theme/
│   │   ├── mod.rs
│   │   ├── light.rs
│   │   ├── dark.rs
│   │   └── tokens.rs             # 设计令牌
│   └── ui/
│       ├── mod.rs
│       ├── preview_panel.rs
│       └── node_canvas.rs
└── python/
    ├── server.py
    ├── registry.py
    ├── executor.py
    ├── device.py
    ├── download_models.py
    ├── requirements.txt
    └── nodes/
        ├── __init__.py
        ├── load_checkpoint.py
        ├── clip_text_encode.py
        ├── empty_latent_image.py
        ├── ksampler.py
        └── vae_decode.py
```

---

## 构建顺序

| 阶段 | 组件 | 文件 | 说明 |
|------|------|------|------|
| 1 | 项目骨架 | `Cargo.toml`, `main.rs`, `lib.rs` | 创建项目，配置合并后的依赖 |
| 2 | DataTypeRegistry | `node/types.rs` | 合并 Value（含 GpuImage） |
| 3 | ConstraintRegistry | `node/constraint.rs` | 直接搬入 |
| 4 | WidgetRegistry + 8 控件 | `node/widget/` | 合并 8 个控件文件 |
| 5 | CategoryRegistry | `node/category.rs` | 直接搬入（含 AI 分类） |
| 6 | NodeRegistry | `node/registry.rs` | 合并（增加 gpu_process） |
| 7 | EvalEngine + Cache | `node/eval.rs` + `node/cache.rs` | 合并（三路径执行） |
| 8 | Serializer | `node/serial.rs` | 直接搬入 |
| 9 | Menu | `node/menu.rs` | 直接搬入 |
| 10 | Processing | `processing/` | 搬入模块化版本（零依赖纯函数库，builtin 节点依赖它） |
| 11 | GPU 模块 | `gpu/` | 搬入 GpuContext + 21 shaders |
| 12 | BackendClient + AiExecutor | `node/backend.rs` + `node/ai_task.rs` | 合并 backend.rs + 搬入 ai_task.rs |
| 13 | Viewer | `node/viewer.rs` | 合并（GPU + AI 字段） |
| 14 | App | `app.rs` | 合并初始化逻辑 |
| 15 | Theme + UI | `theme/` + `ui/` | 搬入 |
| 16 | 全部 Builtin 节点 | `node/builtins/` | 搬入 30 个节点文件 + 新建 curves.rs 占位 |
| 17 | Python 后端 | `python/` | 搬入整个目录 |
| 18 | 编译验证 | - | `cargo build` |
| 19 | 删除旧项目 | - | 删除 builtin-nodes 和 ai-debug 目录 |

---

## 依赖合并（Cargo.toml）

```toml
[package]
name = "nodeimg"
version = "0.1.0"
edition = "2021"

[dependencies]
# GUI
eframe = { version = "0.33", features = ["wgpu"] }  # 来自 builtin-nodes（增加 wgpu feature）
egui-snarl = { version = "0.9", features = ["serde"] }

# ��像
image = "0.25"

# GPU
wgpu = "27"                                           # 来自 builtin-nodes
bytemuck = { version = "1", features = ["derive"] }   # 来自 builtin-nodes

# 文件对话框
rfd = "0.17"

# HTTP（AI 后端通信）
reqwest = { version = "0.12", features = ["json", "blocking"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 图像扩展（缩略图、文件格式支持）
egui_extras = { version = "0.33", features = ["image", "file"] }

# Base64（AI 节点图像传输）
base64 = "0.22"
```

---

## 节点统计

### 内置节点（37 个文件）

| 分类 | 节点数 | 执行方式 |
|------|-------|---------|
| 数据型 | 2（load_image, save_image） | CPU |
| 生成型 | 4（solid_color, gradient, noise, checkerboard） | GPU + CPU fallback |
| 颜色处理型 | 8（color_adjust, curves*, levels, hue_saturation, color_balance, invert, threshold, lut_apply） | 6/8 GPU + CPU fallback（lut_apply 和 curves 无 shader） |
| 空间变换型 | 5（resize, crop, rotate, flip, distort） | CPU（GPU shader 待开发） |
| 滤镜型 | 8（blur, sharpen, edge_detect, emboss, denoise, pixelate, vignette, film_grain） | 5/8 GPU + CPU fallback（edge_detect, emboss, denoise 无 shader） |
| 合成型 | 3（blend, mask, channel_merge） | GPU + CPU fallback |
| 工具型 | 2（preview, histogram） | CPU |
| AI 型 | 5（load_checkpoint, clip_text_encode, empty_latent_image, ksampler, vae_decode） | 远程 Python 后端 |

*curves 为 v2 占位，需 Curve 约束和 CurveEditor 控件。两个源项目中均未实现，需按 catalog.md 规格新建占位文件。

### GPU Shader 实施状态

| 状态 | 数量 | shader 文件 |
|------|------|------------|
| ✅ 已实现 | 21 | color_adjust, invert, threshold, levels, hue_saturation, color_balance, gaussian_blur_h/v, box_blur_h/v, sharpen, pixelate, vignette, film_grain, blend, mask, channel_merge, solid_color, gradient, checkerboard, noise |
| 🔲 待开发 | 11 | edge_detect_sobel, edge_detect_laplacian, emboss, denoise, lut_apply, histogram, resize, crop, rotate, flip, distort |

---

## 文件来源映射

### 需要合并的文件（6 个核心文件）

这些文件在两个项目中存在但有差异，需要手动合并：

| 文件 | node-image-studio 提供 | builtin-nodes 提供 | 合并基础 |
|------|----------------------|-------------------|---------|
| `node/types.rs` | 基础 Value 枚举 | + GpuImage 变体 | builtin-nodes |
| `node/registry.rs` | 基础 NodeDef | + gpu_process 字段 | builtin-nodes |
| `node/eval.rs` | AI 子图序列化 + 远程执行 | GPU 执行路径 | node-image-studio |
| `node/viewer.rs` | backend, ai_executor, graph_dirty, show_output 对齐修复 | gpu_ctx | node-image-studio（包含更完整的 UI 修复和 graph_dirty 支持） |
| `node/backend.rs` | serialize_ai_subgraph, collect_ai_node_ids, parse_backend_response | register_remote_nodes 兼容 gpu_process 字段 | builtin-nodes（更完整的 NodeDef 兼容），再合并 AI 子图序列化方法 |
| `app.rs` | Python 后端启动, 文件管理, NIS_BACKEND_URL, auto_save | GpuContext 初始化 | node-image-studio |

### 直接搬入的文件

**来自 node-image-studio**（独有文件）：
- `node/ai_task.rs` — AiExecutor
- `node/widget/text_input.rs` — 文本输入控件
- `python/` — 整个 Python 后端目录

**来自 builtin-nodes**（独有文件）：
- `gpu/mod.rs`, `gpu/pipeline.rs`, `gpu/texture.rs` — GPU 基础设施
- `gpu/shaders/*.wgsl` — 21 个 compute shaders
- `processing/` — 模块化版本（5 个文件）
- `node/builtins/` — 26 个高级节点文件（builtin-nodes 独有的 30 个中减去 4 个共有的）

**需新建的文件**：
- `node/builtins/curves.rs` — v2 占位文件（两个项目中都不存在，需按 catalog.md 规格新建空壳）

**两项目共有（无差异，任取其一）**：
- `node/constraint.rs`, `node/category.rs`, `node/cache.rs`, `node/serial.rs`, `node/menu.rs`
- `node/widget/` 中的 slider, checkbox, color_picker, dropdown, file_picker, number_input, radio_group
- `node/builtins/` 中的 load_image, color_adjust, preview, save_image
- `theme/`, `ui/`

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| viewer.rs 合并复杂 | 高 | 以 node-image-studio 为基础，逐步添加 GPU 字段和逻辑 |
| builtin 节点引用路径变化 | 中 | 统一 crate 路径，批量替换 import |
| wgpu feature 影响编译时间 | 低 | eframe 已有 wgpu 后端，编译增量不大 |
| AI 节点与 GPU 节点的 eval 路径冲突 | 中 | 按优先级判断：gpu_process > process > AI 远程 |
| Cargo.toml 依赖冲突 | 低 | 两个项目使用相同版本的 eframe、egui-snarl 等核心依赖 |
| GPU readback 性能 | 中 | GpuImage → Image 转换（GPU readback）是昂贵操作，仅在 Save Image 或需要 CPU Image 的下游节点时触发 |
| 旧 .nis 文件兼容性 | 低 | 全新项目不保证与旧 autosave.nis 兼容，Serializer 的 version compat 机制可处理缺失字段 |

---

## 合并后删除

完成合并并通过 `cargo build` 验证后：

1. 删除 `nodestudio/node-image-studio-builtin-nodes/` 目录
2. 删除 `nodestudio/node-image-studio-ai-debug/` 目录
3. 保留 `nodestudio/node-image-studio/` 的 git 历史（可选）
4. `nodestudio/nodeimg/` 成为唯一的项目目录

---

## 不包含在本设计中

- 新功能开发（如缺失的 11 个 GPU shader）
- 代码重构或优化
- 测试套件
- CI/CD 配置
- 文档迁移（现有 docs/ 内容的整合）
