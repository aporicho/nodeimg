# show_body 迁移设计

**Issue**: #65
**分支**: `refactor/show-body-transport`
**日期**: 2026-03-26

## 目标

将 `show_body()` 从直接访问 engine 内部类型迁移到通过 `ProcessingTransport` 接口调用，彻底删除 `NodeViewer` 中的重复状态字段。

## 背景

#57 引入了 `ProcessingTransport` trait 和 `LocalTransport`，但 `show_body()` 仍直接使用 `EvalEngine`、`Cache`、`NodeRegistry` 等 engine 内部类型。导致 `NodeViewer` 同时持有 `transport` 和这些内部组件，产生双重状态问题（如 AI 节点需注册两次）。

## 设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 执行模式 | 保持同步 | 本地节点零延迟，用户体验优先 |
| Transport API | 细粒度方法 | 按需调用，职责清晰，远程 Transport 更灵活 |
| AI 执行 | ExecutionManager 替代 AiExecutor | 统一执行路径 |
| WidgetRegistry | 不动 | 不在本次范围，已记录 #67 |

## 1. Transport trait 扩展

`ProcessingTransport` 新增 3 个方法：

```rust
pub trait ProcessingTransport: Send + Sync + 'static {
    // --- 现有方法不变 ---
    fn execute(&self, request: GraphRequest, progress: Sender<ExecuteProgress>) -> Result<(), String>;
    fn invalidate(&self, node_id: NodeId);
    fn invalidate_all(&self);
    fn is_compatible(&self, from_type: &str, to_type: &str) -> bool;
    fn health_check(&self) -> Result<HealthResponse, String>;
    fn node_types(&self) -> Result<Vec<NodeTypeDef>, String>;

    // --- 新增 ---

    /// 同步执行本地节点（跳过 AI），返回每个节点的输出值。
    fn evaluate_local_sync(&self, request: GraphRequest) -> Result<HashMap<NodeId, Value>, String>;

    /// 查询某个节点的缓存结果（不触发执行）。
    fn get_cached(&self, node_id: NodeId) -> Option<Value>;

    /// 检查图中是否有待执行的 AI 节点。
    fn pending_ai_execution(&self, request: GraphRequest) -> Option<(NodeId, serde_json::Value)>;

    /// 检查添加一条连接后是否会产生环。用于 connect() 中的环检测。
    fn would_create_cycle(&self, target: NodeId, connections: &[ConnectionInfo]) -> bool;
}
```

### LocalTransport 实现

- `evaluate_local_sync` — 内部调 `EvalEngine::evaluate(backend=None)`，操作自有的 registry/cache/gpu_ctx
- `get_cached` — 锁 cache，调 `cache.get(node_id)`，clone 返回
- `pending_ai_execution` — 内部调 `EvalEngine::pending_ai_execution()`
- `would_create_cycle` — 内部调 `EvalEngine::topo_sort()`，返回是否出错（有环）

## 2. show_body 迁移

逻辑流程不变，调用对象从"自己的字段"变为"transport 的方法"：

```
show_body 每帧流程：

1. widget_registry.render() — 渲染参数控件
   ↓ 参数变了？
2. transport.invalidate(node_id)              ← 原 self.cache.invalidate()
   ↓
3. transport.evaluate_local_sync(request)     ← 原 EvalEngine::evaluate()
   → HashMap<NodeId, Value>
   ↓
4. transport.pending_ai_execution(request)    ← 原 EvalEngine::pending_ai_execution()
   → 有 AI？execution_manager.submit()
   ↓
5. execution_manager.poll()                   ← 原 self.ai_executor.poll_results()
   → ExecuteProgress::NodeCompleted / Error
   ↓
6. transport.get_cached(node_id)              ← 原 self.cache.get()
   → 图片 → egui 纹理 → 预览
```

### connect() 环检测迁移

`connect()` 方法当前直接调用 `EvalEngine::topo_sort()` 和 `Connection` 类型做环检测。迁移后改为：

```rust
// 迁移前
EvalEngine::topo_sort(to.id.node.0, &connections).is_err()

// 迁移后
transport.would_create_cycle(to.id.node.0, &connection_infos)
```

`ConnectionInfo` 是 transport 模块中定义的可序列化连接描述，替代 engine 内部的 `Connection` 类型。

show_body 和 connect() 迁移后不再 use 的 engine 类型：`EvalEngine`、`Cache`、`Connection`、`NodeRegistry`、`DataTypeRegistry`、`AiExecutor`、`BackendClient`。

## 3. NodeViewer 字段清理

```rust
pub struct NodeViewer {
    // 保留
    pub transport: Arc<LocalTransport>,
    pub node_type_defs: Vec<NodeTypeDef>,
    pub widget_registry: WidgetRegistry,
    pub execution_manager: ExecutionManager,     // 新增，替代 ai_executor
    pub textures: HashMap<NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub theme: Arc<dyn Theme>,
    pub gpu_ctx: Option<Arc<GpuContext>>,        // Arc 共享，非重复
    pub backend_status: Option<String>,
    ai_errors: HashMap<NodeId, String>,
    pub graph_dirty: bool,

    // 删除
    // node_registry: NodeRegistry           ← 通过 transport
    // type_registry: DataTypeRegistry       ← 通过 transport
    // category_registry: CategoryRegistry   ← 通过 transport
    // cache: Cache                          ← 通过 transport
    // backend: Option<BackendClient>        ← 通过 transport
    // ai_executor: AiExecutor               ← 被 execution_manager 替代
}
```

### app.rs 双重注册消除

```rust
// 迁移前
transport.register_remote_nodes(&backend);
backend.register_remote_nodes(&mut viewer.node_registry, &mut viewer.type_registry);

// 迁移后
transport.register_remote_nodes(&backend);
viewer.node_type_defs = transport.node_types().unwrap_or_default();
```

## 4. ai_task.rs 删除

AiExecutor 的功能由 ExecutionManager 替代：

| AiExecutor | ExecutionManager（需扩展） |
|------------|---------------------------|
| `spawn()` | `submit(request)` |
| `poll_results(&mut cache)` | `poll() -> Vec<ExecuteProgress>` |
| `cancel()` / `cancel_all()` | `cancel()` / `cancel_all()` |
| `running: HashMap<NodeId, u64>` — 支持多并发 | 当前 `running: bool` — 仅单任务 |
| per-node generation | 需扩展为 per-node generation |

### ExecutionManager 并发扩展

当前 ExecutionManager 只支持单个后台任务（`running: bool` + 单个 `generation`）。AiExecutor 支持按 trigger node 追踪多个并发 AI 任务。需要扩展 ExecutionManager：

```rust
pub struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    // 从单任务��为多任务
    tasks: HashMap<NodeId, TaskState>,  // key = trigger node
    repaint: Option<egui::Context>,
}

struct TaskState {
    generation: u64,
    rx: mpsc::Receiver<(u64, ExecuteProgress)>,
}
```

- `submit(trigger_node, request)` — 按 trigger node 追踪，多个 AI 节点可并发
- `poll()` — 遍历所有活跃任务，收集结果
- `cancel(node_id)` — 取消指定节点的任务
- `cancel_all()` — 取消所有任务

### 适配要点

- **结果入 cache**：`transport.execute()` 内部执行时已将结果写入 cache，poll 返回的 `ExecuteProgress::NodeCompleted` 用于 UI 更新
- **错误收集**：从 `ExecuteProgress::Error` 提取错误写入 `ai_errors`
- **AI 触发流程**：show_body 检测 `pending_ai_execution` → 构造 `GraphRequest` → `execution_manager.submit(trigger_node, request)`

### 删除清单

- `crates/nodeimg-engine/src/internal/ai_task.rs`
- `lib.rs` 中 `pub use internal::ai_task`
- show_body 中所有 `self.ai_executor.*` 调用

## 5. engine 导出收紧

```rust
// 迁移后 lib.rs
pub(crate) mod builtins;  // 注册在 LocalTransport::new() 内部完成，外部不需要
pub(crate) mod internal;
pub mod transport;

pub use internal::cache::NodeId;
pub use internal::registry::NodeInstance;
```

删除的导出：`ai_task`、`backend`、`cache`、`eval`、`registry`，以及 `NodeDef`、`NodeRegistry`、`ProcessFn`、`GpuProcessFn` 等内部类型。

`menu` 模块已通过 `LocalTransport::generate_menu()` 封装，不再需要直接导出。

## 效果

- engine crate 对外只暴露 `transport` 模块（公共 API）+ `NodeId` + `NodeInstance`（数据类型）
- NodeViewer 通过 Transport 接口与 engine 交互，不持有任何 engine 内部组件
- AI 节点注册只需一次，消除双重状态 bug 面积
- show_body 逻辑不变，用户体验不变（本地节点同步、AI 节点异步）
