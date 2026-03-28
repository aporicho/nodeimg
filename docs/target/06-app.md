# App 层

> App 层的逻辑/渲染分离架构，框架无关的核心逻辑设计

## 总览

`nodeimg-app` 在内部分为两层：**逻辑层**持有所有状态和业务规则，**渲染层**仅负责把状态画到屏幕上。两层之间没有循环依赖——渲染层读取逻辑层的状态，通过事件/命令把用户操作反馈给逻辑层；逻辑层不引用任何 UI 框架类型。

```mermaid
flowchart TD
    subgraph 渲染层["渲染层（frontend）"]
        UIRenderer["UI Renderer\n(eframe / egui)"]:::frontend
        Canvas["节点画布\n节点位置 / 连线绘制"]:::frontend
        Preview["预览面板\n纹理展示 / 缩放平移"]:::frontend
        WidgetRender["控件渲染\n按 WidgetType 画 UI"]:::frontend
        MenuRender["菜单渲染\n节点选择器 / 右键菜单"]:::frontend
        ThemeApply["主题应用\n颜色 Token 注入"]:::frontend
    end

    subgraph 逻辑层["逻辑层（service）"]
        AppState["AppState\n多 Tab 管理 / 后端管理\n主题数据"]:::service
        GraphCtrl["GraphController\nGraph 模型 / 连接验证\n构建 GraphRequest"]:::service
        ExecMgr["ExecutionManager\n提交 / 轮询 / 取消\n进度回调"]:::service
        WidgetReg["WidgetRegistry\n参数类型 + 约束 → WidgetType"]:::service
        UndoMgr["UndoManager\n不可变状态 + 结构共享"]:::service
        SearchIdx["NodeSearchIndex\n模糊匹配搜索"]:::service
        KeyMap["KeybindingMap\n快捷键 → AppCommand"]:::service
    end

    Transport["ProcessingTransport\n(trait)"]:::transport

    UIRenderer --> Canvas
    UIRenderer --> Preview
    UIRenderer --> WidgetRender
    UIRenderer --> MenuRender
    UIRenderer --> ThemeApply

    WidgetRender -->|"查控件类型"| WidgetReg
    Canvas -->|"读图结构"| GraphCtrl
    Preview -->|"读结果纹理"| AppState
    MenuRender -->|"读节点目录"| AppState

    UIRenderer -->|"用户操作 → 命令"| GraphCtrl
    UIRenderer -->|"执行触发"| ExecMgr

    GraphCtrl --> AppState
    ExecMgr --> AppState

    逻辑层 <-->|"submit / poll / cancel"| Transport

    classDef frontend  fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service   fill:#6DBFA0,stroke:#5BAD8E,color:#fff
```

逻辑层通过 `ProcessingTransport` trait 与服务层通信，不感知底层是 `LocalTransport` 还是 `HttpTransport`。

---

## 逻辑层/渲染层分离

**设计意图：** 渲染层与 `eframe/egui` 强耦合，逻辑层不引用任何框架类型。当 UI 框架迁移（如切换到 Web 前端或原生 UI 工具包）时，逻辑层可以整体复用，只需为新框架重新实现渲染层。

**分离边界：**

| 职责 | 归属 |
|------|------|
| 图状态、节点参数、项目文件 | 逻辑层 |
| 连接合法性验证 | 逻辑层（GraphController） |
| 控件类型映射 | 逻辑层（WidgetRegistry） |
| 参数校验（范围、枚举合法性） | 逻辑层（WidgetRegistry） |
| 将状态画成像素 | 渲染层 |
| 框架事件处理（鼠标/键盘） | 渲染层 |
| 主题颜色的实际注入 | 渲染层（ThemeApply） |

**通信方式：** 渲染层不直接修改逻辑层状态——用户操作通过命令对象（`AppCommand` 枚举）传入逻辑层，逻辑层统一处理后更新状态，渲染层在下一帧读取新状态。这与 Elm/Redux 的单向数据流类似，便于调试和回放。

---

## WidgetRegistry 归属（决策 D20）

`WidgetRegistry` 属于**逻辑层**，不属于渲染层。

**原因：** 控件类型的选择依赖对数据类型和约束的理解（例如：`Float + Range` 约束应映射为滑块，`Enum` 应映射为下拉框）。这是业务规则，不是渲染细节。若放入渲染层，每次框架迁移都需要重新实现控件映射逻辑，且无法独立测试。

**分工说明：**

- `nodeimg-engine`（服务层）：通过 `NodeDef` 提供每个参数的 `DataType` 和 `Constraint`。
- `WidgetRegistry`（逻辑层）：根据 `DataType + Constraint` 的组合，映射为 `WidgetType` 枚举值。
- 渲染层：只接收 `WidgetType`，不做类型判断，直接调用对应的 egui 绘制函数。

```mermaid
flowchart LR
    NodeDef["NodeDef\n(DataType + Constraint)"]:::service
    WR["WidgetRegistry\n映射规则"]:::service
    WT["WidgetType\n(逻辑层输出)"]:::service
    Render["渲染层\n按 WidgetType 画控件"]:::frontend

    NodeDef -->|"查询"| WR
    WR -->|"返回"| WT
    WT -->|"传入"| Render

    classDef frontend  fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service   fill:#6DBFA0,stroke:#5BAD8E,color:#fff
```

**映射示例：**

| DataType | Constraint | WidgetType |
|----------|-----------|------------|
| `Float` | `Range(min, max)` | `Slider` |
| `Float` | 无 | `DragValue` |
| `Int` | `Range(min, max)` | `SliderInt` |
| `String` | `Enum(variants)` | `Dropdown` |
| `Bool` | — | `Checkbox` |
| `Color` | — | `ColorPicker` |
| `Image` | — | `ImagePreview` |

---

## ExecutionManager 工作机制（决策 D23）

`ExecutionManager` 负责管理图执行的全生命周期：提交、进度追踪、取消、结果收割。

**提交流程：**

1. `GraphController` 将当前图构建为 `GraphRequest`（序列化的节点图结构）。
2. `ExecutionManager` 调用 `Transport.execute(graph_request)`，通过 channel 接收 `TaskId`。
3. `TaskId` 存入 `AppState.pending_task`，UI 展示执行中状态。

**进度事件推送：**

`Transport.poll(task_id)` 以流的形式推送 `ProgressEvent`：

| 事件 | 含义 |
|------|------|
| `NodeStarted { node_id }` | 某节点开始执行，UI 高亮该节点 |
| `NodeCompleted { node_id, result }` | 某节点执行完毕，结果写入 `ResultCache` |
| `NodeFailed { node_id, error }` | 某节点执行失败，UI 展示错误标记 |
| `Progress { node_id, step, total }` | AI 节点迭代进度（来自 SSE 转发） |

**取消机制：** `ExecutionManager` 持有当前任务的 `CancelToken`（内部为 `AtomicBool`）。调用 `cancel()` 设置标志位；执行线程在每个节点间隙检查标志，提前退出。取消后 `AppState.pending_task` 清空，UI 恢复空闲状态。

**结果存放：** 所有 `NodeCompleted` 事件携带的结果由 `ExecutionManager` 写入 `ResultCache`（`AppState` 持有引用）。渲染层在下一帧从 `ResultCache` 读取预览数据，通过 `TextureCache` 转为 GPU 纹理展示。

```mermaid
flowchart LR
    GC["GraphController\n构建 GraphRequest"]:::service
    EM["ExecutionManager"]:::service
    TP["Transport\nexecute / poll / cancel"]:::transport
    RC["ResultCache\n(AppState)"]:::service
    UI["渲染层\n进度 / 结果展示"]:::frontend

    GC -->|"GraphRequest"| EM
    EM -->|"submit"| TP
    TP -->|"TaskId"| EM
    TP -->|"ProgressEvent 流"| EM
    EM -->|"NodeCompleted → 写入"| RC
    EM -->|"进度事件 → 状态更新"| UI
    RC -->|"读取"| UI

    classDef frontend  fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service   fill:#6DBFA0,stroke:#5BAD8E,color:#fff
```

---

## 节点渲染器

节点渲染器负责把单个节点的 `NodeDef`（参数列表）画成可交互的控件组。控件类型由 `WidgetRegistry` 决策，渲染层执行绘制。

**参数类型到控件的映射流程：**

```mermaid
flowchart LR
    Param["参数\n(name + DataType\n+ Constraint)"]:::service
    WR["WidgetRegistry\n映射"]:::service
    WT["WidgetType"]:::service

    WT -->|"Float + Range"| Slider["Slider"]:::frontend
    WT -->|"Float（无约束）"| Drag["DragValue"]:::frontend
    WT -->|"String + Enum"| Drop["Dropdown"]:::frontend
    WT -->|"Bool"| Check["Checkbox"]:::frontend
    WT -->|"Color"| CP["ColorPicker"]:::frontend
    WT -->|"Image"| IP["ImagePreview"]:::frontend
    WT -->|"覆写：widget(CurveEditor)"| CE["CurveEditor\n（预置控件库）"]:::frontend

    Param --> WR --> WT

    classDef frontend  fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service   fill:#6DBFA0,stroke:#5BAD8E,color:#fff
```

**控件覆写机制（决策 D24）：**

`node!` 宏允许节点定义时显式指定某个参数的控件类型，覆盖 `WidgetRegistry` 的默认映射：

```rust
node! {
    name: "ColorGrade",
    params: [
        param!("curve", DataType::Float, widget: CurveEditor),
        param!("strength", DataType::Float, Constraint::Range(0.0, 1.0)),
    ],
    ...
}
```

`widget: CurveEditor` 直接绑定到预置控件库中的 `CurveEditor` 组件，渲染层无需经过 `WidgetRegistry` 查表。不指定 `widget` 的参数走默认映射路径。

**`widget.rs` 的废除（决策 D24）：** 原有的 `widget.rs` 将所有控件逻辑集中在一个文件中，导致增加新控件时需要修改中心文件，且控件与参数类型之间的关联隐式存在。新方案中，控件选择要么由 `WidgetRegistry` 的映射表决定（数据驱动，可测试），要么在 `node!` 宏中显式声明（局部可见，零隐式依赖）。`widget.rs` 不再承担控件路由职责。

---

## Undo/Redo（决策 D32）

`UndoManager` 基于不可变状态 + 结构共享实现，每次操作生成新版本的 `GraphState`，旧版本入 undo 栈。节点通过 `Arc<Node>` 持有，修改时只 clone 被改的节点，未改变的节点在版本间共享引用。

```rust
struct UndoManager {
    undo_stack: Vec<GraphState>,  // 旧版本
    redo_stack: Vec<GraphState>,  // 被撤销的版本
    max_steps: usize,             // 上限（默认 100）
}

struct GraphState {
    nodes: HashMap<NodeId, Arc<Node>>,
    connections: Vec<Connection>,
    // 不含缓存和图像数据
}
```

**操作流程：**

```mermaid
flowchart LR
    classDef service fill:#6DBFA0,stroke:#5BAD8E,color:#fff

    OP["用户操作\n修改 Node B 参数"]:::service
    PUSH["push 当前 GraphState\n到 undo 栈"]:::service
    CLONE["clone Node B\n(其他 Arc 共享)"]:::service
    NEW["生成新 GraphState\n清空 redo 栈"]:::service

    OP --> PUSH --> CLONE --> NEW
```

**进入 undo 栈的操作：** 添加/删除节点、连线/断线、修改参数、移动节点。

**不进入 undo 栈的操作：** 执行图（结果缓存不回滚）、保存文件、缩放/平移画布。

**归属：** 逻辑层。`UndoManager` 是 `ProjectTab` 的成员，每个项目 tab 独立维护自己的撤销历史。

---

## 节点搜索

双击画布空白处或按 Space 唤出搜索浮窗，输入关键词实时过滤节点列表，回车将选中节点添加到画布。

```mermaid
flowchart LR
    classDef frontend fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service fill:#6DBFA0,stroke:#5BAD8E,color:#fff

    INPUT["用户输入关键词"]:::frontend
    INDEX["NodeSearchIndex\n.query(text)"]:::service
    RESULTS["Vec<SearchResult>\n按相关度排序"]:::service
    RENDER["渲染结果列表"]:::frontend
    ADD["GraphController\n.add_node(type, pos)"]:::service

    INPUT --> INDEX --> RESULTS --> RENDER
    RENDER -->|"用户选中"| ADD
```

**搜索范围：**

- 节点名称（如 "KSampler"）
- 节点分类（如 "采样"）
- 关键词/别名（如输入 "blur" 匹配到 "Blur"）

**搜索算法：** 模糊匹配（子串匹配），结果按相关度排序——名称精确匹配优先，分类匹配次之。

**归属：** 逻辑层提供 `NodeSearchIndex`（启动时从 `NodeRegistry` 构建），渲染层只负责绘制搜索框和结果列表。

---

## 键盘快捷键（决策 D33）

逻辑层维护 `KeybindingMap`，映射快捷键组合到 `AppCommand` 枚举。渲染层捕获键盘事件后查表触发对应命令，不硬编码任何快捷键。

**默认快捷键：**

| 快捷键 | 命令 |
|--------|------|
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| Ctrl+S | 保存项目 |
| Ctrl+O | 打开项目 |
| Ctrl+N | 新建项目 |
| Delete / Backspace | 删除选中节点/连线 |
| Ctrl+A | 全选 |
| Ctrl+C / Ctrl+V | 复制/粘贴节点 |
| Ctrl+D | 复制选中节点 |
| Space 或双击画布 | 唤出节点搜索 |
| Ctrl+Enter | 执行图 |
| Escape | 取消执行 / 关闭搜索 |
| Ctrl+T | 新建 tab |
| Ctrl+W | 关闭当前 tab |
| Ctrl+Tab | 切换到下一个 tab |

**可自定义：** 快捷键配置存在 `config.toml` 的 `[keybindings]` 段中，用户可覆盖默认值。添加新快捷键只需在 `AppCommand` 枚举增加变体并在默认映射表加一行，不影响渲染层。

---

## 多项目 Tab（决策 D34）

App 支持同时打开多个项目，每个项目独立一个 tab，类似浏览器标签页。

```mermaid
flowchart TD
    classDef frontend fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service fill:#6DBFA0,stroke:#5BAD8E,color:#fff

    AppState["AppState\ntabs: Vec<ProjectTab>\nactive_tab: usize"]:::service

    Tab1["ProjectTab 1\nGraphController\nUndoManager\nExecutionState"]:::service
    Tab2["ProjectTab 2\nGraphController\nUndoManager\nExecutionState"]:::service
    Tab3["ProjectTab 3\n..."]:::service

    SharedCache["ResultCache + TextureCache\n(跨 tab 共享)"]:::service

    AppState --> Tab1
    AppState --> Tab2
    AppState --> Tab3

    Tab1 --> SharedCache
    Tab2 --> SharedCache
    Tab3 --> SharedCache
```

```rust
struct AppState {
    tabs: Vec<ProjectTab>,
    active_tab: usize,
    // ResultCache 和 TextureCache 跨 tab 共享
}

struct ProjectTab {
    title: String,
    file_path: Option<PathBuf>,
    graph_controller: GraphController,
    undo_manager: UndoManager,
    execution_state: ExecutionState,
    dirty: bool,  // 有未保存修改
}
```

**关键设计：**

- 每个 tab 有独立的 `GraphController`、`UndoManager`、执行状态——切换 tab 不丢失任何上下文
- `dirty` 标记未保存修改，关闭 tab 时提示保存
- 执行是 tab 级别的——一个 tab 在执行时可以切换到另一个 tab 编辑，不阻塞
- `ResultCache` 和 `TextureCache` 跨 tab 共享，避免 VRAM 重复占用，缓存条目通过 `(TabId, NodeId)` 区分归属
