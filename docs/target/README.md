# nodeimg 目标架构

本目录是 nodeimg 目标架构的文档体系。目标架构描述系统**应当成为的样子**，是设计决策的集中记录，也是重构和新功能开发的参考基准。

---

## 文档索引

| 文件 | 主题 | 定位 |
|------|------|------|
| [00-context.md](./00-context.md) | 系统上下文 | nodeimg 系统与外部世界的边界和交互对象 |
| [01-overview.md](./01-overview.md) | 调用链总览 | 系统顶层架构，模块间如何协作，crate 如何依赖 |
| [02-service-layer.md](./02-service-layer.md) | 服务层内部 | EvalEngine 如何调度执行，服务层内部模块如何组织 |
| [03-executors.md](./03-executors.md) | 三路执行器 | 图像处理、AI 推理、模型 API 三路执行器的详细设计 |
| [04-transport.md](./04-transport.md) | Transport + server | Transport trait 实现协议透明性，server 接口定义远端通信契约 |
| [05-graph.md](./05-graph.md) | 节点图数据模型 | nodeimg-graph crate 的数据结构和操作 API |
| [06-app.md](./06-app.md) | App 层 | 逻辑/渲染分离架构，框架无关的核心逻辑设计 |
| [07-node-framework.md](./07-node-framework.md) | 节点架构 | 节点开发者体验——如何用最少的代码定义新节点 |
| [08-concurrency.md](./08-concurrency.md) | 并发模型 | 线程模型、并行策略、前端异步交互方式 |
| [09-error-handling.md](./09-error-handling.md) | 错误处理 | 分层 Error 类型、传播路径、前端展示策略 |
| [10-config.md](./10-config.md) | 配置系统 | 配置源、配置项清单、加载时机 |
| [11-cross-cutting.md](./11-cross-cutting.md) | 横切��注点 | 安全、日志、测试、性能、部署、版本 |
| [12-decisions.md](./12-decisions.md) | 设计决策表 | 所有架构决策的索引 |
| [13-risks.md](./13-risks.md) | 风险与技术债 | 已识别的风险和已知的技术债务 |
| [14-glossary.md](./14-glossary.md) | 术语表 | 项目中使用的领域术语定义 |
| [15-python-backend-protocol.md](./15-python-backend-protocol.md) | Python 后端协议 | AI 执行器与 Python 推理后端的通信协议、数据格式、生命周期管理 |

---

## 文档模板

所有文件（除 `README.md` 和 `12-decisions.md`）遵循统一模板：

```markdown
# 标题

> 一句话定位（这个模块是什么、解决什么问题）

## 总览

概述 + 核心 mermaid 图（一张图说清楚整体结构）

## [主题小节]

按逻辑展开：
- 设计意图和规则
- 必要时配 mermaid 图
- 关键接口/数据结构用 rust 代码块示意
- 约束和决策理由在上下文中内联说明
```

核心原则：**定位 → 总览图 → 按主题展开**（约束和决策内联，不单独列出）。

---

## Mermaid 配色规范（风格 A：柔和专业）

全局语义化 classDef，**所有文件统一使用**，不得自行引入其他颜色：

```
classDef frontend    fill:#6C9BCF,stroke:#5A89BD,color:#fff
classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
classDef api         fill:#E8A87C,stroke:#D6966A,color:#fff
classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff
classDef future      fill:#B0B8C1,stroke:#9EA6AF,color:#fff,stroke-dasharray:5 5
```

### 语义映射

| classDef | 用于 | 色调 |
|----------|------|------|
| `frontend` | GUI、CLI、App 层组件 | 柔蓝 |
| `transport` | Transport trait、协议层、HttpTransport | 淡紫 |
| `service` | EvalEngine、Registry、Cache、服务层组件 | 薄荷绿 |
| `ai` | AI 执行器、Python 后端、SDXL 相关 | 柔红 |
| `api` | 模型 API 执行器、云端 provider | 柔橙 |
| `foundation` | nodeimg-types、nodeimg-graph、基础数据结构 | 柔黄 |
| `compute` | nodeimg-gpu、GpuContext、shader、pipeline | 青绿 |
| `future` | 远期功能、未实现部分 | 灰色虚线 |

### 图表规则

- 只用 hex 颜色，不用颜色名
- 用 classDef 语义类名，不在节点上内联 `style`
- 每张图配简短文字说明，不依赖图自解释
- flowchart 方向：系统层级用 `TB`（上到下），时序流程用 `LR`（左到右）
