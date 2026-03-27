# 系统架构

定义节点系统的组件划分、接口约定和依赖关系。将 domain.md 中的领域概念落地为可编码的系统结构。

**核心原则：** 系统不硬编码任何具体类型。所有数据类型、约束、控件、分类、节点都通过注册表注册。新增任何类型不需要修改已有代码，只需调用注册函数。

---

## 组件总览

| 组件 | 职责 |
|------|------|
| DataTypeRegistry | 管理数据类型、引脚颜色、兼容规则、转换函数 |
| ConstraintRegistry | 管理约束类型、校验函数 |
| WidgetRegistry | 管理「数据类型 + 约束」→ 控件渲染的映射（一对多） |
| CategoryRegistry | 管理节点分类、显示名称、排序 |
| NodeRegistry | 管理节点定义（引脚、参数、处理函数） |
| EvalEngine | 拓扑排序、并行调度、求值执行 |
| Cache | 节点输出缓存、局部失效、GPU 纹理管理 |
| Serializer | 节点图的保存与加载 |
| Menu | 从注册表生成分类菜单和搜索 |

| BackendClient | 与 AI 计算后端的 HTTP 通信，句柄管理 |

注册表之间有层级依赖，引擎依赖注册表，不存在反向依赖。

---

## 1. DataTypeRegistry

管理所有数据类型的注册和查询。

**接口约定：**
- **注册：** 类型标识、显示名称、引脚颜色
- **注册兼容规则：** 源类型 + 目标类型 + 转换函数
- **查询引脚颜色：** 给定类型 → 返回颜色
- **查询兼容性：** 给定源类型 + 目标类型 → 是否可连接
- **执行转换：** 给定源类型 + 目标类型 + 数据 → 转换后的数据

**依赖：** 无，最底层组件。

**内置注册：** Image、Mask、Float、Int、Color、Boolean、String 及它们之间的兼容矩阵（见 domain.md）。

---

## 2. ConstraintRegistry

管理所有约束类型的注册和校验。

**接口约定：**
- **注册：** 约束标识、校验函数（值是否满足约束）
- **查询：** 给定约束标识 → 返回约束定义
- **校验：** 给定约束 + 值 → 合法 / 不合法

**依赖：** 无。与 DataTypeRegistry 平级，互不依赖。

**内置注册：** Range(min, max)、Enum(options)、FilePath(filters)。

---

## 3. WidgetRegistry

管理「数据类型 + 约束」到控件的映射，支持一对多。

**接口约定：**
- **注册：** 匹配条件（数据类型 + 约束类型）+ 控件标识 + 渲染函数 + 是否为默认
- **查询可用控件：** 给定数据类型 + 约束 → 返回可用控件列表（含默认标记）
- **查询默认控件：** 给定数据类型 + 约束 → 返回默认控件
- **渲染：** 给定控件标识 + 参数值 + 是否禁用 → 绘制 UI，返回是否变化

**禁用状态：** 参数引脚已连线时，控件以只读/灰色状态渲染，显示当前值但不可交互。

**依赖：** DataTypeRegistry、ConstraintRegistry。

**内置控件：**

| 控件 | 文件 | 说明 |
|------|------|------|
| Slider | `widget/slider.rs` | 浮点数滑块，拖拽交互，值显示 |
| IntSlider | `widget/slider.rs` | 整数滑块，步进为 1 |
| Checkbox | `widget/checkbox.rs` | 勾选/取消 |
| ColorPicker | `widget/color_picker.rs` | 颜色选择弹窗 |
| Dropdown | `widget/dropdown.rs` | 下拉列表，选项渲染 |
| FilePicker | `widget/file_picker.rs` | 文件对话框触发，路径显示 |
| NumberInput | `widget/number_input.rs` | 数值输入框，手动输入 |
| TextInput | `widget/text_input.rs` | 文本输入框 |
| RadioGroup | `widget/radio_group.rs` | 单选按钮组 |

---

## 4. CategoryRegistry

管理节点的功能分类，供菜单分组使用。

**接口约定：**
- **注册：** 分类标识、显示名称、排序权重
- **查询：** 给定分类标识 → 返回显示名称
- **列表：** 返回所有分类，按权重排序

**依赖：** 无。独立组件。

**内置注册：** 数据型、生成型、颜色处理型、空间变换型、滤镜型、合成型、工具型、AI 型。

---

## 5. NodeRegistry

管理所有节点定义的注册和查询。是最核心的注册表，其他组件围绕它工作。

**接口约定：**
- **注册：** 节点定义（见下方）
- **查询：** 给定节点类型标识 → 返回完整定义
- **列表：** 返回所有已注册节点，可按分类过滤
- **实例化：** 给定节点类型标识 → 创建节点实例（参数填充默认值，生成唯一 ID）

**节点定义包含：**
- 类型标识（唯一）
- 显示标题
- 所属分类（引用 CategoryRegistry 中的分类标识）
- 输入引脚列表（名称 + 数据类型 + 必需/可选）
- 输出引脚列表（名称 + 数据类型）
- 参数列表（名称 + 数据类型 + 约束 + 默认值 + 控件覆盖）
- 启用预览区（布尔，声明节点是否显示 PreviewArea）
- 处理函数

**依赖：** DataTypeRegistry、ConstraintRegistry、WidgetRegistry、CategoryRegistry。

---

## 6. EvalEngine

执行节点图的求值计算。

**接口约定：**
- **求值：** 给定目标节点 + 节点图 → 返回计算结果
- **拓扑排序：** 给定节点图 → 返回执行顺序（或环路错误）
- **失效标记：** 给定变化的节点 → 返回需要重算的节点集合

**求值流程：**
1. 从目标节点出发，收集上游依赖子图
2. 拓扑排序，检测环路
3. 识别无依赖关系的节点组，可并行求值
4. 按序/并行执行：取输入 → 类型转换 → 调用处理函数 → 缓存结果
5. 错误隔离：单个节点失败不阻断其他独立路径

**并行策略：**
- 拓扑排序后，同一层级（无互相依赖）的节点可并行执行
- 图像处理是 CPU 密集型，通过线程池并行可显著提升性能
- 具体线程池实现由此组件内部决定，对外只暴露求值接口

**依赖：** NodeRegistry、DataTypeRegistry、Cache。

---

## 7. Cache

管理节点求值结果和 GPU 纹理资源。

**接口约定：**
- **写入：** 给定节点 ID + 结果 → 存入缓存
- **读取：** 给定节点 ID → 返回缓存结果（或未命中）
- **局部失效：** 给定节点 ID → 失效该节点及所有下游节点
- **全局失效：** 清空所有缓存
- **纹理管理：** 给定节点 ID + 图像数据 → 返回 GPU 纹理句柄（已有则复用，无则上传）

**失效规则：**
- 参数变化 → 局部失效（当前节点 + 下游）
- 连接变化 → 局部失效（受影响节点 + 下游）
- 纹理跟随数据失效：数据缓存失效时，对应纹理一并清除

**依赖：** 无运行时依赖。

---

## 8. Serializer

节点图的保存与加载。

**接口约定：**
- **保存：** 给定节点图 → 输出 JSON 数据
- **加载：** 给定 JSON 数据 → 还原节点图

**保存内容：**
- 格式版本号（如 `version: 1`，格式变更时递增）
- 节点列表（类型标识 + 位置 + 参数值）
- 连接列表（源节点/引脚 → 目标节点/引脚）
- 不保存：缓存结果、GPU 纹理、运行时状态

**版本兼容：**
- 加载时通过 NodeRegistry 查找节点定义
- 参数缺失 → 用默认值填充
- 参数多余（旧版本遗留） → 忽略
- 节点类型不存在（插件未加载） → 跳过并警告

**依赖：** NodeRegistry。

---

## 9. Menu

从注册表生成节点创建菜单。

**接口约定：**
- **生成菜单：** 读取 NodeRegistry 全部节点 + CategoryRegistry 全部分类 → 返回分组后的菜单结构
- **搜索过滤：** 给定关键词 → 返回匹配的节点列表（匹配标题和分类名称）
- **创建节点：** 用户选择菜单项 → 调用 NodeRegistry 实例化节点并插入图中

**菜单结构：**
- 一级：分类（按 CategoryRegistry 的排序权重）
- 二级：该分类下的节点（按注册顺序）
- 顶部：搜索框（实时过滤，跨分类匹配）

**依赖：** NodeRegistry、CategoryRegistry。

---

## 10. BackendClient

与 AI 计算后端通信的 HTTP 客户端。

**接口约定：**
- **POST 请求：** 给定 endpoint + JSON body → 返回 JSON 响应（或错误）
- **配置：** base URL（默认 `http://localhost:8188`）

**职责边界：**
- BackendClient 只负责 HTTP 通信，不理解节点语义
- AI 节点的处理函数调用 BackendClient 发送请求，解析响应
- 请求在后台线程执行，不阻塞 UI 线程

**依赖：** 无。独立组件。被 AI 类节点的处理函数调用。

---

## 渲染入口：Viewer

`viewer.rs` 实现 egui-snarl 的 `SnarlViewer` trait，是将所有组件串联起来的渲染入口。它不是一个独立组件，而是读取注册表来通用渲染任意节点。

**渲染流程：**
- 读 NodeRegistry → 获取引脚列表、参数列表、是否启用预览区
- 读 DataTypeRegistry → 获取引脚颜色
- 读 WidgetRegistry → 渲染参数控件
- 读 CategoryRegistry → 头部分类标识
- 读 Cache → 获取 GPU 纹理句柄用于 PreviewArea 渲染

**内含结构组件：**

| 结构组件 | 说明 |
|---------|------|
| NodeFrame | 节点外壳 — 背景色、圆角、阴影 |
| Header | 标题栏 — 标题文字 + 分类色标 |
| PinRow | 引脚行 — 引脚圆点 + 标签 |
| ParamRow | 参数行 — 标签 + 控件（或禁用态） |
| PreviewArea | 预览区 — 图像缩略图 |

---

## 数据模型

节点图使用 `Snarl<NodeInstance>` 持有拓扑和节点数据。

**NodeInstance（节点实例）：**
- `type_id` — 节点类型标识（对应 NodeRegistry 中的定义）
- `params` — 参数键值表（参数名 → 当前值）

NodeInstance 不包含引脚定义、处理函数等——这些由 NodeRegistry 通过 `type_id` 查询获得。NodeInstance 只存储实例级别的可变状态（参数值）。

---

## 依赖关系

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
```

---

## 文件结构

节点系统相关的所有代码放在 `src/node/` 目录下，保持内聚。

```
src/
├── main.rs              # 入口
├── app.rs               # App 框架 + 布局
├── processing/          # 纯图像处理算法（builtins/ 中的节点调用这些函数，此目录不依赖节点系统的任何类型）
│   ├── mod.rs           # 统一导出所有公开函数
│   ├── color.rs         # color_adjust, hsl_adjust, levels, color_balance, invert, threshold, lut_apply
│   ├── filter.rs        # gaussian_blur, box_blur, sharpen, edge_detect_sobel, edge_detect_laplacian, emboss, denoise, pixelate, vignette, film_grain
│   ├── transform.rs     # distort（resize/crop/rotate/flip 使用 image crate 内置功能，简单包装）
│   ├── composite.rs     # blend, mask_apply, channel_split, channel_merge
│   └── generate.rs      # solid_color, gradient, perlin_noise, checkerboard
└── node/                # 节点系统（全部在此）
    ├── mod.rs           # 统一导出
    ├── types.rs         # DataTypeRegistry + 内置数据类型
    ├── constraint.rs    # ConstraintRegistry + 内置约束
    ├── category.rs      # CategoryRegistry + 内置分类
    ├── registry.rs      # NodeRegistry
    ├── eval.rs          # EvalEngine
    ├── cache.rs         # Cache
    ├── serial.rs        # Serializer
    ├── menu.rs          # Menu
    ├── viewer.rs        # SnarlViewer 实现 + 结构组件
    ├── backend.rs       # BackendClient（HTTP 客户端）
    ├── widget/          # 参数控件（每种控件一个文件）
    │   ├── mod.rs       # WidgetRegistry
    │   ├── slider.rs    # Slider + IntSlider
    │   ├── checkbox.rs  # Checkbox
    │   ├── color_picker.rs
    │   ├── dropdown.rs
    │   ├── file_picker.rs
    │   ├── number_input.rs
    │   ├── text_input.rs
    │   └── radio_group.rs
    └── builtins/        # 内置节点定义（每个节点一个文件，完整列表见 catalog.md）
        ├── mod.rs       # 统一注册所有内置节点
        ├── load_image.rs
        ├── save_image.rs
        ├── solid_color.rs
        ├── gradient.rs
        ├── noise.rs
        ├── checkerboard.rs
        ├── color_adjust.rs
        ├── curves.rs
        ├── levels.rs
        ├── hue_saturation.rs
        ├── color_balance.rs
        ├── invert.rs
        ├── threshold.rs
        ├── lut_apply.rs
        ├── resize.rs
        ├── crop.rs
        ├── rotate.rs
        ├── flip.rs
        ├── distort.rs
        ├── blur.rs
        ├── sharpen.rs
        ├── edge_detect.rs
        ├── emboss.rs
        ├── denoise.rs
        ├── pixelate.rs
        ├── vignette.rs
        ├── film_grain.rs
        ├── blend.rs
        ├── mask.rs
        ├── channel_merge.rs
        ├── preview.rs
        └── histogram.rs
```

> **注意：** AI 节点（Load Checkpoint、CLIP Text Encode 等）不存在于 Rust 前端。它们由 Python 后端通过 `GET /node_types` 动态注册到 NodeRegistry，前端自动生成 UI。详见 `protocol.md`。

#### GPU 子系统

```
src/gpu/
├── mod.rs               # GpuContext（wgpu device + queue + pipeline cache）
├── texture.rs           # GpuTexture 封装
├── pipeline.rs          # compute pipeline 创建、dispatch、bind group 辅助函数
└── shaders/             # WGSL compute shaders（30 个，每个 GPU 节点对应一个或多个）
```

### 构建顺序

| 阶段 | 组件 | 文件 |
|------|------|------|
| 1 | DataTypeRegistry | `node/types.rs` |
| 2 | ConstraintRegistry | `node/constraint.rs` |
| 3 | WidgetRegistry | `node/widget/mod.rs` + `node/widget/*.rs` |
| 4 | CategoryRegistry | `node/category.rs` |
| 5 | NodeRegistry | `node/registry.rs` |
| 6 | EvalEngine + Cache | `node/eval.rs` + `node/cache.rs` |
| 7 | Serializer | `node/serial.rs` |
| 8 | Menu | `node/menu.rs` |
| 9 | Viewer | `node/viewer.rs` |
| 10 | 内置节点 | `node/builtins/` |
