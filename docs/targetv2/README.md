# targetv2 —— nodeimg 架构骨架

> **"机制一次设计好,内容持续开放"** —— targetv2 的核心目标。
>
> **产品宣言:** nodeimg 以统一节点图承载三类一等执行来源: 图形处理节点、可控 Python 后端节点、云端 API 节点。图片流、视频流、实时反馈共享同一套执行语义。

targetv2 是 `docs/target/` 的继任文档体系。它在 target 已冻结真源的基础上,**补齐所有必要扩展机制的占位**,使得未来加任何新东西都可以是"填槽"而不是"挖槽"。

**首版 Demo 目标:**

1. **图片 demo:** Python/API 多轮生成 -> 节点级历史对比 -> 选中一个版本 -> 图形处理 -> 导出图片。
2. **视频 demo:** `load_video` 或 API 生成视频 -> 按 frame 求值的图形处理节点 -> `save_video` 输出。
3. **综合 demo:** `Python 出主图和 logo -> API 生视频 -> 图形节点合成/调色 -> 输出视频`。

这三个 demo 共同证明:

- **可控 Python 后端:** Python 节点是底层工作流节点,例如 `load_checkpoint`、`clip_text_encode`、`ksampler`、`vae_decode`。
- **云端 API:** API 节点是黑盒任务节点,输出接口稳定、少而关键。
- **图形处理:** 图片和视频共享同一套图形处理节点;视频在引擎里主要按 frame 求值。
- **实时反馈:** `Continuous` 是首版能力,目标是尽可能快的持续反馈,不写死固定帧率承诺。
- **结果历史:** 多轮生成的候选结果是工作流核心,支持浏览、当前版本选用、标记和清理。

**为支持三类执行器平权目标**,targetv2 在原有 12 条机制之上**追加 6 条**(M1~M6),共 **18 条机制**,见 `roadmap.md` §2。

**首版关键约束:**

- ✅ 视频 I/O(MP4)是首版功能
- ✅ `Continuous` 持续 cook 是首版功能,目标是尽可能快的反馈
- ✅ Python 可控后端节点和云端 API 节点是首版一等能力
- ✅ 用户可见结果历史和执行器内部 disk cache 是两套不同机制
- ❌ 多通道 EXR 读写(机制保留,文件 I/O 延后)
- ❌ 3D 几何 / 渲染(完全不在路线图)
- ❌ 音频 / 实时摄像头 / OCIO(占位,延后)

---

## 与 target 的关系

- targetv2 **不推翻** target 的任何冻结真源(①~⑦)
- targetv2 **新增** 真源 ⑧(交互回路)
- targetv2 **显式化** 四个隐含概念: Session 层 / Planner-Runtime 分工 / Capability 路由 / CookingContext
- targetv2 **改写** 三处叙事: Project-Graph 正交 / `List<T>` 判据 / `ImageValue` 运行时表示
- targetv2 **补齐** 18 条扩展机制(原 12 + 6 条扩展 M1~M6,见 `roadmap.md`)
- targetv2 **重新定义** `ImageValue` 为多通道 + 元数据容器(M1)
- targetv2 **新增** `ParamValueMode` 替换 `ParamDef.default: Value`,支持 keyframed / expression 参数(M2)
- targetv2 **新增** `ExecutionMode::OneShot/Continuous`,首版 Continuous 真实实现(M3)
- targetv2 **新增** `NodeDef.realtime_capable: bool` 字段,区分快/慢节点(M4)
- targetv2 **新增** Executor `on_plan_started` / `on_plan_finished` lifecycle hooks(M5)
- targetv2 **新增** 执行器持久化 disk cache 模式(M6)——与用户可见候选历史分离

target 的 `5.x.x`(项目文件)、`6.x.x`(Python 后端)文档**在 targetv2 下保持引用**,不重复。targetv2 首轮重点改写 `0.x.x` 真源层和 `4.x.x` 引擎层；现在新增 `2.x.x` GUI 架构基线，用于承接 UI / engine 集成。

targetv2 现在新增 GUI 架构基线文档，用于承接首版 demo 的 UI / engine 集成工作。旧 `docs/target/2.x.x` 仍作为历史资料和细节参考；targetv2 的 GUI 状态归属、Tree 模型和文件结构以 `docs/targetv2/2.x.x` 为准。

---

## 目录结构

### 全局文档

- `README.md` —— 本文件
- `spec.md` —— 文档规范
- `architecture-invariants.md` —— **Tier 0 真源,8 条**
- `first-principles-architecture.md` —— **8 层分层模型**
- `extensibility.md` —— 可扩展骨架
- `roadmap.md` —— 迁移与演进路径

### 0.x.x —— 共享真源

| 编号 | 文件 | 内容 |
|------|------|------|
| `0.0.0` | `0.0.0-architecture.md` | 整体架构 |
| `0.0.1` | `0.0.1-engine-contract.md` | 引擎共享契约 |
| `0.0.2` | `0.0.2-module-contract-template.md` | 模块契约模板 |
| `0.1.0` | `0.1.0-models.md` | 类型、节点、图模型 |
| `0.1.1` | `0.1.1-execution-models.md` | 执行请求、签名、缓存键 |
| `0.1.2` | `0.1.2-interaction-loop.md` | **交互回路(真源 ⑧)** |
| `0.1.3` | `0.1.3-cooking-context.md` | **CookingContext** |
| `0.1.4` | `0.1.4-capability.md` | **Capability 路由** |
| `0.1.5` | `0.1.5-value-materialization.md` | **Value 物化协议** |

粗体表示 targetv2 新增。

### 4.x.x —— 引擎模块

| 编号 | 文件 | 内容 |
|------|------|------|
| `4.0.0` | `4.0.0-engine.md` | 引擎总览 |
| `4.1.0` | `4.1.0-scheduler.md` | Scheduler facade |
| `4.1.1` | `4.1.1-planner.md` | **Planner(纯函数)** |
| `4.1.2` | `4.1.2-runtime.md` | **Runtime(副作用)** |
| `4.1.3` | `4.1.3-capability-registry.md` | **Capability 注册表** |
| `4.11.2` | `4.11.2-executor-contract.md` | 执行器契约 |
| `4.12.0` | `4.12.0-session.md` | **Session 层** |

粗体表示 targetv2 新增。其他 `4.x.x` 模块(GraphController、NodeManager、Cache、Artifact、ProjectManager、Events、各 Executor)从 target 继承,仅需按 targetv2 的真源变更做增量修订,不在本目录重写。

### 2.x.x —— GUI 模块

| 编号 | 文件 | 内容 |
|------|------|------|
| `2.0.0` | `2.0.0-gui.md` | GUI 总览 |
| `2.1.0` | `2.1.0-gui-runtime.draft.md` | RuntimeSystems 过渡边界 |
| `2.2.0` | `2.2.0-gui-tree.draft.md` | GUI Tree 状态模型 |
| `2.3.0` | `2.3.0-gui-modules.draft.md` | GUI 文件结构与模块边界 |
| `2.4.0` | `2.4.0-gui-layering.md` | GUI 从业务状态到 GPU 绘制的分层边界 |

---

## 阅读顺序

**第一次读 targetv2,建议:**

1. `architecture-invariants.md` —— 先看 8 条真源
2. `first-principles-architecture.md` —— 再看 8 层分层
3. `0.1.0-models.md` + `0.1.1-execution-models.md` —— 核心数据模型
4. `0.1.2-interaction-loop.md` —— 真源 ⑧
5. `0.1.3~0.1.5` —— 三个新补的机制
6. `extensibility.md` —— 扩展入口
7. `roadmap.md` —— 落地顺序
8. `2.0.0-gui.md` —— GUI 总览与 Tree 架构入口
9. `2.4.0-gui-layering.md` —— GUI 从业务状态到 GPU 绘制的分层边界

**日常查询,按问题分:**

- "这个字段能不能改?" → `architecture-invariants.md`
- "这个层的责任?" → `first-principles-architecture.md`
- "加一个新类型/节点/执行器?" → `extensibility.md`
- "签名怎么构造?" → `0.1.1-execution-models.md`
- "批处理怎么表达?" → `0.1.3-cooking-context.md`
- "为什么快反馈是真源?" → `0.1.2-interaction-loop.md`

---

## 状态

所有 targetv2 文档为 **首版定稿**(无后缀)或 `.draft`(仍在讨论)。

代码实现仍以 `docs/target/` 为准,直到 targetv2 全文被批准后整体替换。
