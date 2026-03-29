# AI 执行器

> 定位：Rust 与 Python 推理后端之间的协议桥——HTTP + SSE 通信、Handle 透传、取消与超时。

## 架构总览

```mermaid
flowchart TB
    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5

    %% ── Rust 侧入口 ──
    EE["EvalEngine\n拓扑排序后逐节点分发\nexecutor == ExecutorType::AI"]:::service

    subgraph RUST["Rust 侧（AI 执行器）"]
        direction TB

        subgraph R_RECV["① 接收节点"]
            direction TB
            R1A["node_type"]:::service
            R1B["inputs"]:::service
            R1C["params"]:::service
        end

        subgraph R_SER["② 输入序列化"]
            direction TB
            R2A["Image → bytes\nmultipart part"]:::service
            R2B["Handle → id 字符串"]:::service
            R2C["Float / Int / String\n→ 原样"]:::service
        end

        subgraph R_REQ["③ 构建 HTTP 请求"]
            direction TB
            R3A["POST /node/execute"]:::transport
            R3B["Part 1: JSON\n节点信息"]:::transport
            R3C["Part 2+: binary\n图像数据（可选）"]:::transport
            R3A --> R3B
            R3A --> R3C
        end

        subgraph R_RESP["④ 读取响应"]
            direction TB
            R4A["SSE 流\nHandle 输出"]:::transport
            R4B["multipart\nImage 输出"]:::transport

            R4A1["progress 事件\n→ 转发进度回调"]:::transport
            R4A2["result 事件\n→ 解析 handle_id"]:::transport
            R4B1["Part 1: JSON\n尺寸/格式"]:::transport
            R4B2["Part 2: binary\n图像字节"]:::transport

            R4A --> R4A1
            R4A --> R4A2
            R4B --> R4B1
            R4B --> R4B2
        end

        subgraph R_PARSE["⑤ 响应解析"]
            direction TB
            R5A["handle_id\n→ Value::Handle"]:::service
            R5B["image_bytes\n→ Value::Image"]:::service
        end

        subgraph R_CANCEL_S["取消机制"]
            direction TB
            RCA["POST /node/cancel\nexecution_id"]:::service
            RCB["设置 CancelToken"]:::service
            RCA --> RCB
        end

        subgraph R_TIMEOUT_S["超时机制"]
            direction TB
            RTA["按节点类型差异化"]:::service
            RTB["LoadCheckpoint: 120s\nCLIPTextEncode: 30s\nKSampler: 600s"]:::service
            RTC["收到 progress\n重置计时器"]:::service
            RTA --> RTB
            RTA --> RTC
        end

        R_RECV --> R_SER --> R_REQ --> R_RESP --> R_PARSE
    end

    %% ── 网络边界 ──
    HTTP["HTTP localhost\n独立进程通信"]:::transport

    subgraph PYTHON["Python 侧（FastAPI + PyTorch 推理后端）"]
        direction TB

        subgraph SERVER["server.py — HTTP 路由"]
            direction LR
            EP_EXEC["/node/execute\nPOST"]:::ai
            EP_CANCEL["/node/cancel\nPOST"]:::ai
            EP_RELEASE["/handles/release\nPOST"]:::ai
            EP_HEALTH["/health\nGET"]:::ai
        end

        subgraph EXECUTOR["executor.py — 执行调度"]
            direction TB
            subgraph EX_QUEUE_S["请求入队"]
                EX_QUEUE["并发请求入队"]:::ai
            end
            subgraph EX_SCHED_S["调度策略"]
                EX_SCHED_L["轻量节点\n可并行"]:::ai
                EX_SCHED_H["重型节点\n独占 GPU"]:::ai
            end
            subgraph EX_CANCEL_S["取消控制"]
                EX_CANCEL_FLAG["取消标志位\n迭代节点每步检查"]:::ai
            end
            EX_QUEUE_S --> EX_SCHED_S
        end

        subgraph HANDLE["handle_store.py — Handle 生命周期"]
            direction TB
            subgraph HS_IN["写入"]
                HS_STORE["store(obj)\n→ handle_id"]:::ai
            end
            subgraph HS_OUT["读取"]
                HS_RESOLVE["resolve(id)\n→ GPU 对象"]:::ai
            end
            subgraph HS_DEL["释放"]
                HS_RELEASE["release(ids)\ndel + gc.collect()\n+ empty_cache()"]:::ai
            end
            subgraph HS_QUERY["查询"]
                HS_LIST["list_all()\n→ HandleInfo\n+ VRAM 占用"]:::ai
            end
        end

        subgraph NODES["nodes/*.py — 纯执行函数"]
            direction LR
            N_LOAD["load_checkpoint\n→ Model"]:::ai
            N_CLIP["clip_text_encode\n→ Conditioning"]:::ai
            N_EMPTY["empty_latent\n→ Latent"]:::ai
            N_KSAMP["ksampler\n→ Latent\n(progress + cancel)"]:::ai
            N_VAE["vae_decode\n→ Image bytes"]:::ai
        end

        SERVER --> EXECUTOR
        EXECUTOR -->|"还原 Handle"| HANDLE
        EXECUTOR -->|"调用节点函数"| NODES
        NODES -->|"Python 专属类型"| HANDLE
        EP_HEALTH --> HANDLE
        EP_RELEASE --> HANDLE
        EP_CANCEL --> EX_CANCEL_S
    end

    %% ── 连接关系 ──
    EE -->|"executor: AI"| R_RECV
    R_REQ --> HTTP
    HTTP --> SERVER
    R_CANCEL_S --> HTTP

    R_PARSE -->|"写入 ResultCache"| CACHE["ResultCache\nRwLock 保护\nHandle 豁免 LRU"]:::service

    %% ── 缓存失效触发释放 ──
    CACHE -->|"条目失效\n且类型为 Handle"| RELEASE_CALL["POST /handles/release\n批量释放"]:::transport
    RELEASE_CALL --> EP_RELEASE
```

---

## Rust 侧工作流

AI 执行器是 Rust 与 Python 后端之间的协议桥。两侧职责明确分离：

**Rust 侧（AI 执行器）五阶段：**

1. **接收节点**：从 `EvalEngine` 获取 `node_type`、`inputs`、`params`
2. **输入序列化**：`Image` 转 bytes（multipart），`Handle` 转 id 字符串，基础类型原样传递
3. **构建 HTTP 请求**：`POST /node/execute`，multipart/form-data 格式
4. **读取响应**：SSE 流（Handle 输出）或 multipart（Image 输出），转发 progress 事件给 UI
5. **响应解析**：`handle_id` → `Value::Handle`，`image_bytes` → `Value::Image`

**Python 侧（FastAPI + PyTorch）四模块：**

- **server.py**：HTTP 路由层（`/node/execute`、`/handles/release`、`/node/cancel`、`/health`）
- **executor.py**：执行调度（并发入队、轻量节点可并行/重型节点独占 GPU、取消标志位）
- **handle_store.py**：Handle 生命周期（store / resolve / release / list_all）
- **nodes/*.py**：纯执行函数（load_checkpoint、ksampler 等）

## Handle 存储

Python 后端维护一张 `handle_id → GPU 对象` 的映射表。当节点返回的是 PyTorch Tensor、模型权重或 CLIP embedding 等 Python 专属类型时，后端将其存入映射表并返回一个不透明的 `handle_id`。Rust 侧将其包装为 `Value::Handle`，在后续节点中作为输入透传，无需跨进程传输大体积数据。

当 `EvalEngine` 的缓存失效时，它会调用 `/handles/release`，按 `handle_id` 列表批量释放 Python 侧的 GPU 内存，避免 VRAM 泄漏。

Handle 协议的完整定义（ID 格式、release 接口规范）见 [50-python-protocol.md](./50-python-protocol.md)。

## 进度反馈

`POST /node/execute` 返回 SSE 流：

- 非迭代节点直接推送一条 `result` 事件后关闭流。
- 迭代节点（如 `KSampler`）每步推送一条 `progress` 事件（字段：`step`、`total`），最后推送 `result` 事件。Rust 侧将 `progress` 事件转发给 UI 层的进度回调，实现实时进度展示。

SSE 事件流的完整格式规范见 [50-python-protocol.md](./50-python-protocol.md)。

## 设计决策

- **D25**：AI 节点通过 HTTP + SSE 与 Python 推理后端通信
