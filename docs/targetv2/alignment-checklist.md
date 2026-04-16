# targetv2 文档对齐清单

> 依据: `decision-log.md`。
>
> 本文件只记录“哪些正式文档需要怎样对齐”,不替代正式规范。状态取值:
>
> - `todo`: 尚未对齐
> - `partial`: 文档已有部分内容,但与 decision log 不完全一致
> - `aligned`: 已对齐
> - `blocked`: 需要先做上游决策

---

## 总体状态

| 主题 | 状态 | 目标归属 |
|---|---|---|
| 三类执行器平权 | aligned | `README.md`, `roadmap.md`, `architecture-invariants.md`, `4.11.2-executor-contract.md` |
| 图片 demo / 视频 demo / 综合 demo | aligned | `README.md`, `roadmap.md` |
| Python 可控后端底层节点 | partial | `0.1.0-models.md`, `4.11.2-executor-contract.md`, `extensibility.md` |
| API 黑盒任务节点 | aligned | `0.1.0-models.md`, `4.11.2-executor-contract.md` |
| 参数动画 `Constant + Animated(Curve)` | partial | `0.1.0-models.md`, `0.1.1-execution-models.md`, `0.1.3-cooking-context.md` |
| 参数连线优先级 | aligned | `0.1.0-models.md`, `4.1.1-planner.md` |
| Continuous 尽可能快 | aligned | `0.1.1-execution-models.md`, `4.1.2-runtime.md`, `4.12.0-session.md` |
| 慢节点 cache-only / warm 后恢复 | partial | `0.1.1-execution-models.md`, `4.1.2-runtime.md`, `0.1.2-interaction-loop.md` |
| 结果历史是工作流核心 | aligned | `architecture-invariants.md`, `0.1.0-models.md`, `4.6.0-artifact.draft.md` |
| 显式产物管理节点 | partial | `architecture-invariants.md`, `0.1.0-models.md`, `4.6.0-artifact.draft.md`, `extensibility.md` |
| 当前选中版本参与图语义 | aligned | `0.1.1-execution-models.md`, `4.1.1-planner.md`, `4.6.0-artifact.draft.md` |
| 项目 bundle 自包含 | aligned | `5.0.0-project-file.draft.md`, `architecture-invariants.md` |

---

## 文档逐篇对齐项

### `README.md`

状态: `aligned`

- [x] 删除“单一实时视频调色工作流证明全部机制”的首版主叙事。
- [x] 改为“三类执行器平权”: 图形处理节点、可控 Python 后端节点、云端 API 节点。
- [x] 明确首版最低 demo:
  - 图片 demo
  - 视频 demo
  - 三类串联综合 demo
- [x] 综合 demo 写成 `Python 出主图和 logo -> API 生视频 -> 图形节点合成/调色 -> 输出视频`。
- [x] `Continuous` 表述从“60fps 硬承诺”改为“尽可能快的持续反馈”。

### `roadmap.md`

状态: `aligned`

- [x] 路线图改为图片、视频、综合 demo 三线收口。
- [x] 把 `load_video` / `save_video` 归为视频文件与按帧图像流的边界节点。
- [x] 增加 Python executor adapter 阶段。
- [x] 增加显式产物管理节点阶段。
- [x] 增加结果历史面板最低形态: 单节点一个结果库面板。
- [x] 增加 API 节点原生历史和产物管理节点共享抽象的阶段。

### `architecture-invariants.md`

状态: `aligned`

- [x] 将“产物管理能力/节点 + 当前选中版本参与图语义”提升为真源级约束。
- [x] 明确结果历史不是纯后台 cache,而是用户可见工作流状态。
- [x] 明确 selected artifact/current version 进入执行签名。
- [x] 明确项目文件原则: 图结构、源素材、候选历史、导出历史尽量自包含。
- [x] 明确不进项目文件: 当前 frame、临时界面状态、undo/redo、preview cache、凭证。
- [x] 修正 `Continuous` 首版承诺: 尽可能快,不是固定 60fps。
- [x] 明确执行器三类主线平权,不以单一路线作为首版核心。

### `0.1.0-models.md`

状态: `aligned`

- [x] 将参数模型收敛为 `ParamValueMode::Constant | Animated(Curve)`;表达式首版不写。
- [x] 标量动画规则:
  - `Float/Int/Color` 可插值
  - `Bool/String` 跳变
- [x] 明确参数连线优先于本地值和关键帧。
- [x] 明确本地值和关键帧在连线覆盖期间保留为 fallback。
- [x] 明确连线传递的是当前 `CookingContext` 下采样后的具体 `Value`,不是整条 curve。
- [x] 增加显式产物管理节点模型。
- [x] 产物管理节点为通用节点,不按图片/视频拆类型。
- [x] 产物管理节点一次主要管理一个结果流。
- [x] API 节点可原生自带历史能力。
- [x] Python 底层工作流的历史主要通过显式产物管理节点暴露。
- [x] API 节点输出接口稳定、少而关键。

### `0.1.1-execution-models.md`

状态: `aligned`

- [x] `Continuous` 首版要求改为尽可能快的持续反馈。
- [x] 删除或弱化固定 `60fps` 硬承诺。
- [x] 明确慢节点在 `Continuous` 中 cache-only:
  - cache hit 使用缓存
  - cache miss 发事件并使用占位
- [x] 明确暂停 Continuous 后跑一次 `OneShot Full` warm,再恢复。
- [x] 明确 Continuous 中参数/图变化在 tick 边界热替换 plan。
- [x] 明确 selected artifact id 进入签名。
- [x] 明确切换选中版本后只重算下游。
- [x] 明确 Runtime 消费 `ExecutionPlan`,Planner 负责构造签名和输入来源。

### `4.1.1-planner.md`

状态: `aligned`

- [x] 强化 `ExecutionPlan` 自包含,不引用 `Arc<Graph>`。
- [x] `PlannedNode` 包含:
  - `type_id`
  - `requires`
  - sampled params
  - input sources
  - output specs
  - exec signature
  - realtime capable
  - artifact policy
- [x] Planner 负责参数采样。
- [x] Planner 负责参数连线覆盖规则。
- [x] Planner 从 artifact selection query 读取 selected artifact id。
- [x] `propagate_artifact_selection_change` 语义改为: changed node 本身不重跑,下游 dirty。
- [x] 明确静态图接入视频链时由规划/输入语义广播到每帧。

### `4.1.2-runtime.md`

状态: `aligned`

- [x] Runtime 不再回读 Graph。
- [x] Runtime 消费 `ExecutionPlan` 和 `ExecutionMode`。
- [x] Runtime 执行 `PinSource` 输入解析。
- [x] Runtime 管 single-flight、cache、artifact 写入/回源、事件和取消。
- [x] Continuous tick loop 使用当前 plan,图变更时在 tick 边界换 plan。
- [x] 慢节点 cache-only 和占位事件写清楚。
- [x] Preview 中途预览只进入临时预览,不进正式候选历史。
- [x] 后台生成完成后正式结果回填节点/产物管理节点历史。

### `4.11.2-executor-contract.md`

状态: `aligned`

- [x] 明确三类执行来源平权:
  - 图形处理节点
  - 可控 Python 后端节点
  - 云端 API 节点
- [x] Python 节点定义为可控后端底层工作流节点,例如 `load_checkpoint`、`clip_text_encode`、`ksampler`、`vae_decode`。
- [x] API 节点定义为黑盒任务节点。
- [x] API 参数采用共同核心参数 + provider 专属扩展参数。
- [x] 若同种能力可由不同后端执行,用户层面倾向显式区分节点来源,不是自动路由。
- [x] 执行器内部状态允许存在,但节点对外仍按 frame 求值。
- [x] 执行器不拥有用户可见输出历史,结果历史由 Runtime/Artifact 抽象管理。

### `4.12.0-session.md`

状态: `aligned`

- [x] 当前 frame / 播放头位置明确为 Session 状态,不进项目文件。
- [x] preview cache、undo/redo、临时窗口和焦点状态不进项目文件。
- [x] Session 提供结果历史面板的交互入口,但不拥有正式 artifact store。
- [x] 切换当前版本、标记版本、清理历史要进入 undo/redo 语义。
- [x] Continuous 暂停 warm 后恢复的交互路径归 Session 描述。

### `../target/4.6.0-artifact.draft.md`

状态: `aligned`

- [x] Artifact 从“后台产物模块”提升为“候选历史 / 当前选用版本”的工作流核心。
- [x] 区分系统后台 cache 和用户可见候选历史。
- [x] 支持历史浏览、当前版本选用、标记、清理、撤销恢复。
- [x] 选中旧候选版本不重跑上游。
- [x] `save_image` / `save_video` 导出结果也进入节点输出历史。
- [x] `load_image` / `load_video` 源素材进入项目 bundle。
- [x] API 节点原生历史和显式产物管理节点共享抽象。
- [x] 产物管理首版区分静态结果和时序结果。
- [x] 中途预览不进入正式候选历史。

### `../target/5.0.0-project-file.draft.md`

状态: `aligned`

- [x] 项目文件表现为单文件 bundle。
- [x] 图结构、源素材、候选历史、导出历史尽量自包含。
- [x] 节点历史中的当前选中版本要保存。
- [x] 节点位置、分组展开状态、画布视口等编辑器布局保存。
- [x] 当前查看 frame、播放头、当前选中节点、焦点面板、打开的历史窗口不保存。
- [x] 凭证绝不进项目文件。
- [x] 非敏感后端目标信息可以进项目。
- [x] 后端不可用时允许离线浏览和挑选历史,执行时报错。

---

## 决策编号映射

| 决策范围 | 覆盖文档 |
|---|---|
| 1-20 高层摘要 | `README.md`, `roadmap.md`, `architecture-invariants.md` |
| 21-25 参数动画与时间模型 | `0.1.0-models.md`, `0.1.1-execution-models.md`, `0.1.3-cooking-context.md`, `4.1.1-planner.md` |
| 26-30 Continuous 与实时 | `0.1.1-execution-models.md`, `4.1.2-runtime.md`, `4.12.0-session.md` |
| 31-40 结果历史与项目自包含 | `architecture-invariants.md`, `4.6.0-artifact.draft.md`, `5.0.0-project-file.draft.md` |
| 41-45 扩展入口与注册 | `extensibility.md`, `0.1.4-capability.md`, `4.11.2-executor-contract.md` |
| 46-58 Session、项目、凭证 | `4.12.0-session.md`, `5.0.0-project-file.draft.md` |
| 59-62 文档主例子 | `README.md`, `roadmap.md` |
| 63-66 生成节点与普通节点历史策略 | `0.1.0-models.md`, `4.6.0-artifact.draft.md`, `4.11.2-executor-contract.md` |
| 67-87 产物管理节点 | `architecture-invariants.md`, `0.1.0-models.md`, `4.1.1-planner.md`, `4.6.0-artifact.draft.md` |
| 88-97 Python/API 层级 | `0.1.0-models.md`, `4.11.2-executor-contract.md`, `extensibility.md` |
| 98-100 后续落地 | 本清单与实施计划 |

---

## 下一步

1. 已更新 `README.md` 和 `roadmap.md`,消除最显眼的旧主叙事。
2. 已更新 `architecture-invariants.md`,把产物管理和 selected version 提升为真源。
3. 已同步 `0.1.0-models.md` 与 `0.1.1-execution-models.md`,冻结接口术语。
4. 已回填 `4.1.1`, `4.1.2`, `4.11.2`, `4.12.0` 的模块接口。
5. 已回填 `../target/4.6.0-artifact.draft.md` 与 `../target/5.0.0-project-file.draft.md`。
6. 本轮 C1-C10 已完成。下一步进入代码侧接口落地;剩余 `extensibility.md`、`0.1.2`、`0.1.3` 可在代码前做一次补充巡检。
