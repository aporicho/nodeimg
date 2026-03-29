# 调用链总览

> 系统顶层架构，模块间如何协作，crate 如何依赖

## 总览

nodeimg 的调用链以 **ProcessingTransport trait** 为枢纽：GUI 和 CLI 都只面向这一统一接口，不感知底层协议。服务被分为两类——**交互服务**（轻量查询，始终本地）和**计算服务**（重操作，协议可替换）。

```mermaid
flowchart TD
    subgraph FRONTENDS["前端"]
        direction LR
        GUI["GUI\n(nodeimg-app)"]:::frontend
        CLI["CLI\n(nodeimg-cli)"]:::frontend
    end

    PT["ProcessingTransport trait"]:::transport

    FRONTENDS --> PT

    subgraph 交互服务["交互服务（始终本地）"]
        direction LR
        subgraph IT_QUERY["查询"]
            IT_node["node_types()"]:::service
            IT_reg["registry()"]:::service
        end
        subgraph IT_DATA["数据"]
            IT_serial["serialize()"]:::service
            IT_menu["menu()"]:::service
        end
    end

    subgraph 计算服务["计算服务（协议可替换）"]
        direction LR
        subgraph CS_EXEC["执行"]
            CS_exec["execute()"]:::service
        end
        subgraph CS_IO["数据传输"]
            CS_up["upload()"]:::service
            CS_dl["download()"]:::service
        end
    end

    PT --> 交互服务
    PT --> 计算服务

    subgraph 本地协议["本地协议"]
        LT_i["LocalTransport"]:::transport
    end
    交互服务 --> 本地协议

    subgraph 计算协议["计算协议选择"]
        direction LR
        LT_c["LocalTransport\n本地直调"]:::transport
        HT["HttpTransport\nREST"]:::transport
        GRPC["gRPC\n远期"]:::future
    end

    计算服务 --> 计算协议

    subgraph ENGINE_S["服务层 (nodeimg-engine)"]
        direction LR
        SVC_CORE["核心调度"]:::service
        SVC_EXEC["执行器"]:::service
        SVC_CACHE["缓存"]:::service
    end

    subgraph SERVER_S["nodeimg-server"]
        SRV["HTTP 包装"]:::transport
    end

    本地协议 --> ENGINE_S
    LT_c --> ENGINE_S
    HT --> SERVER_S
    SERVER_S --> ENGINE_S

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

前端只看到统一的 `ProcessingTransport` trait，不感知底层是直调还是 HTTP 或 gRPC。

---

## 交互服务与计算服务

分离的核心原因是**操作特性不同**：

| 维度 | 交互服务 | 计算服务 |
|------|---------|---------|
| 典型操作 | 查节点类型、构建菜单、序列化图 | 执行图、上传/下载图像 |
| 数据量 | 小（元数据、结构体） | 大（像素数据、执行结果） |
| 延迟要求 | 低延迟（UI 响应） | 可容忍较高延迟 |
| 协议需求 | 无需跨进程，始终本地 | 需要支持本地/远端可替换 |

交互服务固定走 `LocalTransport` 直调，消除协议协商开销，保证 UI 流畅。计算服务则通过配置决定协议，支持嵌入式（LocalTransport）、分离部署（HttpTransport）和远期集群化（gRPC）。

---

## 交互服务流程

交互服务始终在同一进程内完成，无网络往返。

```mermaid
flowchart LR
    FE["前端\n(GUI / CLI)"]:::frontend

    NT["Transport.node_types()"]:::transport
    MN["Transport.menu()"]:::transport
    SR["Transport.serialize(graph)"]:::transport

    LT["LocalTransport"]:::transport

    NR["NodeRegistry.list_all()\n→ Vec&lt;NodeDef&gt;"]:::service
    MB["Menu.build()\n→ MenuStructure"]:::service
    SZ["Serializer.save(graph)\n→ ProjectData"]:::service

    FE --> NT --> LT --> NR
    FE --> MN --> LT --> MB
    FE --> SR --> LT --> SZ
```

三条查询路径均由 `LocalTransport` 直接转发给服务层的对应模块，不经过任何序列化/反序列化，返回值是 Rust 原生类型。

---

## 计算服务协议选择

计算服务通过配置在启动时绑定协议，运行期不切换。三种协议的适用场景：

| 协议 | 适用场景 | 备注 |
|------|---------|------|
| `LocalTransport` | 单机嵌入，GUI 直接调用引擎 | 默认，零开销 |
| `HttpTransport` | 前端与服务端分离部署，REST API | 通过 nodeimg-server 暴露 |
| gRPC | 远期集群/分布式场景 | 未实现，接口对齐即可接入 |

**约束**：三种协议均实现相同的 `ProcessingTransport` trait，前端代码无需修改即可切换。接口对齐是唯一门槛。

---

## Crate 依赖图

```mermaid
flowchart BT
    subgraph FOUNDATION["基础层"]
        direction LR
        types["nodeimg-types\n领域类型、Position"]:::foundation
        graph_c["nodeimg-graph\n节点图数据模型"]:::foundation
    end

    subgraph COMPUTE["计算层"]
        direction LR
        subgraph GPU_S["nodeimg-gpu"]
            gpu_ctx["GpuContext\nGpuTexture"]:::compute
            gpu_pipe["pipeline 辅助"]:::compute
        end
        subgraph PROC_S["nodeimg-processing"]
            proc_hist["histogram 计算"]:::compute
            proc_lut["LUT 解析"]:::compute
        end
    end

    subgraph SERVICE["服务层"]
        subgraph ENG_S["nodeimg-engine"]
            eng_svc["服务层 + Transport"]:::service
            eng_exec["执行器"]:::service
            eng_nodes["内置节点\n节点自带 shader"]:::service
        end
    end

    subgraph TRANSPORT["传输层"]
        server["nodeimg-server\n库 crate，HTTP 包装"]:::transport
    end

    subgraph FRONTEND["前端层"]
        direction LR
        subgraph APP_S["nodeimg-app"]
            app_logic["GUI 逻辑层"]:::frontend
            app_render["渲染层"]:::frontend
        end
        subgraph CLI_S["nodeimg-cli"]
            cli_exec["exec"]:::frontend
            cli_serve["serve"]:::frontend
            cli_batch["batch"]:::frontend
        end
    end

    subgraph AI_S["AI 层"]
        subgraph PY_S["Python 推理后端"]
            py_api["FastAPI"]:::ai
            py_model["SDXL"]:::ai
        end
    end

    graph_c --> types
    GPU_S --> types
    PROC_S --> types
    ENG_S --> types
    ENG_S --> graph_c
    ENG_S --> GPU_S
    ENG_S --> PROC_S
    server --> ENG_S
    APP_S --> types
    APP_S --> graph_c
    APP_S --> ENG_S
    CLI_S --> types
    CLI_S --> graph_c
    CLI_S --> ENG_S
    CLI_S --> server
    PY_S -. "运行时 HTTP" .-> ENG_S

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

**关键约束说明：**

- **GUI 不依赖 nodeimg-gpu**：GPU 资源由 engine 内部管理，GUI 层通过 Transport 接口获取结果，不直接持有 `GpuContext`。
- **CLI 内嵌 server**：`nodeimg-cli` 在 `serve` 子命令下直接启动 `nodeimg-server`，因此显式依赖该 crate。
- **nodeimg-gpu 定位为运行时基础设施**：只包含 `GpuContext`、`GpuTexture`、pipeline 辅助函数等基础设施；shader（WGSL）跟随节点文件夹，不集中存放在 gpu crate。
- **Python 后端为运行时依赖**：编译期不存在依赖关系，通过 HTTP 协议在运行时调用，图中用虚线表示。
