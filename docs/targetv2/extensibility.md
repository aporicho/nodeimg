# 可扩展骨架

> 本文档说明 nodeimg 引擎如何在**不推翻核心真源**的前提下,持续新增类型、节点、来源、执行行为、能力、调度维度与展示行为。

> 它**不替代** `architecture-invariants.md`、`0.1.0-models.md`、`0.1.1-execution-models.md`,而是回答一个更具体的问题:**以后要加新东西时,只改哪里。**

---

## 0. 扩展代价分级

targetv2 的扩展点**不是全部零代价**。三个等级:

| 等级 | 含义 | 例子 | 代价 |
|------|------|------|------|
| **零代价扩展** | 内容集合无限增长,不需要改现有代码 | 新 `CapabilityId` 字符串、新 `DimensionId`、新 Provider、新 Node、新 Executor、新 `Constraint` 类型 | 只加代码,不改现有 |
| **轻破坏扩展** | 封闭枚举加变体,所有 match 点需更新一次 | 新 `AtomicType`(含占位转正外的**真新增**)、新 `MaterializationTarget`、新 `DataType` 构造器 | 一次性扫描 + 编译器强制更新;但不影响语义和签名 |
| **破坏性扩展** | 需要修改真源层 | 改 `NodeDef` 核心字段、改 `ExecSignature` 构造公式、改 `single-flight` 语义、改 purity 值域 | 先改真源,所有下游文档同步,可能需要 cache 清零 |

**关键区分**:

- **占位类型转正**(如 `Video` 从 `[占位]` 变成真实支持)= **零代价** —— 类型已经在枚举里
- **真的新增**(如 `PointCloud` 根本不在首版占位清单)= **轻破坏** —— 枚举加变体

首版占位清单(`AtomicType` 18 个、`MaterializationTarget` 4 个)足够宽,**减少**未来破坏性扩展的概率——但不能**消除**。

---

## 1. 目标

> **机制冻结,内容开放。**

nodeimg 的可扩展性目标不是"预先支持所有未来功能",而是:

1. 新增一种数据类型时,不需要推翻节点模型
2. 新增一种节点来源时,不需要重写注册系统
3. 新增一种执行器或策略时,不需要污染静态模型
4. 新增一种展示方式时,不需要把 GUI 细节塞进真源
5. 新增一种调度维度(批处理 / 序列帧 / 参数扫描)时,优先通过 `CookingContext` 接入
6. 新增一种能力时,优先通过 `Capability` 接入,不新建执行器枚举
7. 新增一种前端时,走 Runtime 直通或 Session 特权路径

---

## 2. 顶层定位

nodeimg 的长期目标是:

> **面向新时代图像 / 视频 / 矢量图形 / 设计排版的自动化创作引擎。**

骨架设计原则:

- **先服务图像 + 序列帧 + 少量 AI**
- **明确为视频 / 音频 / 矢量 / 设计排版留标准扩展口**
- **不在第一版实现所有能力,但必须让新增能力有固定接入路径**
- **交互回路是永恒约束,不随产品形态变化而放宽**

---

## 3. 五个变化轴 + 一个对外入口

targetv2 保留 target 时代识别的**五条变化轴**,但在工程上**收敛为一个 `NodeRegistry` façade**:

```
 ┌─────────────────────────────────────┐
 │                                     │
 │   NodeRegistry (唯一对外入口)         │
 │                                     │
 │   NodeRegistry::register(source)    │
 │                                     │
 └──────────────┬──────────────────────┘
                │ 内部路由
                ▼
  ┌─────┬─────┬─────┬─────┬─────┬────────┐
  │ 类型 │ 节点 │ 来源 │ 执行 │ 展示 │ Capability │
  │ 扩展 │ 扩展 │ 扩展 │ 扩展 │ 扩展 │ 扩展       │
  └─────┴─────┴─────┴─────┴─────┴────────┘
```

`NodeRegistry` 是外部扩展的**唯一**入口。不支持其他扩展通道。这是真源 ② 的结构化保障。

---

## 4. 扩展点逐一说明

### 4.1 类型扩展点

**权威文档:** `0.1.0-models.md` §1.1

**机制:**

- `AtomicType`(原子类型)
- `DataType = Atomic(T) | List(T)`(数据类型)
- `Value`(值)
- `Materializable` 协议(见 `0.1.5-value-materialization.md`)

**规则:**

1. 新增一种一等数据能力,优先表现为**新增原子类型**
2. 首版 `AtomicType` 清单已扩充到 18 种(含占位)—— 检查是否已经在占位里,再决定是否需要真的新增
3. 批处理 / 序列能力**不**走 `List<T>`,走 `CookingContext`(见 §4.6)
4. **真值集合**走 `List<T>`(检测器输出、调色板等)
5. 新增原子类型是**破坏性变更**,必须先修改真源文档
6. 新增占位类型**转正**(从 `[占位]` 变成有执行器支持)**不是**破坏性变更,因为类型已在系统中存在

### 4.2 静态节点扩展点

**权威文档:** `0.1.0-models.md` §2.6

**机制:**

- `NodeDef`
- `InputPinDef` / `OutputPinDef`
- `ParamDef`
- `Constraint`

**规则:**

1. 所有节点都必须先落到静态契约层
2. `NodeDef` 只承载静态身份、执行归类、纯性、静态引脚、静态参数、默认执行策略和轻展示元数据
3. 动态参数 schema、GUI 渲染细节、运行时状态**不得**进入 `NodeDef`
4. **`NodeDef.requires: Vec<CapabilityId>` 是路由主键**(targetv2 新增): 节点声明需要什么 Capability,而不是直接绑执行器

### 4.3 来源扩展点

**权威文档:** `0.1.0-models.md` §4

**机制:**

```rust
trait NodeSource {
    fn collect(&self) -> Vec<NodeRegistration>;
}

struct NodeRegistration {
    static_def: NodeDef,
    schema_provider: Option<Arc<dyn SchemaProvider>>,
    presentation_provider: Option<Arc<dyn PresentationProvider>>,
    capability_bindings: Vec<(CapabilityId, Arc<dyn Executor>)>,
}
```

**规则:**

1. 所有节点来源都必须先产出统一 `NodeRegistration`
2. 一个来源可以返回多个节点
3. `schema_provider` 与 `presentation_provider` 都是可选层,按**节点类型**提供
4. `capability_bindings` 是可选的——来源层可以同时携带执行器实现(如 Python Provider 既是节点来源又是 Capability 提供者)
5. 不允许给来源层开"自定义执行绑定"后门
6. 所有来源经 **`NodeRegistry::register(source)`** 进入系统,不通过其他通道

**这意味着:**

- Rust builtins
- Python 节点扫描
- API Provider
- 未来插件节点目录
- 未来远程节点目录

都应通过同一来源接口接入。

### 4.4 执行扩展点

**权威文档:** `0.1.1-execution-models.md`;`0.1.4-capability.md`;`4.11.2-executor-contract.md`

**机制:**

- `ExecutionRequest` / `ExecutionOutputs`
- `ExecSignature` / `CacheKey`
- `ExecutionPolicy`
- **`Capability`**(targetv2 新增)
- **`Executor` trait 的 `provides()` 方法**(targetv2 新增)

**规则:**

1. 执行器统一接口,不因节点来源不同而分裂
2. **Runtime 路由按 `Capability` 查表**,不按 `executor_type` switch
3. 新增一种执行器 = 实现 `Executor` trait + 声明提供的 Capability
4. 新执行器**不改** `ExecutorType` 枚举,不改 Runtime 分发代码
5. 缓存、产物、触发、重试等策略统一经 `ExecutionPolicy` 暴露
6. `ExecSignature` 与 `CacheKey` 严格分离

**这意味着:**

- 未来新增 `VideoExecutor` / `AudioExecutor` / `VectorExecutor` / `LayoutExecutor` 时,都声明新的 `CapabilityId`,实现 `Executor::provides()`,注册到 `CapabilityRegistry`——**不改核心**
- 同一 Capability 可以有**多个**执行器(例如 GPU 模糊 + CPU fallback),Runtime 按局部性选择
- 执行器版本升级通过 `Capability.version` 表达,经 `capability_version` 进入 `ExecSignature`

### 4.5 展示扩展点

**权威文档:** `0.1.0-models.md` §1.5、§2.5;`2.x.x` GUI 文档

**机制:**

- `ParamDef` + `Constraint`
- `PresentationProvider`

**规则:**

1. 参数语义在 `ParamDef` 中定义
2. 参数显示位置、节点特例渲染、折叠规则由 `PresentationProvider` 决定
3. 默认展示规则必须有统一协议,不能散在 GUI 代码中
4. 轻展示元数据(`description` / `tags` / `icon`)可进 `NodeDef`
5. GUI 私有渲染细节**不得**进入真源

### 4.6 调度维度扩展点(targetv2 新增)

**权威文档:** `0.1.3-cooking-context.md`

**机制:**

- `CookingContext`
- `DimensionId` / `DimensionValue`
- `CookingContextRange`

**规则:**

1. 新增一种"图多次求值"语义(序列帧、批处理、参数扫描、多种子),**必须**走 `CookingContext`,不走 `List<T>`
2. 新增一个 `DimensionId` 不是破坏性变更——首版 `cooking_context_hash` 字段已经在 `ExecSignature` 公式里,新加维度只是让这个 hash 从恒零变成有意义的值
3. `CookingContext` 不与 `List<T>` 互相转换,除非通过**显式 collect / spread 节点**

**这意味着:**

- 未来加 frame / seed / variant / resolution_tier 等维度 = "填槽",不动核心
- 旧节点不使用该维度 → `cooking_context_hash` 恒为 0 → 签名不变 → 缓存继续命中

### 4.7 Capability 扩展点(targetv2 新增)

**权威文档:** `0.1.4-capability.md`;`4.1.3-capability-registry.md`

**机制:**

- `CapabilityId`(开放字符串)
- `Capability` 结构(含 `version`、`input_types`、`output_types`、`side_effects`)
- `CapabilityRegistry` 启动期注册表

**规则:**

1. 新增 Capability = 声明新 `CapabilityId` + 实现 `Executor::provides()` + 启动时注册
2. Capability 版本升级(算法变更、参数扩充)通过递增 `version` 表达
3. `capability_version` 字段已经在 `ExecSignature` 公式里——升级版本不改公式,只改值
4. 运行时**不**支持 Capability 热插拔(冻结决议 M3)

### 4.8 Session 层扩展点(targetv2 新增)

**权威文档:** `4.12.0-session.md`;`0.1.2-interaction-loop.md`

**机制:**

- Session 生命周期 = 打开 Project / scratch Graph
- Session 拥有 undo / preview cache / events / run handle 引用
- CLI / Server 可以**不经过** Session,直通 Runtime

**规则:**

1. 新增一种交互场景(比如"多选节点批量参数修改预览"),归 Session 扩展
2. 不允许把"仅 UI 才关心的状态"塞进 Runtime 或 Cache
3. 不允许让 CLI / Server 被迫创建假 Session
4. Session 的 single-flight 权限是**引用**,不是**权限**

### 4.9 前端扩展点(targetv2 新增)

**权威文档:** `0.0.1-engine-contract.md`;`4.1.2-runtime.md`

**机制:**

- `EngineFacade`(同进程 API)
- `SessionHandle`(可选,给交互式前端)

**规则:**

1. 新增前端(如 Server、Desktop App、Mobile App)可以直接调用 `EngineFacade` 的 Runtime 路径
2. 如果新前端需要交互特权(preview 热路径、undo / redo、事件订阅),**可以**创建 `SessionHandle`,但这不是强制
3. 新前端**不必**经过 Session

---

## 5. 新增某物时,只改哪里

### 5.1 新增原子类型

例如: `Point3D`(3D 点)

应修改:

1. `0.1.0-models.md` §1.1 `AtomicType` 枚举
2. `Value` 对应变体
3. 类型匹配与显示规则
4. 必要时补相关节点与执行器支持

不应修改:

- `NodeDef` 结构本身
- `ExecutionRequest` 结构本身
- `ExecSignature` 公式

### 5.2 新增调度维度

例如: `CookingContext.dimensions["retry_count"]`

应修改:

1. `0.1.3-cooking-context.md` §6 首版维度清单(加一条)
2. Planner 中对应的枚举规则(如果需要特殊展开逻辑)

**不应修改:**

- `ExecSignature` 公式(字段已存在)
- `ExecutionRequest` 结构(字段已存在)
- `List<T>` 白名单

### 5.3 新增 Capability

例如: `VideoDecoder` 节点需要的 `video.decode`

应修改:

1. `0.1.4-capability.md` §2.1 首版 Capability 清单
2. 新建实现该 Capability 的 `Executor`
3. Runtime 启动代码注册新 Executor

**不应修改:**

- `NodeDef` 结构
- `ExecutorType` 枚举
- Runtime 路由核心代码

### 5.4 新增节点来源

例如: 插件目录节点

应修改:

1. 新建一种 `NodeSource`
2. 产出 `Vec<NodeRegistration>`
3. 启动时 `NodeRegistry::register(source)`

**不应修改:**

- `NodeManager` 的核心注册接口
- `NodeDef` 结构

### 5.5 新增动态参数能力

例如: API 节点模型切换后参数变化

应修改:

1. 对应 `SchemaProvider` 实现
2. 对应来源层实现

**不应修改:**

- `NodeDef` 静态字段集合

### 5.6 新增展示能力

例如: 参数改为只在 inspector 显示

应修改:

1. 默认展示规则协议
2. 或 `PresentationProvider`

**不应修改:**

- `ParamDef` 的核心语义字段

### 5.7 新增执行策略

例如: 未来新增 TTL 缓存策略

应修改:

1. `architecture-invariants.md`
2. `0.1.1-execution-models.md`
3. 对应 Runtime / CacheManager 模块文档

**不应修改:**

- `NodeDef` 之外另起一套隐藏执行配置系统

### 5.8 新增 MaterializationTarget

例如: 未来新增 `SharedMemory` 变体

应修改:

1. `0.1.5-value-materialization.md` §2.1 枚举
2. **所有** Value 类型的 `Materializable` 实现(至少处理新 target → `UnsupportedTarget`)
3. Runtime 的调度器对应分支

**不应修改:**

- 节点 schema
- Connection 校验
- `ExecSignature` 构造
- `NodeDef` 结构
- `CacheKey` / 其他签名相关结构

**诚实说明: 这是"轻破坏性变更"**

与其他扩展点不同(加 AtomicType / 加 Capability / 加 DimensionId 都是零破坏),加一个 `MaterializationTarget` 变体**会**要求每个 `Materializable` 实现至少做一次编译级更新——这是 Rust 封闭枚举的代价。

**缓解:** 新变体可以在每个 Value 类型默认返回 `Err(UnsupportedTarget)`,不需要立即实现所有目标。但编译器会强制每个 `match` 点都处理到,这本身就是一次"扫描+修改"。

**结论:** MaterializationTarget **不是完全开放集,是有限开放集**。加变体是"小幅度影响",不是"零成本"。本文档诚实承认这一点,不标榜为"完全填槽"。

### 5.9 新增前端

例如: HTTP Server

应修改:

1. 新建 Server 模块
2. 调用 `EngineFacade` 的 Runtime 路径
3. 事件序列化为 HTTP SSE 或 WebSocket

**不应修改:**

- `EngineFacade` 接口本身
- Session 层
- Runtime 内部

---

## 6. 禁止事项

为保持可扩展性,以下做法明确禁止:

1. 把布局、视口、折叠状态塞进 `Graph` 或 `Node`
2. 把动态参数 schema 写回静态 `NodeDef`
3. 把 GUI 渲染细节写进 `ParamDef` / `NodeDef`
4. 给某一类节点单独开"绕过统一执行轴"的执行捷径
5. 为批处理 / 序列帧另造一套与 `CookingContext` 平行的调度体系
6. 在节点 schema 或 Connection 校验中引用 `MaterializationTarget`
7. 把 `single-flight` 权限塞进 Session
8. 让 CLI / Server 被迫创建假 Session
9. 直接 `match executor_type` 做路由(应走 `CapabilityRegistry`)
10. 把 `cooking_context_hash` / `capability_version` 从 `ExecSignature` 公式里拿掉

---

## 7. 最小设计心智

以后新增功能时,先问自己:

1. 这是**新类型**吗? → §4.1
2. 这是**新节点静态契约**吗? → §4.2
3. 这是**新来源**吗? → §4.3
4. 这是**新执行策略 / 执行器**吗? → §4.4
5. 这是**新展示方式**吗? → §4.5
6. 这是**新调度维度**吗? → §4.6
7. 这是**新能力**吗? → §4.7
8. 这是**新交互场景**吗? → §4.8
9. 这是**新前端**吗? → §4.9

只要能把问题稳定归入这 9 类之一,就不应该推翻核心真源。

如果问题**无法**归入任何一类,说明真源清单不完整——应先补真源,而不是绕过。

---

## 8. 与其他文档的关系

- 总不变量: `architecture-invariants.md`
- 分层模型: `first-principles-architecture.md`
- 静态模型真源: `0.1.0-models.md`
- 执行模型真源: `0.1.1-execution-models.md`
- 交互回路真源: `0.1.2-interaction-loop.md`
- CookingContext: `0.1.3-cooking-context.md`
- Capability: `0.1.4-capability.md`
- Materialization: `0.1.5-value-materialization.md`
- 模块文档模板: `0.0.2-module-contract-template.md`

本文档的职责不是定义真源,而是帮助团队在新增能力时**知道该去改哪里,不该改哪里**。
