# nodeimg 目标架构

本目录是 nodeimg 目标架构的文档体系。目标架构描述系统**应当成为的样子**，是设计决策的集中记录，也是重构和新功能开发的参考基准。

---

## 总架构图

**核心原则：Engine 始终本地运行，Graph 由 Engine 拥有。** App 通过命令接口操作 Engine；远端能力仅限 AI 推理和云端 API。

5 张竖向模块图，每张聚焦一个维度。组件标注对应文档和决策 ID，实线 = 当前，虚线 = 远期。

---

### 图 1 — 系统分层与边界

> 谁和谁通信、边界在哪。对应 [00](./00-context.md)、[01](./01-overview.md)、[11](./11-cross-cutting.md)。

```mermaid
flowchart TB
    User["用户\nGUI 画布 · CLI 批处理"]:::frontend

    subgraph LOCAL["本地进程（始终同进程）"]
        direction TB
        APP["App 层\nnodeimg-app · nodeimg-cli\n→ 06-app.md"]:::frontend
        ENGINE["Engine 层\nnodeimg-engine（拥有 Graph）\n→ 02-service-layer.md"]:::service
        APP <-->|"同进程直调\n命令 · 查询 · channel"| ENGINE
    end

    FS["本地文件系统\n.nodeimg · .png · .jpg · .exr"]:::foundation
    Python["Python 推理后端\nFastAPI + PyTorch\n→ 15-python-backend-protocol.md"]:::ai
    CloudAPI["云端模型 API\nOpenAI · Stability · 通义"]:::api
    RemoteExec["远端执行器 nodeimg-server\nCLI batch 分发 (远期)"]:::future

    User <-->|"编辑 · 执行 · 预览"| APP
    ENGINE <-->|"文件读写"| FS
    ENGINE -->|"HTTP + SSE"| Python
    Python -->|"Handle / Image"| ENGINE
    ENGINE -->|"HTTPS REST"| CloudAPI
    CloudAPI -->|"Image"| ENGINE
    APP -.->|"GraphRequest 分发"| RemoteExec

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

---

### 图 2 — App 层内部

> 逻辑/渲染分离，GUI 与 CLI 并列。对应 [06](./06-app.md)、[10](./10-config.md)。

```mermaid
flowchart TB
    subgraph Config["AppConfig (D08, D09) → 10-config.md"]
        direction TB
        CFG_PRI["优先级：CLI > ENV > TOML > 默认值"]:::foundation
        CFG_LOAD["启动时一次性加载"]:::foundation
    end

    subgraph RENDER["渲染层（eframe/egui，可替换）"]
        direction TB
        Canvas["节点画布\n拖拽 · 连线 · 选中"]:::frontend
        Preview["预览面板\n纹理展示 · 缩放"]:::frontend
        Widgets["控件渲染\n按 WidgetType 绘制"]:::frontend
        MenuR["菜单 · 搜索浮窗"]:::frontend
        ThemeApply["主题注入 (D36)\nDark/Light · Theme trait"]:::frontend
    end

    subgraph LOGIC["逻辑层（框架无关，可复用）"]
        direction TB

        subgraph AS["AppState"]
            direction TB
            AS_TABS["tabs: Vec＜ProjectTab＞ (D34)"]:::service
            AS_STATE["active_tab · executing"]:::service
            AS_THEME["Arc＜dyn Theme＞"]:::service
        end

        subgraph EM["ExecutionManager (D23, D35)"]
            direction TB
            EM_CH["channel · 200ms 防抖"]:::service
            EM_CANCEL["CancelToken · try_recv"]:::service
        end

        subgraph UM["UndoManager (D32)"]
            direction TB
            UM_STACK["undo/redo 栈 · Arc＜Node＞"]:::service
            UM_LIMIT["结构共享 · max 100 步"]:::service
        end

        WidgetReg["WidgetRegistry (D20, D24)\nDataType + Constraint → WidgetType"]:::service
        SearchIdx["NodeSearchIndex\n模糊匹配 · 名称/分类/别名"]:::service

        subgraph KM["KeybindingMap (D33)"]
            direction TB
            KM_MAP["快捷键 → AppCommand"]:::service
            KM_CFG["config.toml 可覆盖"]:::service
        end
    end

    CLI["nodeimg-cli\nexec · serve · batch\n→ 06-app.md"]:::frontend

    Config --> LOGIC
    RENDER -->|"AppCommand"| LOGIC
    LOGIC -->|"读取状态 → 下一帧渲染"| RENDER

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
```

---

### 图 3 — Engine 层内部

> Graph 所有权、调度、缓存、三路执行。对应 [02](./02-service-layer.md)、[03](./03-executors.md)、[05](./05-graph.md)。

```mermaid
flowchart TB
    subgraph ENGINE["nodeimg-engine（始终本地）"]
        direction TB

        subgraph GR["Graph (05)"]
            direction TB
            GR_DATA["nodes · connections"]:::foundation
            GR_MUT["add_node · connect（环检测）"]:::foundation
            GR_QUERY["topo_sort · validate"]:::foundation
        end

        subgraph REG["NodeRegistry (D18)"]
            direction TB
            REG_INV["inventory 自动收集"]:::service
            REG_SUB["DataTypeRegistry\nCategoryRegistry"]:::service
        end

        subgraph EE["EvalEngine (02)"]
            direction TB
            EE_TOPO["拓扑排序 → 逐节点分发"]:::service
            EE_ROUTE["按 ExecutorType 路由"]:::service
            EE_HIT["缓存命中跳过"]:::service
        end

        Serializer["Serializer (D17)\nGraph ←→ JSON\nversion + MigrationRegistry"]:::service

        Menu["Menu · 分类菜单构建"]:::service

        subgraph CACHE["Cache (D03)"]
            direction TB
            subgraph RC_S["ResultCache (D04)"]
                direction TB
                RC_TYPE["Value::Image · Value::Handle"]:::service
                RC_POLICY["失效驱动 + LRU\nHandle 豁���"]:::service
                RC_LOCK["RwLock 并发保护"]:::service
            end
            subgraph TC_S["TextureCache (D05)"]
                direction TB
                TC_TYPE["GpuTexture 预览纹理"]:::service
                TC_POLICY["LRU + VRAM 上限"]:::service
                TC_THREAD["仅 UI 线程访问"]:::service
            end
        end

        subgraph EXEC["三路执行器 (03)"]
            direction TB
            subgraph IE_S["图像处理执行器"]
                direction TB
                IE_TYPE["ExecutorType::Image"]:::compute
                IE_STRAT["gpu_process 优先\nprocess 回退"]:::compute
            end
            subgraph AE_S["AI 执行器"]
                direction TB
                AE_TYPE["ExecutorType::AI"]:::ai
                AE_STRAT["HTTP → Python\nSSE · Handle"]:::ai
            end
            subgraph ME_S["模型 API 执行器"]
                direction TB
                ME_TYPE["ExecutorType::API"]:::api
                ME_STRAT["ApiProvider (D11)\n认证 · 限流"]:::api
            end
        end
    end

    EE --> GR
    EE --> REG
    EE --> IE_S
    EE --> AE_S
    EE --> ME_S
    IE_S --> RC_S
    AE_S --> RC_S
    ME_S --> RC_S

    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
```

---

### 图 4 — 执行数据流（参数变更 → 预览刷新）

> 标注线程边界和数据类型。对应 [06](./06-app.md)（D23、D35）、[08](./08-concurrency.md)、[15](./15-python-backend-protocol.md)。

```mermaid
sequenceDiagram
    box rgb(108,155,207,0.15) UI 主线程
        participant R as 渲染层
        participant L as 逻辑层 (ExecMgr)
    end
    box rgb(109,191,160,0.15) 后台执行线程
        participant EE as EvalEngine
        participant EX as 执行器
    end
    participant RC as ResultCache (RwLock)
    participant TC as TextureCache

    R->>L: ① SetParam(node, key, value)
    L->>EE: ② engine.set_param()
    L->>RC: ③ invalidate → 递归标脏下游
    Note over L: ④ 脏节点全为 Image?
    alt 是 → 200ms 防抖自动执行
        L->>EE: ⑤ execute(request, sender)
    else 含 AI/API → 等用户 Ctrl+Enter
    end
    loop 逐节点
        EE->>EX: ⑥ 分发 (shader / HTTP / HTTPS)
        EX-->>RC: ⑦ 写锁 → 写入 Value
        EX-->>L: ⑧ sender.send(NodeCompleted)
    end
    R->>L: ⑨ try_recv()
    L->>RC: 读锁 → 取 Value
    alt Value::Image
        R->>TC: ⑩ CPU→GPU 上传
    else Value::GpuImage
        R->>TC: ⑩ 零拷贝
    end
    R->>R: ⑪ 渲染纹理
```

---

### 图 5 — Crate 依赖 + 节点注册

> 编译期依赖方向和节点自动发现流程。对应 [01](./01-overview.md)、[07](./07-node-framework.md)（D18、D21）。

```mermaid
flowchart BT
    subgraph types["nodeimg-types"]
        direction TB
        T_CORE["Value · DataType · Constraint"]:::foundation
        T_ID["NodeId · Position · ExecutorType"]:::foundation
    end

    subgraph graph_c["nodeimg-graph"]
        direction TB
        GC_DATA["Graph · Node · Connection"]:::foundation
        GC_OPS["图操作 · 图查询"]:::foundation
    end

    subgraph gpu["nodeimg-gpu (D21)"]
        direction TB
        GPU_CTX["GpuContext · GpuTexture"]:::compute
        GPU_PIPE["pipeline · buffer"]:::compute
        GPU_NOTE["纯运行时 不含 shader"]:::compute
    end

    processing["nodeimg-processing\nhistogram · LUT 解析"]:::compute

    subgraph engine["nodeimg-engine"]
        direction TB
        ENG_CORE["Graph 所有权 · EvalEngine"]:::service
        ENG_INFRA["Cache · 三路执行器 · Registry"]:::service
        ENG_NODE["内置节点（自带 shader）"]:::service
    end

    server["nodeimg-server\n远端执行端点 (远期)"]:::future
    app["nodeimg-app\nApp 逻辑层 + 渲染层"]:::frontend
    cli["nodeimg-cli\nexec · serve · batch"]:::frontend
    python["Python 推理后端\n(运行时 HTTP)"]:::ai

    graph_c --> types
    gpu --> types
    processing --> types
    engine --> types
    engine --> graph_c
    engine --> gpu
    engine --> processing
    server -.-> engine
    app --> types
    app --> engine
    cli --> types
    cli --> engine
    cli -.-> server
    python -. "运行时 HTTP" .-> engine

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

节点注册流程（编译期 → 启动期）：

```mermaid
flowchart TB
    subgraph FOLDER["每个节点 = 一个文件夹 (D21)"]
        direction TB
        MOD["mod.rs\nnode! 宏定义"]:::service
        SHADER["shader.wgsl\nGPU 路径（可选）"]:::compute
        CPUF["cpu.rs\nCPU 回退（可选）"]:::compute
    end

    NM["node! 宏展开\n生成 NodeDef"]:::transport
    INV["inventory::submit!\n编译期收集"]:::transport
    ITER["inventory::iter\n启动时遍历"]:::service
    REG["NodeRegistry.register()"]:::service

    FOLDER --> NM --> INV --> ITER --> REG

    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
```

---

### 关键架构约束

| 约束 | 说明 | 文档 |
|------|------|------|
| Engine 始终本地 | App 与 Engine 同进程直调，不经网络 | 01, 02 |
| Graph 归 Engine | App 通过命令操作图，不直接持有 Graph | 05, 06 |
| GPU+CPU 协作 | GPU 做像素运算，CPU 做 I/O 和分析，Image 节点 200ms 防抖自动执行 | 06 (D35) |
| AI 用户可控 | AI/API 节点 Ctrl+Enter 手动，旧结果保留 | 06 (D35) |
| 远端仅限推理 | Python 和云端 API 可远端，图像处理始终本地 | 03, 11, 15 |
| 批量分发远期 | CLI batch 可分发 GraphRequest 到远端执行器 | 04, 11 |
| Handle 豁免 LRU | Handle 不被淘汰，释放由失效或 VRAM 不足驱动 | 02, 15 (D04) |
| 节点内聚 | 一节点 = 一文件夹（mod.rs + shader + cpu） | 07 (D21) |
| 声明式注册 | node! 宏 + inventory，新增节点不改聚合文件 | 07 (D18) |
| 逻辑/渲染分离 | 逻辑层���引用 egui，换框架只重写渲染层 | 06 |

### 文档覆盖对照

| 图 | 覆盖文档 |
|---|---------|
| 1. 系统分层 | 00（边界）、01（协作）、11（部署模式） |
| 2. App 内部 | 06（逻辑/渲染、D20–D36）、10（配置） |
| 3. Engine 内部 | 02（调度、Cache）、03（执行器）、05��Graph） |
| 4. 执行数据流 | 06（D23、D35）、08（线程、channel）、15（SSE） |
| 5. Crate + 注册 | 01（依赖图）、07（node! 宏、D18、D21） |
| 横跨 | 04（远端执行）、09（错误）、12–14（决策、风险、术语）、15（Python 协议）、16（节点目录） |

---

## 文档索引

| 文件 | 主题 | 定位 |
|------|------|------|
| [00-context.md](./00-context.md) | 系统上下文 | nodeimg 系统与外部世界的边界和交互对象 |
| [01-overview.md](./01-overview.md) | 调用链总览 | 系统顶层架构，模块间如何协作，crate 如何依赖 |
| [02-service-layer.md](./02-service-layer.md) | 服务层内部 | EvalEngine 如何调度执行，服务层内部模块如何组织 |
| [03-executors.md](./03-executors.md) | 三路执行器 | 图像处理、AI 推理、模型 API 三路执行器的详细设计 |
| [04-transport.md](./04-transport.md) | Transport + server | Transport trait 实现协议透明性，server 接口定义远端通信契约 |
| [05-graph.md](./05-graph.md) | 节点图数据模型 | nodeimg-graph crate 的数据结构和操作 API |
| [06-app.md](./06-app.md) | App 层 + CLI | 逻辑/渲染分离架构，框架无关的核心逻辑设计，CLI 子命令规格 |
| [07-node-framework.md](./07-node-framework.md) | 节点架构 | 节点开发者体验——如何用最少的代码定义新节点 |
| [08-concurrency.md](./08-concurrency.md) | 并发模型 | 线程模型、并行策略、前端异步交互方式 |
| [09-error-handling.md](./09-error-handling.md) | 错误处理 | 分层 Error 类型、传播路径、前端展示策略 |
| [10-config.md](./10-config.md) | 配置系统 | 配置源、配置项清单、加载时机 |
| [11-cross-cutting.md](./11-cross-cutting.md) | 横切关注点 | 安全、日志、测试、性能、部署、版本 |
| [12-decisions.md](./12-decisions.md) | 设计决策表 | 所有架构决策的索引 |
| [13-risks.md](./13-risks.md) | 风险与技术债 | ��识别的风险和已知的技术债务 |
| [14-glossary.md](./14-glossary.md) | 术语表 | 项目中使用的领域术语定义 |
| [15-python-backend-protocol.md](./15-python-backend-protocol.md) | Python 后端协议 | AI 执行器与 Python 推理后端的通信协议、数据格式、生命周期管理 |
| [16-node-catalog.md](./16-node-catalog.md) | 节点目录 | v1 全量节点规格（AI 推理 + 图像处理 + 工具 + 模型 API） |

---

## 文档模板

所有文件（除 `README.md` 和 `12-decisions.md`）遵循统一模板：

```markdown
# 标题

> 一句话定位（这个模块是什么、解决什么问题）

## 总览

概述 + 核心 mermaid 图（一张图说清楚整体结构）

## [主题小节]

按逻辑展开：
- 设计意图和规则
- 必要时配 mermaid 图
- 关键接口/数据结构用 rust 代码块示意
- 约束和决策理由在上下文中内联说明
```

核心原则：**定位 → 总览图 → 按主题展开**（约束和决策内联，不单独列出）。

---

## Mermaid 配色规范（风格 A：柔和专业）

全局语义化 classDef，**所有文件统一使用**，不得自行引入其他颜色：

```
classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

### 语义映射

| classDef | 用于 | 色调 |
|----------|------|------|
| `frontend` | GUI、CLI、App 层组件 | 柔蓝 |
| `transport` | Transport trait、协议层、HttpTransport | 淡紫 |
| `service` | EvalEngine、Registry、Cache、服务层组件 | 薄荷绿 |
| `ai` | AI 执行器、Python 后端、SDXL 相关 | 柔红 |
| `api` | 模型 API 执行器、云端 provider | 柔橙 |
| `foundation` | nodeimg-types、nodeimg-graph、基础数据结构 | 柔黄 |
| `compute` | nodeimg-gpu、GpuContext、shader、pipeline | 青绿 |
| `future` | 远期功能、未实现部分 | 灰色虚线 |

### 图表规则

- 只用 hex 颜色，不用颜色名
- 用 classDef 语义类名，不在节点上内联 `style`
- 每张图配简短文字说明，不依赖图自解释
- flowchart 方向：系统层级用 `TB`（上到下），时序流程用 `LR`（左到右）
