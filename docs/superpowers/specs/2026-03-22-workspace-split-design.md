# Workspace 拆分设计

关联 Issue：#13（workspace 拆分 + eval engine 后移，前端薄客户端化）

## 目标

将 nodeimg 从单 crate 重构为 Rust workspace，拆成 6 个 crate，实现：

1. UI 与计算逻辑的编译期隔离，为 iced 迁移（#14）铺路
2. 引擎可独立运行，为远端执行（#12）铺路
3. 模块职责清晰，降低维护成本

## 约束

- `cargo run` 行为完全不变，功能不丢
- 这次只做代码搬迁和接口调整，不改算法实现
- 不实现 transport trait 和远端模式（后续 Issue）

## 分层架构

```
依赖方向：只能从上往下，不能反过来
═══════════════════════════════════════════

第 5 层 · 应用
┌──────────────────┐  ┌──────────────────┐
│  nodeimg-app     │  │  nodeimg-server   │
│  编辑器 UI       │  │  HTTP 服务（空壳） │
└────────┬─────────┘  └────────���──────────┘
         │                     │
         ▼                     ▼
第 4 层 · 引擎
┌──────────────────────────────────────────┐
│  nodeimg-engine                           │
│  执行引擎 · 节点注册表 · 内置节点 · AI代理 │
└────────┬──────────────┬──────────────────┘
         │              │
         ▼              ▼
第 3 层 · 计算
┌─────────────────┐  ┌─────────────────────┐
│  nodeimg-gpu     │  │  nodeimg-processing  │
│  GPU 计算管线    │  │  CPU 图像算法        │
│  WGSL shaders   │  │                      │
└────────┬────────┘  └────────┬────────────┘
         │                    │
         ▼                    ▼
第 2 层 · 基础
┌──────────────────────────────────────────┐
│  nodeimg-types                            │
│  数据类型 · GPU纹理 · 节点定义 · 约束 · 序列化│
└──────────────────────────────────────────┘
```

## 各 crate 详细设计

### nodeimg-types（第 2 层 · 基础）

所有 crate 共用的数据定义。不含任何业务逻辑。

**外部依赖**：serde, serde_json, image, wgpu, bytemuck

**文件结构**：
```
crates/nodeimg-types/src/
├── lib.rs              # 公开导出
├── value.rs            # Value 枚举（Image, GpuImage, Mask, Float, Int, Color, Boolean, String）
├── data_type.rs        # DataTypeId, DataTypeInfo, DataTypeRegistry
├── node_def.rs         # NodeDef, PinDef, ParamDef, ProcessFn, GpuProcessFn
├── constraint.rs       # Constraint 枚举（slider 范围、dropdown 选项等）
├── category.rs         # CategoryId
├── gpu_texture.rs      # GpuTexture 结构体（从 gpu/texture.rs 搬来）
└── serial.rs           # 项目文件序列化/反序列化
```

**来源映射**：
| 新文件 | 原文件 |
|--------|--------|
| value.rs | src/node/types.rs（Value 枚举部分） |
| data_type.rs | src/node/types.rs（DataType 部分） |
| node_def.rs | src/node/registry.rs（NodeDef、PinDef、ParamDef、ProcessFn、GpuProcessFn） |
| constraint.rs | src/node/constraint.rs |
| category.rs | src/node/category.rs |
| gpu_texture.rs | src/gpu/texture.rs |
| serial.rs | src/node/serial.rs |

**关键决策**：
- GpuTexture 放在这里，因为它是 Value 枚举的一部分，属于基础数据类型
- ProcessFn 和 GpuProcessFn 类型别名放在 node_def.rs 中，因为它们是节点定义的一部分
- nodeimg-types 依赖 wgpu 是合理的——GPU 纹理是这个项目的基础数据类型

---

### nodeimg-gpu（第 3 层 · GPU 计算）

GPU 计算管线管理和着色器。不含数据类型定义（在 types 中）。

**外部依赖**：wgpu, bytemuck, nodeimg-types

**文件结构**：
```
crates/nodeimg-gpu/src/
├── lib.rs              # 公开 GpuContext 和管线相关 API
├── context.rs          # GpuContext（设备初始化、管线缓存）
├── pipeline.rs         # compute pipeline 创建辅助
└── shaders/            # 21 个 WGSL 着色器（原样搬迁）
    ├── gaussian_blur_h.wgsl
    ├── gaussian_blur_v.wgsl
    ├── crop.wgsl
    └── ...
```

**来源映射**：
| 新文件 | 原文件 |
|--------|--------|
| context.rs | src/gpu/mod.rs（GpuContext 结构体） |
| pipeline.rs | src/gpu/pipeline.rs |
| shaders/ | src/gpu/shaders/（原样） |

**注意**：texture.rs 已移到 nodeimg-types/gpu_texture.rs

---

### nodeimg-processing（第 3 层 · CPU 算法）

纯 CPU 图像处理算法。无状态函数，输入图片 → 输出图片。

**外部依赖**：image, nodeimg-types

**文件结构**：
```
crates/nodeimg-processing/src/
├── lib.rs
├── color.rs            # 颜色调整（亮度、对比度、色相…）
├── filter.rs           # 滤镜（模糊、锐化、边缘检测…）
├── transform.rs        # 变换（缩放、旋转、裁剪、翻转…）
├── composite.rs        # 合成（混合、遮罩、通道合并…）
└── generate.rs         # 生成（纯色、渐变、噪声、棋盘格…）
```

**来源映射**：全部来自 src/processing/，原样搬迁。

---

### nodeimg-engine（第 4 层 · 引擎）

节点图的编排和执行。这是项目的核心业务逻辑层。

**外部依赖**：nodeimg-types, nodeimg-gpu, nodeimg-processing, reqwest, serde_json, base64

**文件结构**：
```
crates/nodeimg-engine/src/
├── lib.rs
├── registry.rs         # NodeRegistry（节点注册和查询）
├── eval.rs             # EvalEngine（拓扑排序、执行图、调度 GPU/CPU/AI）
├── cache.rs            # Cache（结果缓存和 invalidation）
├── backend.rs          # BackendClient（AI 后端 HTTP 通信）
├── ai_task.rs          # AiExecutor（AI 任务异步执行）
├── menu.rs             # 节点菜单（搜索、分类浏览）
├── widget_def.rs       # WidgetId, WidgetRegistry 定义（不含 UI 渲染）
└── builtins/           # 32 个内置节点定义
    ├── mod.rs           # register_all()
    ├── blur.rs
    ├── sharpen.rs
    ├── load_image.rs
    └── ...（其余 29 个）
```

**来源映射**：
| 新文件 | 原文件 |
|--------|--------|
| registry.rs | src/node/registry.rs（去掉 NodeDef 定义，保留 NodeRegistry） |
| eval.rs | src/node/eval.rs |
| cache.rs | src/node/cache.rs |
| backend.rs | src/node/backend.rs |
| ai_task.rs | src/node/ai_task.rs |
| menu.rs | src/node/menu.rs |
| widget_def.rs | src/node/widget/mod.rs（仅 WidgetId 和定义部分） |
| builtins/ | src/node/builtins/（原样） |

**需要修改的文件**：
- **ai_task.rs**：当前 `spawn()` 需要 `egui::Context` 调用 `request_repaint()`。改为接受 `Box<dyn Fn() + Send>` 回调，由 app 层注入 repaint 函数。
- **registry.rs**：NodeDef、PinDef、ParamDef 的定义已移到 nodeimg-types，这里只保留 NodeRegistry（注册表容器）和 NodeInstance（运行时实例）。

---

### nodeimg-app（第 5 层 · 编辑器）

用户交互界面。依赖 engine 获取计算能力。

**外部依赖**：nodeimg-engine, eframe, egui_extras, egui-snarl, rfd

**文件结构**：
```
crates/nodeimg-app/src/
├── main.rs             # 程序入口（eframe 启动）
├── app.rs              # NodeImgApp（eframe::App 实现）
├── viewer.rs           # NodeViewer（SnarlViewer 实现，连接 UI 和引擎）
├── ui/
│   ├── mod.rs
│   ├── node_canvas.rs  # 节点画布
│   └── preview_panel.rs # 预览面板
├── widget/             # 参数控件 UI 渲染
│   ├── mod.rs           # Widget trait（含 egui 渲染）
│   ├── slider.rs
│   ├── checkbox.rs
│   ├── dropdown.rs
│   ├── color_picker.rs
│   ├── file_picker.rs
│   ├── number_input.rs
│   ├── radio_group.rs
│   └── text_input.rs
└── theme/
    ├── mod.rs           # Theme trait
    ├── tokens.rs        # 设计 token
    ├── dark.rs          # 暗色主题
    └── light.rs         # 亮色主题
```

**来源映射**：
| 新文件 | 原文件 |
|--------|--------|
| main.rs | src/main.rs |
| app.rs | src/app.rs |
| viewer.rs | src/node/viewer.rs |
| ui/ | src/ui/（原样） |
| widget/ | src/node/widget/（UI 渲染部分） |
| theme/ | src/theme/（原样） |

---

### nodeimg-server（第 5 层 · 服务空壳）

本次仅创建空壳，预留位置。

**外部依赖**：nodeimg-engine

**文件结构**：
```
crates/nodeimg-server/src/
└── main.rs             # placeholder: fn main() { println!("nodeimg-server: not implemented yet"); }
```

---

## 依赖关系矩阵

```
              types  gpu  processing  engine  app  server
types           -     -      -          -      -     -
gpu             ✓     -      -          -      -     -
processing      ✓     -      -          -      -     -
engine          ✓     ✓      ✓          -      -     -
app             ✓     -      -          ✓      -     -
server          -     -      -          ✓      -     -
```

app 不直接依赖 gpu 和 processing，通过 engine 间接访问。

## 需要修改的代码（非原样搬迁）

### 1. ai_task.rs：去除 egui 依赖

**现状**：`AiExecutor::spawn()` 接收 `egui::Context`，任务完成后调用 `ctx.request_repaint()`。

**改为**：接收 `Box<dyn Fn() + Send + Sync>` 回调。

```rust
// 改前（engine 层）
pub fn spawn(&mut self, ctx: &egui::Context, ...) { ... }

// 改后（engine 层）
pub fn spawn(&mut self, on_complete: Box<dyn Fn() + Send + Sync>, ...) { ... }

// app 层调用时
executor.spawn(Box::new(move || ctx.request_repaint()), ...);
```

### 2. registry.rs：拆分定义和容器

**现状**：NodeDef、PinDef、ParamDef、NodeRegistry、NodeInstance 全在一个文件。

**改为**：
- 定义（NodeDef、PinDef、ParamDef、ProcessFn、GpuProcessFn）→ nodeimg-types/node_def.rs
- 容器（NodeRegistry、NodeInstance）→ nodeimg-engine/registry.rs

### 3. widget 模块：拆分定义和渲染

**现状**：WidgetId、Widget trait（含 egui 渲染）、具体控件实现，全在 src/node/widget/。

**改为**：
- WidgetId + WidgetRegistry 定义 → nodeimg-engine/widget_def.rs
- Widget trait + 具体渲染实现 → nodeimg-app/widget/

### 4. types.rs：拆分为多文件

**现状**：Value 枚举、DataTypeId、DataTypeInfo、DataTypeRegistry 全在一个文件。

**改为**：
- Value 枚举 → nodeimg-types/value.rs
- DataType 相关 → nodeimg-types/data_type.rs

### 5. Cargo.toml：workspace 配置

**根 Cargo.toml**：
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

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
image = "0.25"
wgpu = "27"
bytemuck = { version = "1", features = ["derive"] }
```

## 搬迁验证标准

每个阶段完成后：
1. `cargo build` 通过
2. `cargo test` 通过（如有测试）
3. `cargo run -p nodeimg-app` 行为与原来的 `cargo run` 一致

## 后续 Issue（本次不做）

- **Transport trait + LocalTransport**：在 engine 中定义 ProcessingTransport trait，app 通过 trait 调用 engine
- **nodeimg-server 实现**：HTTP 服务 + headless wgpu 初始化
- **HttpTransport**：app 可连接远端 server
- **GPU/CPU 双实现精简**：统一计算路径，减少重复实现
