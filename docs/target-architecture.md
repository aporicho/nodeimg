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
    gpu["nodeimg-gpu"]
    proc["nodeimg-processing"]
    engine["nodeimg-engine<br/><i>服务层 · Transport · 执行器</i>"]
    server["nodeimg-server<br/><i>库 crate · HTTP 包装</i>"]
    app["nodeimg-app<br/><i>GUI</i>"]
    cli["nodeimg-cli<br/><i>exec · serve · batch</i>"]

    gpu --> types
    proc --> types
    engine --> types
    engine --> gpu
    engine --> proc
    app --> types
    app --> engine
    server --> engine
    cli --> types
    cli --> engine
    cli --> server

    style types fill:#F39C12,color:#333
    style gpu fill:#45B39D,color:#fff
    style proc fill:#58D68D,color:#333
    style engine fill:#9B59B6,color:#fff
    style server fill:#95A5A6,color:#fff
    style app fill:#4A90D9,color:#fff
    style cli fill:#3498DB,color:#fff
```

> GUI 不依赖 nodeimg-gpu（图像处理 GPU 归服务层）。CLI 内嵌 server，一个二进制搞定。

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
