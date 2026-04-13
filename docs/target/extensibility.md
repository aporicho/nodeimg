# 可扩展骨架

> 本文档说明 nodeimg 引擎如何在**不推翻核心真源**的前提下，持续新增类型、节点、来源、执行行为与展示行为。

> 它**不替代** `architecture-invariants.md`、`0.1.0-models.md`、`0.1.1-execution-models.md`，而是回答一个更具体的问题：**以后要加新东西时，只改哪里。**

---

## 1. 目标

nodeimg 的可扩展性目标不是“预先支持所有未来功能”，而是：

1. 新增一种数据类型时，不需要推翻节点模型；
2. 新增一种节点来源时，不需要重写注册系统；
3. 新增一种执行器或策略时，不需要污染静态模型；
4. 新增一种展示方式时，不需要把 GUI（图形用户界面，GUI）细节塞进真源；
5. 批处理、序列帧、结构化数据等新能力，优先通过既有扩展点接入，而不是额外再造一套体系。

---

## 2. 顶层定位

nodeimg 的长期目标不是“单一图像滤镜编辑器”，而是：

> **面向新时代图像 / 视频 / 矢量图形 / 设计排版的自动化工作室引擎。**

这意味着骨架必须同时容纳：

1. **媒体处理**：图像、视频、音频；
2. **图形设计**：矢量图形、模板、版式参数；
3. **自动化编排**：批处理、序列帧、结构化参数；
4. **AI 增强**：生成、理解、重写、辅助设计；
5. **未来扩展**：新增类型、新节点来源、新执行器，而不推翻核心真源。

因此，骨架设计原则是：

- **先服务图像与序列帧**；
- **明确为视频 / 音频 / 矢量 / 设计排版留标准扩展口**；
- **不在第一版实现所有能力，但必须让新增能力有固定接入路径。**

---

## 3. 五个扩展点

### 3.1 类型扩展点

权威文档：`0.1.0-models.md`

这一层固定以下骨架：

- `AtomicType`（原子类型，AtomicType）
- `DataType = Atomic(T) | List(T)`（数据类型，DataType）
- `Value`（值，Value）

规则：

1. 新增一种一等数据能力，优先表现为**新增原子类型**；
2. 批处理 / 序列能力统一走 `List<T>`（列表类型，List<T>）；
3. 不为每种批量语义另造一种类型系统；
4. 新增原子类型是**破坏性变更**，必须先修改真源文档。

### 3.2 静态节点扩展点

权威文档：`0.1.0-models.md`

这一层固定以下骨架：

- `NodeDef`（节点定义，NodeDef）
- `InputPinDef` / `OutputPinDef`（输入/输出引脚定义，InputPinDef / OutputPinDef）
- `ParamDef`（参数定义，ParamDef）
- `Constraint`（约束，Constraint）

规则：

1. 所有节点都必须先落到静态契约层；
2. `NodeDef` 只承载静态身份、执行归类、纯性、静态引脚、静态参数、默认执行策略和轻展示元数据；
3. 动态参数 schema（参数结构，schema）、GUI 渲染细节、运行时状态不得进入 `NodeDef`。

### 3.3 来源扩展点

权威文档：`0.1.0-models.md`

统一注册骨架：

```rust
NodeSource -> Vec<NodeRegistration>

struct NodeRegistration {
    static_def: NodeDef,
    schema_provider: Option<SchemaProvider>,
    presentation_provider: Option<PresentationProvider>,
}
```

规则：

1. 所有节点来源都必须先产出统一 `NodeRegistration`（节点注册载体，NodeRegistration）；
2. 一个来源可以返回多个节点；
3. `schema_provider` 与 `presentation_provider` 都是可选层，且按**节点类型**提供；
4. 不允许给来源层开“自定义执行绑定”后门。

这意味着：

- Rust builtins（内置节点，builtins）
- Python 节点扫描
- API Provider（提供方，Provider）
- 未来插件节点目录

都应通过同一来源接口接入。

### 3.4 执行扩展点

权威文档：`0.1.1-execution-models.md`

这一层固定以下骨架：

- `ExecutionRequest`（执行请求，ExecutionRequest）
- `ExecutionOutputs`（执行输出，ExecutionOutputs）
- `ExecSignature`（执行签名，ExecSignature）
- `CacheKey`（缓存键，CacheKey）
- `ExecutionPolicy`（执行策略，ExecutionPolicy）

规则：

1. 执行器统一接口，不因节点来源不同而分裂；
2. 执行器的长期设计目标是**可增**，而不是固定只有首版的几个执行器类别；
3. 新节点类型只要能进入 `ExecutionInputs` / `ExecutionOutputs`，就不应重写调度器骨架；
4. 缓存、产物、触发、重试等策略统一经 `ExecutionPolicy` 暴露；
5. `ExecSignature` 与 `CacheKey` 严格分离，避免把运行时噪声写进语义身份。

这意味着：

- 现有执行器类别（如图像 / AI / API）是**首版集合**，不是终局集合；
- 未来新增 `VideoExecutor`（视频执行器，VideoExecutor）、`AudioExecutor`（音频执行器，AudioExecutor）、`VectorExecutor`（矢量执行器，VectorExecutor）、`LayoutExecutor`（排版执行器，LayoutExecutor）时，应接入统一执行轴，而不是旁路实现；
- 节点描述统一、值类型统一、执行器可插拔。

### 3.5 展示扩展点

权威文档：`0.1.0-models.md`（展示元数据边界）+ 后续 GUI 文档

这一层固定以下原则：

- 参数语义在 `ParamDef` 中定义；
- 参数显示位置、节点特例渲染、折叠规则等由 `PresentationProvider`（展示提供器，PresentationProvider）决定；
- 默认展示规则必须有统一协议，不能散在 GUI 代码中。

规则：

1. 轻展示元数据（如 `description`、`tags`、`icon`）可进 `NodeDef`；
2. GUI 私有渲染细节不得进入真源；
3. 新增一种特殊节点展示方式时，优先通过 `PresentationProvider` 覆盖默认规则，而不是给 `NodeDef` 新增 GUI 专属字段。

---

## 4. 新增某物时，只改哪里

### 4.1 新增原子类型

例如：`VectorGraphic`（矢量图形，VectorGraphic）

应修改：

1. `AtomicType`
2. `Value` 对应变体
3. 类型匹配与显示规则
4. 必要时补相关节点与执行器支持

不应修改：

- `NodeDef` 结构本身
- `ExecutionRequest` 结构本身

### 4.2 新增列表支持

例如：开放 `List<Audio>`（音频列表，List<Audio>）

应修改：

1. `0.1.0-models.md` 中 `List<T>` 白名单
2. 列表执行规则
3. 对应节点的列表处理语义

不应修改：

- `DataType` 的构造方式

### 4.3 新增节点来源

例如：未来新增“插件目录节点”

应修改：

1. 新建一种 `NodeSource`
2. 产出 `Vec<NodeRegistration>`

不应修改：

- `NodeManager` 的核心注册接口
- `NodeDef` 结构

### 4.4 新增动态参数能力

例如：API 节点模型切换后参数变化

应修改：

1. `SchemaProvider`
2. 对应来源层实现

不应修改：

- `NodeDef` 静态字段集合

### 4.5 新增展示能力

例如：参数改为只在 inspector（检查面板，inspector）显示

应修改：

1. 默认展示规则协议
2. 或 `PresentationProvider`

不应修改：

- `ParamDef` 的核心语义字段

### 4.6 新增执行策略

### 4.7 新增执行器类别

例如：未来新增 `VectorExecutor`（矢量执行器，VectorExecutor）

应修改：

1. 执行器注册表 / 执行器枚举或 kind（种类，kind）定义
2. 对应执行器实现
3. 必要时补相关节点的 `executor_type` / 执行归类

不应修改：

- `NodeDef` 静态字段集合本身
- `ExecutionRequest` 结构本身
- 图模型骨架

例如：未来新增 TTL（生存时间，TTL）缓存策略

应修改：

1. `architecture-invariants.md`
2. `0.1.1-execution-models.md`
3. 对应调度器 / 缓存模块文档

不应修改：

- `NodeDef` 之外另起一套隐藏执行配置系统

---

## 5. 禁止事项

为保持可扩展性，以下做法明确禁止：

1. 把布局、视口、折叠状态塞进 `Graph`（图，Graph）或 `Node`（节点实例，Node）
2. 把动态参数 schema 写回静态 `NodeDef`
3. 把 GUI 渲染细节写进 `ParamDef` / `NodeDef`
4. 给某一类节点单独开“绕过统一执行轴”的执行捷径
5. 为批处理/序列帧另造一套与 `List<T>` 平行的数据体系

---

## 6. 最小设计心智

以后新增功能时，先问自己：

1. 这是**新类型**吗？
2. 这是**新节点静态契约**吗？
3. 这是**新来源**吗？
4. 这是**新执行策略/新执行语义**吗？
5. 这是**新执行器类别**吗？
6. 这是**新展示方式**吗？

只要能把问题稳定归入这 6 类之一，就不应该推翻核心真源。

---

## 7. 与其他文档的关系

- 总不变量：`architecture-invariants.md`
- 静态模型真源：`0.1.0-models.md`
- 执行模型真源：`0.1.1-execution-models.md`
- 模块文档模板：`0.0.2-module-contract-template.md`

本文档的职责不是定义真源，而是帮助团队在新增能力时**知道该去改哪里，不该改哪里**。
