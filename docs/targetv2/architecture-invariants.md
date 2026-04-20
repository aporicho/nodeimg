# 架构不变量

> 所有 targetv2 文档的前置真源清单。任何引擎接口变更必须能追溯到本文档的某一条真源,否则视为真源清单不完整,应先补真源再改接口。

---

## §0 本文档的地位

本文档列出 nodeimg 引擎的"最底层决议"——它们不是任何单个模块的设计,而是所有模块设计的**前置条件**。

**地位:**

- 是 `0.x.x`、`4.x.x` 所有文档的前置引用。
- 若某条真源发生变更,所有引用它的下游文档必须同步更新。
- 若某个接口变更无法追溯到本文档的某一条真源,说明真源清单不完整,应先补真源再改接口——**不允许绕过**。

**不是什么:**

- 不是模块设计文档,不承载任何具体接口或字段定义。
- 不是 changelog,不记录增量;它记录的是"一旦定下就不应轻易动"的决议。
- 不替代 `0.0.2-module-contract-template.md` 的模块契约模板;两者互补——模板约束"单模块如何写",真源约束"所有模块的共同前提"。

**与 target 的差异:**

- target 是 7 条真源;targetv2 是 **8 条**——新增 真源 ⑧ · 交互回路
- 真源 ①~⑦ 保留,内容仅做细化,不改语义

---

## §1 四层心智模型

```
┌─────────────────────────────────────────────┐
│ 第 -1 层 · 公理层                            │
│   A. 数据公理  DataType 闭集 + Value 身份    │
│   B. 计算公理  节点 = (Inputs,Params)→Outputs│
│                分纯 / 非纯                    │
│   C. 组合公理  Graph 有向无环、类型匹配、     │
│                不可变值类型                   │
│   D. 交互公理  引擎对"参数变更 → 结果刷新"    │
│                路径负有结构责任               │
└─────────────────────────────────────────────┘
                    ↓ 决定
┌─────────────────────────────────────────────┐
│ 第  0 层 · 真源层(8 条,见 §2)              │
│   ① 责任边界   ② 抽象模式   ③ 数据模型      │
│   ④ 图模型     ⑤ 节点模型   ⑥ 执行模型     │
│   ⑦ 持久化边界 ⑧ 交互回路                   │
└─────────────────────────────────────────────┘
                    ↓ 派生
┌─────────────────────────────────────────────┐
│ 第  1 层 · 派生层                            │
│   NodeDef / Graph / ExecutionRequest /      │
│   ExecutionOutputs / EngineEvent /          │
│   ExecSignature / CacheKey / CookingContext/│
│   Capability / Materialization / Session /  │
│   EvaluationFidelity / 各模块接口            │
└─────────────────────────────────────────────┘
```

**公理层**回答"系统代表什么",**真源层**回答"系统谁负责什么",**派生层**回答"接口长什么样"。三层的责任不同,**不允许混淆**。

公理层一旦更改,真源层和派生层必须同步重审;真源层一旦更改,派生层必须同步重审;派生层变更不回溯影响上层。

**公理 D(交互公理)是 targetv2 新增**: 它承认"对人表现如何"和"内部自洽"一样是引擎的第一性问题。没有它,真源 ⑧ 就是悬空的章节。

---

## §2 八条真源

每条真源按统一模板描述:

```
① 定义         一句话
② 状态         已冻结 / 部分冻结 / 待确认
③ 权威文档     真正装载细节的 .md 路径
④ 连锁影响     改这条会推倒的模块
⑤ 反证/风险    不冻结会出什么事
⑥ 变更门槛     是否允许普通 PR、必须同步哪些文档
```

---

### 真源 ① · 责任边界

| 字段 | 内容 |
|------|------|
| **定义** | 引擎内外、各子模块之间的状态所有权归属。谁拥有状态,谁负责变更。 |
| **状态** | 已冻结 |
| **权威文档** | `0.0.2-module-contract-template.md` §全局设计约束;`first-principles-architecture.md` §分层所有权清单;`4.x.x` 各模块文档的"职责边界"章节 |
| **连锁影响** | GraphController、Planner、Runtime、Scheduler、各 Executor、ProjectManager、ArtifactManager、Session 的接口表 |
| **反证/风险** | **历史教训**:`move_node` 反复返工的根因不是代码质量问题,而是"布局归不归引擎管"一直没写死。责任边界模糊 → 同一行为在不同模块里各实现一遍 → 接口反复重写。 |
| **变更门槛** | 必须先改本文档 + `0.0.2` + `first-principles-architecture.md`;普通 PR 不得直接变更责任归属。新增模块时必须显式声明其责任边界。 |

**当前已冻结的责任决议:**

1. 布局不属于引擎(归 GUI)。
2. Planner 是纯函数,不持有副作用;Runtime 持有副作用。
3. 执行器完成计算并返回 `Value`,**不执行任何持久化 I/O**。是否将输出持久化为 `Artifact` 由 Runtime 依据 `ArtifactPolicy` 判定,`ArtifactManager` 负责实际写入。执行器无落盘权限。
4. Provider 属于 APIExecutor,不是独立模块;但其 `param_schema()` 能力作为公共只读接口,允许 NodeManager 间接查询。
5. **`single-flight` 全局守卫由 Runtime(Layer 4)持有**;Session / CLI / Server 等调用方只能申请启动,不能直接变更状态。(targetv2 新增)
6. **Session(Layer 5)拥有 undo 栈、preview 热路径缓存、事件订阅者注册**;不拥有 single-flight 权限、不拥有正式 cache、**不持有 RunHandle 引用**(查询当前 run 走 `scheduler.query_state()`)。(targetv2 新增)
7. **Project(Layer 6)与 Graph 正交**:Project 是容器 / 持久化边界,不参与执行语义。"无 Project 的 Graph"必须能存在。(targetv2 新增)
8. **ArtifactManager 拥有用户可见候选历史和当前版本选择**。Runtime 只负责按 `ArtifactPolicy` 把正式完成的结果登记进去;Session / GUI 只发起浏览、选用、标记、清理等交互,不直接拥有正式历史。
9. **显式产物管理节点是普通图节点**。它不是 GUI 侧面板的隐式状态;API 节点原生历史与显式产物管理节点必须共享同一套 Artifact / History 抽象。

---

### 真源 ② · 抽象模式

| 字段 | 内容 |
|------|------|
| **定义** | 跨模块交互的固定机制:单一 trait、单一广播通道、静态注册、单一扩展入口。 |
| **状态** | 已冻结 |
| **权威文档** | `0.0.2-module-contract-template.md` §全局设计约束;`4.11.2-executor-contract.md`;`4.8.0-events.impl.md`、`4.8.1-engine-event.impl.md`;`extensibility.md` |
| **连锁影响** | 所有 Executor、Provider、Event subscriber、NodeManager 注册流程 |
| **反证/风险** | 如果 Executor 允许多 trait 并行,Runtime 的路由和 health 检查将无法统一;如果事件允许多通道,GUI 和 CLI 的同步保障会产生竞态。 |
| **变更门槛** | 必须先改本文档 + 至少一份权威文档;涉及 trait 分裂或通道分裂的变更需要全组评估。 |

**当前已冻结的抽象模式:**

1. 所有执行器实现同一 `Executor` trait。
2. 所有事件经由 `EventSystem` 单点 `publish(event)`。
3. Provider 启动时静态注册,不支持运行时热插拔。
4. **所有节点来源统一经 `NodeRegistry` façade 注册**,内部按五条变化轴路由。对外只有一个扩展入口。(targetv2 明文化)
5. **执行器路由的主键是 `Capability`**,不是 `executor_type`。`executor_type` 可作为 Capability 的粗粒度分组,但不是权威分类。(targetv2 新增)
6. **执行器接口含 `on_plan_started` / `on_plan_finished` plan-level lifecycle hooks**(targetv2 M5 新增)。两个方法 default no-op,执行器可以 override 来管理跨 execute() 的资源(如视频 encoder 的 finalize、模型权重的 warm-up/release)。
7. **执行器允许持有跨 execute() 的内部状态,包括持久化到磁盘的 cache**(targetv2 M6 新增)。这与"执行器无落盘权限"(真源 ① §3)**不冲突**——前者是"内部资源管理"(decoder pool / 模型权重 / API 调用结果 cache),后者禁止"输出 ExecutionOutputs 落盘为外部 Artifact"。两者的判据是: **数据的所有权在执行器内部还是流出执行器边界**。详见 `4.11.2-executor-contract.md` §长寿命内部状态。
8. **节点原生历史与显式产物管理节点共享 Artifact 抽象**。差异只在入口形态:某些 API 节点可原生展示历史,Python 底层工作流主要通过显式产物管理节点暴露历史;底层版本、选中、标记、清理语义必须一致。

---

### 真源 ③ · 数据模型

| 字段 | 内容 |
|------|------|
| **定义** | `DataType` 是**闭集枚举**;`Value` 本体**不拥有**稳定的全局身份;`Value` 是**可物化句柄 + 物化协议**,不是"携带表示的值"。 |
| **状态** | 已冻结(targetv2 新增物化协议子项) |
| **权威文档** | `0.1.0-models.md`;`0.1.5-value-materialization.md` |
| **连锁影响** | `Value → PinDef → NodeDef → Connection 类型匹配 → ExecSignature → Cache → Artifact → 所有 Executor` 整条下游链;Runtime 的局部性调度 |
| **反证/风险** | **最高风险**:`Value::Image` 可同时是 CPU bitmap 或 GPU texture。若硬要对 `Value` 本体做 hash,只有两种选择:hash 字节 → GPU 回读性能灾难;hash handle → 相同内容的两次独立产出 hash 不等,缓存命中率崩塌。唯一的出路是**拒绝**给 `Value` 定义全局 hash,由签名系统(真源 ⑥)间接表达身份;物化细节由 Runtime 通过 Materialization 协议调度。 |
| **变更门槛** | 任何新增 `DataType` 或更改 `Value` 的 hash / equal 语义均为破坏性变更,须显式 changelog + 引擎版本升级。Materialization 协议接口变更同样为破坏性。 |

**已冻结子项:**

- `DataType` 是封闭枚举,新增走破坏性流程。
- `Value` **本体不提供全局可比的 hash 或 equality**。身份由**其来源节点的执行签名**间接表达。
- **`Value` 是 opaque handle**,通过 `Materializable` 协议按需物化到 `MaterializationTarget` 之一(见 `0.1.5-value-materialization.md`)。节点 schema / Connection 不得引用 `MaterializationTarget`。(targetv2 新增)
- **`AtomicType` 首版清单足够宽**,包含 `Video`、`Audio`、`VectorGraphic`、`LayoutDocument`、`BoundingBox`、`Keypoint`、`RemoteRef` 等占位类型,避免未来加类型触发破坏性变更。占位类型可标为"无执行器支持",但类型系统中必须存在。(targetv2 新增)
- **`ImageValue` 从首版就是多通道 + 元数据容器**,不是固定 RGBA。`channels: BTreeMap<ChannelId, ChannelData>` 支持任意通道集合(首版启用 `R/G/B/A`,其他通道保留占位);`metadata: ImageMetadata` 支持色彩空间、时间戳、帧号等(首版启用 `color_space`)。`ChannelId` 是**开放字符串**,加新通道是"填槽"。(targetv2 新增)
- **`List<T>` 是值的集合容器,不是调度维度**。调度维度由 `CookingContext` 承担(见真源 ⑥ 和 `0.1.3-cooking-context.md`)。`List<Image>` 在首版为**过渡白名单**。(targetv2 新增)

---

### 真源 ④ · 图模型

| 字段 | 内容 |
|------|------|
| **定义** | `Graph = (nodes, connections)`,不可变值类型,不含任何布局信息,满足有向无环 / 类型匹配 / 单输入的连接约束。 |
| **状态** | 已冻结 |
| **权威文档** | `0.1.0-models.md`;`4.2.0-graph-controller.impl.md` |
| **连锁影响** | GraphController、Planner、ProjectManager、项目文件 schema、undo 栈、Planner 的脏标记传播 |
| **反证/风险** | **历史教训**:`move_node` 问题反复出现,根因就是布局是否入图一直摇摆。一旦入图,图事件会因为纯视觉操作被触发,脏标记逻辑失真。 |
| **变更门槛** | 必须先改本文档 + `0.1.0` + `4.2.0`;特别禁止"临时加一个布局字段到 Graph 里"这类局部让步。 |

**已冻结子项(含 targetv2 细化):**

- `Graph` 是 Layer 1 的纯数据结构,不持有运行时状态。
- 脏状态、增量失效、执行计划都属于 **Layer 2 Planner** 的职责,不属于 Graph 本身。
- Graph 可以**脱离 Project 独立存在**——CLI scratch、单元测试、临时求值是合法路径(真源 ①)。

---

### 真源 ⑤ · 节点模型

| 字段 | 内容 |
|------|------|
| **定义** | `NodeDef` 是瘦核心;`purity`(纯 / 非纯)是 `NodeDef` 的独立字段,**不绑定在 `executor_type` 上**;动态参数由 Provider 通过 `param_schema()` 提供,不硬编码进 `NodeDef`;**节点声明需要的 Capability 集合**,由 Runtime 按 Capability 路由到执行器。 |
| **状态** | **已冻结**(targetv2 新增 Capability 字段) |
| **权威文档** | `0.1.0-models.md`;`0.1.4-capability.md`;`4.4.0-node-manager.impl.md` |
| **连锁影响** | 所有 Executor、Planner 路由、Cache 签名决策、Auto 模式触发策略、ArtifactPolicy、CapabilityRegistry |
| **反证/风险** | 如果 `purity` 保持绑定在 `executor_type`,未来出现"本地确定性 AI 节点"或"随机噪声 Image 节点"时,既有规则无法表达,必须回头拆分。如果路由按 `executor_type` 枚举 switch,加新执行器类别时要改核心分发逻辑。 |
| **变更门槛** | `NodeDef` 静态字段新增可走普通 PR(附 `0.1.0` 同步更新);但涉及 `purity`、`executor_type`、`requires: Vec<CapabilityId>`、`param_schema()` 语义的变更必须先改本文档。 |

**已冻结子项:**

- **`purity` 的值域固定为二元**:`Pure | Impure`。以二元枚举形式写死,不得留作"开放 enum 待扩展"或"字符串标签"。
- **`purity` 判定源 = `NodeDef` 必填字段,由节点作者手动标注**。引擎**不推断**、**不按 `executor_type` 给默认值**、**不读执行历史反推**。
- **`ArtifactPolicy` 默认行为**:`purity = Impure` 默认登记为 `Artifact`;`purity = Pure` 默认不登记。导出型节点不靠 `purity` 判定。
- **`NodeDef.requires: Vec<CapabilityId>` 是路由主键**(targetv2 新增)。Runtime 按此字段查 `CapabilityRegistry`,选择合适的执行器。`executor_type` 字段保留作为 Capability 的粗粒度分组,但不是权威分类。
- **三类执行来源平权**:图形处理节点、可控 Python 后端节点、云端 API 节点都是普通节点来源,统一通过 `NodeRegistry` + `CapabilityRegistry` 进入图语义。节点来源不改变 Graph / Planner / Runtime 的执行模型。
- **Python 节点与 API 节点层级固定**:Python 节点首版是可控后端的底层工作流节点;API 节点首版是黑盒任务节点,输出接口稳定、少而关键。若同种能力可由不同后端执行,用户层面倾向显式区分节点来源,不是自动路由。
- **`ParamDef.default_mode: ParamValueMode`(targetv2 新增):** 参数从静态 `Value` 升级为 `Constant | Animated(Curve)`。表达式不进首版模型。`Node.params` 类型同步从 `HashMap<String, Value>` 改为 `HashMap<String, ParamValueMode>`。Planner 在构造 ExecSignature 时**先按当前 CookingContext 采样动画参数**,再 hash——避免"同 curve 在不同 frame 签名相同"的静默数据损坏。
- **`cooking_sensitivity` 的实例级自动推导:** Planner 计算 effective_sensitivity = NodeDef 静态声明 ∪ 节点实例的 Animated 参数依赖维度。这让节点作者**不需要预判**哪些节点会有动画参数——默认空,用户加 keyframe 时 Planner 自动让该实例对相应维度敏感。详见 `0.1.0-models.md` §2.5.2。
- **`NodeDef.realtime_capable: bool`(targetv2 M4 新增,必填):** 标记节点是否适合参与尽可能快的持续 cook(`ExecutionMode::Continuous`)。
  - `true` (默认): GPU shader、算术运算、本地处理节点
  - `false`: AI 推理、API 调用、文件 I/O、长时间运算
  - **Continuous 模式下:** Runtime 对 `realtime_capable=false` 节点**不重新执行**;命中正式 cache 则用,未命中则发 `NodeMissingInContinuous` 事件 + 用占位输出
  - **OneShot 模式下:** 该字段无效,所有节点正常执行
  - 详见 `4.1.2-runtime.md` §Continuous 模式 + `0.1.2-interaction-loop.md`
- **显式产物管理节点是通用节点**:它一次主要管理一个结果流,随输入类型处理静态结果或时序结果,对下游输出当前选中版本;首版不按图片 / 视频拆成不同节点,也不额外暴露版本元数据 pin。

---

### 真源 ⑥ · 执行模型

| 字段 | 内容 |
|------|------|
| **定义** | `ExecutionRequest / ExecutionOutputs` 的形状固定;取消粒度 = **run 级 + node 级**;single-flight(同一时刻最多一个活动执行,由 Runtime 全局持有);**语义签名与缓存键严格分离**:`ExecSignature`(语义签名)= 上游签名闭包 + 本节点参数 hash + selected artifact identity + **CookingContext hash** + **Capability version**,**不含 generation、不 hash `Value` 本体**;`CacheKey`(缓存键)= `(NodeId, OutputPin, ExecSignature, generation)`。 |
| **状态** | **已冻结**(targetv2 新增 CookingContext 与 Capability 输入) |
| **权威文档** | `0.1.1-execution-models.md`;`4.1.1-planner.md`;`4.1.2-runtime.md`;`4.11.2-executor-contract.md` |
| **连锁影响** | Planner、Runtime、CacheManager、ArtifactManager、所有 Executor、EngineFacade 的 `execute / cancel_execution / get_execution_state` 接口 |
| **反证/风险** | **高风险 A**:`ExecSignature` 若包含 `Value` 的字节 hash → GPU 回读造成 order-of-magnitude 性能损失;若 hash handle → 缓存命中率崩塌。<br>**高风险 B**:若把 generation 并入 `ExecSignature`,则每次 cache clear / 项目切换都会导致所有签名失效。<br>**高风险 C**(targetv2 新增):如果首版 `ExecSignature` 公式**不**包含 selected artifact identity、`cooking_context_hash` 和 `capability_version`,未来加入它们时所有历史签名失效或下游错误复用旧版本——静默踩坑。注意:target → targetv2 首版的迁移**会**触发一次 cache 清零(因为 hash 输入增加,位级别不同),这是**唯一一次**破坏性代价;之后填入真实 hash 不再破坏,因为旧节点的 `cooking_sensitivity` 声明让它们保持恒零。<br>**高风险 D**(targetv2 新增):如果 single-flight 守卫归属 Session,CLI / Server 路径就没有 single-flight 约束——静默踩坑。 |
| **变更门槛** | 任何涉及执行请求形状、取消粒度、`ExecSignature` 构造、`CacheKey` 构造的变更,必须先改本文档 + `0.1.1` + `4.11.2`。 |

**已冻结子项:**

- 取消粒度 = run 级 + node 级。
- single-flight:同一时刻最多一个活动执行,**由 Runtime(Layer 4)全局持有**。Session 只持有当前 run handle 的引用,不持有权限。
- **语义签名与缓存键分离**:`ExecSignature` 纯粹回答"这是哪次语义计算",与 generation 无关;generation 只在 `CacheKey` 层存在。
- `ExecSignature` 完整构造输入:
  - `upstream_signatures`(传递闭包)
  - `params_hash`
  - `node_def_type_id`
  - `node_def_version`
  - `selected_artifact_identity` —— 当节点输入来自候选历史或产物管理节点时,当前选中版本是语义输入
  - `cooking_context_hash` **[占位]** —— 首版恒为 0,未来填入 CookingContext 的 hash
  - `capability_version` **[占位]** —— 首版恒为 0,未来填入 Capability 的版本
- **`generation +1` 的触发集严格限定为 `{clear_cache, replace_graph}`**。
- **`ExecSignature` 的 hash 算法首版选定 `xxHash3-128`**。换算法视为破坏性变更。
- **切换当前候选版本是图语义变化**:选中旧候选版本后,产物拥有节点本身不因选择变化而重跑;Planner 只标记其下游 dirty,下游签名因 selected artifact identity 变化而改变。
- **`ExecutionRequest` 必须包含 `fidelity: EvaluationFidelity` 字段**(targetv2 新增),值域 `Preview | Full`。首版 `Preview` 和 `Full` 的执行器行为可以一致,但字段必须存在。
- **`Runtime::try_start_run` 接收 `ExecutionMode`**(targetv2 新增):`OneShot { fidelity }` 是首版主要模式;**`Continuous { tick_source, target_fps }` 首版真实实现**。Continuous 模式下:
  - Runtime 进入 tick loop,按 `target_fps` 持续 cook 同一 plan
  - clock-driven cooking_context(frame 维度按时钟推进)
  - `realtime_capable=false` 的节点**不重新执行**(只读 cache + 发缺失事件)
  - 退出: 用户调用 `stop_continuous()` / `cancel`
  - 与 OneShot 互斥: 一次最多一个 Continuous run + single-flight 守卫与 OneShot 共享
  - 目标是尽可能快的持续反馈,不承诺固定帧率
- **Planner 是纯函数**(targetv2 新增),签名为 `(Graph, CookingContext, DirtyState) → ExecutionPlan`。首版 CookingContext 可以为空。
- **Planner 必须产出自包含 `ExecutionPlan`**:Runtime 消费 plan,不回读 Graph。参数采样、参数连线覆盖、输入来源解析、selected artifact query 都属于 Planner 构造计划时的职责。

---

### 真源 ⑦ · 持久化边界

| 字段 | 内容 |
|------|------|
| **定义** | 三类状态的归属:**进项目文件** / **只在运行时** / **跨项目持久但不进项目文件**。 |
| **状态** | 已冻结(framework);用户可见历史与当前版本归属已冻结 |
| **权威文档** | `5.0.0-project-file.draft.md`;`../target/4.6.0-artifact.draft.md`;`4.7.0-project-manager.impl.md`;`4.12.0-session.md` |
| **连锁影响** | 项目文件 schema、打开 / 保存流程、undo 栈设计、缓存初始化、后端凭证管理、Session 生存期 |
| **反证/风险** | 若把 undo 栈放进项目文件 → 项目文件膨胀且跨 session 行为不定;若把 Provider 凭证放进项目文件 → 凭证跟随项目文件泄漏;若把缓存放进项目文件 → 打开项目需恢复一个可能已失效的缓存快照;若把 preview cache 放进正式 cache → 交互路径与 export 路径互相干扰;若把用户候选历史当后台 cache → 关项目后用户丢失可继续工作的版本选择。 |
| **变更门槛** | 任何状态迁移(如把"只在运行时"迁入项目文件)必须先改本文档 + `5.0.0` + `4.7.0`。 |

**三类归属完整表(targetv2 逐行填齐):**

| 状态项 | 归属 |
|--------|------|
| Graph | 进项目文件 |
| 编辑器布局快照(节点位置、分组展开、画布视口) | 进项目文件,但不进入 Graph 语义 |
| 源素材(`load_image` / `load_video`) | 进项目文件 bundle |
| 用户可见候选历史(节点输出历史) | 进项目文件 |
| 当前选中的 Artifact / 候选版本索引 | 进项目文件,并参与图执行语义 |
| 导出历史(`save_image` / `save_video` 输出) | 进项目文件 |
| Artifact 物理存储 | 进项目文件 bundle(或外部存储,由项目文件 schema 明确) |
| CookingContext 默认值(如项目级 frame range) | 进项目文件 |
| 当前交互查看的 CookingContext(如当前查看 frame=42) | 只在运行时(Session) |
| Capability 路由表 | 只在运行时(CapabilityRegistry,启动时注册,冻结) |
| NodeRegistry 的节点定义索引 | 只在运行时(NodeManager,启动时扫描,M6 冻结) |
| NodeManager 的 capability_version 快照 | 只在运行时(NodeManager,启动阶段 4 从 CapabilityRegistry 一次性读) |
| single-flight 守卫 | 只在运行时(Runtime) |
| 正式 cache 条目 | 只在运行时(**CacheManager** 所有,Runtime 读写) |
| `GenerationId` | 只在运行时(**CacheManager** 所有) |
| preview cache | 只在运行时(Session) |
| Plan memoization cache | 只在运行时(Scheduler facade / Planner 内部) |
| DirtyState | 只在运行时(Scheduler facade) |
| undo / redo 栈 | 只在运行时(Session 拥有,GraphController 委托实现);**CLI 模式下 GraphController 不维护 undo 栈** |
| GUI 路径的事件订阅者 | 只在运行时(Session) |
| CLI / Server 路径的事件订阅者 | 只在运行时(EventSystem,直接订阅) |
| 当前交互 run 状态查询 | 只在运行时(Session 通过 scheduler.query_state() 主动查询,不持有引用) |
| 执行进度 | 只在运行时(Runtime) |
| 执行器持久化 disk cache | 跨项目持久,不进项目文件 |
| 后端 HTTP 客户端连接池 | 跨项目持久,不进项目文件 |
| Provider 凭证 | 跨项目持久,不进项目文件 |
| Python 后端连接状态 | 跨项目持久,不进项目文件 |
| 当前选中节点 / 焦点面板 / 打开的历史窗口 | 只在运行时(Session,作为交互状态) |

---

### 真源 ⑧ · 交互回路(targetv2 新增)

| 字段 | 内容 |
|------|------|
| **定义** | 引擎的设计决策必须对"参数变更 → 预览刷新"路径负责。此路径是一等工程关切,不是事后优化的产物。 |
| **状态** | 已冻结(targetv2 首次写入) |
| **权威文档** | `0.1.2-interaction-loop.md`;`4.1.2-runtime.md`;`4.12.0-session.md` |
| **连锁影响** | Planner(增量失效)、Runtime(preview/full 分路)、Cache(preview 缓存)、EventSystem(脏标记传播节流)、Session |
| **反证/风险** | 交互慢从来不是一次犯错的结果,是一连串小决定累加的结果。事前约束便宜,事后重构昂贵。几乎所有"交互笨拙"的节点编辑器都是这么崩的。 |
| **变更门槛** | 涉及真源 ①②③⑤⑥⑦⑧ 的 PR 必须在描述中含 "对交互回路的影响" 字段;"无影响" 也要写。真源 ④ 通常是图快照变更,不直接影响交互回路,故不纳入强制披露范围——但仍鼓励披露。 |

**已冻结子项(五条结构义务,可在 PR review 肉眼判断):**

1. **增量失效默认开启**:Planner 在参数 / 连线变化时只重算受影响子图;"全量重算"不作为默认模式存在。
2. **preview / full 分路**:执行路径必须区分两个求值模式;preview 模式允许低精度、低分辨率、跳过 artifact 写入。字段为 `ExecutionRequest.fidelity: EvaluationFidelity`。
3. **交互路径优先**:preview 与 full 竞争资源时 preview 优先;具体调度规则由 `0.1.2-interaction-loop.md` 定义。
4. **PR 披露义务**:涉及真源 ①②③⑤⑥⑦⑧ 的变更必须披露对交互回路的影响。
5. **Session 不独占 single-flight**:single-flight 不变量由 Runtime 全局持有;Session 只持有当前 run handle 的**引用**。

**非目标(明文不做):**

- 本真源**不**规定具体数字预算(如 "<200ms")。数字预算放独立 benchmark 文档,不进真源。
- 本真源**不**要求首版就实现 preview 加速。首版可以让 Preview 模式行为等同 Full,但**字段必须存在**。
- 本真源**不**强制 Session 从第一版就存在所有功能。首版 Session 可以只有三件事(undo 栈、preview 缓存引用、run handle 引用)。

---

## §3 最优先"静默踩坑"冻结点

以下 5 条**藏在**真源 ③⑤⑥⑦里,但它们是整份文档里**最容易被无声踩坑**的决策。写错不会立刻报错,只会表现为"缓存命中率慢慢下降"、"某类节点永远无法表达"或"用户选中的版本被下游错误复用"。单独列出以强调:

| # | 冻结点 | 归属真源 | 必须明文的内容 |
|---|--------|---------|--------------|
| ① | **`DataType` 封闭 / 开放** | 真源 ③ | `DataType` 是封闭枚举;`AtomicType` 首版清单足够宽以容纳未来一年能力 |
| ② | **`ExecSignature` 身份语义** | 真源 ⑥ | 完整构造输入含 selected artifact identity、`cooking_context_hash` 与 `capability_version` **[占位]**;**不对 `Value` 本体做 hash**;**generation 不参与签名** |
| ③ | **`purity` 独立于 `executor_type`** | 真源 ⑤ | `NodeDef` 必须有独立 `purity` 字段;`executor_type` 只决定粗粒度 Capability 分组 |
| ④ | **single-flight 归 Runtime 而非 Session** | 真源 ①/⑥ | Runtime 是全局守卫持有者;Session 只持有引用。CLI / Server 路径不经过 Session 也受约束 |
| ⑤ | **当前选中产物版本属于图语义** | 真源 ⑥/⑦ | 切换候选版本只使下游 dirty,不重跑拥有历史的上游节点;当前选中版本进项目文件并参与下游签名 |

这 5 条必须在第一版就写死,否则后续任何模块文档都可能"按当时的默认值"做出不可逆的假设。

---

## §4 文档维护规则

### 4.1 三种事实标记

本文档及所有 targetv2 文档必须区分三类内容:

| 标记 | 含义 | 使用规则 |
|------|------|---------|
| **既有决议** | 已有明确出处的决策 | 默认状态,无需标记 |
| **推断** | 作者根据上下文推出的结论 | 以 "推断:..." 开头或加 `[推断]` 前缀 |
| **待确认** | 已识别但尚未决议的问题 | 以 "待确认:..." 开头或加 `[待确认]` 前缀 |

**绝不允许把推断冒充既有决议写入文档。**历史上多次返工源于此类混淆。

### 4.2 真源变更流程

当某条真源需要变更时,必须按顺序执行:

1. **先改本文档**:说明变更原因、新旧差异、版本号递增。
2. **同步权威文档**:更新该真源"权威文档"字段列出的所有 `.md`。
3. **补 changelog**:在 `roadmap.md` 或专门的 changelog 文件中记录变更。
4. **审查下游派生层**:逐一检查该真源影响的模块接口是否需要同步变更。

**反向流程被明令禁止:不允许先改某个模块接口,再倒推修改真源。**

### 4.3 PR 引用规则

所有涉及引擎接口的 PR,描述中必须包含以下之一:

- **`该变更属于真源 X 的派生层调整`**:说明属于派生层变更,无需改真源。
- **`该变更须先改真源 X`**:说明需要先改真源,PR 应先分拆。
- **`该变更无法追溯到任何真源,应先补真源`**:说明真源清单不完整,PR 应暂缓。

Reviewer 必须验证引用的真源确实对应,不得放行无真源追溯的接口变更。

### 4.4 交互回路披露义务(新)

涉及真源 ①②③⑤⑥⑦⑧ 的 PR,除了上面的真源引用外,必须在描述中含 "对交互回路的影响" 字段,可取值:

- **`无影响`**:说明该变更不触及 Planner 增量失效、Runtime preview/full 分路、Session preview 热路径、事件传播节流
- **`改善`**:说明该变更缩短了某条交互路径,附具体说明
- **`风险`**:说明该变更可能延长某条交互路径,附缓解措施

留白的 PR 不得合并。

---

## §5 验证清单

每次修改本文档后,自检:

1. **8 条真源是否都有完整的 6 字段**(定义 / 状态 / 权威文档 / 连锁影响 / 反证 / 变更门槛)?
2. §3 的 4 个静默踩坑点是否都能在 §2 的真源里找到归属?
3. 所有 "待确认" 条目是否都有对应的下一步行动(补哪份文档、谁负责)?
4. 所有 "权威文档" 字段指向的 `.md` 是否真的存在或已在规划中?
5. 本文档的公理层与 `0.1.0-models.md`、`0.1.1-execution-models.md` 是否一致?
6. 真源 ⑧ 的五条结构义务是否都能在 `0.1.2-interaction-loop.md` 找到派生描述?

---

## §6 "机制冻结 / 内容开放" 分界表

为让"一次设计好,后面只做扩展"这个目标**可验证**,本节给出机制与内容的明确分界。

| 维度 | 机制(冻结) | 内容(开放) |
|------|-----------|-----------|
| 类型系统 | `DataType = Atomic(T) \| List(T)`;`Value` 无全局 hash;Materialization 协议 | `AtomicType` 枚举成员集合 |
| 节点定义 | `NodeDef` 字段集 + `purity` 二元 + 作者标注 + `requires: Vec<CapabilityId>` | 有哪些 `NodeDef` |
| 图模型 | `Graph = (nodes, connections)`、不可变、DAG、无布局 | 图里有哪些节点 |
| 图求值 | `Planner` 是纯函数 `(Graph, CookingContext, DirtyState) → Plan` | `CookingContext` 维度集合 |
| 批次 / 序列 | `CookingContext` 是唯一的图多次求值机制 | 支持哪些维度 |
| 集合值 | `List<T>` 作为端口值,不嵌套 | 白名单里的 T |
| 执行路由 | 按 `Capability` 路由 | 有哪些 `Capability` |
| 执行器契约 | 单一 `Executor` trait,声明 Capability,无落盘权限 | 有哪些执行器实现 |
| 身份系统 | `ExecSignature` = 闭包 + 参数 hash + selected artifact identity + context hash + capability version | 签名作用的具体节点 |
| 缓存系统 | `CacheKey = (NodeId, OutputPin, ExecSignature, generation)` | 有哪些缓存后端 / 策略 |
| 产物系统 | 用户可见候选历史 + 当前版本选择 + `ArtifactPolicy` + 显式产物管理节点共享同一抽象 | 有哪些 Artifact 类型 / 哪些节点原生带历史 |
| 持久化边界 | 三分法完整覆盖(见真源 ⑦) | 具体字段归属 |
| 扩展入口 | 单一 `NodeRegistry` façade,内部按五轴路由 | 有哪些 Source / Provider |
| 交互回路 | preview/full 二分,Session 拥有预览路径 | 具体 preview 策略 |
| 事件系统 | 单一 `EventSystem.publish(event)` | 有哪些事件类型 |
| 局部性 | `Value` 是 opaque handle + Materialization 协议 | 有哪些物化后端 |
| 分层归属 | 8 层职责边界 | 每层内部实现细节 |
| Session 边界 | Session 拥有 preview/undo/events;不拥有 single-flight | Session 内部结构 |
| Runtime 边界 | Runtime 拥有 single-flight 全局守卫 | Runtime 内部调度算法 |
| Project-Graph 关系 | 正交,Graph 可脱离 Project | 项目文件格式细节 |

**左列不能动。右列可以无限扩展。**

---

## §7 模块级冻结决议索引

> 以下决议**跨模块可见**,但不属于 §2 八条核心真源。§7 只做索引,不做定义;定义在各自的权威文档里。变更任何一条须先在本文档修改此表,再改权威文档。

| 编号 | 决议 | 结论 | 权威文档 | 变更门槛 |
|------|------|------|---------|---------|
| M1 | 项目文件格式 | `.nodeimg` = ZIP 压缩包;内部结构与解压后目录树一致 | `5.0.0-project-file.draft.md` §Bundle 内部结构 | 影响序列化/打开/保存;修改须更新序列化层 + 本表 |
| M2 | Python 后端启动策略 | Lazy 启动:首次执行 AI 节点时才 spawn Python 进程 | `6.0.0-python-backend.draft.md` §进程生命周期 | 影响首次 AI 执行的冷启动延迟预期;修改须更新引擎 Contract + 本表 |
| M3 | Capability 注册表存活期(新) | Capability 在 Runtime 启动时注册,运行时不支持热插拔 | `0.1.4-capability.md`;`4.1.3-capability-registry.md` | 影响插件系统;修改须更新 Runtime 启动流程 + 本表 |
| M4 | Session 生存期(新) | Session 生存期 = 打开 Project / scratch Graph 到关闭;不跨进程 | `4.12.0-session.md` | 影响 UI 状态持久化决策;修改须更新 Session 文档 + 本表 |
| M5 | preview cache 与正式 cache 分离(新) | preview cache 归 Session,正式 cache 归 CacheManager;二者不共享存储 | `0.1.2-interaction-loop.md`;`4.5.0-cache.impl.md` | 影响缓存管理器拆分;修改须更新两份缓存文档 + 本表 |
| M6 | NodeManager 注册存活期(targetv2 新增) | NodeDef 在引擎启动时完成注册,冻结后运行时不支持动态添加节点。与 M3 平行,保障 Planner 从 NodeManager 读取的 `capability_version` 快照不会在运行时变化 | `0.1.0-models.md`;`4.4.0-node-manager.impl.md`;`4.1.3-capability-registry.md` §注册流程 | 影响插件热加载;修改须先改本表,再改 NodeManager 接口 |
| M7 | CacheManager 所有权(targetv2 新增) | 正式 cache 的**所有权**归 CacheManager,`GenerationId` 归 CacheManager;Runtime 是**用户**,负责读写但不持有所有权。preview cache 归 Session | `4.5.0-cache.impl.md`;`4.1.2-runtime.md` | 影响缓存清理路径;修改须更新 CacheManager 文档 + 本表 |
| M8 | 执行器持久化 disk cache 标准目录(targetv2 M6 新增) | 执行器可以管理跨 session 的 disk cache,标准位置为 `~/.cache/nodeimg/<executor_name>/`(Linux/macOS)或 `%LOCALAPPDATA%\nodeimg\cache\<executor_name>\`(Windows)。每个执行器自管 cache key、文件命名、淘汰策略。**不**进项目文件,不与 Artifact 系统耦合 | `4.11.2-executor-contract.md` §长寿命内部状态 | 影响 AI / video / 模型权重 等长时间外部资源的复用策略;修改须更新执行器约定 + 本表 |
| M9 | Executor lifecycle hooks(targetv2 M5 新增) | `Executor` trait 含 `on_plan_started(plan)` / `on_plan_finished(plan, status)` 两个 default no-op 方法。Runtime 在 Plan 边界调用对应 hook。用于跨 execute() 的资源生命周期管理(视频 encoder finalize、temp file 清理等) | `4.11.2-executor-contract.md`;`4.1.2-runtime.md` | 影响所有视频 / 流式输出执行器;修改须更新 Executor trait |
| M10 | 三类执行来源平权 | 图形处理节点、可控 Python 后端节点、云端 API 节点都是首版一等执行来源。Python 节点是底层可控工作流节点;API 节点是黑盒任务节点 | `README.md`;`roadmap.md`;`4.11.2-executor-contract.md` | 影响 NodeRegistry、CapabilityRegistry、ExecutorContract;修改须更新三处主叙事和执行器契约 |
| M11 | 用户可见候选历史 | Python/API 多轮生成候选、导出结果、当前选中版本是工作流状态,不是后台 cache。候选历史和当前选择随项目保存 | `../target/4.6.0-artifact.draft.md`;`5.0.0-project-file.draft.md`;`4.12.0-session.md` | 影响 Artifact、Project、Session、undo/redo;修改须更新项目文件和产物文档 |
| M12 | 显式产物管理节点 | `artifact_manager` 是图里的通用节点,一次主要管理一个结果流,输出当前选中版本。API 节点原生历史与它共享 Artifact 抽象 | `0.1.0-models.md`;`4.1.1-planner.md`;`../target/4.6.0-artifact.draft.md` | 影响节点模型、Planner dirty 传播、Artifact selection query;修改须同步模型与执行文档 |

---

## §8 派生层变更索引

targetv2 相对 target 的派生层变更清单。变更生效日:targetv2 文档体系定稿之日。

| 文档 | 变更类型 | 说明 |
|------|---------|------|
| `0.1.0-models.md` | 新增字段 / 模型收敛 | `NodeDef.requires: Vec<CapabilityId>`;`AtomicType` 首版清单扩充;`ParamValueMode = Constant \| Animated(Curve)`;显式产物管理节点模型 |
| `0.1.1-execution-models.md` | 公式扩充 | `ExecSignature` 加 selected artifact identity、`cooking_context_hash`、`capability_version` 占位(target → targetv2 会触发一次 cache 清零);`ExecutionRequest` 加 `fidelity` 字段;`Continuous` 表述为尽可能快的持续反馈 |
| `0.1.2-interaction-loop.md` | 新建 | 真源 ⑧ 权威文档 |
| `0.1.3-cooking-context.md` | 新建 | CookingContext 类型与枚举规则 |
| `0.1.4-capability.md` | 新建 | Capability 路由主键 |
| `0.1.5-value-materialization.md` | 新建 | Materializable 协议 |
| `4.0.0-engine.md` | 组件清单扩充 | 加 Planner、Runtime、CapabilityRegistry、Session |
| `4.1.0-scheduler.md` | 拆分 | Scheduler 降格为 facade,核心职责拆分到 4.1.1 Planner 和 4.1.2 Runtime |
| `4.1.1-planner.md` | 新建 | Planner 作为纯函数层,产出自包含 `ExecutionPlan`,负责参数采样、输入来源和 artifact selection query |
| `4.1.2-runtime.md` | 新建 | Runtime 作为副作用层,持有 single-flight,消费 plan 并管理正式 cache / artifact 写入 |
| `4.1.3-capability-registry.md` | 新建 | Capability 注册表 |
| `4.11.2-executor-contract.md` | 接口增补 | 执行器声明 `provides()`;接收 `fidelity` 字段;明确图形处理 / Python 可控后端 / API 黑盒任务三类来源 |
| `4.12.0-session.md` | 新建 | Session 层所有权清单;当前 frame、preview cache、undo/redo 与结果历史面板交互边界 |
| `extensibility.md` | 小幅更新 | 加 NodeRegistry façade 对外入口说明;补 Capability / CookingContext / Session 扩展路径 |
| `first-principles-architecture.md` | 重写 | 8 层分层模型 + Project-Graph 正交 |
