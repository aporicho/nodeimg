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

    /// 健康检查（启动时调一次）
    fn health_check(&self) -> Result<HealthResponse>;

    /// 获取可用节点类型列表（启动时调一次）
    fn node_types(&self) -> Result<Vec<NodeTypeDef>>;
}
```

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
    params: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize)]
struct ConnectionRequest {
    from_node: NodeId,
    from_pin: String,
    to_node: NodeId,
    to_pin: String,
}
```

**设计决策：使用 Rust 结构��而非 serde_json::Value。** 编译时类型检查，LocalTransport 直接用结构体零开销，RemoteTransport 通过 Serialize 自动转 JSON。

### LocalTransport

```rust
struct LocalTransport {
    registry: NodeRegistry,
    eval_engine: EvalEngine,
    cache: Cache,
    gpu_context: Arc<GpuContext>,
    backend: Option<BackendClient>,
}
```

执行流程：
1. 接收 `GraphRequest`
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
}
```

- `submit(graph)`: generation +1，spawn 后台线程调用 `transport.execute()`
- `poll()`: 每帧非阻塞检查，generation 不匹配的结果丢弃
- 替代现有的 `AiExecutor`

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
├── app.rs                ← 改用 Transport 初始化
├── execution.rs          ← ExecutionManager（新，替代 ai_task.rs 的职责）
├── node/                 ← 不动
├── gpu/                  ← 不动
├── theme/                ← 不动
└── ui/                   ← 不动
```

## 对现有代码的影响

### 要改的
- `nodeimg-engine/lib.rs`：重组导出，只暴露 Transport 相关公开 API
- `nodeimg-engine` 文件结构：按上述方案重组
- `nodeimg-app/node/viewer.rs`：NodeViewer 不再直接持有 EvalEngine/Cache，改为持有 `Arc<dyn ProcessingTransport>`
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

## 未来扩展

- `RemoteTransport`：实现同一 trait，背后走网络调用 nodeimg-server
- nodeimg-server：实现 HTTP 端点，内部调用 engine 执行
- 协议升级：从 HTTP+JSON 换到 gRPC/WebSocket，只加新 Transport 实现，app 不用动
