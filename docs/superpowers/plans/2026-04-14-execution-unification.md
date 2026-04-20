# 执行模型收口设计（Execution Unification Design）

> 目标：删除旧调度执行路径，统一为 `Session/CLI/AI → SchedulerFacade → Planner → Runtime → CapabilityRegistry → Executor`。

> 本文是**落地计划文档**，不是正式规范文档。若与 `docs/targetv2/` 中的正式规范、`architecture-invariants.md`（架构不变量）冲突，以后者为准；本文负责把规范翻译为可实施的迁移方案。

---

## 1. 结论

本专项只做一件事：**把“节点怎么被执行”收口为唯一真路径**。

这是一次**架构收口专项**，不是功能扩展计划。

本次设计明确：

- 删除旧调度器（`SchedulerManager`）主路径，不保留兼容壳
- 删除 `NodeDef.executor_type`
- 删除旧 `Auto/Manual` 模式
- `Engine` 降级为纯门面（Facade），不再自己持有执行循环
- GUI / CLI / AI 操作员首版共用同一执行内核
- 首版只把 `OneShot`（单次执行）彻底做对
- `Continuous`（连续执行）保留接口与错误占位，不进入首版主实现
- 自动预览属于 `Session / GUI` 触发策略，不属于 Runtime 执行模式
- 保留用户体验：运行整图、运行当前节点/子图、自动预览、导出

---

## 2. 问题陈述

当前仓库同时存在两套执行体系：

### 2.1 新路径（部分成形）

位于 `engine/src/lib.rs` 与 `engine/src/execution.rs`：

- 已有 `CapabilityRegistry` 路由
- 已有统一执行器契约（`Executor`）
- 已有 `ExecutionMode::OneShot / Continuous`
- 已有 `EvaluationFidelity`
- 已有 `CookingContext`
- 已有 plan 级生命周期钩子（`on_plan_started / on_plan_finished`）

### 2.2 旧路径（仍在主代码中）

位于 `engine/src/scheduler/*`：

- 旧 `SchedulerManager`
- 旧 `Auto / Manual` 模式
- 旧 `ExecutorRegistry`
- 旧 `node_runner` 按类型分发
- 旧 `ExecutorType` 参与执行决策

### 2.3 风险

双轨并存会导致：

1. 文档与代码真源不一致
2. 新节点无法判断该接哪条路径
3. 横切机制（`fidelity`、元数据继承、生命周期、缓存）没有唯一挂点
4. GUI / CLI / AI 三端难以共享一致行为
5. 后续功能扩展持续叠加技术债

---

## 3. 本次专项范围

### 3.1 纳入范围

- 执行入口统一
- 规划与执行职责切分
- 旧调度执行路径删除
- `NodeDef` 执行语义收口
- 统一 `ExecutionPlan` 作为 Runtime 输入
- 统一三端（GUI / CLI / AI）进入同一执行内核
- `OneShot` 主链正确性
- 契约测试与迁移验证

### 3.2 不纳入范围

- 新节点能力扩展
- 新 provider（提供方）接入
- `Continuous` 真正实现
- 完整 `Session` 层 UI 功能扩展
- `ffmpeg` 分发
- 完整产品发布流程

---

## 4. 设计原则

### 4.1 单一真路径

任何节点执行都必须经过：

`外层入口 → SchedulerFacade → Planner → Runtime → CapabilityRegistry → Executor`

禁止再存在第二条真实执行路径。

> 注：这里删除的是旧 `SchedulerManager` 与旧调度语义，不是删除“调度 facade（门面）”这一层概念。新系统仍保留薄的 `SchedulerFacade`。

### 4.2 触发策略与执行模式分离

- 自动预览 / 手动点击：是**触发策略**
- `OneShot / Continuous`：是**执行模式**

触发策略归 `Session / GUI / CLI` 外层；执行模式归 Runtime 内核。

### 4.3 Planner 纯函数，Runtime 唯一副作用中心

- Planner 只负责“该跑什么”
- Runtime 只负责“怎么跑”

### 4.4 节点执行语义只由 `requires` 表达

- 删除 `executor_type`
- 新旧节点一律靠 `NodeDef.requires` 参与路由
- 没有 `requires` 的节点视为非法定义，首版不再允许进入主执行链
- 现存 builtin（内建）节点必须一并改造到 Capability 路径，不允许 `def.execute` 旁路长期保留

### 4.5 用户体验优先保持不变

对用户保留：

- 自动预览
- 运行整图
- 运行当前节点/子图
- 导出

改变只发生在内部实现。

### 4.6 外部接口兼容，内部语义重构

本专项允许保留用户可见命令名、按钮语义、Facade 方法名；但这些入口的内部映射必须改为新执行路径。

### 4.7 三端名义共核必须落实为“不可旁路”

- CLI 不得再旁路到旧 `Engine::evaluate*` 路径
- GUI 不得保留旧调度专用捷径
- AI 操作员不得走专用执行捷径

“GUI / CLI / AI 共核”指三者共享同一个 `SchedulerFacade → Planner → Runtime` 执行主链，而不是只共享部分类型定义。

---

## 5. 目标架构

### 5.1 调用链

#### GUI / AI 路径

`SessionHandle → SchedulerFacade → Planner → Runtime`

#### CLI / Server 路径

`EngineFacade → SchedulerFacade（无 Session）→ Planner → Runtime`

#### Runtime 下游

`Runtime → CapabilityRegistry → Executor`

辅助依赖：

- `NodeManager`：NodeDef 查询
- `CacheManager`：正式 cache
- `ArtifactManager`：产物与回源
- `EventSystem`：事件广播

### 5.2 目标分层

#### Engine / EngineFacade

职责：

- 模块装配
- 暴露统一外部接口
- 生命周期管理

不再负责：

- 拓扑排序
- 输入解析
- 执行循环
- 路由执行器
- cache 读写编排

#### SchedulerFacade

职责：

- 接收外部执行请求
- 持有 `DirtyState`
- 调用 Planner 生成 `ExecutionPlan`
- 调用 Runtime 启动执行
- 接收 Runtime 终态事件并清理 `DirtyState`
- 在图变化时协调 cache 失效，再决定是否触发新的执行请求

不负责：

- 真正执行节点
- 维护旧执行器注册表
- 按类型决定执行器

#### Planner

职责：

- 纯函数规划
- 构造 `ExecutionPlan`
- 计算 `ExecSignature`
- 展开 `CookingContextRange`
- 推导 `DirtyState` 传播
- 把图中的输入来源前置编码为 `PinSource`

#### Runtime

职责：

- `single-flight` 全局守卫
- 消费 `ExecutionPlan`
- 查询/写入 cache
- 路由 Capability
- 调执行器
- 处理 lifecycle hooks
- 处理取消、进度、终态

不再回读 Graph 做现场依赖分析。

#### Session

本专项不完整实现 Session，但明确保留其边界：

- preview cache（预览缓存）归 Session
- 正式 cache 归 Runtime / CacheManager
- Runtime 不拥有 preview cache

---

## 6. 核心数据模型

### 6.1 外部请求

```rust
struct ExecuteRequest {
    target: ExecuteTarget,
    mode: ExecutionMode,
}

enum ExecuteTarget {
    Graph,
    Node(NodeId),
    Subgraph { root: NodeId },
}
```

说明：

- “运行整图”对应 `Graph`
- “运行当前节点/子图”对应 `Node/Subgraph`
- 首版最小必须交付 `Graph` 与 `Node`
- `Subgraph` 可在首版内部退化为“以节点为根求闭包”的实现方式
- 自动预览只是外层何时发这个请求，不是另一种模式

### 6.2 执行模式

```rust
enum ExecutionMode {
    OneShot { fidelity: EvaluationFidelity },
    Continuous { tick_source: TickSource, target_fps: f32 },
}
```

首版约束：

- `OneShot` 完整实现
- `Continuous` 进入 Runtime 后统一返回 `ContinuousModeNotImplemented`
- GUI 不暴露 `Continuous` 入口

### 6.3 Planner 产物

```rust
struct ExecutionPlan {
    subtasks: Vec<PlanSubtask>,
}

struct PlanSubtask {
    cooking_context: CookingContext,
    layers: Vec<Vec<PlannedNode>>,
}

struct PlannedNode {
    node_id: NodeId,
    type_id: String,
    params: HashMap<String, Value>,
    input_sources: HashMap<String, PinSource>,
    requires: Vec<CapabilityId>,
    exec_signature: ExecSignature,
    cache_hit_expected: bool,
}
```

约束：

- `ExecutionPlan` 自洽，不依赖 `Graph` 活体对象
- Runtime 不再自己现场分析图
- Runtime 只按 `input_sources` + 已执行上游结果 + 常量值取输入

### 6.4 Runtime 运行时对象

```rust
struct RunHandle {
    run_id: RunId,
    execution_id: ExecutionId,
    state: Arc<Mutex<RunState>>,
    cancel_token: Arc<dyn CancelToken>,
}
```

---

## 7. 关键行为定义

### 7.1 自动预览

自动预览不是 Runtime 模式。其语义是：

1. GUI / Session 观察到参数或连线变化
2. 外层根据交互策略决定需要预览
3. 外层发起一次：

```rust
ExecuteRequest {
    target,
    mode: ExecutionMode::OneShot {
        fidelity: EvaluationFidelity::Preview { ... }
    }
}
```

因此用户体验不变，但内核概念更干净。

### 7.2 运行整图

运行按钮语义：

```rust
ExecuteRequest {
    target: ExecuteTarget::Graph,
    mode: ExecutionMode::OneShot { fidelity: Full },
}
```

### 7.3 运行当前节点/子图

```rust
ExecuteRequest {
    target: ExecuteTarget::Node(node_id),
    mode: ExecutionMode::OneShot { fidelity: Full 或 Preview },
}
```

### 7.4 导出

导出本质上也是一次 `OneShot Full`，区别在于目标子图中含有副作用节点（如 `save_video`）。

### 7.5 外部接口兼容策略

本专项后仍保留以下用户语义：

- GUI 的自动预览
- GUI 的运行整图按钮
- GUI 的运行当前节点/子图按钮
- CLI 的执行整图 / 执行目标节点能力
- AI 操作员对当前 Session 的执行调用

变化只在内部：这些入口统一映射到新的 `ExecuteRequest`。

### 7.6 图变化最小顺序约束

图变化（参数修改、连线修改、undo/redo、artifact 选用变化）进入新链路时，最小顺序为：

1. `GraphController` 提交变化
2. `SchedulerFacade` 获取变化后快照
3. `Planner` 推导新的 `DirtyState`
4. `SchedulerFacade` 协调 `CacheManager.invalidate_subgraph(...)`
5. 若外层触发策略要求自动预览，再发起新的 `OneShot Preview`

禁止出现“先执行后失效”的顺序倒置。

### 7.7 事件契约（最小集）

为了保证 GUI / CLI / AI 观察到一致状态，首版至少保留以下执行事件：

- `ExecutionAccepted`
- `ExecutionStarted`
- `ExecutionProgress`
- `ExecutionCancelled`
- `ExecutionFinished`
- `ExecutionFailed`
- `ExecutionPreempted`

若首版不做完整排队恢复，也必须通过事件把“被谁抢占、是否需要外层重提”表达清楚。

---

## 8. 模块改造设计

### 8.1 `engine/src/lib.rs`

#### 现状

- 持有图、节点管理器、能力注册表、cache
- 直接执行拓扑循环

#### 目标

- 降级为组装根与门面
- 只创建并持有：
  - `GraphController`
  - `NodeManager`
  - `CapabilityRegistry`
  - `CacheManager`
  - `Planner`
  - `Runtime`
  - `SchedulerFacade`

#### 删除/迁移

- `evaluate_order` → Runtime
- 生命周期绑定与执行循环 → Runtime
- 图分析/临时输入组装 → Planner

### 8.2 `engine/src/scheduler/manager.rs`

#### 处理方式

删除。

#### 替代物

新建或重写为 `SchedulerFacade`：

- 持有 `DirtyState`
- 读取 `GraphController` 快照
- 调 Planner
- 调 Runtime
- 广播事件

不再包含：

- 旧 `ExecutorRegistry`
- 旧 `node_runner`
- 旧执行状态机
- 旧 `Auto/Manual`

### 8.3 `engine/src/scheduler/planner/*`

#### 处理方式

保留目录中的可复用纯函数资产，但升级为 targetv2 正式 Planner。

#### 必做改造

- 删除对 `ExecutorType` 的依赖
- 删除对旧 `ExecutionMode::Auto/Manual` 的依赖
- 产物改为 targetv2 `ExecutionPlan`
- 输入来源统一为 `PinSource`
- `ExecSignature` 只读 `NodeDef.requires` 与 capability version
- Runtime 所需输入必须全部提前编码到 Plan 中

### 8.4 `engine/src/scheduler/runtime/*`

#### 处理方式

旧 runtime 目录中的类型分发、旧注册表与旧节点运行器整体删除或重写。

#### Runtime 正式职责

- 消费 `ExecutionPlan`
- 解析 `input_sources`
- 查询 `CapabilityRegistry`
- 构造 `NodeExecutionRequest`
- 处理 plan 生命周期
- 对正式 cache 的读写完全以 `ExecSignature + generation` 为准
- 只允许声明了副作用 Capability 的节点产生外部可见写入

### 8.5 `engine/src/node_manager/model/node_def.rs`

#### 处理方式

删除 `executor_type` 字段。

#### 新约束

- `requires` 为必填且非空
- 没有 `requires` 的节点不能注册进入主系统
- `def.execute` 不再是主执行入口；若保留，仅可作为迁移期编译辅助，不得进入运行时主链
- 本专项结束时，主系统中不得再依赖 `def.execute` 跑节点

### 8.6 节点注册

所有 `NodeDef` 注册点需要调整：

- 删除 `executor_type: ...`
- 保证 `requires` 完整
- 本地内建节点也必须通过 `Capability` 路径运行

---

## 9. 迁移策略

### 9.1 一次性切换原则

由于已明确：

- 不保留旧调度器兼容壳
- 不保留 `ExecutorType`

因此迁移策略采用**主干一次性切换**，而不是双写并存。

### 9.2 迁移步骤

#### 第 1 步：固定共享执行模型

- 统一 `execution.rs` 为共享执行真源
- 清理类型命名，避免旧/新 `ExecutionMode` 并存
- 对外旧 API 若省略 mode，短期兼容映射为 `OneShot { fidelity: Full }`

#### 第 2 步：实现正式 Runtime

- 从 `Engine` 中抽出执行循环
- 把 cache / lifecycle / capability 路由全部下沉

#### 第 3 步：实现正式 Planner

- 用 `ExecutionPlan` 替代现场拓扑执行
- 删去旧 `Auto/Manual` 相关规划逻辑

#### 第 4 步：建立 SchedulerFacade

- 承接 `DirtyState`
- 承接图变化通知
- 承接执行请求

#### 第 5 步：替换三端入口

- GUI / AI 走 `Session → SchedulerFacade`
- CLI 走 `EngineFacade → SchedulerFacade`

#### 第 6 步：删除旧路径

- 删除旧调度器
- 删除旧类型
- 删除旧分流测试

---

## 10. 运行时行为细化

### 10.1 Runtime 启动流程（OneShot）

1. 校验 mode 为 `OneShot`
2. 执行 `single-flight` 决策
3. 对 plan 中涉及的 executor 去重分组
4. 依次调用 `on_plan_started`
5. 逐 subtask、逐 layer、逐 node 执行
6. 每个节点先查正式 cache
7. 未命中则按 capability 路由执行
8. 写回 cache
9. 全 plan 结束后按终态调用 `on_plan_finished`
10. 发布结束事件

若某个 executor 的 `on_plan_started` 失败，则：

- 当前 run 直接进入 Error
- 所有已成功收到 `on_plan_started` 的 executor，仍必须收到 `on_plan_finished(Error)`

### 10.2 OneShot 的首版 single-flight 策略

为了不破坏当前自动预览体验，首版 OneShot 至少需要满足：

- 新 Preview 到来 + 当前 Preview 运行中 → 取消旧 Preview，启动新 Preview
- 新 Preview 到来 + 当前 Full 运行中 → 取消 Full，并向外层报告被抢占
- 新 Full 到来 + 当前 Preview 运行中 → 可拒绝或排队，但行为必须显式、一致
- 新 Full 到来 + 当前 Full 运行中 → 拒绝并返回明确错误

> 首版决策：允许**不自动恢复**被 Preview 抢占的 Full，但必须发出明确事件，由外层决定是否重提。

> 说明：若首版不做完整 pending full（挂起的完整执行）恢复，也必须至少保证自动预览不会因简单“已有运行则拒绝”而明显退化。

### 10.3 失败与取消

- 任一节点失败 → 当前 run 失败
- 取消信号到达 → 尽快退出并报告 `Cancelled`
- 即使失败/取消，也必须跑 `on_plan_finished`

### 10.4 Preview 与 Full

- Preview / Full 只通过 `EvaluationFidelity` 传入
- 执行器必须诚实报告 `fidelity_achieved`
- Runtime 根据 `fidelity_achieved` 决定 cache 写入级别
- Session preview cache 与 Runtime 正式 cache 的所有权不得混淆

### 10.5 Continuous

- 外部若传入 `Continuous`，Runtime 必须返回统一错误
- 不允许静默降级成 `OneShot`

---

## 11. 对用户体验的映射

| 用户动作 | 外层行为 | 内核请求 |
|---|---|---|
| 改参数后自动预览 | Session 自动触发 | `OneShot + Preview` |
| 点击运行整图 | 手动触发 | `OneShot + Full + Graph` |
| 点击运行当前节点/子图 | 手动触发 | `OneShot + Full/Preview + Node/Subgraph` |
| 点击导出 | 手动触发 | `OneShot + Full + 含 side-effect 节点的目标` |

因此本专项不改变用户表层工作流。

---

## 12. 验收标准

### 12.1 架构验收

- `Engine` 不再自己执行节点
- 系统中不存在第二条真实执行路径
- `NodeDef` 不再包含 `executor_type`
- 系统中不存在 `Auto/Manual` 执行模式
- 所有节点路由只依赖 `requires`

### 12.2 功能验收

- GUI 自动预览仍可工作
- 运行整图仍可工作
- 运行当前节点/子图仍可工作
- 图像生成、视频生成、保存视频现有链路行为不退化

### 12.3 测试验收

- Planner 纯函数测试
- Runtime capability 路由测试
- lifecycle hooks 调用测试
- Preview / Full fidelity 测试
- Graph / Node 两类 target 测试
- GUI / CLI / AI 共核集成测试
- 图变化 → dirty propagate → cache invalidate → preview 触发顺序测试
- 旧 public API 兼容映射测试

### 12.4 一次性切换守门条件

在删除旧路径前，新路径至少必须通过以下现有链路回归：

- API 图像生成
- API 视频生成
- 保存视频

未通过不得执行旧路径删除。

---

## 13. 风险与缓解

### 风险 1：回归面过大

缓解：

- 先补执行契约测试
- 迁移按 Runtime → Planner → Facade 顺序推进

### 风险 2：旧节点定义迁移成本高

缓解：

- 提前扫描所有 NodeDef
- 一次性补齐 `requires`
- 同步把 builtin（内建）节点纳入 Capability 路径

### 风险 3：用户体验被内部改造波及

缓解：

- 以“用户动作 → 内核请求”映射表作为回归基线
- 把自动预览响应性纳入回归验收

### 风险 4：`Continuous` 占位引发误用

缓解：

- 明确返回 `ContinuousModeNotImplemented`
- 不在 GUI 暴露入口

---

## 14. 本专项后的下一步

执行模型收口完成后，下一优先级建议为：

1. 视频/媒体元数据统一继承机制（如 `fps`、duration、frame_count）
2. `Session` 层正式化
3. `Continuous` 真实现
4. 外部依赖分发（如 `ffmpeg`）

---

## 15. 最终决策摘要

- **删旧调度器**：是
- **删 `ExecutorType`**：是
- **废弃 `Auto/Manual`**：是
- **自动预览归 Session / GUI**：是
- **保留运行整图 / 运行当前节点/子图**：是
- **GUI / CLI / AI 共用同一执行内核**：是
- **本轮只把 `OneShot` 做对**：是
- **`Engine` 降级为纯门面**：是
