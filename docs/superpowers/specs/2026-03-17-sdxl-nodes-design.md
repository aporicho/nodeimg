# SDXL Nodes — Minimal Demo Design

## Overview

为 Node Image Studio 添加 SDXL text-to-image pipeline。Python FastAPI 后端执行完整的 AI 节点图，Rust 前端负责 UI 编辑和触发执行。一次执行 = 一次 HTTP 请求。

领域概念见 `docs/domain.md`、`docs/backend-domain.md`。通信协议见 `docs/protocol.md`。后端架构见 `docs/backend-architecture.md`。

## Rust 前端变更

### 动态节点注册

前端启动时调用 `GET /node_types` 拉取后端支持的节点定义，动态注册到 NodeRegistry。AI 节点是"空壳"——有引脚、参数 UI、连接规则，但没有本地 ProcessFn。

如果后端不在线，跳过注册，只保留本地图像处理节点。

### BackendClient

新增 `src/node/backend.rs`：

```rust
pub struct BackendClient {
    client: reqwest::blocking::Client,
    base_url: String,
}
```

- `fetch_node_types()` — 拉取后端节点定义
- `execute_graph(graph_json)` — 发送图执行请求
- `health_check()` — 检测后端是否在线

### 执行流程

用户触发执行时：
1. 前端识别图中的 AI 节点子图
2. 序列化为 protocol.md 定义的 JSON 格式
3. 后台线程调用 `BackendClient::execute_graph()`
4. 收到结果后，解码 base64 PNG 为 Image
5. 将 Image 传给下游节点（Preview、SaveImage）

### AI 节点的数据类型

AI 节点之间的���接类型（MODEL、CLIP、VAE、CONDITIONING、LATENT）只在前端用于连接兼容性检查。实际数据不经过前端——全在后端内存中。

前端 DataTypeRegistry 注册这些类型时只需 ID 和显示名称，不需要 Value 变体或转换函数。

### New Files

| File | Role |
|------|------|
| `src/node/backend.rs` | BackendClient |

### Modified Files

| File | Change |
|------|--------|
| `Cargo.toml` | 添加 `reqwest = { version = "0.12", features = ["blocking", "json"] }` |
| `src/node/types.rs` | 注册 AI 数据类型（仅 ID + 名称） |
| `src/node/category.rs` | 添加 "ai" 分类 |
| `src/theme/light.rs` | 添加 ai 分类颜色 |
| `src/theme/dark.rs` | 添加 ai 分类颜色 |
| `src/app.rs` | 创建 BackendClient，启动时拉取节点类型 |
| `src/node/viewer.rs` | 执行时识别 AI 子图，调用后端 |

## Python 后端

完整架构见 `docs/backend-architecture.md`。

### 文件结构

```
python/
├── server.py           # FastAPI 路由
├── registry.py         # NodeRegistry
├── executor.py         # GraphExecutor（拓扑排序 + 逐节点执行）
├── nodes/
│   ├── __init__.py     # 注册所有节点
│   ├── load_checkpoint.py
│   ├── clip_text_encode.py
│   ├── empty_latent_image.py
│   ├── ksampler.py
│   └── vae_decode.py
└── requirements.txt
```

### requirements.txt

```
torch
diffusers
transformers
safetensors
accelerate
fastapi
uvicorn
```

### 启动

```bash
cd python && pip install -r requirements.txt && uvicorn server:app --host 0.0.0.0 --port 8188
```

## Out of Scope

- 采样进度条 / 逐步预览
- img2img, ControlNet, LoRA, IP-Adapter
- 模型目录扫描
- Python 环境自动安装
- 远程服务器发现
- 多 batch 结果处理
