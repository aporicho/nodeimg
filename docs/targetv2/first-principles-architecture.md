# 第一原理架构

> nodeimg 引擎的 8 层分层模型。受 `architecture-invariants.md` 管辖。

---

## §0 本文档的地位

本文档给出 **nodeimg 引擎的分层模型**——"每一层拥有什么、不拥有什么、谁是消费者"。

它回答的问题:

1. 当我要加一个功能时,这个功能属于哪一层?
2. 当我要修改一个字段时,它的所有权是谁?
3. 当我要新建一个模块时,它的上游和下游分别是什么?

它**不**回答:

- 每个模块的具体接口(见 `4.x.x` 模块文档)
- 每种数据模型的字段定义(见 `0.1.0`、`0.1.1`)
- 每个真源的决议(见 `architecture-invariants.md`)

---

## §1 八层分层图

```
Layer 7 · Interface Layer
         UI / CLI / Server(三个对等消费者)
                    ↓
Layer 6 · Project Layer
         容器 / 持久化边界 ── 与 Graph 正交
                    ↓
Layer 5 · Session Layer
         undo / preview / events / run handle 引用
                    ↓
Layer 4 · Runtime
         执行 / single-flight / cache / artifact / materialization
                    ↓
Layer 3 · Capability Layer
         Capability 注册表 / 路由 / 局部性策略
                    ↓
Layer 2 · Planner
         纯函数: (Graph, CookingContext, DirtyState) → ExecutionPlan
                    ↓
Layer 1 · Graph Layer
         Graph = (nodes, connections) 不可变值
                    ↓
Layer 0 · Algebra Layer
         DataType / Value / Purity / 节点 = 函数
```

**方向说明:**

- 下层 **不知道** 上层的存在(`Layer 0` 不知道有 `Runtime`,更不知道有 `Session`)。
- 上层 **使用** 下层的抽象,但**不能绕过**中间层。
- `Interface` 层可以直接穿过 `Session` 调到 `Runtime`——这是 CLI / Server 的合法路径,不是违规。

**Project 层的特殊性**: Project 画在 Layer 6,但它**与 Graph 正交**——Project 不在执行链路上,它是容纳 Graph 的外部边界。画在 6 层是为了表达"Project 依赖底层概念,但不依赖执行时序"。详见 §8。

---

## §2 Layer 0 · 代数层(Algebra)

**定义:** 纯类型、纯值、纯函数描述——系统的**元语言**。

**拥有:**

- `DataType` 的定义(`Atomic(T) | List(T)`)
- `AtomicType` 枚举
- `Value` 的抽象形态(不含实现细节)
- `Materializable` 协议的**接口** (`0.1.5-value-materialization.md`)
- "节点 = 函数 `(Inputs, Params) → Outputs`" 的语义
- `Purity` 的定义和二元值域

**不拥有:**

- 任何运行时状态
- 任何具体节点
- 任何执行细节
- `ImageValue` 的 CPU/GPU 双缓存(那是 Layer 3/4 的实现细节)

**消费者:** Layer 1(Graph)引用这里定义的类型;所有上层间接依赖。

**权威文档:** `0.1.0-models.md` §1(类型系统);`0.1.5-value-materialization.md`

---

## §3 Layer 1 · 图层(Graph)

**定义:** `Graph = (nodes, connections)`——不可变值,DAG,类型匹配,单输入。**只是数据结构,不是"会跑的图"**。

**拥有:**

- `Graph` / `Node` / `Connection` 的数据结构
- 图的静态约束校验(DAG、类型匹配、连接单入)
- 图的结构变更语义(增删节点、增删连线、改参数)
- 图的不可变/结构共享实现

**不拥有:**

- 布局信息(归 GUI)
- 脏状态(归 Layer 2 Planner)
- 执行状态(归 Layer 4 Runtime)
- 撤销重做栈(归 Layer 5 Session,逻辑上委托回到 GraphController)
- 选中态、视口等 UI 状态

**消费者:**

- `Layer 2 Planner` 读取 Graph 快照生成执行计划
- `Layer 6 Project` 保存 / 加载 Graph 到项目文件
- `Layer 5 Session` 管理 undo 栈时保留 Graph 历史版本
- `Layer 7 Interface` 的 UI 画布渲染 Graph

**权威文档:** `0.1.0-models.md` §3;`4.2.0-graph-controller.impl.md`

**关键非对称:** Graph 是不可变**值**,不是"引用"。每次编辑产生一个新 `Graph` 值。这让 undo / redo / diff / 快照都是纯函数操作。

---

## §4 Layer 2 · 规划层(Planner)

**定义:** **纯函数**: `(Graph, CookingContext, DirtyState, NodeDefQuery) → ExecutionPlan`。不持有任何副作用,不持有可变状态。

**拥有:**

- 依赖分析算法(拓扑排序、传递闭包)
- 增量失效**算法**(从 `DirtyState` 算出"必须重算的子图")——**算法**,不是状态
- `CookingContext` 枚举策略(按维度展开图的多次求值)
- `ExecSignature` 构造算法
- 脏状态传播**算法**(`propagate_dirty` 是纯函数)
- 执行计划(`ExecutionPlan`)生成

**不拥有:**

- **`DirtyState`**(归 Scheduler facade)—— Planner 纯函数不持有状态,只接受 `&DirtyState` 做只读输入
- 实际执行(归 Layer 4 Runtime)
- 缓存读写(归 Layer 4 Runtime)
- 取消、超时、进度(归 Layer 4 Runtime)
- 节点定义注册(归 NodeManager,Planner 通过 `NodeDefQuery` trait 只读查询)
- **Capability 路由表**(归 Layer 3 CapabilityRegistry;但 Planner **不直接依赖** Layer 3——`capability_version` 由 NodeManager 启动时快照到 NodeDef,Planner 从 NodeManager 读)

**消费者:**

- `Layer 4 Runtime` 接收 `ExecutionPlan` 并执行

**关键性质: Planner 是纯函数**

- 给同样的 `(Graph, CookingContext, DirtyState)`,永远产出同样的 `ExecutionPlan`
- Planner 没有 `&mut self`(除了内部 memoization)
- Planner 可以被**测试**为纯函数: 给固定输入,断言固定输出
- Planner 可以被 **CLI、Server、UI** 任意调用,不影响其他调用方

**权威文档:** `4.1.1-planner.md`

---

## §5 Layer 3 · 能力层(Capability)

**定义:** Capability 注册表 + 路由表 + 局部性策略。决定"哪个执行器会被选中来处理某个 Capability 请求"。

**拥有:**

- `CapabilityId` 枚举(开放字符串,首版少量固定值)
- `Capability` 结构定义
- 注册表 `(CapabilityId → Vec<Arc<dyn Executor>>)`
- 路由规则(含局部性、预算、优先级)
- 执行器健康检查(首版可选)

**不拥有:**

- 实际执行(归 Runtime,Layer 4)
- 节点定义(归 NodeManager)
- 缓存(归 Runtime)

**消费者:**

- `Layer 4 Runtime` 按 `NodeDef.requires: Vec<CapabilityId>` 查路由表

**为什么独立为一层:**

- 首版的路由决策可以很简单("Capability → 唯一执行器")
- 但**把它独立为一层**意味着:未来加 `VideoExecutor` / `VectorExecutor` / 多执行器竞争 / 局部性优化 时,**只改这一层**
- 如果路由逻辑散在 Runtime 或 Scheduler 里,加一种执行器都要改核心分发代码——那就是"动核心"

**权威文档:** `0.1.4-capability.md`;`4.1.3-capability-registry.md`

---

## §6 Layer 4 · 运行时(Runtime)

**定义:** 执行副作用的唯一所在地。持有 `single-flight` 全局守卫、cache 读写、artifact 写入、进度事件发送、取消令牌、Materialization 调度。

**拥有:**

- **`single-flight` 全局守卫(真源 ①/⑥)**
- 正式 `CacheManager` 的写路径
- `ArtifactManager` 的写路径
- 执行器调度(拿着 Capability 查表)
- 取消令牌 / 进度 sink
- `Value` 的 Materialization 调度(决定什么时候把 `Value` 物化到 CPU / GPU)
- `ExecutionRequest.fidelity` 的语义落实(preview / full)

**不拥有:**

- 规划(归 Layer 2 Planner,纯函数)
- 节点定义(归 NodeManager,只读引用)
- Capability 路由表(归 Layer 3 CapabilityRegistry,只读查询)
- preview 热路径缓存(归 Layer 5 Session)
- undo 栈(归 Layer 5 Session)

**消费者:**

- `Layer 5 Session` 调用 Runtime 启动交互式执行
- `Layer 7 Interface`(CLI / Server)可以**直接**调用 Runtime,不经过 Session

**核心不变量: single-flight**

- Runtime 持有一个 `Option<RunHandle>`
- 任何调用方(Session、CLI、Server)想启动执行,都要先调 Runtime 的 `try_start_run()`
- 返回 `Err(RunAlreadyInFlight)` 时调用方必须等待或取消
- Session 可以持有 `RunHandle` 的**引用**,但**不能直接写**守卫状态

**关键性质: Runtime 无会话**

- Runtime 不知道"当前选中哪个节点"
- Runtime 不知道"当前是 GUI 还是 CLI 在调用"
- Runtime 只接收 `ExecutionPlan` 和 `fidelity` 等显式参数
- 这让 Runtime 可以被**任何**前端复用

**权威文档:** `4.1.2-runtime.md`

---

## §7 Layer 5 · 会话层(Session)

**定义:** 一次"打开 Project"或"新建 scratch Graph"的生存期。持有交互性状态,是 UI 交互和 Runtime 之间的翻译层。

**生存期:**

- 创建于 "打开 Project" 或 "新建 scratch Graph"
- 销毁于 "关闭 Project" 或 "退出 App"
- **不跨进程、不进项目文件**
- **CLI / Server 可以不创建 Session**

**拥有:**

- undo / redo 栈(可以委托给 GraphController 的内部实现,但生存期归 Session 管)
- **preview 热路径缓存**(与正式 cache 分离,允许低精度 / 低分辨率)
- 事件总线订阅者注册
- 当前选中项、焦点节点、脏标记前缘(UI 状态)
- 当前交互 `RunHandle` 的**引用**(不是权限)

**不拥有:**

- **single-flight 权限**(归 Runtime)
- 正式 `cache`(归 Runtime)
- Artifact 历史(归 Project)
- 节点定义(归 NodeManager)
- Graph 数据结构本体(归 GraphController,Session 引用)

**与 Scheduler 的分工:**

- Runtime **无会话**: 接受 `ExecutionRequest` 并返回 `EngineEvent` 流;不知道"当前选中节点"
- Session 负责 **把交互意图翻译成 ExecutionRequest**: 用户改参数 → Session 决定 "需要刷新 preview 节点 Y" → 构造请求
- 这个分工让 Runtime 保持纯粹(CLI / Server / 测试可直接复用),把"交互性"隔离在 Session 层

**preview 路径特权:**

- Session 的 preview 路径可以请求低精度版本
- 可以跳过某些持久化 artifact 写入
- 使用独立 LRU 缓存(不污染正式 cache)
- 正式执行路径(export / 批量生产)不经过 Session 的 preview 路径

**权威文档:** `4.12.0-session.md`

---

## §8 Layer 6 · 容器层(Project)

**定义:** 项目文件、资产库、artifact 历史、模板——**用户看到的文件**。

**关键非对称: Project 与 Graph 正交**

- Project 不是 Graph 的上位层
- Project 不参与执行语义(不进 Planner、Runtime、Cache)
- 一个 Project 可以包含 0 或多个 Graph
- **一个 Graph 可以不属于任何 Project**(CLI scratch、单元测试、临时求值)
- 这最后一条是正交性的**试金石**: 如果 "无 Project 的 Graph" 在架构里不能存在,说明 Project 实际上还是上位层

**拥有:**

- Project 文件生命周期(新建 / 打开 / 保存 / 关闭)
- Asset Library(跨项目服务 / 未来)
- Artifact 历史与选用索引
- 模板库
- 项目级设置(GPU 预算、backend 偏好)

**不拥有:**

- Graph 本体(归 GraphController;Project 通过引用容纳)
- 执行语义(任何)
- Session 状态(Session 生存期与 Project 生存期**重合**但归属不同)

**消费者:**

- `Layer 7 Interface` 的打开 / 保存 / 选用 artifact 操作
- `Layer 5 Session` 在生存期管理时引用当前 Project

**权威文档:** `4.7.0-project-manager.impl.md`;`5.0.0-project-file.draft.md`

---

## §9 Layer 7 · 接口层(Interface)

**定义:** UI / CLI / Server 三个对等消费者。

**关键非对称: 三个前端地位对等**

- UI 不比 CLI 特殊
- UI 的"特权"(preview 热路径、undo、事件订阅)通过 **Session 层**获得,**不通过绕过 Runtime** 获得
- CLI / Server 可以**直接**调用 Runtime(跳过 Session),这是合法路径

**UI 路径:**

```
UI → Session → Runtime → ExecutorPool → (CacheManager, ArtifactManager)
     ↑                   ↓
     └──── EventBus ◀────┘
```

**CLI 路径:**

```
CLI → Runtime → ExecutorPool → (CacheManager, ArtifactManager)
       ↓
     stdout (事件序列化)
```

**CLI 的 GraphController 构造:** CLI 通过 `GraphController::new_without_undo()` 构造,**不**维护 undo 栈——CLI 没有交互式撤销需求。如果 CLI 调用 `graph_controller.undo()`,返回 `Err(UndoDisabled)`。这样 CLI 路径既没有 Session,也没有 undo 栈(两者是分开的决定,但共同保证 CLI 的轻量)。

**Server 路径:**

```
Server → Runtime → ExecutorPool → (CacheManager, ArtifactManager)
          ↓
        HTTP Response (事件流)
```

**为什么这样设计:**

- 如果 CLI 被迫创建一个 "假 Session" 才能跑,就说明 Session 是执行的必经路径——那它就不该是 Layer 5,它就该是 Layer 4 的一部分
- 现在的分层让 "CLI 不需要 Session" 成为试金石,Session 因此是**真正的**交互层

**权威文档:** `2.0.0-gui.md`(UI);`3.0.0-cli.future.md`(CLI);Server 未定

---

## §10 依赖方向的严格规则

**允许:**

- Layer N 读 / 调用 Layer N-1、N-2、...、0
- Layer 7 可以跨层调用(UI → Runtime 经 Session;CLI → Runtime 直接)
- Layer 6(Project)可以跨层调用 Layer 1(Graph)——Project 容纳 Graph

**禁止:**

- Layer N 知道 Layer N+1 的存在
- Layer 2(Planner)持有任何副作用
- Layer 4(Runtime)持有 UI 状态或会话状态
- Layer 5(Session)持有 single-flight 权限
- **Layer 2(Planner)直接依赖 Layer 3(CapabilityRegistry)** —— Planner 必须经由 NodeManager 间接读 Capability 版本

**注意跨层跳跃是合法的:** Layer 7 可以直接调 Layer 4,不需要穿过 5、6。这是接口层与业务层的天然解耦。

## §10.5 跨层服务(Cross-cutting Services)

有几个模块**不在 8 层的纵向里**——它们是横跨多层的服务,不参与方向约束:

| 跨层服务 | 被谁调用 | 特性 |
|---------|---------|------|
| **NodeManager** | Planner (L2) / Runtime (L4) / GraphController (L1) / Session (L5) | 节点定义查询 + 启动期快照 |
| **EventSystem** | Runtime (L4) / Planner (L2) / Session (L5) / Scheduler facade | 事件发布 + 订阅 |
| **ProjectFile I/O** | Project (L6) / (特殊情况下)Session | 文件读写(同进程但隔离) |

**跨层服务的特征:**

- 它们是**只读或者受限可写**的
- 它们不参与"方向规则"——任何层都可以引用
- 它们是"查询者",不是"执行者"
- 新增一个跨层服务需要明确声明它的**只读契约**,避免变相违反层方向

**这解决了"Planner 依赖 CapabilityRegistry"的分层违反:**

- 原设计: Planner (L2) → CapabilityRegistry (L3) ❌ 逆向
- 新设计: Planner (L2) → NodeManager (跨层) ← CapabilityRegistry (L3) ✓
- NodeManager 在启动时从 CapabilityRegistry 快照 `capability_version`,之后 Planner 只读 NodeManager

**冻结决议:** NodeManager 不属于 8 层纵向结构——它是跨层服务。这条首次在 targetv2 明文化。

---

## §11 分层所有权交叉验证表

| 状态 | 所有者 |
|------|-------|
| `DataType` 枚举 | Layer 0 |
| `Value` 抽象 | Layer 0 |
| `Materializable` 协议 | Layer 0 定义,Layer 4 实现 |
| `Graph` / `Node` / `Connection` | Layer 1 GraphController |
| DAG 校验 | Layer 1 GraphController |
| **`DirtyState`** | **Scheduler facade**(不是 Planner!Planner 是纯函数) |
| 脏状态推导算法 | Layer 2 Planner(纯函数) |
| `ExecSignature` | Layer 2 Planner 构造,Layer 4 Runtime 使用 |
| `CookingContext` 枚举策略 | Layer 2 Planner |
| `ExecutionPlan`(不含 run_id / generation) | Layer 2 Planner 产出,Layer 4 Runtime 消费 |
| `Capability` 注册表 | Layer 3 CapabilityRegistry |
| Capability 路由决策 | Layer 3 CapabilityRegistry |
| `NodeDef` / NodeManager | 跨层服务 |
| `EventSystem` | 跨层服务 |
| **single-flight 守卫** | **Layer 4 Runtime** |
| `run_id` / `generation` | Layer 4 Runtime(try_start_run 时生成) |
| 正式 Cache 读写 | Layer 4 Runtime |
| Artifact 写入 | Layer 4 Runtime |
| Materialization 调度 | Layer 4 Runtime |
| 取消令牌 / 进度 sink | Layer 4 Runtime |
| undo / redo 栈(生存期) | Layer 5 Session(逻辑实现委托 GraphController);**CLI 模式下 GraphController 构造为 no-undo,不维护栈** |
| **preview 热路径 cache** | **Layer 5 Session** |
| 事件订阅者(GUI 路径) | Layer 5 Session |
| 事件订阅者(CLI / Server 路径) | EventSystem(跨层服务) |
| 当前 RunHandle 引用 | Layer 5 Session(Weak,不是权限) |
| UI 选中态、焦点 | Layer 5 Session(引擎侧) / Layer 7(GUI 细节) |
| Project 文件生命周期 | Layer 6 Project |
| Asset Library | Layer 6 Project |
| Artifact 历史 | Layer 6 Project |
| GUI 布局、视口 | Layer 7 GUI(不属于引擎) |

---

## §12 常见误判对照表

| 错误直觉 | 正确归属 | 原因 |
|---------|---------|------|
| "Scheduler 是核心层" | Scheduler 是 **facade**,核心是 Layer 2 Planner + Layer 4 Runtime 的拆分 | Scheduler 在 target 时代混合了规划与执行;targetv2 拆开 |
| "Project 是 Graph 的上位层" | 正交,不是上下 | Project 不参与执行;"无 Project 的 Graph" 必须合法 |
| "Session 就是 UI 状态" | Session 是交互层的**服务**,可以被任何前端消费——但 CLI / Server 通常不用 | UI 依赖 Session 的 preview / undo / events;CLI 通常直接走 Runtime |
| "single-flight 由 Session 持有" | 由 **Runtime** 持有 | CLI / Server 不经 Session,如果 single-flight 归 Session,CLI / Server 就没有这条不变量 |
| "Capability 是长期目标,首版用 executor_type" | 首版就该是 **Capability 路由**,executor_type 是 Capability 的粗粒度分组 | 如果路由按 executor_type 枚举 switch,加一种执行器要改核心分发——那就是"动核心" |
| "CookingContext 等有需要再加" | 首版就该有,哪怕维度为空 | 如果 ExecSignature 公式不包含 context hash,未来加入时全部签名失效,缓存清零 |

---

## §13 与其他文档的关系

- `architecture-invariants.md` §1 四层心智模型 —— 本文档是"派生层"的骨架
- `0.1.0-models.md` —— Layer 0、Layer 1 的数据定义
- `0.1.1-execution-models.md` —— Layer 2、Layer 4 的共享模型
- `0.1.2-interaction-loop.md` —— Layer 5 Session 的 preview 机制依据
- `0.1.3-cooking-context.md` —— Layer 2 Planner 的 context 枚举规则
- `0.1.4-capability.md` —— Layer 3 的注册表规范
- `0.1.5-value-materialization.md` —— Layer 0/Layer 4 的物化协议
- `4.0.0-engine.md` —— 本文档的组件视图
- `4.12.0-session.md` —— Layer 5 的模块级文档
