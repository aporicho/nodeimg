# targetv2 —— nodeimg 架构骨架

> **"机制一次设计好,内容持续开放"** —— targetv2 的核心目标。
>
> **产品宣言:** nodeimg **集 DaVinci / TouchDesigner / Nuke 三家之大成** —— 融合三家的核心模型(per-frame cook + ambient time context + 节点无时间感),首版用单一 demo 完整证明这套机制。

targetv2 是 `docs/target/` 的继任文档体系。它在 target 已冻结真源的基础上,**补齐所有必要扩展机制的占位**,使得未来加任何新东西都可以是"填槽"而不是"挖槽"。

**首版 Demo · 实时视频调色工作流:**

```
[ai_video_generate]   ← AI 节点(慢,跨 session disk 持久化)
       ↓
[primary_color_grade] ← lift/gamma/gain,可拖滑块 + 加 keyframe 动画
       ↓
[gaussian_blur]       ← 实时
       ↓
       ├──→ [save_video]               (导出 MP4)
       │
       └──→ [pixel_fluid_animation]    (TouchDesigner 风格实时流体)
              ↓
            [preview_output]            (60fps 实时)
```

这一个 demo 覆盖三家的核心:

- **DaVinci**: keyframed primary grade + 序列帧调色 + 完整导出
- **TouchDesigner**: 60fps Continuous 模式 + 实时流体生成节点 + 拖滑块即时反馈
- **Nuke**: per-frame cook + ambient time context(虽然首版不做多通道 EXR,但机制保留)

**为支持单 demo 目标**,targetv2 在原有 12 条机制之上**追加 6 条**(M1~M6),共 **18 条机制**,见 `roadmap.md` §2。

**首版关键约束:**

- ✅ 视频 I/O(MP4)是首版功能
- ✅ 60fps 实时持续 cook(Continuous 模式)是首版功能
- ✅ AI 输出**持久化到本地磁盘**(避免重抽卡)
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
- targetv2 **新增** 执行器持久化 disk cache 模式(M6)—— AI 输出跨 session 复用

target 的所有 `2.x.x`(GUI)、`5.x.x`(项目文件)、`6.x.x`(Python 后端)文档**在 targetv2 下保持引用**,不重复。targetv2 只改写 `0.x.x` 真源层和 `4.x.x` 引擎层。

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
