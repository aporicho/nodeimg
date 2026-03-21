# 计算后端 — 系统架构

定义 Python 计算后端的组件划分和接口约定。领域概念见 `backend-domain.md`，通信协议见 `protocol.md`。

---

## 组件总览

| 组件 | 职责 |
|------|------|
| NodeRegistry | 注册和查询节点类型定义 |
| GraphExecutor | 接收节点图 JSON，拓扑排序，逐节点执行 |
| Server | FastAPI 路由层，HTTP 接口 |

节点的执行逻辑（加载模型、编码文本、采样等）直接写在各节点的执行函数中，不单独抽组件。

---

## 1. NodeRegistry

管理后端所有节点类型的注册和查询。

**接口：**
- `register(node_type: str, definition: NodeDef)` — 注册节点类型
- `get(node_type: str) -> NodeDef` — 查询节点定义
- `list_all() -> Dict[str, NodeDef]` — 返回所有节点定义（供 GET /node_types）

**NodeDef 包含：**
- 输入引脚列表（名称 + 类型）
- 输出引脚列表（名称 + 类型）
- 参数列表（名称 + 类型 + 默认值 + 约束）
- 执行函数：`execute(inputs: Dict, params: Dict) -> Dict`

**依赖：** 无。

---

## 2. GraphExecutor

核心组件。接收一个完整的节点图，执行并返回结果。

**接口：**
- `execute(graph_json: Dict) -> Dict` — 执行图，返回输出节点的结果

**执行��程：**
1. 解析 `graph_json` 中的 nodes 和 connections
2. 拓扑排序（检测环路）
3. 按拓扑序逐节点执行：
   - 从 `results` 字典中收集上游输出作为输入
   - 合并参数
   - 调用 `NodeRegistry.get(type).execute(inputs, params)`
   - 结果存入 `results[node_id]`
4. 取 `output_node` 的结果，序列化后返回

**中间结果存储：**
```python
results: Dict[str, Dict[str, Any]] = {}
# results["1"] = {"model": <UNet>, "clip": <CLIP>, "vae": <VAE>}
# results["5"] = {"latent": <tensor>}
```

所有中间数据（模型、张量）在 results 字典中，执行完毕后随请求生命周期释放。

**依赖：** NodeRegistry。

---

## 3. Server

FastAPI 路由层。

**路由：**

| Method | Path | 行为 |
|--------|------|------|
| POST | /execute | 调用 GraphExecutor.execute()，返回结果 |
| GET | /health | 返回服务状态、GPU 信息 |
| GET | /node_types | 调用 NodeRegistry.list_all()，返回节点定义 |

**依赖：** GraphExecutor, NodeRegistry。

---

## 节点执行函数

每个节点类型有一个执行函数。以 SDXL 节点为例：

### LoadCheckpoint
```python
def execute(inputs, params):
    path = params["checkpoint_path"]
    pipe = StableDiffusionXLPipeline.from_single_file(path, torch_dtype=torch.float16)
    pipe.to("cuda")
    return {
        "model": pipe.unet,
        "clip": (pipe.tokenizer, pipe.text_encoder, pipe.tokenizer_2, pipe.text_encoder_2),
        "vae": pipe.vae,
    }
```

### CLIPTextEncode
```python
def execute(inputs, params):
    clip = inputs["clip"]  # (tokenizer, text_encoder, tokenizer_2, text_encoder_2)
    text = params["text"]
    # 编码文本...
    return {"conditioning": prompt_embeds}
```

### EmptyLatentImage
```python
def execute(inputs, params):
    latent = torch.zeros(params["batch_size"], 4, params["height"] // 8, params["width"] // 8,
                         dtype=torch.float16, device="cuda")
    return {"latent": latent}
```

### KSampler
```python
def execute(inputs, params):
    # 取输入
    model, positive, negative, latent = inputs["model"], inputs["positive"], inputs["negative"], inputs["latent"]
    # 配置 scheduler，执行采样循环...
    return {"latent": sampled_latent}
```

### VAEDecode
```python
def execute(inputs, params):
    vae, latent = inputs["vae"], inputs["latent"]
    image = vae.decode(latent).sample
    # tensor → PIL → PNG → base64
    return {"image": base64_png}
```

---

## 文件结构

```
python/
├── server.py           # FastAPI 路由 + 启动入口
├── registry.py         # NodeRegistry
├── executor.py         # GraphExecutor
├── nodes/              # 节点执行函数（每个一个文件）
│   ├── __init__.py     # 注册所有节点
│   ├── load_checkpoint.py
│   ├── clip_text_encode.py
│   ├── empty_latent_image.py
│   ├── ksampler.py
│   └── vae_decode.py
└── requirements.txt
```

---

## 依赖关系

```
NodeRegistry（无依赖）
     ↑
GraphExecutor（依赖 NodeRegistry）
     ↑
Server（依赖 GraphExecutor + NodeRegistry）
```

---

## 构建顺序

| 阶段 | 组件 | 文件 |
|------|------|------|
| 1 | NodeRegistry | `registry.py` |
| 2 | GraphExecutor | `executor.py` |
| 3 | 节点执行函数 | `nodes/*.py` |
| 4 | Server | `server.py` |
