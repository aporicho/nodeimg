# docs/targetv2/ 文档体系重构设计

## 背景

当前 `docs/target/` 目录按主题编号组织（00-16 共 17 个文件 + README + mermaid.md），但一个文档里经常混合多个子模块的内容。需要重新拆分为一个子模块一个文档的结构，输出到 `docs/targetv2/`。

## 设计原则

1. **每个文档讲一个读者能独立理解的概念单元**
2. **100-300 行为理想大小**，50 行以下考虑合并，500 行以上考虑拆分
3. **文档间通过 markdown 链接引用**，同一概念不在多处重复描述
4. **保留所有 mermaid 图**，每个文档新增/保留一张 mermaid.md 风格的细粒度架构总览图
5. **保留所有设计决策 D01-D36 的内联说明**
6. **统一文档模板**：定位 → 架构总览图 → 按主题展开 → 设计决策

## 文件列表（24 个）

```
docs/targetv2/
├── README.md                    — 索引 + 全局架构图 + 关键约束        (~250 行)
│
├── 00-context.md                — 系统上下文与边界                    (~100)
├── 01-overview.md               — 调用链总览 + crate 依赖图           (~240)
│
├── 10-types.md                  — 数据类型体系 + Handle 边界           (~170)
├── 11-graph.md                  — 节点图数据模型 + 操作/查询 API       (~210)
│
├── 20-engine.md                 — EvalEngine 调度 + Cache + 模块图     (~180)
├── 21-executors.md              — 三路执行器 + AI 完整架构图           (~527)
├── 22-node-framework.md         — node! 宏 + 文件夹约定 + inventory    (~313)
│
├── 30-transport.md              — Transport trait + Local/Http/Server  (~283)
│
├── 40-app-overview.md           — App 架构 + 启动关闭 + 逻辑渲染分离   (~180)
├── 41-app-execution.md          — ExecutionManager + 自动执行 + 预览   (~164)
├── 42-app-editor.md             — Tab + Undo + 搜索 + 快捷键 + Widget (~264)
├── 43-app-theme.md              — 主题系统                            (~137)
├── 44-cli.md                    — nodeimg-cli exec/serve/batch        (~130)
│
├── 50-python-protocol.md        — Python 协议 + Handle + VRAM         (~332)
├── 51-python-lifecycle.md       — 进程生命周期 + 超时 + 并发 + 架构    (~288)
│
├── 60-concurrency.md            — 线程模型与并行策略                    (~80)
├── 61-error-handling.md         — 分层 Error 类型                      (~120)
├── 62-config.md                 — 配置系统                             (~90)
├── 63-cross-cutting.md          — 安全/观测/测试/性能/部署/版本        (~210)
│
├── 70-decisions.md              — 设计决策索引 D01-D36 (加链接)         (~60)
├── 71-risks.md                  — 风险与技术债 TD01-TD07                (~100)
├── 72-glossary.md               — 术语表                               (~35)
└── 73-node-catalog.md           — 节点目录 ~86 节点                    (~950)
```

编号区间含义：0x=全局、1x=基础层、2x=引擎层、3x=传输层、4x=前端层、5x=AI层、6x=横切、7x=附录。

## 内容迁移矩阵

### 需要拆分的文件

**02-service-layer.md → 20-engine.md + 10-types.md**

| 02 中的章节 | 目标文件 | 说明 |
|------------|---------|------|
| §1 EvalEngine 分发逻辑 | 20-engine.md | |
| §2 engine 内部模块图 | 20-engine.md | |
| §3 数据类型边界与 Handle 机制 | **10-types.md** | 本质是类型体系设计 |
| §4 Cache 架构 | 20-engine.md | |
| §5 ResultEnvelope | 20-engine.md | |

**05-graph.md → 10-types.md + 11-graph.md**

| 05 中的章节 | 目标文件 | 说明 |
|------------|---------|------|
| §1 总览 | 11-graph.md | 重写为 graph 专属总览 |
| §2 与 nodeimg-types 的边界 (D22) | 10-types.md | |
| §3 数据类型体系 (Value/DataTypeId/Constraint) | 10-types.md | |
| §4 核心数据结构 | 11-graph.md | |
| §5 图操作 API | 11-graph.md | |
| §6 图查询 API | 11-graph.md | |
| §7 序列化 | 11-graph.md | |

**06-app.md → 40/41/42/43/44**

| 06 中的章节 | 目标文件 | 说明 |
|------------|---------|------|
| §1 总览 + 架构图 | 40-app-overview.md | |
| §2 启动与关闭 | 40-app-overview.md | |
| §3 逻辑层/渲染层分离 | 40-app-overview.md | |
| §4 主题系统 (D36) | 43-app-theme.md | |
| §5 WidgetRegistry (D20) | 42-app-editor.md | |
| §6 ExecutionManager (D23, D35) | 41-app-execution.md | |
| §7 节点渲染器 (D24) | 42-app-editor.md | |
| §8 Undo/Redo (D32) | 42-app-editor.md | |
| §9 节点搜索 | 42-app-editor.md | |
| §10 键盘快捷键 (D33) | 42-app-editor.md | |
| §11 多项目 Tab (D34) | 42-app-editor.md | |
| §12 nodeimg-cli | 44-cli.md | |

**15-python-backend-protocol.md → 50/51**

| 15 中的章节 | 目标文件 | 说明 |
|------------|---------|------|
| §1-6 接口总览 + 4 个 HTTP API | 50-python-protocol.md | |
| §7 Handle 机制 | 50-python-protocol.md | |
| §8 VRAM 管理 | 50-python-protocol.md | |
| §9 进程生命周期 (6 状态) | 51-python-lifecycle.md | |
| §10 超时机制 | 51-python-lifecycle.md | |
| §11 并发模型 | 51-python-lifecycle.md | |
| §12 Python 端内部架构 | 51-python-lifecycle.md | |

**mermaid.md → 合并入 21-executors.md**

mermaid.md 的完整 AI 执行器架构图（267 行 mermaid 代码）+ SSE 事件流格式 + Handle ID 格式，全部合并到 21-executors.md 的 AI 执行器章节，替换 03 中的简化版。

### 不变的文件（改编号 + 补架构图）

| 源文件 | 目标文件 | 变化 |
|--------|---------|------|
| 00-context.md | 00-context.md | 不变 |
| 01-overview.md | 01-overview.md | 不变 |
| 03-executors.md | 21-executors.md | 合并 mermaid.md |
| 04-transport.md | 30-transport.md | 不变 |
| 07-node-framework.md | 22-node-framework.md | 补架构图 |
| 08-concurrency.md | 60-concurrency.md | 补架构图 |
| 09-error-handling.md | 61-error-handling.md | 补架构图 |
| 10-config.md | 62-config.md | 补架构图 |
| 11-cross-cutting.md | 63-cross-cutting.md | 不变 |
| 12-decisions.md | 70-decisions.md | 加文件路径链接 |
| 13-risks.md | 71-risks.md | 不变 |
| 14-glossary.md | 72-glossary.md | 不变 |
| 16-node-catalog.md | 73-node-catalog.md | 不变 |

## Handle 机制的三文件分布

Handle 是跨层概念，在 3 个文件中各讲一个层面：

| 文件 | 角度 | 内容 |
|------|------|------|
| 10-types.md | "Handle 是什么" | Value::Handle 定义、Rust/Python 边界图、DataTypeId 表 |
| 20-engine.md | "Handle 怎么缓存" | 豁免 LRU 策略(D04)、失效时调用 release |
| 50-python-protocol.md | "Handle 怎么跨进程管理" | ID 格式、创建/释放 API、VRAM 恢复 |

每处通过 markdown 链接互相引用，不重复内容。

## 设计决策 D01-D36 归属

| 文件 | 承载的决策 |
|------|-----------|
| 10-types.md | D22 |
| 20-engine.md | D03, D04, D05, D12, D19 |
| 21-executors.md | D11, D25, D29, D31 |
| 22-node-framework.md | D18, D21, D24 |
| 30-transport.md | D10 |
| 41-app-execution.md | D23, D35 |
| 42-app-editor.md | D20, D32, D33, D34 |
| 43-app-theme.md | D36 |
| 50-python-protocol.md | D27, D28 |
| 51-python-lifecycle.md | D26, D30 |
| 60-concurrency.md | D06, D07 |
| 61-error-handling.md | D01, D02 |
| 62-config.md | D08, D09 |
| 63-cross-cutting.md | D13, D14, D15, D16, D17 |
| 70-decisions.md | D01-D36 索引表 + 各文件链接 |

36 个决策全部有归属，无遗漏。

## 文档模板

每个文档遵循统一结构：

```markdown
# {模块名}

> 定位：一句话说明这个模块是什么、为什么存在。

## 架构总览

​```mermaid
flowchart TB
    classDef service     fill:#6DBFA0,stroke:#5BAD8E,color:#fff
    classDef transport   fill:#A78BCA,stroke:#9579B8,color:#fff
    classDef ai          fill:#E88B8B,stroke:#D67979,color:#fff
    classDef foundation  fill:#E8CC6E,stroke:#D6BA5C,color:#333
    classDef compute     fill:#6DB8AD,stroke:#5BA69B,color:#fff

    %% 使用 subgraph 划分内部子系统
    %% 子图内用多个节点展示组件的输入输出和内部流程
    %% 节点用 ["名称\n说明"] 格式
​```

## {主题 1}

## {主题 2}

...

## 设计决策

- **D{xx}**: {内联说明}
```

Mermaid 图要求：
- 统一 5 色 classDef（service 绿 / transport 紫 / ai 红 / foundation 黄 / compute 青）
- 使用 subgraph 划分子系统
- 子图内展示组件内部结构和交互关系
- 节点格式 `["名称\n说明"]`
- 风格对标 `docs/target/mermaid.md`

## 交叉引用关系

```
10-types ←── 11-graph, 20-engine
20-engine ←── 21-executors, 41-app-execution
30-transport ←── 20-engine, 40-app-overview, 41-app-execution
50-python-protocol ←── 21-executors, 20-engine
```

## 关键设计决策回顾

### 为什么 02 不拆成 3 个文件

02 只有 240 行，拆成 eval/cache/registry 各 60-80 行过于碎片化。EvalEngine 调度和 Cache 是 engine 内部的有机整体——调度完成后结果写入 Cache，读者需要在一处看到完整执行流。Registry 的详细内容已在 22-node-framework.md 中覆盖。

### 为什么 04 不拆

283 行不大。Transport trait 的方法签名和 Server 的 HTTP 端点一一对应，读者需要同时看到两者的映射关系。拆开后两个文件各 ~140 行且需要频繁交叉引用。

### 为什么 Python 拆成"协议+Handle+VRAM"和"生命周期"

Handle 机制只有 70 行，独立成文太薄。而进程生命周期（6 状态 + 超时 + 并发 + 架构）有 238 行，是一个完整的运行时管理主题。拆分边界是：一个讲"怎么通信"，一个讲"怎么管理进程"。

### 为什么 21-executors.md 允许 527 行

其中 267 行是 mermaid.md 的完整 AI 执行器架构图代码，37 行是 SSE/Handle ID 参考格式。主体文字约 220 行。三路执行器是一个完整概念，读者需要在一处看到对比和混合图执行示例。
