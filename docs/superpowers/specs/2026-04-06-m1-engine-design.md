# M1 引擎核心设计

> feature/m1-engine 分支的实现设计。推倒重建 nodeimg-engine，按目标架构文档落地。

## 关联

- Issue: #7（M1 引擎核心）、#20（撤销/重做）
- 分支: `feature/m1-engine`
- 目标文档: 0.0.0、0.1.0、0.2.0、4.0.0、4.2.0、4.3.0、4.4.0、4.9.0

---

## 决策记录

| 决策 | 选项 | 结论 |
|------|------|------|
| 重建方式 | 推倒重建 / 渐进迁移 / 最小闭环 | 推倒重建 |
| app crate 处理 | 兼容层 / 改调用点 / 架空 | 架空（从 workspace 移除） |
| 旧代码处理 | 删除 / legacy 子模块 / git 参考 | 删除，git 是参考 |
| 节点 API | 手写 / node! 宏 / trait+inventory | node! 过程宏 + inventory |
| execute 签名 | 同步 / 异步 | 异步（async fn） |
| 执行入口 | 简单 evaluate / 最小调度器 / 严格按文档拆 | Engine::evaluate()，Scheduler 延后 M2 |
| undo/redo 实现 | 结构共享 / 命令模式 / 暴力克隆 | Arc 结构共享 |
| nodeimg-types | 照用 / 精简 / 重写 | 精简重构 |
| M1 交付物 | 库+测试 / +CLI 例程 / +server | 库 + 集成测试 |

### 与目标文档的有意偏差

以下偏差经讨论确认，实现后需回去修改对应目标文档：

1. **DataType 改为开放类型**（偏离 0.1.0）：`DataType(String)` 而非闭合枚举。原因：未来插件系统需要自定义数据类型。
2. **Value::Image 懒转换**（偏离 0.1.0）：Image 内部持有 CPU + GPU 双缓存，按需转换。原因：支持无 GPU 环境 + 避免不必要的显存搬运。
3. **execute 签名 async**（偏离 4.3.0）：所有执行函数统一 async。原因：M3 AI 节点天然异步，统一签名避免后续改动。

---

## 仓库重构

按 0.2.0 文件架构重组 workspace：去 `crates/` 包裹、去 `nodeimg-` 前缀、合并 gpu/processing 进 engine。

### 删除

- `crates/nodeimg-gpu/` — 内容合并到 `engine/src/executors/image/gpu.rs`
- `crates/nodeimg-processing/` — 内容合并到 `engine/src/executors/image/cpu.rs` + 各节点 cpu.rs
- `crates/nodeimg-server/` — 删除（目标架构无对应）
- `crates/nodeimg-engine/` — 删除（重写为 `engine/`）
- `crates/nodeimg-types/` — 删除（重写为 `types/`）
- `crates/` 目录本身

### 保留但架空

- `crates/nodeimg-app/` — 物理文件保留（下个 PR 的 iced 迁移参考），但从 workspace 成员移除

### 新建

```
nodeimg/
├── types/                  基础类型层
│   └── src/
│       ├── lib.rs
│       ├── value.rs        DataType, Value, Image
│       ├── constraint.rs   Constraint
│       ├── id.rs           NodeId, PinRef
│       ├── geometry.rs     Vec2
│       └── texture.rs      GpuTexture
│
├── engine/                 引擎核心
│   └── src/
│       ├── lib.rs          Engine facade
│       ├── graph/          4.2.0 节点图控制器
│       │   ├── mod.rs      Graph, Node, Connection
│       │   ├── version.rs  GraphState（Arc<Graph> + undo/redo 栈）
│       │   ├── editor.rs   Graph 编辑方法
│       │   ├── validate.rs 结构校验（环检测 + 类型兼容）
│       │   └── topology.rs 拓扑查询
│       ├── graph_controller.rs  GraphController 包装层
│       ├── registry/       4.4.0 节点管理器
│       │   ├── mod.rs      NodeManager
│       │   ├── def.rs      NodeDef, PinDef, ParamDef, ExecuteFn
│       │   └── inventory.rs
│       ├── executors/
│       │   ├── mod.rs
│       │   └── image/      4.9.0 图像执行器
│       │       ├── mod.rs  ImageExecutor
│       │       ├── context.rs ExecContext
│       │       ├── gpu.rs  GpuExecutor
│       │       └── cpu.rs  CpuExecutor
│       └── builtins/
│           ├── mod.rs
│           ├── load_image/{mod.rs}
│           ├── save_image/{mod.rs}
│           ├── brightness/{mod.rs, shader.wgsl}
│           └── contrast/{mod.rs, shader.wgsl}
│
├── macros/                 过程宏（node! 宏）
│   └── src/lib.rs
│
└── docs/
```

与 0.2.0 文档的唯一新增：`macros/` crate。Rust 过程宏必须独立 crate，语言限制。需回去补到 0.2.0 文档。

---

## 基础类型（types/）

### DataType

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DataType(pub String);

// const 上下文不能调 String::from()，用关联函数代替
impl DataType {
    pub fn image() -> Self { Self("image".into()) }
    pub fn float() -> Self { Self("float".into()) }
    pub fn int() -> Self { Self("int".into()) }
    pub fn bool() -> Self { Self("bool".into()) }
    pub fn color() -> Self { Self("color".into()) }
    pub fn string() -> Self { Self("string".into()) }
}
```

开放类型，内置关联函数提供便利。兼容性检查走注册表（M1 最简实现，M2 做完整 Registry）。

### Value

```rust
pub enum Value {
    Image(Image),
    Float(f32),
    Int(i64),
    Bool(bool),
    Color([f32; 4]),
    String(String),
}
```

后续里程碑新增：Mask（M2）、Handle（M3）。

### Image

懒转换设计，内部持有 CPU 和 GPU 双缓存：

```rust
use std::sync::OnceLock;

pub struct Image {
    cpu: OnceLock<Arc<DynamicImage>>,
    gpu: OnceLock<Arc<GpuTexture>>,
}

impl Image {
    pub fn from_cpu(img: DynamicImage) -> Self;   // 设置 cpu OnceLock，gpu 留空
    pub fn from_gpu(tex: GpuTexture) -> Self;     // 设置 gpu OnceLock，cpu 留空
    pub fn as_gpu(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> &Arc<GpuTexture>;
    pub fn as_cpu(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> &Arc<DynamicImage>;
}
```

用 OnceLock 实现内部可变性：首次调用 `as_gpu`/`as_cpu` 时转换并写入 OnceLock，后续调用直接返回缓存。方法签名是 `&self`（不需要 `&mut self`），对节点透明。

### Constraint

```rust
pub struct Constraint {
    pub type_id: String,
    pub params: serde_json::Value,
}
```

M1 内置三种 type_id：`range`、`enum`、`file_path`。M2 加 ConstraintRegistry。

### NodeId / PinRef / Vec2

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

pub struct PinRef {
    pub node: NodeId,
    pub pin: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
```

### GpuTexture

从原 nodeimg-gpu 搬入 types/src/texture.rs。持有 wgpu::Texture + width/height + format。上下文（device/queue/pipeline cache）留在 engine/src/executors/image/gpu.rs。

---

## 节点定义与 NodeManager（engine/src/registry/）

### NodeDef

一个节点类型的完整说明书：

```rust
pub struct NodeDef {
    pub type_id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub params: Vec<ParamDef>,
    pub execute: ExecuteFn,
}

pub struct PinDef {
    pub name: String,
    pub data_type: DataType,
    pub optional: bool,
}

pub struct ParamDef {
    pub name: String,
    pub data_type: DataType,
    pub constraint: Option<Constraint>,
    pub default_value: Value,
}

pub type ExecuteFn = Box<
    dyn Fn(ExecContext<'_>, HashMap<String, Value>)
        -> Pin<Box<dyn Future<Output = Result<HashMap<String, Value>>>>>
        + Send + Sync
>;
```

当前代码的 `process` + `gpu_process` 两个函数合并为一个 `execute`。节点内部通过 `ctx.gpu()` / `ctx.cpu()` 自行选择执行路径。

### node! 宏（macros/）

开发者写声明式定义：

```rust
node! {
    name: "brightness",
    title: "亮度",
    category: "颜色校正",
    inputs:  [image: Image required],
    outputs: [image: Image],
    params:  [brightness: Float range(-1.0, 1.0) default(0.0)],
    execute: |ctx, inputs| {
        ctx.gpu(|gpu| gpu.run_shader(include_str!("shader.wgsl"), &inputs))
    },
}
```

宏展开生成：
1. 一个 NodeDef 实例（填充所有字段）
2. 一个 `inventory::submit!` 调用（编译期自动注册）

### NodeManager

```rust
pub struct NodeManager {
    nodes: HashMap<String, NodeDef>,
}

impl NodeManager {
    pub fn from_inventory() -> Self;
    pub fn get(&self, type_id: &str) -> Option<&NodeDef>;
    pub fn list_by_category(&self, category: &str) -> Vec<&NodeDef>;
    pub fn search(&self, query: &str) -> Vec<&NodeDef>;
    pub fn default_params(&self, type_id: &str) -> Option<HashMap<String, Value>>;
    pub fn pin_type(&self, type_id: &str, pin_name: &str) -> Option<&DataType>;
}
```

inventory 编译期收集 → 启动时 `from_inventory()` 一次性加载。重复注册覆盖并日志警告。

---

## GraphController（engine/src/graph/ + graph_controller.rs）

### 数据结构

```rust
// graph/mod.rs
pub struct Graph {
    pub nodes: HashMap<NodeId, Arc<Node>>,
    pub connections: Vec<Connection>,
    next_id: u64,
}

pub struct Node {
    pub id: NodeId,
    pub type_id: String,
    pub params: HashMap<String, Value>,
    pub position: Vec2,
}

pub struct Connection {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}
```

Graph 是不可变值——所有修改产生新 Graph。clone() 便宜：HashMap 里存 Arc<Node>，只复制指针。

### 版本管理器（graph/version.rs）

```rust
pub struct GraphState {
    pub current: Arc<Graph>,
    undo_stack: Vec<Arc<Graph>>,
    redo_stack: Vec<Arc<Graph>>,
    max_undo: usize,
}
```

按 0.1.0 文档使用 Arc<Graph>。Scheduler（M2）可零成本拿快照。

操作：
- `commit(graph)` — current 入 undo 栈，新 graph 成为 current，清 redo 栈
- `preview(graph)` — 替换 current，不入栈（拖动中实时预览）
- `undo()` / `redo()` — 栈间交换
- `replace(graph)` — 替换 current，清空两个栈（项目加载）
- `snapshot()` — 返回 Arc<Graph> 的 clone（零成本）

### 图编辑器（graph/editor.rs）

```rust
impl Graph {
    pub fn add_node(&self, type_id: &str, position: Vec2, defaults: HashMap<String, Value>) -> (Graph, NodeId);
    pub fn remove_node(&self, id: NodeId) -> Graph;
    pub fn connect(&self, conn: Connection) -> Graph;
    pub fn disconnect(&self, from: NodeId, from_pin: &str, to: NodeId, to_pin: &str) -> Graph;
    pub fn set_param(&self, id: NodeId, key: &str, value: Value) -> Graph;
    pub fn move_node(&self, id: NodeId, position: Vec2) -> Graph;
}
```

每个方法返回新 Graph，不修改 self。内部用 `Arc::make_mut` 只 clone 被改的节点。

### 结构校验器（graph/validate.rs）

```rust
pub fn validate_connection(
    graph: &Graph,
    conn: &Connection,
    node_manager: &NodeManager,
) -> Result<(), ConnectionError>;
```

环检测（DFS）+ 类型兼容检查（查 NodeManager 获取引脚 DataType）。

### 拓扑查询器（graph/topology.rs）

```rust
pub fn topo_sort(graph: &Graph, target: NodeId) -> Result<Vec<NodeId>, CycleError>;
pub fn upstream(graph: &Graph, id: NodeId) -> HashSet<NodeId>;
pub fn downstream(graph: &Graph, id: NodeId) -> HashSet<NodeId>;
```

### GraphController 包装层（graph_controller.rs）

```rust
pub struct GraphController {
    state: GraphState,
    node_manager: Arc<NodeManager>,
}

impl GraphController {
    pub fn add_node(&mut self, type_id: &str, position: Vec2) -> Result<NodeId>;
    pub fn remove_node(&mut self, id: NodeId) -> Result<()>;
    pub fn connect(&mut self, conn: Connection) -> Result<()>;
    pub fn disconnect(&mut self, from: NodeId, from_pin: &str, to: NodeId, to_pin: &str) -> Result<()>;
    pub fn set_param(&mut self, id: NodeId, key: &str, value: Value, preview: bool) -> Result<()>;
    pub fn move_node(&mut self, id: NodeId, position: Vec2);
    pub fn undo(&mut self) -> bool;
    pub fn redo(&mut self) -> bool;
    pub fn snapshot(&self) -> Arc<Graph>;
    pub fn replace(&mut self, graph: Graph);
}
```

GraphController 协调：校验 → 编辑 → 版本管理。Engine 只做委派。

---

## ImageExecutor + ExecContext（engine/src/executors/image/）

### ExecContext

```rust
pub struct ExecContext<'a> {
    gpu: Option<&'a GpuExecutor>,
    cpu: &'a CpuExecutor,
}

impl ExecContext<'_> {
    pub fn gpu<T>(&self, f: impl FnOnce(&GpuExecutor) -> T) -> Result<T, NoGpuError>;
    pub fn cpu<T>(&self, f: impl FnOnce(&CpuExecutor) -> T) -> T;
}
```

gpu 是 Option——无 GPU 环境为 None，调用时返回错误。

### GpuExecutor

```rust
pub struct GpuExecutor {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipelines: Mutex<HashMap<String, wgpu::ComputePipeline>>,
}

impl GpuExecutor {
    pub fn run_shader(&self, shader_src: &str, inputs: &[&Image], params: &[u8]) -> Image;
}
```

Pipeline 同名 shader 只编译一次，后续复用。所有 shader 16x16 workgroup size。

### CpuExecutor

```rust
pub struct CpuExecutor;

impl CpuExecutor {
    pub fn load_image(&self, path: &str) -> Result<Image>;
    pub fn save_image(&self, image: &Image, path: &str) -> Result<()>;
}
```

M1 只需文件读写。M2 加 LUT 解析等。

### ImageExecutor

```rust
pub struct ImageExecutor {
    gpu: Option<GpuExecutor>,
    cpu: CpuExecutor,
}

impl ImageExecutor {
    pub fn context(&self) -> ExecContext<'_>;
}
```

---

## Engine（engine/src/lib.rs）

```rust
pub struct Engine {
    pub graph: GraphController,
    pub node_manager: Arc<NodeManager>,
    pub executor: ImageExecutor,
}

impl Engine {
    pub fn new(gpu: Option<GpuExecutor>) -> Self;

    pub async fn evaluate(&self, target: NodeId) -> Result<HashMap<NodeId, HashMap<String, Value>>> {
        let graph = self.graph.snapshot();
        let order = topo_sort(&graph, target)?;
        let ctx = self.executor.context();
        let mut results: HashMap<NodeId, HashMap<String, Value>> = HashMap::new();

        for node_id in order {
            let node = graph.nodes.get(&node_id).unwrap();
            let def = self.node_manager.get(&node.type_id).unwrap();
            let inputs = collect_inputs(&graph, node_id, &results);
            let merged = merge_params(def, &node.params);
            let mut all_inputs = inputs;
            all_inputs.extend(merged);
            let outputs = (def.execute)(ctx.clone(), all_inputs).await?;
            results.insert(node_id, outputs);
        }
        Ok(results)
    }
}
```

M1 无缓存、无增量、无脏标记。每次 evaluate 从头算。M2 加 Scheduler + Cache。

---

## 内置节点

4 个节点，一节点一目录，node! 宏定义：

| 节点 | 类型 | 执行方式 |
|------|------|---------|
| LoadImage | 纯 CPU | `ctx.cpu(\|cpu\| cpu.load_image(path))` |
| SaveImage | 纯 CPU | `ctx.cpu(\|cpu\| cpu.save_image(image, path))` |
| Brightness | 纯 GPU | `ctx.gpu(\|gpu\| gpu.run_shader(SHADER, ...))` |
| Contrast | 纯 GPU | `ctx.gpu(\|gpu\| gpu.run_shader(SHADER, ...))` |

shader.wgsl 从 git 历史复制。builtins/mod.rs 为空——inventory 自动收集，不需要 register_all()。

---

## 集成测试

engine/tests/integration.rs 覆盖：

| 测试场景 | 验证 |
|---------|------|
| 创建图 → 添加节点 → 连线 → 执行 → 验证输出 | 端到端流程 |
| 连线类型不兼容 → 拒绝 | 结构校验 |
| 连线成环 → 拒绝 | 环检测 |
| set_param(preview=true) 不入 undo 栈 | 版本管理 |
| 多次 undo/redo → 状态正确 | undo/redo |
| 无 GPU 环境执行纯 CPU 节点 → 成功 | 无 GPU fallback |
| 无 GPU 环境执行 GPU 节点 → 返回错误 | 错误处理 |
| node! 宏定义的节点被 NodeManager 收集到 | inventory 注册 |

---

## 目标文档同步（实现前完成）

以下目标文档需先同步本设计的决策，再开始写代码。

### 0.1.0 数据模型

1. **DataType**：将枚举列举改为开放类型说明
   - 删除 `DataType = image | mask | float | int | bool | color | string | handle`
   - 改为 `DataType(String)` 开放类型 + 内置关联函数
   - 说明兼容性检查走注册表
2. **Value::Image**：将 `Image | GpuTexture` 改为懒转换 Image
   - 说明 Image 内部持有 CPU + GPU 双缓存（OnceLock）
   - 说明按需转换、对节点透明
3. **Constraint**：已经是开放结构，确认和 DataType 一致的设计理由

### 0.2.0 文件架构

1. 顶层结构新增 `macros/` crate（proc-macro = true，放 node! 宏）
2. 说明 Rust 过程宏必须独立 crate 的语言限制

### 4.3.0 节点

1. **execute 签名**：标注为 async
   - `execute: async |ctx, inputs| { ... }`
   - 说明原因：M3 AI 节点天然异步，统一签名
2. **ExecuteFn 类型**：补充完整签名（Box<dyn Fn(ExecContext, HashMap) -> Pin<Box<Future>>>）
3. **process + gpu_process 合并**：删除双函数描述，统一为单一 execute + ExecContext

### 4.9.0 图像执行器

1. **格式转换**：删除独立的"格式转换"组件描述
   - 说明格式转换已移入 Image 类型内部（懒转换）
   - ExecContext 不再负责格式转换，节点看到的 Image 自动按需转换
