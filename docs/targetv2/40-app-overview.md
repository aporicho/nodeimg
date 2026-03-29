# App 层架构

> 定位：nodeimg-app 的逻辑/渲染分离架构——框架无关的核心逻辑设计和启动关闭流程。

**子系统文档：**
[41-app-execution.md](./41-app-execution.md) · [42-app-editor.md](./42-app-editor.md) · [43-app-theme.md](./43-app-theme.md) · [44-cli.md](./44-cli.md)

---

## 架构总览

`nodeimg-app` 在内部分为两层：**逻辑层**持有所有状态和业务规则，**渲染层**仅负责把状态画到屏幕上。两层之间没有循环依赖——渲染层读取逻辑层的状态，通过事件/命令把用户操作反馈给逻辑层；逻辑层不引用任何 UI 框架类型。

```mermaid
flowchart TD
    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5

    subgraph 渲染层["渲染层（frontend）"]
        UIRenderer["UI Renderer\n(eframe / egui)"]:::frontend
        Canvas["节点画布\n节点位置 / 连线绘制"]:::frontend
        Preview["预览面板\n纹理展��� / 缩放平移"]:::frontend
        WidgetRender["控件渲染\n按 WidgetType 画 UI"]:::frontend
        MenuRender["菜单渲染\n节点选择器 / 右键菜单"]:::frontend
        ThemeApply["主题应用\n颜色 Token 注入"]:::frontend
    end

    subgraph 逻辑层["逻辑层（service）"]
        subgraph AppState_S["AppState"]
            direction TB
            AS_TABS["多 Tab 管理"]:::service
            AS_BACKEND["后端管理"]:::service
            AS_THEME["主题数据"]:::service
        end

        subgraph GraphCtrl_S["GraphController"]
            direction TB
            GC_GRAPH["Graph 模型"]:::service
            GC_VALID["连接验证"]:::service
            GC_REQ["构建 GraphRequest"]:::service
        end

        subgraph ExecMgr_S["ExecutionManager"]
            direction TB
            EM_SUBMIT["提交 / 轮询 / 取消"]:::service
            EM_PROGRESS["进度回调"]:::service
        end

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
    Canvas -->|"读图结构"| GC_GRAPH
    Preview -->|"读结果纹理"| AS_TABS
    MenuRender -->|"读节点目录"| AS_BACKEND

    UIRenderer -->|"用户操作 → 命令"| GC_VALID
    UIRenderer -->|"执行触发"| EM_SUBMIT

    GraphCtrl_S --> AppState_S
    ExecMgr_S --> AppState_S

    逻辑层 <-->|"execute(request, sender) / cancel"| Transport
```

逻辑层通过 `ProcessingTransport` trait 与服务层通信，不感知底层是 `LocalTransport` 还是 `HttpTransport`。详见 [30-transport.md](./30-transport.md)，Python AI 后端协议详见 [50-python-protocol.md](./50-python-protocol.md)。

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

## 启动序列

GUI 和 CLI 共享核心初始化流程（步骤 1–7），仅末尾入口不同。

```mermaid
flowchart TD
    classDef foundation fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute    fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef service    fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef transport  fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef ai         fill:#E88B8B,stroke:#D67979,color:#fff
    classDef frontend   fill:#6C9BCF,stroke:#5A89BD,color:#fff

    S1["① 加载配置\nCLI > 环境变量 > TOML > 默认值\n→ AppConfig"]:::foundation
    S2["② 初始化 GPU 上下文\nwgpu Adapter → Device → Queue\n→ Option<GpuContext>"]:::compute
    S3["③ 注册节点\ninventory::iter → NodeRegistry\n+ DataTypeRegistry + CategoryRegistry"]:::service
    S4["④ 初始化 Cache\nResultCache + TextureCache\n按 config 设上限"]:::service
    S5["⑤ 创建 Transport\nconfig.transport → LocalTransport 或 HttpTransport"]:::transport
    S6["⑥ 启动 Python 后端\n（若 python_auto_launch = true）\nspawn → 轮询 /health → 就绪"]:::ai
    S7["⑦ 构建逻辑层\nAppState / GraphController\nExecutionManager / WidgetRegistry\nNodeSearchIndex / KeybindingMap"]:::service
    S8A["⑧a GUI 入口\neframe::run → 渲染循环"]:::frontend
    S8B["⑧b CLI 入口\nexec / serve / batch"]:::frontend

    S1 --> S2 --> S3 --> S4 --> S5 --> S6 --> S7
    S7 --> S8A
    S7 --> S8B
```

**关键说明：**

- **步骤②** GPU 初始化可能失败（无兼容 GPU 或驱动问题），此时 `GpuContext` 为 `None`，所有 GPU 节点回退到 CPU 路径。App 正常启动，不阻塞。
- **步骤⑥** Python 后端是可选依赖。启动失败或超时（30 秒）后 App 仍正常运行，AI 节点灰显不可用，图像处理节点不受影响。详见 [50-python-protocol.md](./50-python-protocol.md)。
- **步骤⑤→⑥ 顺序依赖**：`LocalTransport` 需要持有 `BackendClient`（AI 执行器的 HTTP 客户端），因此 Transport 创建在 Python 启动之前，但 `BackendClient` 的连接验证在 Python 就绪后才完成。

---

## 关闭序列

```
App 退出
  │
  ├─ ① 取消所有正在执行的任务（设置 CancelToken）
  ├─ ② 等待后台执行线程退出
  ├─ ③ 关闭 Python 后端
  │     ├─ SIGTERM
  │     ├─ 等待 5 秒
  │     └─ 未退出 → SIGKILL
  ├─ ④ 释放 GPU 资源（GpuContext drop）
  └─ ⑤ ��出进程
```

未保存的项目在渲染层关闭窗口时拦截（`eframe` 的 `on_close_event`），弹出保存确认对话框，不在关闭序列中处理。
