# 计算后端 — 领域概念

定义 Python 计算后端的核心概念。通信协议见 `protocol.md`。

---

## 1. 计算后端

计算后端是一个独立的 Python HTTP 服务，负责接收节点图、执行 AI 推理、返回结果。

### 职责

- 接收前端发来的节点图（JSON），执行全部 AI 节点
- 管理 AI 模型的加载和内存
- 返回执行结果（图像数据）

### 不负责

- UI 渲染、节点图编辑
- 非 AI 节点的执行（图像处理节点由 Rust 前端执行）

---

## 2. 图执行

后端的核心操作是**接收一个完整的节点图并执行它**。

### 执行流程

1. 接收 JSON 格式的节点图（节点定义 + 连接关系 + 输出节点 ID）
2. 拓扑排序，确定执行顺序
3. 逐个执行节点：
   - 收集输入：从已执行的上游节点取输出
   - 读取参数：从节点定义中取参数值
   - 调用节点处理函数
   - 存储输出供下游使用
4. 执行完毕，取 output_node 的输出作为最终结果
5. 返回给前端

### 中间数据

所有中间数据（模型对象、张量、conditioning）都在 Python 进程内存中，不通过网络传输。只有最终输出（图像）需要序列化返回前端。

---

## 3. 节点类型

后端有自己的节点类型注册表。每个节点类型定义：

- **类型标识**（如 "LoadCheckpoint"）
- **输入引脚**（名称 + 数据类型）
- **输出引脚**（名称 + 数据类型）
- **参数**（名称 + 类型 + 默认值 + 约束）
- **执行函数**（接收输入 + 参数，返回输出）

前端通过 `GET /node_types` 获取后端支持的所有节点类型，动态注册到前端的 NodeRegistry。这意味着：
- 后端新增节点类型时，前端自动获取，无需修改 Rust 代码
- 前端的 AI 节点 UI（引脚、参数控件）由后端的节点定义驱动

---

## 4. SDXL Pipeline

SDXL text-to-image 是由 5 个节点组成的有向无环图：

```
LoadCheckpoint ──→ CLIPTextEncode (正向) ──┐
       │                                    ├──→ KSampler ──→ VAEDecode ──→ [输出图像]
       ├──→ CLIPTextEncode (反向) ──────────┘         ↑
       │                                              │
       └──→ EmptyLatentImage ─────────────────────────┘
```

### 各节点职责

| 节点 | 做什么 | 输出 |
|------|--------|------|
| LoadCheckpoint | 从文件加载 SDXL 模型，拆为三部分 | UNet 模型、CLIP 编码器、VAE |
| CLIPTextEncode | 用 CLIP 将文本转为向量 | conditioning 张量 |
| EmptyLatentImage | 创建空白隐空间张量 | latent 张量 |
| KSampler | 扩散采样（去噪过程） | 采样后的 latent 张量 |
| VAEDecode | 将 latent 解码为像素图像 | PNG 图像 |

### 采样参数说明

| 参数 | 含义 |
|------|------|
| seed | 随机种子，决定噪声初始化。相同种子 + 相同参数 = 相同输出 |
| steps | 去噪步数。越多质量越高但越慢（通常 20-50） |
| cfg | Classifier-Free Guidance 强度。越高越遵循 prompt（通常 5-15） |
| sampler_name | 采样算法（euler, dpm_2 等），影响风格和速度 |
| scheduler | 噪声调度策略（normal, karras 等），影响去噪节奏 |

---

## 5. 错误场景

| 场景 | 后端行为 |
|------|---------|
| 节点类型未知 | HTTP 400 + `{"error": "Unknown node type: xxx"}` |
| 图有环路 | HTTP 400 + `{"error": "Graph has cycle"}` |
| 模型文件不存在 | HTTP 400 + `{"error": "File not found: ...", "failed_node": "1"}` |
| GPU 内存不足 | HTTP 500 + `{"error": "CUDA out of memory", "failed_node": "5"}` |
| 推理异常 | HTTP 500 + `{"error": "描述", "failed_node": "节点ID"}` |

`failed_node` 字段让前端能在出错的节点上显示错误提示。
