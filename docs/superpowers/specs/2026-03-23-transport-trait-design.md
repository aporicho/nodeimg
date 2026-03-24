# Transport Trait + LocalTransport 设计文档

> Issue: #57 | 依赖: #13（已完成）| 日期: 2026-03-23

## 概述

在 nodeimg-engine 中定义 `ProcessingTransport` trait，将 app 与 engine 的交互从"直接操作内部零件"改为"通过统一接口下单"。实现 `LocalTransport` 作为进程内调用方案，行为不变但架构就位。

## 动机

当前 app 直接持有 EvalEngine、Cache、NodeRegistry、BackendClient，自己做拓扑排序、遍历执行、AI 转发。app 知道太多 engine 内部细节，无法透明切换本地/远端执行。

改完后 app 只需要：编辑节点图 + 显示结果。执行的事全部交给 Transport。

## 架构

```
┌──────────────────────┐
│  nodeimg-app          │
│                      │
│  用户编辑节点图        │
│       │              │
│       ▼              │
│  ExecutionManager     │  管理后台执行 + 轮询结果
│       │              │
│  transport.execute()  │  只调这一个方法
└───────┼──────────────┘
        │
   ┌────┴────────────────────────┐
   │                             │
   ▼                             ▼
LocalTransport               RemoteTransport（未来）
进程内调用                    网络调用（协议待定）
   │                             │
   ▼                             ▼
┌──────────────────────┐  ┌──────────────────────┐
│  engine 内部          │  │  nodeimg-server       │
│  EvalEngine          │  │  接收请求             │
│  Cache               │  │  调 engine 执行        │
│  NodeRegistry        │  │  返回结果             │
│  BackendClient       │  └──────────────────────┘
└──────────────────────┘
```

## 核心设计

### Transport Trait

```rust
trait ProcessingTransport: Send + Sync + 'static {
    /// 执行节点图，逐节点推送结果
    fn execute(&self, request: GraphRequest, progress: Sender<ExecuteProgress>) -> Result<()>;

    /// 缓存失效：通知 transport 某节点参数变了，需要清除该节点及下游缓存
    fn invalidate(&self, node_id: NodeId);

    /// 清除所有缓存
    fn invalidate_all(&self);

    /// 检查两个数据类型是否可以连接
    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool;

    /// 健康检查（启动时调一次）
    fn health_check(&self) -> Result<HealthResponse>;

    /// 获取可用节点类型列表（启动时调一次）
    fn node_types(&self) -> Result<Vec<NodeTypeDef>>;
}
```

### HealthResponse

```rust
#[derive(Serialize, Deserialize)]
struct HealthResponse {
    pub status: String,           // "ok" / "error"
    pub gpu: Option<String>,      // GPU 名称
    pub vram_free_gb: Option<f32>,
}
```

### NodeTypeDef（节点元数据，完全可序列化）

`NodeDef` 包含函数指针（`ProcessFn`、`GpuProcessFn`）和不可序列化的类型（`Value`、`Constraint`），不能直接用于 Transport 接口。`NodeTypeDef` 是纯元数据版本，所有字段均可序列化：

```rust
#[derive(Clone, Serialize, Deserialize)]
struct NodeTypeDef {
    pub type_id: String,
    pub title: String,
    pub category: String,              // CategoryId 序列化为 String
    pub inputs: Vec<PinDefInfo>,
    pub outputs: Vec<PinDefInfo>,
    pub params: Vec<ParamDefInfo>,
    pub has_preview: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct PinDefInfo {
    pub name: String,
    pub data_type: String,             // DataTypeId 序列化为 String
}

#[derive(Clone, Serialize, Deserialize)]
struct ParamDefInfo {
    pub name: String,
    pub data_type: String,             // DataTypeId 序列化为 String
    pub default: ParamValue,           // 用 ParamValue 替代 Value
    pub constraint: Option<ConstraintInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
enum ConstraintInfo {
    Range { min: f64, max: f64 },
    Options(Vec<String>),
}
```

`LocalTransport::node_types()` 负责将内部的 `NodeDef` 转换为 `NodeTypeDef`。

App 启动时通过 `node_types()` 获取一次，保存本地副本，用于构建节点菜单、实例化节点、渲染引脚名称和颜色。

### ExecuteProgress（逐节点推送）

```rust
enum ExecuteProgress {
    /// 一个节点执行完成，附带该节点的全部输出
    NodeCompleted {
        node_id: NodeId,
        outputs: HashMap<String, Value>,
    },
    /// 全部节点执行完成
    Finished,
    /// 执行出错
    Error {
        node_id: Option<NodeId>,
        message: String,
    },
}
```

**设计决策：逐节点推送而非一次性返回。** 用户能实时看到每个节点的执行进度，不用等全部完成。和当前 app 的行为一致。

### GraphRequest（类型安全的节点图描述）

```rust
#[derive(Serialize, Deserialize)]
struct GraphRequest {
    nodes: HashMap<NodeId, NodeRequest>,
    connections: Vec<ConnectionRequest>,
    target_node: NodeId,
}

#[derive(Serialize, Deserialize)]
struct NodeRequest {
    type_id: String,
    params: HashMap<String, ParamValue>,
}

#[derive(Serialize, Deserialize)]
struct ConnectionRequest {
    from_node: NodeId,
    from_pin: String,
    to_node: NodeId,
    to_pin: String,
}
```

**设计决策：使用 Rust 结构体而非 serde_json::Value。** 编译时类型检查，LocalTransport 直接用结构体零开销，RemoteTransport 通过 Serialize 自动转 JSON。

### ParamValue（可序列化的参数值）

`Value` 枚举包含 `Arc<DynamicImage>` 等不可序列化的 variant，不能直接放在 `GraphRequest` 中。`ParamValue` 只包含参数用到的标量类型：

```rust
#[derive(Clone, Serialize, Deserialize)]
enum ParamValue {
    Float(f32),
    Int(i32),
    String(String),
    Boolean(bool),
    Color([f32; 4]),
}
```

类型宽度与 `Value` 的 `Float(f32)` / `Int(i32)` 保持一致，避免转换时精度丢失或溢出。

`LocalTransport` 内部负责将 `ParamValue` 转换为 `Value` 再传给 `EvalEngine`。图像等大数据通过连接传递，不出现在参数中。

### LocalTransport

```rust
struct LocalTransport {
    registry: Mutex<NodeRegistry>,
    type_registry: DataTypeRegistry,
    cache: Mutex<Cache>,
    gpu_context: Arc<GpuContext>,
    backend: Option<BackendClient>,
}
```

注意事项：
- `registry` 和 `cache` 用 `Mutex` 包裹以提供内部可变性（trait 方法是 `&self`）
- `type_registry`：EvalEngine 执行时需要，用于引脚间的自动类型转换
- 不持有 `EvalEngine` 字段——`EvalEngine` 是无状态的 unit struct，所有方法都是关联函数，直接以 `EvalEngine::evaluate(...)` 方式调用

执行流程：
1. 接收 `GraphRequest`，将 `ParamValue` 转换为 `Value`
2. `EvalEngine::topo_sort()` 得出执行顺序
3. 按顺序执行每个节点：
   - 本地节点 → 调用 `process` / `gpu_process`
   - AI 节点 → 通过内部 `BackendClient` 转发 Python
4. 每个节点完成后 `progress.send(NodeCompleted { ... })`
5. 全部完成后 `progress.send(Finished)`

### ExecutionManager（app 侧）

```rust
struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    progress_tx: Sender<(u64, ExecuteProgress)>,
    progress_rx: Receiver<(u64, ExecuteProgress)>,
    generation: u64,
    repaint_ctx: Option<egui::Context>,
}
```

核心方法：
- `submit(graph)`: generation +1，spawn 后台线程调用 `transport.execute()`，线程完成时通过 `repaint_ctx.request_repaint()` 唤醒 egui 刷新
- `poll()`: 每帧非阻塞检查，generation 不匹配的结果丢弃
- `cancel()`: generation +1 使当前执行的结果失效（工人线程继续跑完但结果被丢弃）
- `is_running()`: 检查是否有正在执行的任务

替代现有的 `AiExecutor`。

## 命名

- `LocalTransport` — 进程内调用
- `RemoteTransport` — 远端调用（未来实现，不绑定具体协议）

## 文件结构调整

### nodeimg-engine（重组）

```
nodeimg-engine/src/
├── lib.rs                ← 公开导出：Transport trait + LocalTransport + 相关类型
├── transport/
│   ├── mod.rs            ← Transport trait、GraphRequest、ExecuteProgress 等类型定义
│   └── local.rs          ← LocalTransport 实现
├── internal/
│   ├── mod.rs
│   ├── eval.rs           ← EvalEngine（内部）
│   ├── cache.rs          ← Cache（内部）
│   ├── registry.rs       ← NodeRegistry（内部）
│   ├── backend.rs        ← BackendClient（内部）
│   └── menu.rs           ← 菜单构建（内部）
└── builtins/             ← 不动
    └── ...
```

**关键变化：** engine 分出"门面"（transport/）和"内部"（internal/）。app 只和门面打交道，不再直接访问 EvalEngine、Cache 等内部类型。

### nodeimg-app（小调整）

```
nodeimg-app/src/
├── app.rs                ← 改用 Transport 初���化
├── execution.rs          ← ExecutionManager（新，替代 ai_task.rs 的职责）
├── node/                 ← 不动
├── gpu/                  ← 不动
├── theme/                ← 不动
└── ui/                   ← 不动
```

## App 侧的节点元数据

App 不再直接访问 `NodeRegistry`，而是：

1. 启动时调 `transport.node_types()` 获取 `Vec<NodeTypeDef>`
2. 本地保存这份元数据副本
3. 用它来：构建添加节点菜单、实例化节点模板、检查引脚兼容性、渲染引脚名称和颜色

## 对现有代码的影响

### 要改的
- `nodeimg-engine/lib.rs`：重组导出，只暴露 Transport 相关公开 API
- `nodeimg-engine` 文件结构：按上述方案重组
- `nodeimg-app/node/viewer.rs`：NodeViewer 不再直接持有 EvalEngine/Cache，改为持有 `Arc<dyn ProcessingTransport>` + `Vec<NodeTypeDef>` 元数据副本
- `nodeimg-app/app.rs`：初始化 LocalTransport，创建 ExecutionManager

### 不动的
- EvalEngine、Cache、NodeRegistry、BackendClient 内部逻辑不变
- GPU shader、节点定义（builtins/）不变
- UI 渲染逻辑不变
- Python 后端不变

### 删掉的
- `ai_task.rs`（AiExecutor 被 ExecutionManager 替代）
- app 中直接调用 EvalEngine 的代码

## 同步/异步策略

- **Transport trait 方法是同步的**：描述"能做什么"，不管"怎么调度"
- **App 侧负责异步调度**：ExecutionManager 通过 spawn 线程 + channel 轮询，和现有 AiExecutor 模式一致
- **不引入 async runtime**：不需要 tokio，保持轻量
- `health_check()` / `node_types()`：启动时同步调用
- `execute()`：通过 ExecutionManager 异步调用
- `invalidate()` / `invalidate_all()`：app 参数变更时同步调用

## 迁移步骤

分阶段推进，确保每一步后项目可编译运行：

1. **定义 transport types**：新增 `transport/mod.rs`，定义 trait + 所有关联类型（GraphRequest、ExecuteProgress、ParamValue、NodeTypeDef、HealthResponse），不改现有代码。同时将 `ConversionFn` 改为 `Box<dyn Fn(Value) -> Value + Send + Sync>` 以满足 `ProcessingTransport: Send + Sync` 的要求
2. **重组 engine 文件结构**：将 eval/cache/registry/backend/menu 移入 `internal/`，调整 `lib.rs` 导出
3. **实现 LocalTransport**：在 `transport/local.rs` 中包装现有执行逻辑
4. **新增 ExecutionManager**：在 app 中实现，与旧的 AiExecutor 并行存在
5. **切换 app 到 Transport**：NodeViewer 改用 Transport + ExecutionManager，替换直接调用
6. **删除旧代码**：移除 AiExecutor、app 中直接调用 engine 内部的代码

## 未来扩展

- `RemoteTransport`：实现同一 trait，背后走网络调用 nodeimg-server
- nodeimg-server：实现 HTTP 端点，内部调用 engine 执行
- 协议升级：从 HTTP+JSON 换到 gRPC/WebSocket，只加新 Transport 实现，app 不用动
