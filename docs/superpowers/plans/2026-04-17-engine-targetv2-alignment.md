# Engine targetv2 对齐实施计划

> 本文是落地计划,不是正式规范。若与 `docs/targetv2/decision-log.md`、`architecture-invariants.md` 或其他 targetv2 正式文档冲突,以后者为准。
>
> 目标: 把 2026-04-16 决策记录转化为可执行的文档对齐、接口冻结和代码重构任务,并避免在 Runtime 中继续堆叠跨模块职责。

---

## 目标

将引擎从当前的“API / LibTV 视频 MVP 主线”调整为 targetv2 决策后的“三类执行器平权”架构:

- 图形处理节点、本地/远端可控 Python 后端节点、云端 API 节点三条主线平权。
- 首版同时交付图片 demo、视频 demo、综合 demo。
- 综合 demo 为 `Python 出主图和 logo -> API 生视频 -> 图形节点合成/调色 -> 输出视频`。
- 结果历史、当前版本选用、产物管理节点进入正式图语义。
- `Continuous` 为首版硬要求,但表述为尽可能快的持续反馈,不承诺固定 60fps。
- Runtime 消费 Planner 产出的自包含 `ExecutionPlan`,不再现场回读 `Graph` 做规划。

## 当前状态摘要

当前代码已有的基础:

- `Engine -> SchedulerFacade -> Runtime -> CapabilityRegistry -> Executor` 主链路已存在。
- `NodeRegistry` 是统一扩展入口,Capability 注册在启动时完成并冻结。
- `Executor` trait 已统一,节点执行按 `NodeDef.requires` 路由。
- Artifact 模块已有历史、selected artifact、读写、清理和 Runtime 接线。
- `Session` 已有 preview overlay、preview cache、Full export 路径。

当前主要落差:

- `Runtime` 仍直接读取 `Graph` 做拓扑、输入解析、签名和执行循环。
- `engine/src/scheduler/` 旧调度路径仍留在仓库,虽然没有被 `lib.rs` 导出。
- `Continuous` 目前在 `validate_mode` 中直接返回未实现。
- Python 节点当前是声明式收集,生成的 `execute` 为 declaration-only。
- 参数模型仍是 `HashMap<String, Value>`,没有 `ParamValueMode::Constant | Animated(Curve)`。
- 没有显式“产物管理节点”。
- `load_video` 还没有节点实现。
- targetv2 文档主叙事仍有“单 demo / 60fps / AI video 调色”旧表述,需要回填 decision log。

## 设计原则

1. **文档真源先行**: 先把 decision log 回填到 targetv2 文档,再动大范围代码。
2. **接口先行**: 新能力先落接口和边界,再落具体实现。
3. **Planner 纯函数**: 规划、签名、输入来源、参数采样归 Planner。
4. **Runtime 只消费 Plan**: Runtime 不再读 Graph;只执行 `ExecutionPlan`、管 single-flight、cache、artifact、事件和取消。
5. **Artifact 是图语义**: 当前选中版本参与签名,切换版本只重算下游。
6. **Executor 保持窄接口**: Executor 不知道 Graph、cache、artifact history,只按 capability 执行一次请求。
7. **Session 只管交互状态**: preview cache、undo/redo、播放头归 Session;正式 cache 和 artifact 不归 Session。
8. **增量可验证**: 每个阶段都必须能 `cargo check -p engine`,关键阶段补契约测试。

---

## 交付物

### 文档交付物

- `docs/targetv2/alignment-checklist.md`
- 更新 `docs/targetv2/README.md`
- 更新 `docs/targetv2/roadmap.md`
- 更新 `docs/targetv2/architecture-invariants.md`
- 更新 `docs/targetv2/0.1.0-models.md`
- 更新 `docs/targetv2/0.1.1-execution-models.md`
- 更新 `docs/targetv2/4.1.1-planner.md`
- 更新 `docs/targetv2/4.1.2-runtime.md`
- 更新 `docs/targetv2/4.11.2-executor-contract.md`
- 更新 `docs/targetv2/4.12.0-session.md`
- 增量修订 `docs/target/4.6.0-artifact.draft.md`
- 增量修订 `docs/target/5.0.0-project-file.draft.md`

### 代码交付物

- 新增或重组 `engine/src/planner/`
- Runtime 从 `Graph` 驱动切换到 `ExecutionPlan` 驱动
- `ArtifactSelectionQuery` / `ArtifactHistoryQuery` / `ArtifactValueStore` 接口
- `ParamValueMode` 与参数采样接口
- 显式产物管理节点
- `load_video` 节点和视频按 frame 输入边界
- `Continuous` 最小闭环
- Python executor adapter

---

## 阶段 0: 基线确认与旧路径处理

**目标:** 先确认当前主链路和旧代码边界,避免后续误改未接线模块。

**Files:**

- Inspect: `engine/src/lib.rs`
- Inspect: `engine/src/scheduler_facade.rs`
- Inspect: `engine/src/runtime.rs`
- Inspect: `engine/src/scheduler/`

- [ ] Step 0.1: 确认 `engine/src/scheduler/` 没有被 `engine/src/lib.rs` 导出。
- [ ] Step 0.2: 给旧 scheduler 路径建立处理决策: 删除、迁移测试、或加明确 `legacy` 标记。
- [ ] Step 0.3: 跑基线验证。

```bash
cargo check -p engine
cargo test -p engine --lib
```

**验收:**

- 清楚标明当前有效路径是 `scheduler_facade.rs + runtime.rs`。
- 记录库内测试中依赖 `wgpu` adapter 的已知失败,避免误判为架构改动回归。

---

## 阶段 1: 产出文档对齐清单

**目标:** 把 `decision-log.md` 的 100 条决策映射到具体文档和模块。

**Files:**

- Create: `docs/targetv2/alignment-checklist.md`

- [ ] Step 1.1: 按文档列出需要改的章节。
- [ ] Step 1.2: 按主题建立决策映射:
  - 三类执行器平权
  - 图片 demo / 视频 demo / 综合 demo
  - Python 可控后端底层节点
  - API 黑盒任务节点
  - 参数动画与连线优先级
  - Continuous 与慢节点 cache-only
  - 结果历史和当前版本选用
  - 显式产物管理节点
  - 项目 bundle 自包含边界
- [ ] Step 1.3: 给每项标记状态: `todo / in-progress / aligned / blocked`。

**验收:**

- `decision-log.md` 的高层 20 条和最后 100 条均能在 checklist 中找到归属。
- checklist 不改变正式规范,只指出要改哪里和怎么改。

---

## 阶段 2: 回填 targetv2 文档真源

**目标:** 先让正式文档和 decision log 一致。

**Files:**

- Modify: `docs/targetv2/README.md`
- Modify: `docs/targetv2/roadmap.md`
- Modify: `docs/targetv2/architecture-invariants.md`
- Modify: `docs/targetv2/0.1.0-models.md`
- Modify: `docs/targetv2/0.1.1-execution-models.md`
- Modify: `docs/targetv2/4.1.1-planner.md`
- Modify: `docs/targetv2/4.1.2-runtime.md`
- Modify: `docs/targetv2/4.11.2-executor-contract.md`
- Modify: `docs/targetv2/4.12.0-session.md`

- [ ] Step 2.1: 更新 README 和 roadmap 主叙事。
  - 删除“单一视频调色 demo”作为首版唯一证明的表述。
  - 改为图片 demo、视频 demo、综合 demo。
- [ ] Step 2.2: 更新架构不变量。
  - 将“产物管理能力/节点 + 当前选中版本参与图语义”提升为真源。
  - 修正 `Continuous` 目标为尽可能快,不写死 60fps。
- [ ] Step 2.3: 更新模型文档。
  - `ParamValueMode = Constant | Animated(Curve)`。
  - 参数连线优先于本地值和关键帧。
  - 产物管理节点为正式节点。
  - 当前选中版本进入图语义。
- [ ] Step 2.4: 更新执行文档。
  - Runtime 消费自包含 Plan。
  - selected artifact id 进入 Impure 节点签名。
  - Continuous cache-only 慢节点和 warm 机制。
- [ ] Step 2.5: 更新 executor/session 文档。
  - Python 节点是可控后端底层工作流节点。
  - API 节点是黑盒任务节点。
  - Session 持有播放头、preview cache、undo/redo,不持有正式 cache。

**验收:**

- targetv2 文档内部没有“单 demo / 必须 60fps / API video 是唯一主线”的旧叙事。
- `decision-log.md` 中列出的优先文档均已处理。

---

## 阶段 3: 冻结 Planner / Runtime 接口

**目标:** 先定义接口和数据结构,再迁移实现。

**Files:**

- Create: `engine/src/planner/mod.rs`
- Create: `engine/src/planner/model.rs`
- Create: `engine/src/planner/plan_builder.rs`
- Modify: `engine/src/lib.rs`
- Modify: `engine/src/scheduler_facade.rs`
- Modify: `engine/src/runtime.rs`

**目标接口草案:**

```rust
pub trait Planner {
    fn plan(
        &self,
        graph: &Graph,
        request: &ExecutionRequest,
        context_range: &CookingContextRange,
        dirty: &DirtyState,
        node_defs: &dyn NodeDefQuery,
        artifacts: &dyn ArtifactSelectionQuery,
    ) -> Result<ExecutionPlan, PlannerError>;
}

pub struct ExecutionPlan {
    pub subtasks: Vec<PlanSubtask>,
}

pub struct PlanSubtask {
    pub cooking_context: CookingContext,
    pub layers: Vec<Vec<PlannedNode>>,
}

pub struct PlannedNode {
    pub node_id: NodeId,
    pub type_id: String,
    pub requires: Vec<CapabilityId>,
    pub params: HashMap<String, Value>,
    pub input_sources: HashMap<String, PinSource>,
    pub outputs: Vec<OutputSpec>,
    pub exec_signature: ExecSignature,
    pub realtime_capable: bool,
    pub artifact_policy: ArtifactPolicy,
}

pub enum PinSource {
    Upstream { node_id: NodeId, output: String },
    Constant(Value),
    Param { name: String },
    ArtifactSelection { node_id: NodeId, output: String },
}
```

- [ ] Step 3.1: 增加 planner 模块和模型,暂不切换 Runtime。
- [ ] Step 3.2: 将 `resolve_subgraph`、拓扑顺序、签名构造的纯逻辑复制/迁移到 Planner。
- [ ] Step 3.3: 给 Planner 添加纯函数单测。
- [ ] Step 3.4: 保持现有 Runtime 仍可运行。

**验收:**

- `cargo check -p engine` 通过。
- Planner 可以在测试中给定 Graph 产出稳定 `ExecutionPlan`。
- Runtime 行为暂不变。

---

## 阶段 4: Runtime 改为消费 ExecutionPlan

**目标:** Runtime 不再读取 `Graph` 做规划,只执行 Plan。

**Files:**

- Modify: `engine/src/runtime.rs`
- Modify: `engine/src/scheduler_facade.rs`
- Modify: `engine/src/execution.rs`
- Add tests under `engine/tests/`

- [ ] Step 4.1: 在 `SchedulerFacade::request_execution` 中调用 Planner 生成 Plan。
- [ ] Step 4.2: 将 Runtime 的 `request_execution` 入参从 `&Graph` 迁移到 `ExecutionPlan`。
- [ ] Step 4.3: 将 `Runtime::evaluate_order` 改为 `Runtime::execute_plan_subtask`。
- [ ] Step 4.4: Runtime 输入解析改为读取 `PinSource` 和前序节点结果。
- [ ] Step 4.5: 保留 single-flight、pending full、preview preemption 语义。
- [ ] Step 4.6: 删除 Runtime 内部对 `resolve_subgraph`、`topo_sort` 的依赖。

**验收:**

- `Runtime` 文件中不再出现对 `graph.connections` / `resolve_subgraph` / `topo_sort` 的执行期依赖。
- 现有 `single_flight`、`session`、`runtime_artifacts` 集成测试通过或有明确迁移记录。

---

## 阶段 5: Artifact 历史与当前版本图语义

**目标:** 将 artifact 从 Runtime 辅助能力提升为图执行语义。

**Files:**

- Modify: `engine/src/artifact/`
- Modify: `engine/src/runtime/artifacts/`
- Modify: `engine/src/scheduler_facade.rs`
- Modify: `engine/src/runtime.rs`
- Modify: `engine/src/cache/model/exec_signature.rs`

**接口草案:**

```rust
pub trait ArtifactHistoryQuery {
    fn list_versions(&self, node_id: NodeId, output: &str) -> Vec<ArtifactRecord>;
    fn selected_version(&self, node_id: NodeId, output: &str) -> Option<ArtifactRecord>;
}

pub trait ArtifactSelectionCommand {
    fn select_version(
        &mut self,
        node_id: NodeId,
        output: &str,
        artifact_id: &str,
    ) -> Result<ArtifactSelectionChange, ArtifactError>;
}

pub trait ArtifactValueStore {
    fn persist_output(&mut self, request: CreateArtifactRequest) -> Result<ArtifactRecord, ArtifactError>;
    fn restore_value(&self, artifact_id: &str) -> Result<Value, ArtifactError>;
}
```

- [ ] Step 5.1: 将 selected artifact id 纳入 Impure 节点 `ExecSignature`。
- [ ] Step 5.2: `select_artifact_version` 不重跑上游,只标脏下游。
- [ ] Step 5.3: API 节点原生历史和后续产物管理节点共用同一套接口。
- [ ] Step 5.4: 给 artifact record 增加首版标记字段,用于结果历史简单标记。
- [ ] Step 5.5: 清理策略保留 selected 版本,删除操作可由 undo/redo 恢复。

**验收:**

- 切换 selected artifact 后,下游 cache key 改变。
- 切换 selected artifact 不执行该节点上游。
- 查询、选择、清理、标记有独立测试覆盖。

---

## 阶段 6: 显式产物管理节点

**目标:** 引入图中正式存在的通用“产物管理节点”。

**Files:**

- Modify: `engine/src/node_registry/sources.rs`
- Add: `engine/src/executors/artifact_manager_node.rs` 或等价模块
- Modify: `engine/src/executors/mod.rs`

- [ ] Step 6.1: 定义节点类型,例如 `artifact_manager`。
- [ ] Step 6.2: 节点跟随输入类型工作,接图片输出图片,接视频输出视频。
- [ ] Step 6.3: 单个节点一次管理一个结果流。
- [ ] Step 6.4: 切换版本时只重算下游。
- [ ] Step 6.5: 首版不暴露版本元数据 pin,只输出当前选中主数据流。

**验收:**

- Python 底层工作流可以通过产物管理节点暴露候选历史。
- API 节点原生历史和产物管理节点历史使用一致抽象。

---

## 阶段 7: ParamValueMode 与参数采样

**目标:** 支持 `Constant + Animated(Curve)`,不做表达式。

**Files:**

- Modify: `types/src/value.rs` 或新增 `types/src/param.rs`
- Modify: `engine/src/node_manager/model/param_def.rs`
- Modify: `engine/src/graph/model/graph.rs`
- Modify: `engine/src/planner/`

**接口草案:**

```rust
pub enum ParamValueMode {
    Constant(Value),
    Animated(Curve),
}

pub trait ParamSampler {
    fn sample(
        &self,
        params: &HashMap<String, ParamValueMode>,
        defaults: &[ParamDef],
        context: &CookingContext,
    ) -> Result<SampledParams, ParamError>;
}

pub struct SampledParams {
    pub values: HashMap<String, Value>,
    pub sensitivity: Vec<String>,
}
```

- [ ] Step 7.1: 引入 `ParamValueMode`,先把现有 `Value` 包成 `Constant`。
- [ ] Step 7.2: Graph 存储从 `HashMap<String, Value>` 迁移到 `HashMap<String, ParamValueMode>`。
- [ ] Step 7.3: Planner 在当前 CookingContext 下采样参数。
- [ ] Step 7.4: 连线值优先于本地参数采样值。
- [ ] Step 7.5: `Bool/String` Animated 只支持跳变。

**验收:**

- 现有静态参数行为不变。
- 同一 Animated 参数在不同 frame 采样后能产生不同签名。
- 断开参数连线后 fallback 到本地值或关键帧。

---

## 阶段 8: 视频边界与静态图广播

**目标:** 建立图片和视频共享图形节点的执行语义。

**Files:**

- Add: `engine/src/executors/video/load.rs`
- Modify: `engine/src/executors/video/mod.rs`
- Modify: `engine/src/node_registry/sources.rs`
- Modify: `engine/src/runtime.rs` 或 Planner 输入语义

- [ ] Step 8.1: 实现 `load_video` 节点。
- [ ] Step 8.2: `load_video` 按当前 frame 输出 `image` 和 `fps`。
- [ ] Step 8.3: 静态图接入视频链时,默认广播到每帧。
- [ ] Step 8.4: `save_video` 继续通过 lifecycle hook 管理 encoder。

**验收:**

- `load_video -> color_adjust -> save_video` 可完成最小视频 demo。
- `load_image -> color_adjust -> save_video` 可广播静态图到多帧视频。

---

## 阶段 9: Continuous 最小闭环

**目标:** 实现尽可能快的持续反馈,慢节点 cache-only。

**Files:**

- Modify: `engine/src/runtime.rs`
- Modify: `engine/src/events/types.rs`
- Modify: `engine/src/execution.rs`
- Modify: `engine/src/session.rs`

- [ ] Step 9.1: `ExecutionMode::Continuous { tick_source: OsClock, target_fps }` 不再直接报错。
- [ ] Step 9.2: Runtime 进入 tick loop,每 tick 生成 `CookingContext { frame }`。
- [ ] Step 9.3: `realtime_capable=false` 节点只查 cache/artifact。
- [ ] Step 9.4: 未命中时发 `NodeMissingInContinuous` 事件并产出占位。
- [ ] Step 9.5: 支持暂停 Continuous,跑一次 OneShot Full warm,再恢复。
- [ ] Step 9.6: 图或参数变化时在 tick 边界热替换 Plan。

**验收:**

- Continuous 与 OneShot 共享 single-flight。
- 慢节点不会在 Continuous 中触发远端/API执行。
- 修改参数后无需整次 Runtime 重启即可使用新 Plan。

---

## 阶段 10: Python executor adapter

**目标:** 让现有 `python/nodes/*` 从 declaration-only 变成可控后端底层节点。

**Files:**

- Modify: `engine/build.rs`
- Modify: `engine/src/node_registry/sources.rs`
- Add: `engine/src/executors/python/`
- Inspect/Modify: `python/server.py`
- Inspect/Modify: `python/registry.py`

- [ ] Step 10.1: 定义 Python backend executor capability,例如 `python.<type_id>` 或统一 `python.execute_node`。
- [ ] Step 10.2: 将 Python 节点绑定到 Python executor,而不是 declaration-only execute。
- [ ] Step 10.3: 建立请求/响应协议: node type、inputs、params、handles、outputs。
- [ ] Step 10.4: 支持底层工作流节点: `load_checkpoint`、`clip_text_encode`、`ksampler`、`vae_decode`。
- [ ] Step 10.5: 输出候选历史通过产物管理节点承载。

**验收:**

- Python 最小图片链可执行。
- Python 节点不是高层黑盒任务节点。
- Python 本地/远端部署差异不影响节点定义。

---

## 阶段 11: Demo 收口

**目标:** 按 decision log 交付三个代表性 demo。

- [ ] Step 11.1: 图片 demo: Python/API 多轮生成 -> 历史对比 -> 选择版本 -> 图形处理输出。
- [ ] Step 11.2: 视频 demo: load/generate video -> 图形处理 -> save_video。
- [ ] Step 11.3: 综合 demo: Python 出主图和 logo -> API 生视频 -> 图形节点合成/调色 -> 输出视频。
- [ ] Step 11.4: 为 demo 写最小自动化测试或 smoke test。

**验收:**

- 三类执行器都在 demo 中作为一等能力出现。
- 图片流和视频流同等完整,不是一条完整一条简化。

---

## 推荐执行顺序

1. 阶段 1: 文档对齐清单。
2. 阶段 2: 回填 targetv2 文档。
3. 阶段 3: Planner / Runtime 接口冻结。
4. 阶段 4: Runtime 消费 ExecutionPlan。
5. 阶段 5: Artifact 当前版本进入签名和图语义。
6. 阶段 6: 显式产物管理节点。
7. 阶段 7: ParamValueMode。
8. 阶段 8: load_video 和静态图广播。
9. 阶段 9: Continuous。
10. 阶段 10: Python executor adapter。
11. 阶段 11: 三个 demo 收口。

---

## 模块化验收清单

每次实现阶段结束后检查:

- [ ] Planner 不依赖 `CacheManager` 写接口。
- [ ] Planner 不调用 Executor。
- [ ] Runtime 不读取 `Graph`。
- [ ] Runtime 不计算拓扑结构。
- [ ] Executor 不知道 Graph/cache/artifact history。
- [ ] Session 不持有正式 cache。
- [ ] ArtifactManager 不知道执行拓扑。
- [ ] ProjectManager 不参与执行。
- [ ] NodeRegistry 仍是唯一扩展入口。
- [ ] 当前选中 artifact 版本切换只影响下游。
- [ ] Preview 不写正式 cache,除非 executor 诚实报告 `fidelity_achieved == Full`。
- [ ] Preview 不写 artifact。

## 验证命令

```bash
cargo check -p engine
cargo test -p engine --lib
cargo test -p engine --test single_flight
cargo test -p engine --test session
cargo test -p engine --test runtime_artifacts
cargo test -p engine --test video_pipeline
```

注意: `cargo test -p engine --lib` 当前在无可用 `wgpu` adapter 的环境下可能有 texture cache 测试失败。修测试环境适配应作为独立小任务处理。
