# 目标架构文档体系重构设计

> 完全重构 target-architecture.md 为一套分文件、统一模板、统一配色的架构文档体系。

---

## 1. 背景

现有 `target-architecture.md` 是单文件（~870 行），评审发现：

- **完整度 7/10**：错误处理、Cache、并发模型、配置系统等系统性关注点缺失
- **全面度 6/10**：安全、可观测性、测试、性能目标等横切面未触及
- **模块度 8/10**：Crate 边界好，engine 内部模块边界和几个迁移细节需补充
- **展开度 7/10**：nodeimg-graph、ExecutionManager、交互服务偏薄

需要将单文件拆分为多文件体系，补充缺失章节，统一描述方式和图表风格。

---

## 2. 文档目录结构

```
docs/
├── current/                        # 当前架构（现有文档迁入）
│   ├── architecture.md
│   ├── domain.md
│   ├── catalog.md
│   ├── gpu.md
│   ├── protocol.md
│   ├── backend-architecture.md
│   └── backend-domain.md
│
├── target/                         # 目标架构（全新）
│   ├── README.md                   # 索引 + 配色规范 + 文档模板说明
│   ├── 00-context.md               # 系统上下文
│   ├── 01-overview.md              # 调用链总览 + crate 依赖
│   ├── 02-service-layer.md         # 服务层内部
│   ├── 03-executors.md             # 三路执行器
│   ├── 04-transport.md             # Transport + server 接口
│   ├── 05-graph.md                 # nodeimg-graph 数据模型
│   ├── 06-app.md                   # App 层
│   ├── 07-node-framework.md        # 节点架构
│   ├── 08-concurrency.md           # 并发模型
│   ├── 09-error-handling.md        # 错误处理
│   ├── 10-config.md                # 配置系统
│   ├── 11-cross-cutting.md         # 横切关注点
│   ├── 12-decisions.md             # 设计决策表
│   ├── 13-risks.md                 # 风险与技术债
│   └── 14-glossary.md              # 术语表
│
├── architecture-roadmap.md         # 路线图（保持原位）
├── 开发工作流程.md                  # 工作流程（保持原位）
└── 文档规范.md                      # 文档规范（保持原位）
```

### current/ 目录

将现有文档原样迁入，描述代码的实际状态。不做内容修改。

### target/ 目录

全部新写，描述理想状态。下文详述每个文件的内容设计。

---

## 3. 文档模板

所有 target/ 下的文件（除 README.md 和 12-decisions.md）遵循统一模板：

```markdown
# 标题

> 一句话定位（这个模块是什么、解决什么问题）

## 总览

概述 + 核心 mermaid 图（一张图说清楚整体结构）

## [主题小节 1]

按逻辑展开：
- 设计意图和规则
- 必要时配 mermaid 图（流程图/序列图/类图）
- 关键接口/数据结构用 rust 代码块示意
- 约束和决策理由在上下文中内联说明

## [主题小节 2]

...
```

核心原则：**定位 → 总览图 → 按主题展开（约束和决策内联）**。

---

## 4. Mermaid 图表风格规范

### 配色方案（风格 A：柔和专业）

全局语义化 classDef，所有文件统一使用：

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
| `transport` | Transport trait、���议层、HttpTransport | 淡紫 |
| `service` | EvalEngine、Registry、Cache、服务层组件 | 薄荷绿 |
| `ai` | AI 执行器、Python 后端、SDXL 相关 | 柔红 |
| `api` | 模型 API 执行器、云端 provider | 柔橙 |
| `foundation` | nodeimg-types、nodeimg-graph、基础数据结构 | 柔黄 |
| `compute` | nodeimg-gpu、GpuContext、shader、pipeline | 青绿 |
| `future` | 远期功能、未实现部分 | 灰色虚线 |

### 图表规则

- 只用 hex 颜色，不用颜色名
- 用 classDef 语义类名，不在节点上内联 style
- 每张图配简短文字说明，不依赖图自解释
- flowchart 方向：系统层级用 TB（上到下），时序流程用 LR（左到右）

---

## 5. 各文件内容设计

### 00-context.md — 系统上下文

> nodeimg 系统与外部世界的边界和交互对象。

**内容：**
- 一张上下文图：nodeimg 系统居中，外部参与者环绕（用户、文件系统、Python 推理后端、云端模型 API、项目文件）
- 每个外部参与者的交互方式和数据流向
- 系统边界内包含什么、不包含什么

来源：arc42 模板 §3，现有文档未覆盖。

---

### 01-overview.md — 调用链总览 + crate 依赖

> 系统顶层架构：模块间如何协作，crate 如何依赖。

**内容（来源于现有 target-architecture.md §1 + §3，需修改/补充）：**
- 调用链总览图（交互服务/计算服务分离，Transport trait 可替换）
- **补充：** 交互服务流程图（始终 LocalTransport 直调 Registry/Menu/Serializer）
- Crate 依赖图
- **修改：** nodeimg-gpu 定位更新为"GPU 运行时基础设施"（shader 不再集中在 gpu crate）

---

### 02-service-layer.md — 服务层内部

> EvalEngine 如何调度执行，服务层内部模块如何组织。

**内容（来源于现有 §2，需修改/补充）：**
- EvalEngine 分发逻辑（按节点类型路由到三路执行器）
- Handle 机制（生命周期、Cache 关联、释放路径）
- **新增：** engine 内部模块图和依赖关系

```
engine/
├── registry/      (NodeRegistry, NodeDef)
├── eval/          (EvalEngine, 拓扑排序)
├── executor/      (图像/AI/API 三路执行器)
├── cache/         (ResultCache, TextureCache)
├── transport/     (Transport trait, LocalTransport)
├── serializer/    (Serializer)
├── menu/          (Menu)
└── builtins/      (内置节点)
```

内部依赖：executor → registry + cache，eval → executor + cache + registry，其余独立。

- **新增：** Cache 架构

  **ResultCache：**
  - 缓存内容：图像处理结果（Value::Image / Value::GpuImage）、Handle 条目（Value::Handle）、API 执行结果
  - 淘汰策略：失效驱动（参数变/连接断时递归失效下游）+ 可选 LRU 内存上限
  - Handle 条目豁免 LRU 淘汰（淘汰意味着释放 Python GPU 对象，重算代价高）
  - 失效时根据 entry 类型决定是否通知 Python 释放 Handle

  **TextureCache：**
  - 缓存内容：预览用的 GpuTexture（从 ResultCache 的 Image 上传生成）
  - 淘汰策略：LRU + VRAM 上限
  - 丢失可重新从 Image 上传，代价低

  **并发访问：** ResultCache 被后台执行线程（写入）和 UI 主线程（读取预览）并发访问，使用 `RwLock` 保护——执行线程写锁、UI 线程读锁，读操作不阻塞。TextureCache 仅在 UI 主线程上访问（上传纹理和渲染都在主线程），无需额外同步。

- **新增：** ResultEnvelope

  ```rust
  enum ResultEnvelope {
      Local(Value),            // 直接引用，含 GpuImage，零拷贝
      Remote(SerializedValue), // 序列化后的字节流
  }

  /// 序列化后的执行结果，用于远端传输
  /// 编码格式为 bincode（紧凑二进制），包含图像像素数据和尺寸等元信息
  pub struct SerializedValue(pub Vec<u8>);
  ```

  Transport 层决定路径——LocalTransport 返回 `Local(...)`，HttpTransport 返回 `Remote(...)`。前端 ExecutionManager 统一处理：Local 直接用，Remote 先反序列化再用。前端不感知模式差异。

---

### 03-executors.md — 三路执行器

> 图像处理、AI 推理、模型 API 三路执行器的详细设计。

**内容（来源于现有 §2 执行器部分 + §5 AI 执行器，需修改/补充）：**

- 图像处理执行器（保持现有设计）
- AI 执行器（保持现有 §5 的详细设计：Rust/Python 双侧流程、SSE、Handle 存储/释放）
- **新增：** 模型 API 执行器

  **与 AI 执行器的核心区别：**

  | | AI 执行器 | 模型 API 执行器 |
  |---|----------|----------------|
  | 目标 | 自部署的 Python 推理后端 | 云端大厂 API（OpenAI、Stability、通义…） |
  | 调用模式 | 逐节点调用，Handle 共享模型 | 通常一次调用完成整个生成 |
  | 节点粒度 | 模块化拆分（LoadCheckpoint → CLIP → KSampler → VAEDecode） | 单节点封装整个 API 调用 |
  | 中间状态 | Handle 在 Python VRAM 中 | 无中间状态，无状态调用 |
  | 额外关注 | SSE 进度、Handle 生命周期 | 认证、计费、速率限制、重试 |

  **统一 ApiProvider trait：**

  ```rust
  trait ApiProvider {
      fn authenticate(&self, config: &ProviderConfig) -> Result<Client>;
      fn execute(&self, node_type: &str, inputs: &Inputs, params: &Params) -> Result<Value>;
      fn rate_limit_status(&self) -> RateLimitInfo;
  }
  ```

  新增 provider 只需实现此 trait。认证、速率限制、重试逻辑通过 trait 统一处理。

---

### 04-transport.md — Transport + server 接口

> Transport trait 实现协议透明性，server 接口定义远端通信契约。

**内容（来源于现有 §1 Transport 部分，需修改/补充）：**

- Transport trait 设计（保持现有：Local/HTTP/gRPC 可替换）
- **新增：** nodeimg-server 抽象接口

  server 的接口与 Transport trait 对齐，不绑定具体传输协议：

  **交互服务接口：**
  - `node_types() → Vec<NodeDef>`
  - `menu() → MenuStructure`
  - `serialize(graph) → ProjectData`
  - `deserialize(data) → Result<Graph>`

  **计算服务接口：**
  - `execute(graph_request) → TaskId`
  - `poll(task_id) → Stream<ProgressEvent>`
  - `cancel(task_id)`
  - `upload(file) → FileId`
  - `download(file_id) → Bytes`

  **核心数据类型：**

  ```rust
  /// 执行任务的唯一标识符
  pub struct TaskId(pub u64);

  /// 进度事件流中的事件类型
  pub enum ProgressEvent {
      NodeStarted { node_id: NodeId },
      NodeCompleted { node_id: NodeId, duration_ms: u64 },
      NodeFailed { node_id: NodeId, error: String },
      Progress { node_id: NodeId, step: u32, total: u32 },  // AI 节点采样进度
      Completed { results: ResultEnvelope },
  }

  /// 上传文件的标识符（远端模式下 LoadImage 使用）
  pub struct FileId(pub String);
  ```

  nodeimg-server 是这套接口的一个具体实现。当前实现可用 HTTP（axum），但接口定义不依赖 HTTP。未来可换成 gRPC、WebSocket 或其他协议。

---

### 05-graph.md — nodeimg-graph 数据模型

> 节点图的数据结构和操作 API，nodeimg-graph crate 的完整设计。

**内容（来源于现有 §7，大幅扩充）：**

- Node 结构体定义在 nodeimg-graph（不在 nodeimg-types）
- nodeimg-types 只保留原子类型（NodeId、Position、Value 等）

**核心数据结构：**

```rust
// nodeimg-types（原子类型）
pub type NodeId = u64;
pub struct Position { pub x: f32, pub y: f32 }
pub enum Value { ... }

// nodeimg-graph（图数据模型）
pub struct Graph {
    pub nodes: HashMap<NodeId, Node>,
    pub connections: Vec<Connection>,
}

pub struct Node {
    pub id: NodeId,
    pub type_id: String,
    pub params: HashMap<String, Value>,
    pub position: Position,
}

pub struct Connection {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}
```

**图操作 API：**
- `add_node(type_id, position) → NodeId`
- `remove_node(node_id)` — 同时移除相关连接
- `connect(from_node, from_pin, to_node, to_pin) → Result`
- `disconnect(connection_id)`
- `set_param(node_id, param_name, value)`

**图查询 API：**
- `topo_sort() → Result<Vec<NodeId>, CycleError>`
- `upstream(node_id) → Vec<NodeId>`
- `downstream(node_id) → Vec<NodeId>`
- `validate() → Vec<ValidationError>` — 检查环路、必需输入未连线、类型不兼容

**序列化：**
- `to_json() → ProjectData`
- `from_json(data) → Result<Graph, LoadError>`

---

### 06-app.md — App 层

> App 层的逻辑/渲染分离架构，框架无关的核心逻辑设计。

**内容（来源于现有 §6，需修改/补充）：**

- 逻辑层/渲染层分离（保持现有设计）
- **修改：** WidgetRegistry 明确归属 App 逻辑层

  分工：
  - engine 提供参数元信息（DataType + Constraint）
  - App 逻辑层 WidgetRegistry 把元信息映射为控件类型（Slider、Checkbox…）
  - App 渲染层把控件类型画成具体 UI

- **补充：** ExecutionManager 工作机制

  ExecutionManager 使用与 Transport/server 相同的 `TaskId` 标识执行任务。

  **提交执行：**
  - `submit(graph_request) → TaskId` — 通过 channel 发给后台执行线程，立即返回

  **进度获取：**
  - 后台线程通过 channel 推送进度事件
  - 进度事件类型：`NodeStarted(node_id)`、`NodeCompleted(node_id, duration)`、`NodeFailed(node_id, error)`、`Progress(node_id, step, total)`（AI 节点采样进度）
  - ExecutionManager 持有最新进度状态，UI 每帧读取

  **取消执行：**
  - `cancel(task_id)` — 设置 `AtomicBool` 取消标志
  - EvalEngine 在每个节点执行前检查标志，当前节点跑完后停止

  **��果获取：**
  - 执行完成后结果已在 ResultCache 中
  - ExecutionManager 收到完成事件后通知 UI 刷新预览

---

### 07-node-framework.md — 节点架构

> 节点开发者体验：如何用最少的代码定义一个新节点。

**内容（来源于现有 §8，需补充）：**

- 文件夹约定（保持现有设计）
- **补充：** node! 宏语法示例

  ```rust
  // builtins/brightness/mod.rs
  node! {
      name: "brightness",
      title: "Brightness",
      category: "color",
      inputs: [
          image: Image (required),
      ],
      outputs: [
          image: Image,
      ],
      params: [
          brightness: Float, range(-1.0, 1.0), default(0.0),
      ],
  }
  ```

  框架根据文件夹内容自动决定执行方式：
  - `shader.wgsl` 存在 → 自动绑定为 gpu_process
  - `cpu.rs` 存在 → 自动绑定为 process
  - 都有 → GPU 优先 CPU 回退
  - 都没有 → 按 `executor` 字段路由到 AI 或 API 执行器

  **AI 节点的 node! 宏示例：**

  ```rust
  // builtins/ksampler/mod.rs
  node! {
      name: "ksampler",
      title: "KSampler",
      category: "ai",
      executor: AI,
      inputs: [
          model: Model (required),
          positive: Conditioning (required),
          negative: Conditioning (required),
          latent: Latent (required),
      ],
      outputs: [
          latent: Latent,
      ],
      params: [
          seed: Int, range(0, 2147483647), default(0),
          steps: Int, range(1, 150), default(20),
          cfg: Float, range(1.0, 30.0), default(7.0),
          sampler_name: String, enum(["euler","euler_ancestral","dpm_2","heun","ddim","uni_pc"]), default("euler"),
          scheduler: String, enum(["normal","karras","exponential","sgm_uniform"]), default("normal"),
      ],
  }
  ```

  AI/API 节点文件夹中没有 `shader.wgsl` 和 `cpu.rs`，通过 `executor: AI` 或 `executor: API` 声明执行路径。

- **补充：** 控件覆写机制

  框架提供预置控件库（Slider、IntSlider、NumberInput、Checkbox、Dropdown、ColorPicker、FilePicker、TextInput、CurveEditor、GradientEditor…）。

  WidgetRegistry 提供默认映射（DataType + Constraint → 默认控件）。节点在 node! 宏中通过 `widget(...)` 字段选择兼容的控件覆写默认：

  ```rust
  node! {
      name: "curves",
      // ...
      params: [
          channel: String, enum(["rgb","red","green","blue"]), default("rgb"),
          curve: Curve, widget(CurveEditor),  // 覆写默认映射
      ],
  }
  ```

  不指定 widget 则用 WidgetRegistry 默认映射。新增控件在控件库中实现，新增节点选用已有控件，两边独立扩展。不再使用旧设计中的 widget.rs 文件覆写方式，所有控件统一在预置控件库中实现。

- **修改：** Shader 位置更新

  Shader 跟随节点文件夹（`builtins/brightness/shader.wgsl`），不再集中在 nodeimg-gpu。nodeimg-gpu 变为纯 GPU 运行时基础设施（GpuContext、GpuTexture、pipeline 辅助函数）。

- **修改：** 移除插件系统相关内容

  设计决策表中 `.dylib / .py / .wgsl` 插件支持已取消。节点注册仅通过 inventory 宏静态注册。

---

### 08-concurrency.md — 并发模型

> 系统的线程模型、并行策��、前端异步交互方式。

**内容（全新章节）：**

- **执行模型：后台执行线程 + rayon 同层并行**

  ```
  App 主线程（UI 渲染 60fps）
      ↕ channel（提交任务 / 接收进度）
  后台执行线程
      → EvalEngine 拓扑排序
      → 逐层执行，同层无依赖节点通过 rayon 线程池并行
  ```

- **各类节点的并行行为：**
  - GPU 节点：GpuContext（持有 device/queue）通过 `Arc<Mutex>` 共享。多个 GPU 节点并行时，各自构建 CommandBuffer 后串行提交到 queue（acquire lock → submit → release）。GPU 硬件对无资源冲突的 dispatch 可能流水线重叠执行。
  - CPU 节点：rayon 线程池直接并行，无共享状态约束
  - AI/API 节点：在 rayon 线程中用 `reqwest::blocking` 阻塞调用

- **前端不阻塞策略：**
  - UI 主线程通过 channel 提交任务，不等待结果
  - ExecutionManager 每帧从 channel 读取进度事件，更新 UI 状态
  - 取消通过 `AtomicBool` 标志实现

---

### 09-error-handling.md — 错误处理

> 分层 Error 类型、传播路径、前端展示策略。

**内容（全新章节）：**

- **分层 Error enum：**

  ```
  NodeError          （单个节点执行失败，附带 node_id）
    ↓ 被包装为
  ExecutionError     （EvalEngine 级别，附带执行上下文）
    ↓ 被包装为
  TransportError     （通信层，附带协议信息）
    ↓ 到达前端
  ```

  每层定义自己的 Error 类型，上层包装下层。使用 `thiserror` crate。

- **错误产生位置：**

  | 位置 | 错误类型 | 示例 |
  |------|---------|------|
  | 图像处理执行器 | NodeError | shader 编译失败、图像解码失败、文件不存在 |
  | AI 执行器 | NodeError | CUDA OOM、模型文件缺失、Python 推理异常 |
  | API 执行器 | NodeError | 认证失败、速率限制、网络超时 |
  | EvalEngine | ExecutionError | 环路检测、类型转换失败、必需输入未连线 |
  | Transport | TransportError | HTTP 连接失败、序列化失败、超时 |
  | Cache | NodeError | VRAM 耗尽、Handle 已释放 |

- **前端展示：多位置同时展示**

  | 展示位置 | 触发条件 | 展示方式 |
  |---------|---------|---------|
  | 节点红框 | NodeError 能定位到 node_id | 节点边框变红 + 错误��息 tooltip |
  | 执行进度区 | 执行过程中任何错误 | 进度条标记失败 + 错误摘要 |
  | 全局通知 | TransportError 等系统级错误 | 顶部/底部弹出通知 |

  一个错误可以同时触发多个展示位置。错误沿链路向上传播时，每层根据自己的信息补充展示，不互斥。

---

### 10-config.md — 配置系统

> 配置源、配置项清单、加载时机。

**内容（全新章节）：**

- **配置源优先级：**

  ```
  CLI 参数 > 环境变量 > 配置文件 > 默认值
  ```

- **配置文件：** TOML 格式，路径 `~/.config/nodeimg/config.toml`

- **配置项清单：**

  | 配置项 | 说明 | 默认值 |
  |--------|------|--------|
  | `transport` | 计算服务协议 | `local` |
  | `server_url` | 远端 server 地址 | `http://localhost:8080` |
  | `python_backend_url` | Python 推理后端地址 | `http://localhost:8188` |
  | `python_auto_launch` | 是否自动拉起 Python 后端 | `true` |
  | `result_cache_limit_mb` | ResultCache 内存上限 | `2048` |
  | `texture_cache_limit_mb` | TextureCache VRAM 上限 | `512` |

- **加载时机：** 启动时一次性加载，运行时不热更新（重启生效）

---

### 11-cross-cutting.md — 横切关注点

> 跨层面的约束和策略：安全、日志、测试、性能、部署、版本。

**内容（全新章节，6 个子主题）：**

#### 安全性

标明扩展点位置和默认行为，不展开具体方案：

| 扩展点 | 位置 | 当前默认 | 未来接入 |
|--------|------|---------|---------|
| 认证/鉴权 | Transport 层 / server 接口 | 无认证（本地模式） | 远端模式需认证中间件 |
| 路径校验 | LoadImage / SaveImage 文件操作 | 无限制 | 远端模式限制可访问目录 |
| 后端认证 | AI 执行器 → Python HTTP 调用 | 无认证（localhost） | 预留 token 配置项 |

#### 可观测性 / 日志

使用 `tracing` crate，结构化日志 + span 追踪。

**关键观测点：**

| 事件 | 级别 | 说明 |
|------|------|------|
| 节点执行开始/结束 | INFO | 附带 node_id、耗时 |
| 节点执行失败 | ERROR | 附带 node_id、错误详情 |
| Cache 命中/失效 | DEBUG | 附带 node_id、缓存类型 |
| Transport 请求/响应 | DEBUG | 附带接口名、耗时 |
| Handle 创建/释放 | DEBUG | 附带 handle_id |
| VRAM/内存用量变化 | INFO | 定期采样 |

节点执行计时信息暴露给前端（节点 UI 可展示"耗时 120ms"）。

#### 测试策略

| 层 | 测试类型 | 说明 |
|---|---------|------|
| nodeimg-types / nodeimg-graph | 单元测试 | 纯数据结构 |
| nodeimg-gpu | GPU 集成测试 | 需要 wgpu device，CI 用软件渲染后端 |
| nodeimg-engine | 单元测试 + 集成测试 | Mock Transport 测执行链路，真实节点测端到端 |
| nodeimg-server | 接口测试 | 启动 server，发请求验证响应 |
| nodeimg-app | 逻辑层单元测试 | 框架无关逻辑脱离 UI 测试 |
| 节点 | 节点回归测试 | 固定输入 + 固定参数 → perceptual hash 比对输出 |

#### 性能目标

| 场景 | 目标 |
|------|------|
| 单个图像处理节点（GPU，1080p） | < 10ms |
| 单个图像处理节点（GPU，4K） | < 50ms |
| 单个图像处理节点（CPU 回退，1080p） | < 100ms |
| 20 节点图完整执行（1080p） | < 500ms |
| Cache 命中返回 | < 1ms |
| UI 帧率（执行期间） | 保持 60fps |
| Transport 本地调用开销 | < 0.1ms |
| Transport 远端调用开销（不含执行） | < 50ms |

AI 节点不设性能目标（取决于模型和硬件）。

#### 部署拓扑

三种部署模式：

| 模式 | 说明 | Transport |
|------|------|-----------|
| 全本地 | GUI + engine + Python 全在一台机器 | LocalTransport |
| 半本地 | GUI + engine 本地，Python 推理后端在远端 GPU 服务器 | LocalTransport（AI 执行器 HTTP 到远端 Python） |
| 远端计算 | GUI 本地，server + Python 在远端服务器 | HttpTransport |

半本地模式的意义：AI 推理太重，即使本机有好 GPU，也值得把推理后端放到更强的远端服务器（多卡集群、云端 A100），图像处理留在本地。

#### 版本兼容 / 迁移

**项目文件版本：**
- 文件中带 `version` 字段
- 加载时按版本号做迁移（旧版 → 当前格式）
- 节点类型不存在时跳过并警告
- 参数缺失填默认值、参数多余忽略

**Transport 接口版本：**
- server 暴露版本号
- GUI 连接时检查版本兼容性
- 不兼容时提示用户升级

---

### 12-decisions.md — 设计决策表

> 所有架构决策的索引，记录"选了什么"和"为什么"。

格式不使用通用模板，采用表格形式：

| ID | 主题 | 决策 | 理由 |
|----|------|------|------|
| D01 | 错误处理 | 分层 Error enum（NodeError → ExecutionError → TransportError） | 每层独立演进，thiserror 生态天然支持 |
| D02 | 错误展示 | 多位置同时展示（节点红框 + 进度区 + 全局通知） | 各层独立补充展示信息，不互斥 |
| D03 | Cache 架构 | 分离为 ResultCache + TextureCache | 执行结果和渲染纹理关注点不同，淘汰逻辑不应纠缠 |
| D04 | ResultCache 淘汰 | 失效驱动 + 可选 LRU 上限，Handle 豁免 LRU | Handle 淘汰意味着释放 Python GPU 对象，重算代价高 |
| D05 | TextureCache 淘汰 | LRU + VRAM 上限 | 预览纹理丢了可重新上传，代价低 |
| D06 | 并发模型 | 后台执行线程 + rayon 同层并行 | CPU/GPU 密集型任务用线程池最自然，不引入 async 复杂度 |
| D07 | AI HTTP 调用 | reqwest::blocking 在 rayon 线程中阻塞 | AI 节点通常串行依赖，同层并行 AI 节点场景少 |
| D08 | 配置格式 | TOML，CLI > 环境变量 > 文件 > 默认值 | Rust 生态标准做法 |
| D09 | 配置加载 | 启动时加载，不热更新 | 简单，配置变更频率极低 |
| D10 | server 接口 | 抽象服务接口与 Transport trait 对齐 | 不绑定具体传输协议，允许后期自由切换 |
| D11 | API 执行器 | 统一 ApiProvider trait | 认证/速率限制/重试逻辑每个 provider 都需要，统一避免重复 |
| D12 | ResultEnvelope | Transport 层决定 Local/Remote 路径 | 前端不感知模式差异 |
| D13 | 安全性 | 预留扩展点，当前不实现 | 做好接口，后期直接接入 |
| D14 | 可观测性 | tracing crate + 关键观测点 | Rust 生态标准，不需要架构级特殊设计 |
| D15 | 节点回归测试 | perceptual hash 比对 | 像素级比对在 GPU 浮点误差下 flaky |
| D16 | 部署模式 | 全本地 / 半本地 / 远端计算 | AI 推理太重，即使本机有好 GPU 也值得远端执行 |
| D17 | 版本兼容 | 项目文件 version + 迁移，Transport 接口版本协商 | 允许增量升级 |
| D18 | 插件系统 | 移除 | 当前阶段不需要，inventory 静态注册足够 |
| D19 | engine 内部 | 不拆 crate，mod 文件夹组织 | 避免 workspace 复杂度，内部模块图保证清晰 |
| D20 | WidgetRegistry | 移到 App 逻辑层 | engine 不应知道控件概念，行业标准做法（Blender、ComfyUI） |
| D21 | nodeimg-gpu 定位 | GPU 运行时基础设施（不含 shader） | shader 跟随节点文件夹 |
| D22 | Node 结构体归属 | 放 nodeimg-graph | types 保持轻量原子类型，Node 和 Graph 内聚在一起 |
| D23 | ExecutionManager | channel 通信 + 进度事件 + AtomicBool 取消 | 简单有效，不引入复杂异步机制 |
| D24 | 控件覆写 | node! 宏中 widget(...) 选择预置控件 | 新增控件和新增节点独立扩展 |

---

### 13-risks.md — 风险与技术债

> 已识别的风险和已知的技术债务。

来源：arc42 模板 §11。

**已识别风险：**

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| wgpu 跨平台兼容性 | 部分 GPU/驱动不支持 compute shader，CI 环境无 GPU | 提供 CPU 回退路径，CI 使用 wgpu 软件渲染后端 |
| Python 后端进程稳定性 | Python 进程崩溃或 OOM 导致 AI 节点全部失败 | AI 执行器做超时检测和自动重连，提示用户重启后端 |
| 大图性能瓶颈 | 节点数量 >100 时拓扑排序、Cache 失效递归、UI 渲染可能变慢 | 设性能目标，定期 profiling，考虑增量执行 |
| 远端模式网络不稳定 | 执行中断、结果丢失 | Transport 层做重试，执行结果持久化到 ResultCache |
| VRAM 耗尽 | 多个高分辨率纹理同时驻留 GPU | TextureCache LRU 淘汰 + VRAM 上限配置 |

**已知技术债：**（待具体实现时补充条目）

---

### 14-glossary.md — 术语表

> 项目中使用的领域术语定义。

来源：arc42 模板 §12 + 现有 domain.md。

**核心术语清单：**

| 术语 | 定义 |
|------|------|
| Node | 节点图中的一个处理单元，有输入引脚、输出引脚和参数 |
| Pin | 节点的输入或输出端口，有名称和数据类型 |
| Connection | 两个节点的引脚之间的数据连接 |
| Graph | 由 Node 和 Connection 组成的有向无环图 |
| Value | 在引脚之间传递的数据值（Image、Float、Int、String、Handle…） |
| Handle | AI 执行器中 Python 端 GPU 对象的引用标识符，不含实际数据 |
| DataType | 引脚或参数的数据类型标识 |
| Constraint | 参数的约束条件（Range、Enum、FilePath…） |
| Transport | 前端与服务层之间的通信抽象层 |
| EvalEngine | 负责拓扑排序和节点执行分发的核心调度器 |
| ExecutionManager | App 逻辑层中管理异步执行任务的组件 |
| ResultCache | 缓存节点执行结果的存储 |
| TextureCache | 缓存预览用 GPU 纹理的存储 |
| ResultEnvelope | 包装执行结果的类型��区分本地引用和远端序列化 |
| NodeDef | 节点类型的元信息定义（引脚、参数、分类） |
| WidgetRegistry | 将 DataType + Constraint 映射为 UI 控件类型的注册表 |

详细定义基于现有 domain.md 扩充，补充目标架构新增概念。

---

## 6. 实施方式

1. 创建 `docs/current/` 目录，将现有文档迁入（不修改内容）
2. 创建 `docs/target/` 目录
3. 按上述设计逐文件编写，内容来源：
   - 现有 target-architecture.md 中可复用的部分（调用链、AI 执行器、混合图执行、节点架构等）
   - 本次讨论确定的新设计（错误处理、Cache、并发、配置、server、API 执行器、ResultEnvelope、横切关注点）
   - 需要新写的部分（00-context、13-risks、14-glossary）
4. 删除旧的 `docs/target-architecture.md` 和 `docs/target-architecture-review.md`
5. 更新 CLAUDE.md 中的文档引用
