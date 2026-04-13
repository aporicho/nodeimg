# 文档规范

> targetv2 文档体系的编写约定。

targetv2 是 target 的**继任体系**,在现有冻结真源基础上新增"机制一次设计好,内容持续开放"约束。详细背景见 `README.md`。

---

## 文件命名

### 编号格式

```
x.x.x-name.md
```

| 层级 | 格式 | 示例 |
|------|------|------|
| 顶层模块 | `x.0.0` | `2.0.0-gui.md` |
| 子模块 | `x.x.0` | `2.1.0-canvas.md` |
| 子子模块 | `x.x.x` | `4.11.1-provider.md` |

无编号的文件为全局文档(如 `spec.md`、`README.md`、`architecture-invariants.md`、`extensibility.md`、`first-principles-architecture.md`、`roadmap.md`),不属于任何模块层级。

### 状态后缀

在编号和文件扩展名之间加状态标注:

```
x.x.x-name.{status}.md
```

| 后缀 | 含义 |
|------|------|
| `.impl` | 规格成熟,待代码实现 |
| `.future` | 规划中,未来实现 |
| `.draft` | 设计未成熟,仍在讨论 |

无状态后缀 = 总览 / 索引类文档,或已在 `architecture-invariants.md` 的 Tier 1 真源层冻结。

---

## 编号段位含义

| 段位 | 内容范围 |
|------|---------|
| `0.x.x` | 全局真源层: 架构、契约、共享数据模型、交互回路、能力层、CookingContext、Materialization |
| `1.x.x` | AI 操作员 |
| `2.x.x` | GUI |
| `3.x.x` | CLI |
| `4.x.x` | 节点引擎(Planner / Runtime / Capability / Session / 各 Manager / Executor) |
| `5.x.x` | 项目文件 |
| `6.x.x` | Python 后端 |
| `7.x.x` | Renderer |

---

## 事实标记

所有 targetv2 文档必须区分三类内容:

| 标记 | 含义 | 使用规则 |
|------|------|---------|
| **既有决议** | 已有明确出处的决策 | 默认状态,无需标记 |
| **推断** | 作者根据上下文推出的、未经显式确认的结论 | 以 "推断:..." 开头,或加 `[推断]` 前缀 |
| **待确认** | 已识别但尚未决议的问题 | 以 "待确认:..." 开头,或加 `[待确认]` 前缀 |

**绝不允许把推断冒充既有决议写入文档。**

---

## 占位内容标记

targetv2 引入"占位机制":某些字段/类型在第一版**必须存在**,但首版取值可为恒零或空。这类条目必须显式标注:

| 标记 | 含义 |
|------|------|
| `[占位]` | 机制已冻结,首版实现为空/恒零/无执行器支持 |
| `[首版支持]` | 机制已冻结,首版有完整实现 |
| `[演进项]` | 机制已冻结,首版部分实现,未来扩展 |

示例:

- `AtomicType::Video` `[占位]` —— 类型系统首版收录,无执行器支持
- `CookingContext::dimensions` `[占位]` —— 首版为空 HashMap
- `ExecSignature.capability_version` `[占位]` —— 首版恒为 0

占位项在未来填充内容时 **不视为破坏性变更**,因为机制本身已冻结。

**与 `[推断]` 标记的关系:**

`[占位]` 和 `[推断]` 是**两组正交标记**,可以叠加使用:

- `[占位]` 关注 "首版实现状态"——"机制已冻结但内容为空"
- `[推断]` 关注 "决议来源"——"此字段/规则是作者推断,未经讨论决议"

同一个字段可以同时标注,例如:

- `VideoValue` `[占位][推断]` —— 占位类型(机制已冻结);实现为 marker struct(推断的实现细节,未经决议)
- `cooking_sensitivity` 字段 `[推断]` —— 字段本身是 targetv2 新增的推断设计,尚未经过代码验证

**使用规则:**

- 关键字段首版必须标记一种或多种
- 不允许"既不是既有决议也不标注"的状态——这会造成审查盲区
- `[推断]` 的条目在进入代码实现前应该升级为"既有决议"(通过讨论 / review / 验证)

---

## Mermaid 图规范

### 全局设置

```
%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%
flowchart TB
```

### 节点分类

| 分类 | 颜色 | 说明 |
|------|------|------|
| 内部模块 | 绿 `fill:#6DBFA0` | 当前文档描述的模块自身的前端组件 |
| 逻辑模块 | 蓝 `fill:#5B9BD5` | 当前文档描述的模块自身的后端 / 逻辑层组件 |
| 数据模型 | 橙 `fill:#F4B183` | 纯数据结构,无行为逻辑 |
| 外部模块 | 灰 `fill:#B0B8C1` | 不属于当前模块,但有交互的外部依赖 |

```
classDef internal fill:#6DBFA0,stroke:#5BAD8E,color:#fff
classDef logic fill:#5B9BD5,stroke:#4A8AC4,color:#fff
classDef data fill:#F4B183,stroke:#E09B6D,color:#fff
classDef external fill:#B0B8C1,stroke:#9EA6AF,color:#fff
```

### 连线

- `-->` 单向
- `<-->` 双向
- `-->|"标签"|` 带标签,2–4 字

### 约束

- 方向 `TB`,曲线 `basis`
- 中文标签,15 字以内
- 单张图不超过 15–20 个节点

---

## 真源引用规范

所有 `4.x.x` 模块文档引用共享模型时,必须通过显式路径引用,不允许重复定义:

- 类型与节点模型 → `0.1.0-models.md`
- 执行模型 → `0.1.1-execution-models.md`
- 交互回路 → `0.1.2-interaction-loop.md`
- CookingContext → `0.1.3-cooking-context.md`
- Capability → `0.1.4-capability.md`
- Materialization → `0.1.5-value-materialization.md`
- 真源与职责边界 → `architecture-invariants.md`
- 分层与 Session → `first-principles-architecture.md`

任何模块文档内部**不得重新定义**上述文档中的类型或规则。
