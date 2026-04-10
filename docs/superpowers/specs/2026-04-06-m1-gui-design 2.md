# M1 GUI 设计

> feature/m1-iced-canvas 分支的实现设计。egui → iced 迁移，自建节点画布，最小 demo 闭环。

## 关联

- Issue: #14（egui → iced 迁移）、#79（自建 iced 节点图编辑器控件）
- 分支: `feature/m1-iced-canvas`
- 原型: `docs/prototypes/m1-node-design.html`（可交互，浏览器打开）
- 目标文档: 0.2.0、2.0.0、2.1.0、2.1.1、2.2.0

---

## Demo 目标

打开 app，添加 LoadImage + Brightness 节点，连线，运行，看到处理结果。

---

## 决策记录

| 决策 | 选项 | 结论 |
|------|------|------|
| 范围 | 最小闭环 / 路线图原样 / 多搭基础设施 | 最小闭环 |
| 文件结构 | 按 0.2.0 建目录 / 最小结构后拆 | 按 0.2.0 建目录，只填 M1 文件 |
| iced 版本 | 0.13 稳定 / master 0.14-dev | 0.13 稳定版 |
| 架构 | 扁平 / 目标架构对齐 / Canvas 自治 | 目标架构对齐（方案 B） |
| 画布渲染 | iced Canvas widget / 自建 wgpu shader | iced Canvas widget |
| 预览渲染 | CPU 回读 / Shader widget 直渲 | Shader widget GPU 直渲 |
| 参数控件 | 纯 Canvas 自绘 / 浮层 widget | 纯 Canvas 自绘 |
| 节点添加 | 右键菜单 / 工具栏按钮 / 两者 | 工具栏按钮 |
| 执行触发 | 执行选中节点 / 执行所有叶子节点 | 执行所有叶子节点 |
| 节点折叠 | 选中展开/未选中折叠 / 始终展开 | 始终展开，不做折叠 |

---

## 文件结构

```
gui/
├── Cargo.toml
└── src/
    ├── main.rs                 入口，iced::application() 启动
    ├── lib.rs                  App struct, Message, update(), view()
    ├── state.rs                UIState (selection, preview_handle)
    │
    ├── canvas/
    │   ├── mod.rs              NodeCanvas widget (iced Canvas)
    │   ├── state.rs            CanvasState, Viewport, ActiveInteraction
    │   └── controller.rs       CanvasController (hit_test + viewport + drag + connect)
    │
    ├── panels/
    │   ├── mod.rs
    │   ├── preview.rs          PreviewPanel (Shader widget, GPU 直渲)
    │   └── toolbar.rs          Toolbar (悬浮工具条)
    │
    ├── widgets/
    │   ├── mod.rs
    │   ├── slider.rs           自绘 Slider (Canvas 内)
    │   └── floating.rs         FloatingPanel 容器 (拖拽+缩放)
    │
    └── render/
        ├── mod.rs
        ├── background.rs       网格背景
        ├── node.rs             节点渲染
        └── connection.rs       连线渲染
```

依赖：`iced = "0.13"` (features: canvas, wgpu)、`engine`、`types`、`wgpu`

---

## App 架构

```rust
// lib.rs
struct App {
    engine: Engine,
    ui_state: UIState,
    canvas_state: CanvasState,
    canvas_controller: CanvasController,
    preview_panel: PanelState,   // { x, y, width, height, visible }
    toolbar_panel: PanelState,
}

enum Message {
    CanvasEvent(canvas::Event),
    PanelDrag(PanelId, Vec2),
    PanelResize(PanelId, Vec2),
    AddNode(String),
    RunGraph,
    ParamChanged(NodeId, String, Value, bool),
    ExecutionDone(Result<HashMap<NodeId, HashMap<String, Value>>>),
}
```

数据流：用户事件 → iced 分发 → Canvas widget 产生 CanvasEvent → App::update() 委派给 CanvasController → 调用 engine API → 更新状态 → iced 重绘。

布局：`Stack` 叠加——底层画布全屏，上层悬浮工具栏 + 悬浮预览面板。

---

## Canvas Widget + Controller

**NodeCanvas（canvas/mod.rs）**：实现 `canvas::Program`，职责只有渲染和事件转发。draw() 调用 render/ 下的函数，update() 把原始事件包装成 Message 抛出。

**CanvasController（canvas/controller.rs）**：接收 CanvasEvent，处理所有画布交互逻辑。单文件，用方法分组：

- `hit_test` — 鼠标位置命中了什么（引脚 > 节点 > 连线 > 空白）
- `handle_viewport` — 平移（中键拖拽）、缩放（滚轮）
- `handle_drag` — 节点拖拽（拖动中 preview，松手 commit）
- `handle_connect` — 临时连线（松手时 connect 或取消）
- `handle_selection` — 单击选中、Shift 多选、点击空白取消

**CanvasState（canvas/state.rs）**：

```rust
struct CanvasState {
    viewport: Viewport,                    // { offset: Vec2, zoom: f32 }
    interaction: Option<ActiveInteraction>, // Dragging / Connecting
    hover: Option<HoverTarget>,            // Node / Pin / Connection
    cache: canvas::Cache,                  // 状态变化时 clear
}
```

---

## 悬浮面板系统

**FloatingPanel（widgets/floating.rs）**：通用悬浮面板容器。

```rust
struct PanelState {
    position: Vec2,
    size: Vec2,
    visible: bool,
}
```

view() 用 `Stack` 叠加：

```rust
Stack::new()
    .push(canvas)
    .push(FloatingPanel::new("Preview", preview_content).state(&self.preview_panel))
    .push(FloatingPanel::new("", toolbar_content).state(&self.toolbar_panel))
```

- 拖拽：鼠标按住 header 区域移动
- 缩放：鼠标在面板边缘拖拽（预览面板支持）
- 边界约束：不能拖出窗口

---

## 预览面板

**PreviewPanel（panels/preview.rs）**：实现 iced `shader::Program`，wgpu render pass 直接 sample GpuTexture。

- 从 UIState 拿到选中节点的输出 GpuTexture
- 创建 bind group，sample 纹理输出到屏幕
- M1 不做缩放/平移/像素检查
- 底部显示分辨率和格式信息（iced Text widget）

wgpu 共享：Engine 的 GpuExecutor 持有 device/queue，PreviewRenderer 拿同一个引用。

---

## 节点渲染

**render/node.rs**：在 Canvas draw() 里遍历 graph.nodes 逐个绘制。

M1 简单实现：矩形 + 标题 + 引脚 + Slider。后续按 `docs/prototypes/` 的设计改进。

**render/connection.rs**：三阶贝塞尔曲线，颜色跟随源引脚 DataType 配色，线宽 1.5px。

**render/background.rs**：圆点网格，间距 24px，随视口平移缩放。

---

## 自绘 Slider

**widgets/slider.rs**：在 Canvas draw() 内绘制，在 CanvasController 里处理交互。

绘制：轨道（#e4e4e7，4px）、填充（#18181b）、Thumb（白色 14px 圆形）、标签（左参数名，右当前值）。

交互：hit_test 命中 → 拖拽中 `set_param(preview=true)` → 松手 `set_param(preview=false)` 入 undo 栈。

---

## 执行流程

```
用户点击 Run
  → 遍历 graph 找所有叶子节点（无下游连线）
  → 对每个叶子 spawn 异步任务：engine.evaluate(target)
  → Run 按钮禁用
  → 完成 → Message::ExecutionDone
  → 取选中节点输出的 GpuTexture → 更新预览面板
  → Run 按钮恢复
```

引擎新增便利方法：

```rust
impl Engine {
    pub async fn evaluate_all(&self) -> Result<HashMap<NodeId, HashMap<String, Value>>> {
        let graph = self.graph.snapshot();
        let leaves = find_leaves(&graph);
        let mut all_results = HashMap::new();
        for leaf in leaves {
            let results = self.evaluate(leaf).await?;
            all_results.extend(results);
        }
        Ok(all_results)
    }
}
```

---

## 视觉风格（M1 简化版）

M1 先用基础样式跑通 demo，后续按 `docs/prototypes/` 的设计迭代。

- 白色背景 + 圆点网格
- 节点：白底、圆角 8px、1px 边框、标题在顶部
- 引脚：彩色圆点，颜色按 DataType
- 选中：深色边框
- Slider：shadcn 风格（细轨道、白色 thumb、深色填充）

---

## M1 不做的

- 节点折叠
- 节点库弹窗
- 快捷键系统
- 主题系统（硬编码色值）
- 状态栏
- AI 对话面板
- 执行进度显示
- 取消执行
- undo/redo 快捷键（引擎已支持，GUI 入口留到 M2）
- ⊕ 两级引脚展开 + Safe Triangle（M1 用简单引脚显示）
- 内容卡片 / 弹出面板（M1 所有节点用功能节点样式）

---

## 需同步更新的目标文档

| 文档 | 变更 |
|------|------|
| 2.0.0-gui.md | 布局改为画布全屏 + 悬浮面板 |
| 2.1.0-canvas.draft.md | 画布占满窗口 |
| 2.1.2-node.draft.md | 白色极简风；去掉折叠；节点外观统一建模（inline_pins + panel）；引脚改为 ⊕ 两级展开 |
| 2.1.3-param-widgets.draft.md | 补充 TextInput、TextArea 控件映射 |
| 2.2.0-preview.draft.md | 预览面板改为左侧悬浮 |
| 2.5.0-toolbar.draft.md | 工具栏改为悬浮 pill |
| 2.7.0-theme.draft.md | 色彩基调从暗色改为白色极简风 |
| 0.2.0-file-architecture.draft.md | gui/ 新增 widgets/floating.rs |
