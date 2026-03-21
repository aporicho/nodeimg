# 前后端通信协议

定义 Rust 前端与 Python 计算后端之间的通信契约。

---

## 基本约定

- **协议**：HTTP/1.1
- **格式**：JSON（Content-Type: application/json）
- **默认地址**：`http://localhost:8188`
- **错误**：HTTP 200 成功，非 200 失败，失败返回 `{"error": "描述"}`

---

## 执行模型

前端负责节点图的编辑和展示。当用户触发执行时：

1. 前端将 AI 相关的节点子图序列化为 JSON
2. 发送一次 HTTP 请求到后端
3. 后端接收完整的图，自行拓扑排序并依次执行
4. 所有中间数据（模型、张量、conditioning）留在 Python 内存中，不通过 HTTP 传输
5. 后端只返回最终的输出结果（图片数据）

**一次执行 = 一次 HTTP 请求。** 中间步骤不产生网络通信。

---

## Endpoints

### POST /execute

执行一个节点图。

**Request:**
```json
{
  "graph": {
    "nodes": {
      "1": {
        "type": "LoadCheckpoint",
        "params": {
          "checkpoint_path": "/models/sd_xl_base_1.0.safetensors"
        }
      },
      "2": {
        "type": "CLIPTextEncode",
        "params": {
          "text": "a photo of a cat"
        }
      },
      "3": {
        "type": "CLIPTextEncode",
        "params": {
          "text": "ugly, blurry"
        }
      },
      "4": {
        "type": "EmptyLatentImage",
        "params": {
          "width": 1024,
          "height": 1024,
          "batch_size": 1
        }
      },
      "5": {
        "type": "KSampler",
        "params": {
          "seed": 42,
          "steps": 20,
          "cfg": 7.0,
          "sampler_name": "euler",
          "scheduler": "normal"
        }
      },
      "6": {
        "type": "VAEDecode",
        "params": {}
      }
    },
    "connections": [
      {"from_node": "1", "from_output": "model", "to_node": "5", "to_input": "model"},
      {"from_node": "1", "from_output": "clip",  "to_node": "2", "to_input": "clip"},
      {"from_node": "1", "from_output": "clip",  "to_node": "3", "to_input": "clip"},
      {"from_node": "1", "from_output": "vae",   "to_node": "6", "to_input": "vae"},
      {"from_node": "2", "from_output": "conditioning", "to_node": "5", "to_input": "positive"},
      {"from_node": "3", "from_output": "conditioning", "to_node": "5", "to_input": "negative"},
      {"from_node": "4", "from_output": "latent", "to_node": "5", "to_input": "latent"},
      {"from_node": "5", "from_output": "latent", "to_node": "6", "to_input": "latent"}
    ],
    "output_node": "6"
  }
}
```

**graph 结构说明：**
- `nodes`：节点 ID → 节点定义（类型 + 参数）
- `connections`：连接列表（源节点/引脚 → 目标节点/引脚）
- `output_node`：需要返回结果的节点 ID

**Response (成功):**
```json
{
  "outputs": {
    "6": {
      "image": "<base64 编码的 PNG>"
    }
  }
}
```

**Response (失败):**
```json
{
  "error": "CUDA out of memory",
  "failed_node": "5"
}
```

---

### GET /health

健康检查。

**Response:**
```json
{
  "status": "ok",
  "gpu": "NVIDIA RTX 4090",
  "vram_free_gb": 18.2
}
```

---

### GET /node_types

返回后端支持的所有节点类型定义��前端可据此动态生成 AI 节点的引脚和参数 UI。

**Response:**
```json
{
  "node_types": {
    "LoadCheckpoint": {
      "inputs": [],
      "outputs": [
        {"name": "model", "type": "MODEL"},
        {"name": "clip", "type": "CLIP"},
        {"name": "vae", "type": "VAE"}
      ],
      "params": [
        {"name": "checkpoint_path", "type": "STRING", "widget": "file_picker"}
      ]
    },
    "KSampler": {
      "inputs": [
        {"name": "model", "type": "MODEL"},
        {"name": "positive", "type": "CONDITIONING"},
        {"name": "negative", "type": "CONDITIONING"},
        {"name": "latent", "type": "LATENT"}
      ],
      "outputs": [
        {"name": "latent", "type": "LATENT"}
      ],
      "params": [
        {"name": "seed", "type": "INT", "default": 0, "min": 0, "max": 2147483647},
        {"name": "steps", "type": "INT", "default": 20, "min": 1, "max": 150},
        {"name": "cfg", "type": "FLOAT", "default": 7.0, "min": 1.0, "max": 30.0},
        {"name": "sampler_name", "type": "ENUM", "default": "euler", "options": ["euler","euler_ancestral","dpm_2","dpm_2_ancestral","lms","heun","ddim","uni_pc"]},
        {"name": "scheduler", "type": "ENUM", "default": "normal", "options": ["normal","karras","exponential","sgm_uniform"]}
      ]
    }
  }
}
```

前端启动时调用此接口，自动注册 AI 节点类型到 NodeRegistry，无需硬编码。
