# 三路执行器

> 图像处理、AI 推理、模型 API 三路执行器的详细设计

## 总览

`EvalEngine` 在拿到一个节点后，根据 `NodeDef.executor` 字段判断走哪条执行路径：

- `ExecutorType::Image`（缺省）→ **图像处理执行器**（本地 GPU + CPU 协作，按节点职责分派）
- `ExecutorType::AI` → **AI 执行器**（HTTP → Python 推理后端）
- `ExecutorType::API` → **模型 API 执行器**（外部 REST API）

```mermaid
flowchart TD
    EE["EvalEngine\n(nodeimg-engine)"]:::service

    EE -->|"executor: Image"| IE["图像处理执行器"]:::compute
    EE -->|"executor: AI"| AE["AI 执行器"]:::ai
    EE -->|"executor: API"| ME["模型 API 执行器"]:::api

    subgraph 图像处理["图像处理（本地 GPU + CPU 协作）"]
        IE --> GPU["GPU 路径\n像素级运算\nnodeimg-gpu + WGSL shader"]:::compute
        IE --> CPU["CPU 路径\n文件 I/O · 数据分析\nnodeimg-processing"]:::compute
    end

    subgraph AI推理["AI 推理（本地/远端 Python）"]
        AE --> PY["Python 推理后端\nFastAPI + PyTorch"]:::ai
        PY --> HS["Handle 存储\n(GPU 对象映射)"]:::ai
    end

    subgraph 模型API["模型 API（云端）"]
        ME --> AP["ApiProvider trait\n(OpenAI / Stability / 通义…)"]:::api
    end
```

---

## 图像处理执行器

图像处理执行器在**同一进程内**运行，不产生任何网络调用。GPU 和 CPU 是协作关系，各有专长，按节点职责分派：

**GPU 路径（`gpu_process: Some`）— 像素级运算**

执行器从节点定义中取出 shader 源码（通过 `include_str!` 在编译期嵌入，跟随节点文件夹存放），提交给 `nodeimg-gpu` 运行时，按 `16×16` workgroup size 分发计算。所有像素级运算（亮度、对比度、模糊、混合等）走此路径。

**CPU 路径（`process: Some`）— 文件 I/O 与数据分析**

节点函数直接以 `&[u8]`（或 `image::DynamicImage`）为输入输出，调用 `nodeimg-processing` 中的算法。适用于 GPU 无法完成的操作：文件 I/O（`load_image`、`save_image`）、直方图计算、LUT 文件解析。

**分派规则：** 大多数节点只提供一条路径——像素运算只写 `gpu_process`，I/O 和分析只写 `process`。少数节点（如 `gaussian_blur`）同时提供两条路径，此时 GPU 优先，CPU 仅在 GPU 上下文不可用时（无兼容 GPU 或驱动问题）作为降级选项。

---

## AI 执行器

AI 执行器是 Rust 与 Python 后端之间的协议桥。两侧职责明确分离：

```mermaid
flowchart TD
    subgraph Rust侧["Rust 侧（AI 执行器）"]
        direction TB
        subgraph R_INPUT["接收与序列化"]
            R1["接收节点\nnode_type / inputs / params"]:::service
            R2["输入序列化\nImage→bytes\nHandle→id"]:::service
            R1 --> R2
        end
        subgraph R_HTTP["HTTP 请求"]
            R3["POST /node/execute\nmultipart/form-data"]:::transport
        end
        subgraph R_SSE["读取 SSE 流"]
            R4["progress 事件\n→ 转发进度回调"]:::transport
            R5["result 事件\n→ 解析结果"]:::transport
        end
        subgraph R_PARSE["响应解析"]
            R6["handle_id\n→ Value::Handle"]:::service
            R7["image_bytes\n→ Value::Image"]:::service
        end

        R_INPUT --> R_HTTP --> R_SSE --> R_PARSE
    end

    subgraph Python侧["Python 侧（FastAPI 推理后端）"]
        direction TB
        subgraph P_ROUTE["路由层"]
            P1["/node/execute"]:::ai
            P2["/handles/release"]:::ai
        end
        subgraph P_RESOLVE["Handle 还原"]
            P3["id → 真实 GPU 对象"]:::ai
        end
        subgraph P_EXEC["执行节点函数"]
            P4["操作 PyTorch 对象"]:::ai
        end
        subgraph P_OUTPUT["输出判断"]
            P5["Python 专属类型\n→ 存入 Handle 存储"]:::ai
            P6["Image\n→ 编码为 bytes"]:::ai
        end
        subgraph P_PROGRESS["进度推送"]
            P7["SSE progress 事件\nstep / total"]:::ai
        end

        P_ROUTE --> P_RESOLVE --> P_EXEC --> P_OUTPUT
        P_EXEC --> P_PROGRESS
    end

    R_HTTP -. "HTTP POST" .-> P_ROUTE
    P_OUTPUT -. "SSE result" .-> R_SSE
    P_PROGRESS -. "SSE progress" .-> R_SSE

    classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
    classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

**Handle 存储**

Python 后端维护一张 `handle_id → GPU 对象` 的映射表。当节点返回的是 PyTorch Tensor、模型权重或 CLIP embedding 等 Python 专属类型时，后端将其存入映射表并返回一个不透明的 `handle_id`。Rust 侧将其包装为 `Value::Handle`，在后续节点中作为输入透传，无需跨进程传输大体积数据。

当 `EvalEngine` 的缓存失效时，它会调用 `/handles/release`，按 `handle_id` 列表批量释放 Python 侧的 GPU 内存，避免 VRAM 泄漏。

**进度反馈**

`POST /node/execute` 返回 SSE 流：

- 非迭代节点直接推送一条 `result` 事件后关闭流。
- 迭代节点（如 `KSampler`）每步推送一条 `progress` 事件（字段：`step`、`total`），最后推送 `result` 事件。Rust 侧将 `progress` 事件转发给 UI 层的进度回调，实现实时进度展示。

---

## 混合图执行示例

下面是一个 AI 推理与图像处理节点交替执行的典型场景，展示了 `EvalEngine` 的逐节点分发策略：

```mermaid
sequenceDiagram
    participant EE as EvalEngine (Rust)
    participant IE as 图像执行器 (Rust GPU/CPU)
    participant AE as AI 执行器 (HTTP→Python)
    participant PY as Python 后端 (模型在 VRAM)

    EE->>AE: ① LoadCheckpoint(sdxl)
    AE->>PY: POST /node/execute
    PY-->>AE: result: handle_id="model_1"
    AE-->>EE: Value::Handle("model_1")

    EE->>AE: ② CLIP(model_1, "prompt")
    AE->>PY: POST /node/execute
    PY-->>AE: result: handle_id="cond_1"
    AE-->>EE: Value::Handle("cond_1")

    EE->>AE: ③ KSampler(model_1, cond_1, …)
    AE->>PY: POST /node/execute
    PY-->>AE: progress: step=1/20
    PY-->>AE: progress: step=2/20 … step=20/20
    PY-->>AE: result: handle_id="latent_1"
    AE-->>EE: Value::Handle("latent_1")

    EE->>AE: ④ VAEDecode(model_1, latent_1)
    AE->>PY: POST /node/execute
    PY-->>AE: result: image_bytes
    AE-->>EE: Value::Image(图像A)

    EE->>IE: ⑤ Brightness(图像A)
    IE-->>EE: Value::Image(图像B)

    EE->>IE: ⑥ Contrast(图像B)
    IE-->>EE: Value::Image(图像C)

    EE->>AE: ⑦ KSampler2(model_1, 图像C, …)
    AE->>PY: POST /node/execute (复用 model_1)
    PY-->>AE: result: handle_id="latent_2"
    AE-->>EE: Value::Handle("latent_2")

    EE->>AE: ⑧ VAEDecode2(model_1, latent_2)
    AE->>PY: POST /node/execute
    PY-->>AE: result: image_bytes
    AE-->>EE: Value::Image(图像D)

    EE->>IE: ⑨ SaveImage(图像D)
    IE-->>EE: Ok
```

**为什么选择逐节点分发而非子图委托？**

子图委托方案（将连续 AI 节点批量发送给 Python 执行）看似减少了 HTTP 往返次数，但带来了更高的复杂度：

1. **Handle 跨边界传递变复杂**：当图像处理节点的输出（`Value::Image`）作为 AI 节点的输入时，子图边界难以静态划分，必须在运行时动态切割，逻辑复杂。
2. **缓存粒度变粗**：`EvalEngine` 的缓存以节点为粒度。子图委托后，局部节点失效会导致整个子图重新执行，丧失细粒度缓存的优势。
3. **步骤⑦展示了 Handle 复用的关键优势**：`model_1` 在 Python VRAM 中始终存在，第二次 `KSampler` 调用直接透传 `handle_id`，无需重新加载模型，延迟极低。逐节点分发天然支持这种跨越中间图像处理步骤的 Handle 复用。

---

## 模型 API 执行器（新增）

模型 API 执行器针对云端大厂推理 API（如 OpenAI、Stability AI、通义千问）设计，与 AI 执行器的核心差异在于无状态：每次调用都是独立的 HTTP 请求，不存在跨调用共享的 Handle。

| | AI 执行器 | 模型 API 执行器 |
|---|----------|----------------|
| 目标 | 自部署 Python 推理后端 | 云端大厂 API（OpenAI、Stability、通义） |
| 调用模式 | 逐节点调用，Handle 共享模型 | 一次调用完成整个生成 |
| 节点粒度 | 模块化拆分（LoadCheckpoint / KSampler / VAEDecode 分离） | 单节点封装整个 API 调用 |
| 中间状态 | Handle 在 Python VRAM，跨节点复用 | 无中间状态，无状态调用 |
| 额外关注 | SSE 进度、Handle 生命周期管理 | ��证、计费、速率限制、重试 |

**统一 `ApiProvider` trait（决策 D11）**

不同云端 API 在认证方式、请求格式和错误码上差异显著。通过统一 trait 隔离变化点，节点实现只面向 trait，不感知具体厂商：

```rust
trait ApiProvider {
    fn authenticate(&self, config: &ProviderConfig) -> Result<Client>;
    fn execute(&self, node_type: &str, inputs: &Inputs, params: &Params) -> Result<Value>;
    fn rate_limit_status(&self) -> RateLimitInfo;
}
```

每个厂商实现一个结构体（如 `OpenAiProvider`、`StabilityProvider`），在启动时根据配置注入到对应节点。`rate_limit_status()` 供 `EvalEngine` 在调度时决策是否需要退避等待，避免因速率超限产生无效请求。
