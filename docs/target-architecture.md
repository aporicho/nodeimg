# 目标架构

## 1. 调用链总览

统一 trait 接口，协议层可替换。交互服务始终本地，计算服务根据配置选择协议。

```mermaid
graph LR
    GUI["GUI<br/>nodeimg-app"]
    CLI["CLI<br/>nodeimg-cli"]

    TRAIT(("ProcessingTransport<br/><i>统一接口</i>"))

    GUI --> TRAIT
    CLI --> TRAIT

    subgraph 交互服务["交互服务（始终本地）"]
        direction TB
        I_API["node_types · serialize<br/>menu · registry 查询"]
        I_LOCAL["LocalTransport"]
        I_API --> I_LOCAL
    end

    subgraph 计算服务["计算服务（协议可替换）"]
        direction TB
        C_API["execute · upload · download"]
        C_API --> C_DECIDE{"配置选择协议"}
        C_DECIDE -->|"本地"| C_LOCAL["LocalTransport<br/><i>in-process 直调</i>"]
        C_DECIDE -->|"HTTP"| C_HTTP["HttpTransport<br/><i>REST + JSON</i>"]
        C_DECIDE -.->|"未来"| C_GRPC["gRPC / WebSocket / ..."]
    end

    TRAIT --> 交互服务
    TRAIT --> 计算服务

    I_LOCAL --> SERVICE["服务层<br/>nodeimg-engine"]
    C_LOCAL --> SERVICE
    C_HTTP --> SERVER["nodeimg-server"] --> SERVICE
    C_GRPC -.-> SERVICE

    style GUI fill:#4A90D9,color:#fff
    style CLI fill:#3498DB,color:#fff
    style TRAIT fill:#9B59B6,color:#fff
    style I_API fill:#7D3C98,color:#fff
    style I_LOCAL fill:#8E44AD,color:#fff
    style C_API fill:#7D3C98,color:#fff
    style C_DECIDE fill:#9B59B6,color:#fff
    style C_LOCAL fill:#8E44AD,color:#fff
    style C_HTTP fill:#8E44AD,color:#fff
    style C_GRPC fill:#BDC3C7,color:#555,stroke-dasharray: 5 5
    style SERVER fill:#95A5A6,color:#fff
    style SERVICE fill:#2ECC71,color:#fff
```

> 前端只看到统一的 trait 接口，不感知底层协议。协议层可随时替换（HTTP → gRPC → WebSocket），只需对齐接口。

---

## 2. 服务层内部

EvalEngine 拓扑排序后，逐节点按类型分发到执行器。

```mermaid
graph TD
    subgraph 服务层
        REG["NodeRegistry"]
        EVAL["EvalEngine<br/><i>拓扑排序 · 逐节点分发到执行器</i>"]
        CACHE["Cache"]
        SER["Serializer"]
        MENU["Menu"]
        PLUG["插件加载<br/><i>.dylib · .py · .wgsl</i>"]
    end

    PLUG -->|"注册节点"| REG
    REG --> EVAL

    subgraph 图像处理执行器["图像处理执行器（内部决定 CPU / GPU）"]
        IMG["接收图像节点"]
        IMG -->|"process: Some"| CPU["CPU 处理<br/><i>文件I/O · Histogram · LUT</i><br/>nodeimg-processing"]
        IMG -->|"gpu_process: Some"| GPU["GPU 处理<br/><i>WGSL Shaders × 30</i><br/>nodeimg-gpu"]
    end

    subgraph AI执行器["AI 执行器（自部署模型）"]
        BACKEND["BackendClient<br/>→ Python FastAPI<br/><i>本地/自部署 GPU</i>"]
    end

    subgraph API执行器["模型 API 执行器（云端大厂）"]
        API_CLIENT["ApiClient<br/><i>OpenAI · Stability · 通义 · ...</i><br/>认证 · 计费 · 速率限制"]
    end

    EVAL -->|"图像节点"| IMG
    EVAL -->|"AI 节点<br/>自部署"| BACKEND
    EVAL -->|"AI 节点<br/>云端 API"| API_CLIENT

    CPU --> CACHE
    GPU --> CACHE
    BACKEND --> CACHE
    API_CLIENT --> CACHE

    style EVAL fill:#2ECC71,color:#fff
    style REG fill:#2ECC71,color:#fff
    style CACHE fill:#2ECC71,color:#fff
    style SER fill:#2ECC71,color:#fff
    style MENU fill:#2ECC71,color:#fff
    style PLUG fill:#1ABC9C,color:#fff
    style IMG fill:#27AE60,color:#fff
    style CPU fill:#58D68D,color:#333
    style GPU fill:#45B39D,color:#fff
    style BACKEND fill:#E74C3C,color:#fff
    style API_CLIENT fill:#E67E22,color:#fff
```

> EvalEngine 只决定分发给哪个执行器。CPU/GPU 的选择是图像处理执行器内部的事，根据节点声明的 `process` / `gpu_process` 决定。

---

## 3. Crate 依赖

箭头 = "依赖于"。

```mermaid
graph BT
    types["nodeimg-types<br/><i>领域类型 · Position</i>"]
    graph_crate["nodeimg-graph<br/><i>节点图数据模型</i>"]
    gpu["nodeimg-gpu"]
    proc["nodeimg-processing"]
    engine["nodeimg-engine<br/><i>服务层 · Transport · 执行器</i>"]
    server["nodeimg-server<br/><i>库 crate · HTTP 包装</i>"]
    app["nodeimg-app<br/><i>GUI（逻辑层 + 渲染层）</i>"]
    cli["nodeimg-cli<br/><i>exec · serve · batch</i>"]

    graph_crate --> types
    gpu --> types
    proc --> types
    engine --> types
    engine --> graph_crate
    engine --> gpu
    engine --> proc
    app --> types
    app --> graph_crate
    app --> engine
    server --> engine
    cli --> types
    cli --> graph_crate
    cli --> engine
    cli --> server

    style types fill:#F39C12,color:#333
    style graph_crate fill:#F5B041,color:#333
    style gpu fill:#45B39D,color:#fff
    style proc fill:#58D68D,color:#333
    style engine fill:#9B59B6,color:#fff
    style server fill:#95A5A6,color:#fff
    style app fill:#4A90D9,color:#fff
    style cli fill:#3498DB,color:#fff
```

> GUI 不依赖 nodeimg-gpu（图像处理 GPU 归服务层）。CLI 内嵌 server，一个二进制搞定。nodeimg-graph 被 app、cli、engine 共用。

---

## 4. 执行流程

```mermaid
flowchart TD
    A["用户操作"] --> B["GraphRequest"]
    B --> C{"计算服务协议？"}
    C -->|本地| D["LocalTransport"]
    C -->|远端| E["HttpTransport"] --> G["nodeimg-server"]
    D --> F["EvalEngine 拓扑排序"]
    G --> F
    F --> H{"逐节点 → 哪个执行器？"}
    H -->|图像节点| IMG["图像处理执行器"]
    H -->|AI 节点·自部署| K["AI 执行器 → Python"]
    H -->|AI 节点·云端| K2["模型 API 执行器"]
    H -->|插件节点| L["插件执行器"]
    IMG --> IMG_D{"执行器内部决定"}
    IMG_D -->|"process: Some"| I["CPU"]
    IMG_D -->|"gpu_process: Some"| J["GPU"]
    I & J & K & K2 & L --> M["Cache"]
    M --> N{"ResultHandle"}
    N -->|本地| O["零拷贝 GpuTexture"]
    N -->|远端| P["序列化 ImageBytes"]
    O & P --> Q["前端渲染 / CLI 输出"]

    style C fill:#9B59B6,color:#fff
    style H fill:#2ECC71,color:#fff
    style IMG_D fill:#27AE60,color:#fff
    style N fill:#F39C12,color:#333
```

---

## 5. Python AI 后端

```mermaid
graph LR
    ENGINE["服务层<br/>BackendClient"]
    PY["FastAPI :8000"]
    NODES["AI 节点<br/><i>SDXL · CLIP<br/>VAE · KSampler</i>"]
    DEVICE["CUDA / MPS / CPU"]

    ENGINE -->|"HTTP"| PY
    PY --> NODES --> DEVICE

    style ENGINE fill:#2ECC71,color:#fff
    style PY fill:#E74C3C,color:#fff
    style NODES fill:#E74C3C,color:#fff
    style DEVICE fill:#C0392B,color:#fff
```

> 地址可配置，本地模式可自动拉起，不可用时不影响图像处理。前端不感知 Python 后端的存在。

---

## 6. App 层内部架构

nodeimg-app 内部分为**逻辑层**（框架无关）和**渲染层**（框架特定），逻辑层迁移 UI 框架时完全复用。

```mermaid
graph TD
    subgraph 渲染层["渲染层（框架特定，迁移时重写）"]
        direction TB
        RENDERER["UI Renderer<br/><i>egui → iced → ...</i>"]
        NC["节点画布渲染<br/><i>节点/连线/引脚绘制</i>"]
        PP["预览面板渲染<br/><i>图像纹理显示</i>"]
        WR["控件渲染<br/><i>Slider · Checkbox · Dropdown ...</i>"]
        MENU_UI["菜单渲染<br/><i>右键菜单 · 搜索</i>"]
        THEME_APPLY["主题应用<br/><i>颜色/样式 → 框架</i>"]
        RENDERER --- NC
        RENDERER --- PP
        RENDERER --- WR
        RENDERER --- MENU_UI
        RENDERER --- THEME_APPLY
    end

    subgraph 逻辑层["逻辑层（框架无关，迁移时复用）"]
        direction TB
        subgraph AppState["AppState — 应用状态"]
            PROJECT["项目管理<br/><i>打开 · 保存 · 自动保存</i>"]
            BACKEND_MGR["后端管理<br/><i>Python 进程启停</i>"]
            THEME_DATA["主题数据<br/><i>颜色 · 间距 · tokens</i>"]
            FPS["FPS 统计"]
        end
        subgraph GraphController["GraphController — 图操作"]
            GRAPH["Graph 模型<br/><i>nodeimg-graph</i>"]
            VALIDATE["连接验证<br/><i>类型兼容 · 环检测</i>"]
            BUILD_REQ["构建 GraphRequest"]
            INVALIDATE["缓存失效追踪"]
        end
        subgraph ExecMgr["ExecutionManager — 执行管理"]
            SUBMIT["提交执行任务"]
            POLL["轮询结果"]
            CANCEL["取消执行"]
            PROGRESS["进度回调<br/><i>on_progress</i>"]
        end
        subgraph WidgetLogic["控件逻辑"]
            W_REGISTRY["WidgetRegistry<br/><i>类型→控件映射</i>"]
            W_VALIDATE["参数校验<br/><i>约束检查</i>"]
        end
    end

    RENDERER -->|"读取状态<br/>调用方法"| 逻辑层
    逻辑层 -->|"状态变更<br/>事件通知"| RENDERER

    GraphController -->|"GraphRequest"| TRANSPORT(("Transport"))
    ExecMgr -->|"execute()"| TRANSPORT
    AppState -->|"serialize()"| TRANSPORT

    style RENDERER fill:#4A90D9,color:#fff
    style NC fill:#5BA0E9,color:#fff
    style PP fill:#5BA0E9,color:#fff
    style WR fill:#5BA0E9,color:#fff
    style MENU_UI fill:#5BA0E9,color:#fff
    style THEME_APPLY fill:#5BA0E9,color:#fff

    style PROJECT fill:#2ECC71,color:#fff
    style BACKEND_MGR fill:#2ECC71,color:#fff
    style THEME_DATA fill:#2ECC71,color:#fff
    style FPS fill:#2ECC71,color:#fff
    style GRAPH fill:#27AE60,color:#fff
    style VALIDATE fill:#27AE60,color:#fff
    style BUILD_REQ fill:#27AE60,color:#fff
    style INVALIDATE fill:#27AE60,color:#fff
    style SUBMIT fill:#1ABC9C,color:#fff
    style POLL fill:#1ABC9C,color:#fff
    style CANCEL fill:#1ABC9C,color:#fff
    style PROGRESS fill:#1ABC9C,color:#fff
    style W_REGISTRY fill:#58D68D,color:#333
    style W_VALIDATE fill:#58D68D,color:#333

    style TRANSPORT fill:#9B59B6,color:#fff
```

> 渲染层只负责"把状态画出来"和"把用户操作转为方法调用"。所有判断、校验、状态管理在逻辑层。

### 节点渲染器

节点 UI 由框架根据参数元信息自动生成，特殊节点可覆写。

```mermaid
graph TD
    NODE_DEF["节点元信息<br/><i>node! 宏声明的 params</i>"]

    subgraph 自动渲染["默认渲染（框架自动）"]
        MAP{"参数类型 + 约束<br/>→ 控件映射"}
        MAP -->|"Float + Range"| SLIDER["Slider"]
        MAP -->|"Int + Range"| INT_SLIDER["IntSlider"]
        MAP -->|"Bool"| CHECK["Checkbox"]
        MAP -->|"String + Enum"| DROP["Dropdown"]
        MAP -->|"Color"| COLOR["ColorPicker"]
        MAP -->|"String + FilePath"| FILE["FilePicker"]
        MAP -->|"Float 无约束"| NUM["NumberInput"]
        MAP -->|"String + Enum 少选项"| RADIO["RadioGroup"]
    end

    subgraph 覆写["自定义渲染（可选）"]
        CUSTOM["节点文件夹内<br/>widget.rs"]
        CUSTOM_EX["例：Curves 节点<br/>自定义曲线编辑器"]
    end

    NODE_DEF -->|"默认"| MAP
    NODE_DEF -->|"widget.rs 存在"| CUSTOM

    style MAP fill:#9B59B6,color:#fff
    style SLIDER fill:#5BA0E9,color:#fff
    style INT_SLIDER fill:#5BA0E9,color:#fff
    style CHECK fill:#5BA0E9,color:#fff
    style DROP fill:#5BA0E9,color:#fff
    style COLOR fill:#5BA0E9,color:#fff
    style FILE fill:#5BA0E9,color:#fff
    style NUM fill:#5BA0E9,color:#fff
    style RADIO fill:#5BA0E9,color:#fff
    style CUSTOM fill:#E67E22,color:#fff
    style CUSTOM_EX fill:#E67E22,color:#fff
```

```
节点文件夹（含自定义 UI）：
curves/
├── mod.rs              # 元信息
├── cpu.rs              # CPU 处理
└── widget.rs           # 自定义曲线编辑器 UI（可选，覆写默认渲染）

brightness/
├── mod.rs              # 元信息
└── shader.wgsl         # 无 widget.rs → 框架根据 Float+Range 自动生成 Slider
```

> 99% 的节点不需要写 widget.rs。只有 Curves、Gradient Editor 等需要特殊交互的节点才覆写。

---

## 7. nodeimg-graph — 自建图模型

框架无关的节点图数据结构，替代 egui-snarl 的 `Snarl<NodeInstance>`。

```mermaid
graph LR
    subgraph nodeimg-graph
        GRAPH["Graph"]
        NODE["Node<br/><i>id · type_id · params · position</i>"]
        CONN["Connection<br/><i>from_node · from_pin<br/>to_node · to_pin</i>"]
        OPS["图操作<br/><i>add · remove · connect<br/>disconnect · validate</i>"]
        QUERY["图查询<br/><i>upstream · downstream<br/>topo_sort · find_cycles</i>"]
    end

    GRAPH --> NODE
    GRAPH --> CONN
    GRAPH --> OPS
    GRAPH --> QUERY

    GRAPH -->|"转换"| GR["GraphRequest<br/><i>发给 Transport</i>"]
    GRAPH -->|"序列化"| JSON["JSON<br/><i>项目文件</i>"]

    style GRAPH fill:#F39C12,color:#333
    style NODE fill:#F5B041,color:#333
    style CONN fill:#F5B041,color:#333
    style OPS fill:#F5B041,color:#333
    style QUERY fill:#F5B041,color:#333
    style GR fill:#9B59B6,color:#fff
    style JSON fill:#95A5A6,color:#fff
```

> 依赖 nodeimg-types（NodeInstance、Position、Value）。被 app、cli、engine 共用。

---

## 8. 节点架构

新建节点 = 新建文件夹 + mod.rs 写元信息 + 放入 shader.wgsl / cpu.rs。不改任何其他文件。

### 节点框架

```mermaid
graph TD
    subgraph 框架提供["节点框架（开发者不用关心）"]
        MACRO["node! 宏<br/><i>解析 mod.rs 元信息<br/>生成 NodeDef + inventory::submit!</i>"]
        DISCOVER["文件自动发现<br/><i>shader.wgsl 存在 → gpu_process<br/>cpu.rs 存在 → process<br/>都有 → GPU 优先 CPU 回退<br/>都没有 → 按 executor 字段路由</i>"]
        GPU_RT["GPU 运行时<br/><i>自动加载 shader<br/>创建 pipeline · 绑定 buffer<br/>dispatch compute</i>"]
        PARAM["参数构建器<br/><i>float() · int() · dropdown()<br/>file() · color() · range()</i>"]
        INV["inventory 自动注册<br/><i>编译时收集，启动时注入 NodeRegistry</i>"]
    end

    MACRO --> DISCOVER
    MACRO --> PARAM
    DISCOVER --> GPU_RT
    DISCOVER --> INV
    INV --> REG["NodeRegistry"]

    style MACRO fill:#9B59B6,color:#fff
    style DISCOVER fill:#9B59B6,color:#fff
    style GPU_RT fill:#9B59B6,color:#fff
    style PARAM fill:#9B59B6,color:#fff
    style INV fill:#9B59B6,color:#fff
    style REG fill:#2ECC71,color:#fff
```

### 节点文件夹约定

```mermaid
graph LR
    subgraph 纯GPU["brightness/ — 纯 GPU"]
        G_MOD["mod.rs<br/><i>元信息</i>"]
        G_WGSL["shader.wgsl"]
    end
    subgraph CPU_GPU["gaussian_blur/ — CPU + GPU"]
        CG_MOD["mod.rs<br/><i>元信息</i>"]
        CG_WGSL["shader.wgsl"]
        CG_CPU["cpu.rs"]
    end
    subgraph 纯CPU["load_image/ — 纯 CPU"]
        C_MOD["mod.rs<br/><i>元信息</i>"]
        C_CPU["cpu.rs"]
    end
    subgraph AI["ksampler/ — AI 节点"]
        A_MOD["mod.rs<br/><i>元信息 + executor: AI</i>"]
    end
    subgraph API["sd_generate/ — 模型 API"]
        P_MOD["mod.rs<br/><i>元信息 + executor: API</i>"]
    end

    style G_MOD fill:#2ECC71,color:#fff
    style G_WGSL fill:#45B39D,color:#fff
    style CG_MOD fill:#2ECC71,color:#fff
    style CG_WGSL fill:#45B39D,color:#fff
    style CG_CPU fill:#58D68D,color:#333
    style C_MOD fill:#2ECC71,color:#fff
    style C_CPU fill:#58D68D,color:#333
    style A_MOD fill:#E74C3C,color:#fff
    style P_MOD fill:#E67E22,color:#fff
```

> mod.rs 只写元信息（node! 宏），shader.wgsl / cpu.rs 按文件名自动绑定，不用在 mod.rs 中声明。

### 目录结构

按执行器类型分三个目录，所有节点统一 Rust 定义：

```
crates/nodeimg-engine/src/
├── builtins/                 # 图像处理节点
│   ├── brightness/
│   │   ├── mod.rs            # node! { name, category, inputs, outputs, params }
│   │   └── shader.wgsl       # 自动绑定为 gpu_process
│   ├── gaussian_blur/
│   │   ├── mod.rs
│   │   ├── shader.wgsl       # GPU 优先
│   │   └── cpu.rs            # CPU 回退
│   ├── load_image/
│   │   ├── mod.rs
│   │   └── cpu.rs            # 自动绑定为 process
│   └── mod.rs                # inventory::iter 收集
│
├── ai_nodes/                 # AI 自部署节点
│   ├── ksampler/
│   │   └── mod.rs            # node! { ..., executor: AI }
│   ├── load_checkpoint/
│   │   └── mod.rs
│   └── mod.rs
│
└── api_nodes/                # 模型 API 节点
    ├── sd_generate/
    │   └── mod.rs            # node! { ..., executor: API }
    ├── dalle_generate/
    │   └── mod.rs
    └── mod.rs
```

Python 端只保留推理服务，不再定义节点：

```
python/
├── server.py                 # FastAPI 推理服务
├── executor.py               # 推理执行
├── device.py                 # GPU 检测
└── models/                   # 模型文件管理
```

> 所有节点定义统一 Rust 侧。Python 端退化为纯推理服务。

---

## 设计决策

| 决策 | 结论 | 来源 |
|------|------|------|
| 本地 vs 远端 | 统一接口，交互服务始终本地，计算服务协议可替换 | #12 #13 |
| GPU 归属 | 服务层拥有图像处理 GPU，前端 wgpu 仅 UI 渲染 | #12 |
| 逐节点分发 | EvalEngine 按节点类型分发，同一图可混合多种节点 | #12 |
| CPU + GPU | 协作关系，节点声明用哪个 | 现有设计 |
| 进度反馈 | execute() 接受 on_progress 回调 | #4 #77 |
| 返回值 | ResultHandle：Local 零拷贝 / Remote 序列化 | 新决策 |
| Serializer | 在服务层，前端共用 | #73 #78 |
| 位置类型 | nodeimg-types 定义 Position（替代 egui::Pos2） | #78 |
| 插件 | NodeRegistry 注册，支持 .dylib / .py / .wgsl | #27 |
| CLI serve | 内嵌 nodeimg-server（库 crate） | #18 |
| Python 后端 | 可配置、可自动拉起、可独立部署 | #12 |
| 远端文件 | Transport 含 upload/download | 新决策 |
| 模型 API 执行器 | 独立于 AI 执行器，处理云端大厂 API（认证/计费/速率限制） | 新决策 |
| 调度模式扩展 | 帧循环/关键帧是 EvalEngine 新模式，不是新执行器 | #29 #30 |
| 错误处理 | 统一 ExecutionError 传回前端 | 新决策 |
| Graph 模型 | 新建 nodeimg-graph crate，框架无关，替代 egui-snarl | 新决策 |
| App 分层 | app 内部拆 逻辑层（复用）+ 渲染层（重写），不新建 crate | #75 #76 |
| 节点架构 | 所有节点 Rust 定义，每节点一个文件夹，inventory 宏自动注册 | 新决策 |
| 节点目录 | builtins/（图像）、ai_nodes/（自部署AI）、api_nodes/（云端API）三目录 | 新决策 |
| Shader 位置 | 和节点定义放一起（同文件夹），不再集中在 nodeimg-gpu | 新决策 |
| Python 端定位 | 纯推理服务，不再定义节点，节点定义全部在 Rust 侧 | 新决策 |
| 节点渲染 | 框架根据参数类型+约束自动生成 UI，可选 widget.rs 覆写 | 新决策 |
