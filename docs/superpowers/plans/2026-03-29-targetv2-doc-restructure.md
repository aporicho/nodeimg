# docs/targetv2/ 文档体系重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 docs/target/ 的 19 个文件重构为 docs/targetv2/ 的 26 个文件，按子模块拆分，每个文档一个概念单元。

**Architecture:** 读取源文件 → 按 spec 迁移矩阵拆分内容 → 写入新文件 → 补充架构图和交叉引用链接。不修改 docs/target/（保留原版）。

**Tech Stack:** Markdown, Mermaid

**Spec:** `docs/superpowers/specs/2026-03-29-targetv2-doc-restructure-design.md`

---

## 执行策略

26 个文件按依赖顺序分为 10 个 task。先创建不需要拆分的简单文件（直接复制改编号），再处理需要拆分的复杂文件，最后写 README 和更新决策索引。

每个 task 产出 1-4 个文件，完成后立即 commit。

## 统一规范

**文档模板：** 每个文档遵循 `定位 → 架构总��(mermaid) → 按主题展开` 结构。

**Mermaid 配色：** 所有图统一使用以下 classDef（来自 docs/target/README.md）：

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

**架构图风格：** 对标 `docs/target/mermaid.md`——使用 subgraph 划分内部子系统，子图内用多个节点展示组件的输入输出，节点用 `["名称\n说明"]` 格式。

**交叉引用：** 文档间通过相对路径 markdown 链接引用，如 `详见 [10-types.md](./10-types.md)`。

---

### Task 1: 创建目录 + 直接复制的 8 个文件

不需要拆分的文件，直接复制并改编号。内容不变，只更新内部的文件路径引用。

**Files:**
- Create: `docs/targetv2/` 目录
- Create: `docs/targetv2/00-context.md` ← 复制 `docs/target/00-context.md`
- Create: `docs/targetv2/01-overview.md` ← 复制 `docs/target/01-overview.md`
- Create: `docs/targetv2/63-cross-cutting.md` ← 复制 `docs/target/11-cross-cutting.md`
- Create: `docs/targetv2/71-risks.md` ← 复制 `docs/target/13-risks.md`
- Create: `docs/targetv2/72-glossary.md` ← 复制 `docs/target/14-glossary.md`
- Create: `docs/targetv2/73-node-catalog.md` ← 复制 `docs/target/16-node-catalog.md`
- Create: `docs/targetv2/30-transport.md` ← 复制 `docs/target/04-transport.md`

- [ ] **Step 1: 创建目录**

```bash
mkdir -p docs/targetv2
```

- [ ] **Step 2: 复制 7 个不变文件**

```bash
cp docs/target/00-context.md docs/targetv2/00-context.md
cp docs/target/01-overview.md docs/targetv2/01-overview.md
cp docs/target/11-cross-cutting.md docs/targetv2/63-cross-cutting.md
cp docs/target/13-risks.md docs/targetv2/71-risks.md
cp docs/target/14-glossary.md docs/targetv2/72-glossary.md
cp docs/target/16-node-catalog.md docs/targetv2/73-node-catalog.md
cp docs/target/04-transport.md docs/targetv2/30-transport.md
```

- [ ] **Step 3: 更新 01-overview.md 中��内部链接**

将 `01-overview.md` 中引用旧文件名的链接（如 `02-service-layer.md`、`06-app.md`）更新为新文件名。

- [ ] **Step 4: 更新 30-transport.md 中的内部链接**

将 `04-transport.md` → `30-transport.md` 中引用的 `02-service-layer.md` 改为 `20-engine.md`，`06-app.md` 改为 `40-app-overview.md` 等。

- [ ] **Step 5: 更新其他复制文件的内部链接**

检查 63-cross-cutting.md、73-node-catalog.md 中的交叉引用，更新为新文件名。

- [ ] **Step 6: Commit**

```bash
git add docs/targetv2/
git commit -m "docs(targetv2): 创建目录 + 复制 7 个不变文件"
```

---

### Task 2: 补架构图的 4 个文件（60/61/62/24）

这些文件内容基本不变，但需要补充 mermaid.md 风格的细粒度架构总览图。

**Files:**
- Create: `docs/targetv2/60-concurrency.md` ← 基于 `docs/target/08-concurrency.md`，补架构图
- Create: `docs/targetv2/61-error-handling.md` ← 基于 `docs/target/09-error-handling.md`，补架构图
- Create: `docs/targetv2/62-config.md` ← 基于 `docs/target/10-config.md`，补架构图
- Create: `docs/targetv2/24-node-framework.md` ← 基于 `docs/target/07-node-framework.md`，补架构图

- [ ] **Step 1: 创建 60-concurrency.md**

读取 `docs/target/08-concurrency.md`，保留原有内容，补充一张细粒度架构图展示线程模型：UI 主线程 → ExecutionManager → 后台执行线程（rayon 线程池）→ channel 回推。更新内部链接指向新文件名。

- [ ] **Step 2: 创建 61-error-handling.md**

读取 `docs/target/09-error-handling.md`，保留原有内容，补充一张架构图展示三层 Error（NodeError → ExecutionError → TransportError）的分层结构和传播路径。更新内部链接。

- [ ] **Step 3: 创建 62-config.md**

读取 `docs/target/10-config.md`，保留原有内容，补充一张架构图展示配置优先级链路（CLI → ENV → TOML → 默认值）和 AppConfig 的消费者。更新内部链接。

- [ ] **Step 4: 创建 24-node-framework.md**

读取 `docs/target/07-node-framework.md`，保留全部内容（已有图）。检查已有图是否符合 mermaid.md 风格（subgraph + classDef），不符合则增强。更新内部链接：`02-service-layer.md` → `20-engine.md`，`06-app.md` → `42-app-editor.md`（WidgetRegistry 部分），`15-python-backend-protocol.md` → `50-python-protocol.md`。

- [ ] **Step 5: Commit**

```bash
git add docs/targetv2/60-concurrency.md docs/targetv2/61-error-handling.md docs/targetv2/62-config.md docs/targetv2/24-node-framework.md
git commit -m "docs(targetv2): 60/61/62/24 — 补架构图 + 更新链接"
```

---

### Task 3: 拆分 05-graph.md → 10-types.md + 11-graph.md

**Source:** `docs/target/05-graph.md` + `docs/target/02-service-layer.md` §3（部分）

**Files:**
- Create: `docs/targetv2/10-types.md`
- Create: `docs/targetv2/11-graph.md`

- [ ] **Step 1: 创建 10-types.md**

内容来源：
1. 05§2（第 32-43 行）：与 nodeimg-types 的边界 (D22) — 表格，什么属于 types 什么属于 graph
2. 05§3（第 49-116 行）：数据类型体系 — Value 枚举、DataTypeId 表、Constraint 枚举
3. 02§3（第 146-177 行）：数据类型边界 mermaid 图（Rust 类型 vs Python 类型）+ "Rust 侧不直接管理 Python 对象的内存，Handle 只是一个不透明的字符串 ID"

结构：
```
# 数据类型体系
> 定位：nodeimg-types crate 的核心类型定义——Value、DataType、Constraint，以及 Rust/Python 类型边界。
## 架构总览 (新绘 mermaid 图：展示 Value 枚举的分类——Rust 可理解类型 vs Handle 引用类型)
## 与 nodeimg-graph 的边界 (D22) ← 05§2
## Value 枚举 ← 05§3 前半
## DataTypeId 与 Python 专属类型 ← 05§3 DataTypeId 表
## Constraint 枚举 ← 05§3 后半
## Rust / Python 类型边界 ← 02§3 mermaid 图 + "Handle 是不透明 ID"
```

添加链接：`详见 [20-engine.md](./20-engine.md)（Handle 缓存策略）` 和 `详见 [50-python-protocol.md](./50-python-protocol.md)（Handle 跨进程管理）`。

- [ ] **Step 2: 创建 11-graph.md**

内容来源：05§1（总览，重写为 graph 专属）、05§4-7（核心数据结构、���操作 API、图查询 API、序列化）

结构：
```
# 节点图数据模型
> 定位：nodeimg-graph crate——节点图的数据结构和操作 API。
## 架构总览 (保留 05§1 的 mermaid 图)
## 核心数据结构 ← 05§4
## 图操作 API ← 05§5
## 图查询 API ← 05§6
## 序列化 ← 05§7
```

更新链接：`nodeimg-types` → `[10-types.md](./10-types.md)`，`engine 层的 GraphController` → `[40-app-overview.md](./40-app-overview.md)`。

- [ ] **Step 3: Commit**

```bash
git add docs/targetv2/10-types.md docs/targetv2/11-graph.md
git commit -m "docs(targetv2): 10-types + 11-graph — 拆分 05-graph"
```

---

### Task 4: 拆分 02-service-layer.md + 03-executors.md → 20/21/22/23

**Source:** `docs/target/02-service-layer.md`, `docs/target/03-executors.md`, `docs/target/mermaid.md`

**Files:**
- Create: `docs/targetv2/20-engine.md`
- Create: `docs/targetv2/21-executor-image.md`
- Create: `docs/targetv2/22-executor-ai.md`
- Create: `docs/targetv2/23-executor-api.md`

- [ ] **Step 1: 创建 20-engine.md**

内容来源：
1. 02§1（第 59-84 行）：EvalEngine 分发逻辑 + ExecutorType 枚举
2. 02§2（第 87-142 行）：engine 内部模块图
3. 02§3（第 179-184 行，仅 Handle 生命周期规则 4 条）：接在 Cache 章节之后
4. 02§4（第 189-216 行）：Cache 架构（ResultCache + TextureCache）
5. 02§5（第 220-240 行）：ResultEnvelope
6. 03 总览图（第 7-34 行）：三路路由图
7. 03 混合图执行示例（第 135-194 行）：sequence diagram + D31 论证

结构：
```
# 执行引擎
> 定位：nodeimg-engine 的核心调度——EvalEngine 如何拓扑排序、按类型分发、管理缓存。
## 架构总览 (保留 02 的总览 mermaid 图)
## EvalEngine 分发逻辑 ← 02§1 + 03 总览图
## engine 内部模块图 ← 02§2
## Cache 架构 ← 02§4
### Handle 生命周期规则 ← 02§3 的 4 条规则（不含 mermaid 图，那张图在 10-types.md）
## ResultEnvelope ← 02§5
## 混合图执行示例 ← 03 的 sequence diagram
### 为什么选择逐节点分发 (D31) ← 03 ��尾论证
```

添加链接到 21/22/23（各执行器详设）、10-types.md（类型边界）、50-python-protocol.md（Handle 释放协议）。

- [ ] **Step 2: 创建 21-executor-image.md**

内容来源：03 图像处理执行器章节（第 39-50 行）

结构：
```
# 图像处理执行器
> 定位：本地 GPU + CPU 协作的图像处理执行路径——像素运算走 GPU，文件 I/O 和分析走 CPU。
## 架构总览 (新绘 mermaid 图：展示 GPU pipeline 和 CPU 路径的分派、shader 加载、workgroup 分发)
## GPU 路径 ← 03 "GPU 路径" 段落
## CPU 路径 ← 03 "CPU 路径" 段落
## 分派规则 ← 03 "分派规则" 段落
## 设计决策
- **D29**: GPU 优先
```

- [ ] **Step 3: 创建 22-executor-ai.md**

内容来源：
1. 03 AI 执行器章节（第 56-131 行）：Rust/Python 两侧职责 + Handle 存储概念 + 进度反馈概念
2. mermaid.md（第 3-267 行）：精简版——保留 Rust 侧 5 阶段 + Python 侧简化 + 取消/超时子图，去掉 VRAM_RECOVERY（第 164-202 行）和 LIFECYCLE（第 204-266 行）子图

结构：
```
# AI 执行器
> 定位：Rust 与 Python 推理后端之间的协议桥——HTTP + SSE 通信、Handle 透传、取消与超时。
## 架构总览 (mermaid.md 精简版图)
## Rust 侧工作流 ← 03 AI 执行器 mermaid 图的 Rust 部分描述
## Python 侧简述 ← 概要 + 链接到 50-python-protocol.md 和 51-python-lifecycle.md
## Handle 存储 ← 03 "Handle 存储" 段落（概要，详细定义链接到 50）
## 进度反馈 ← 03 "进度反馈" 段落（概要，SSE 格式详见 50）
## 设计决策
- **D25**: AI 节点通过 HTTP + SSE 与 Python 通信
```

- [ ] **Step 4: 创建 23-executor-api.md**

内容来源：03 模型 API 执行器章节（第 198-223 行）

结构：
```
# 模型 API 执行器
> 定位：云端大厂推理 API（OpenAI、Stability AI、通义千问）的执行路径——无状态调用、认证、限流。
## 架构总览 (新绘 mermaid 图：展示 ApiProvider trait + 多厂商实现 + 认证/限流)
## 与 AI 执行器的对比 ← 03 对比表
## ApiProvider trait ← 03 trait 定义 + D11 决策
## 设计决策
- **D11**: ���一 ApiProvider trait 隔离厂商差异
```

- [ ] **Step 5: Commit**

```bash
git add docs/targetv2/20-engine.md docs/targetv2/21-executor-image.md docs/targetv2/22-executor-ai.md docs/targetv2/23-executor-api.md
git commit -m "docs(targetv2): 20/21/22/23 — 拆分 engine + 三路执行器"
```

---

### Task 5: 拆分 06-app.md → 40/41/42/43/44

**Source:** `docs/target/06-app.md`（764 行）

**Files:**
- Create: `docs/targetv2/40-app-overview.md`
- Create: `docs/targetv2/41-app-execution.md`
- Create: `docs/targetv2/42-app-editor.md`
- Create: `docs/targetv2/43-app-theme.md`
- Create: `docs/targetv2/44-cli.md`

- [ ] **Step 1: 创建 40-app-overview.md**

内容来源：06§1（第 1-74 行）总览 + 架构图、06§2（第 77-127 行）启动与关闭、06§3（第 130-148 行）逻辑层/渲染层分离

结构：
```
# App 层架构
> 定位：nodeimg-app 的逻辑/渲染分离架构——框架无关的核心逻辑设计和启动关闭流程。
## 架构总览 (保留 06§1 mermaid 图)
## 逻辑层/渲染层分离 ← 06§3
## 启动序列 ← 06§2 启动部分（含 mermaid 图）
## 关闭序列 ← 06§2 关闭部分
```

添加链接：执行管理 → [41](./41-app-execution.md)，编辑交互 → [42](./42-app-editor.md)，主题 → [43](./43-app-theme.md)，CLI → [44](./44-cli.md)。
更新 Transport 链接 → [30-transport.md](./30-transport.md)，Python 链接 → [50-python-protocol.md](./50-python-protocol.md)。

- [ ] **Step 2: 创建 41-app-execution.md**

内容来源：06§6（第 303-426 行）ExecutionManager + 自动执行策略 + 预览更新链路

结构：
```
# 执行管理
> 定位：ExecutionManager——图执行的全生命周期管理：提交、进度追踪、取消、结果收割、自动执行策略。
## 架构总览 (保留 06§6 的 ExecutionManager flowchart)
## 提交流程 ← 06§6 "提交流程"
## 进度消费（channel 模式）← 06§6 事件表
## 取消机制 ← 06§6 "取消机制"
## 结果存放 ← 06§6 "结果存放"
## 自动执行策略 (D35) ← 06§6 "自动执行策略" + 防抖 + Ctrl+Enter + 混合图示例
## 预览更新链路 ← 06§6 sequenceDiagram
## 设计决策
- **D23**: ExecutionManager 基于 channel 的进度消费
- **D35**: Image 节点自动执行 200ms 防抖，AI/API 节点手动 Ctrl+Enter
```

- [ ] **Step 3: 创建 42-app-editor.md**

内容来源：06§5（第 262-300 行）WidgetRegistry、06§7（第 429-491 行）节点渲染器、06§8（第 495-531 行）Undo、06§9（第 534-562 行）搜索、06§10（第 566-590 行）快捷键、06§11（第 594-663 行）多Tab

结构：
```
# 编辑器交互
> 定位：App 的编辑交互子系统——项目管理、参数编辑、撤销、搜索、快捷键。
## 架构总览 (新绘 mermaid 图：展示 Tab/Undo/Widget/Search/Keybinding 的关系)
## 多项目 Tab (D34) ← 06§11（含 mermaid 图 + 代码）
## Undo/Redo (D32) ← 06§8（含 mermaid 图 + 代码）
## WidgetRegistry (D20) ← 06§5（含 mermaid 图 + 映射表）
## 节点渲染器 (D24) ← 06§7（含 mermaid 图）
## 节点搜索 ← 06§9（含 mermaid 图）
## 键盘快捷键 (D33) ← 06§10（含快捷键表）
```

- [ ] **Step 4: 创建 43-app-theme.md**

内容来源：06§4（第 152-258 行）主题系统

结构：
```
# 主题系统
> 定位：语义化颜色接口——Theme trait 定义配色，Dark/Light 实现，渲染层只取色不硬编码。
## 架构总览 (保留 06§4 的 Theme flowchart)
## Theme trait ← 06§4 trait 定义
## 语义化颜色映射 ← 06§4 节点头部颜色表 + 引脚颜色表
## Design Tokens ← 06§4 token 表
## 归属与切换 ← 06§4 末尾
## 设计决策
- **D36**: Theme 通过 trait 定义语义化颜色接口
```

- [ ] **Step 5: 创建 44-cli.md**

内容来源：06§12（第 666-764 行）nodeimg-cli

结构：
```
# nodeimg-cli
> 定位：命令行前端——面向无界面场景的 exec/serve/batch 子命令。
## 架构总览 (保留 06§12 的 CLI flowchart)
## exec 子命令 ← 06§12 exec 部分
## serve 子命令 ← 06§12 serve 部分
## batch 子命令 ← 06§12 batch 部分
## 与 GUI 的共享与差异 ← 06§12 对比表
```

- [ ] **Step 6: Commit**

```bash
git add docs/targetv2/40-app-overview.md docs/targetv2/41-app-execution.md docs/targetv2/42-app-editor.md docs/targetv2/43-app-theme.md docs/targetv2/44-cli.md
git commit -m "docs(targetv2): 40/41/42/43/44 — 拆分 06-app"
```

---

### Task 6: 拆分 15-python-backend-protocol.md + mermaid.md 剩余 → 50/51

**Source:** `docs/target/15-python-backend-protocol.md`, `docs/target/mermaid.md`（VRAM_RECOVERY + LIFECYCLE 子图）

**Files:**
- Create: `docs/targetv2/50-python-protocol.md`
- Create: `docs/targetv2/51-python-lifecycle.md`

- [ ] **Step 1: 创建 50-python-protocol.md**

内容来源：
1. 15§1-6（第 1-208 行）：总览 + 4 个 HTTP API 接口定义
2. 15§7（第 210-248 行）：Handle 机制——ID 格式、Rust/Python 两侧行为、生命周期规则
3. 15§8（第 250-282 行）：VRAM 管理
4. mermaid.md VRAM_RECOVERY 子图（第 164-202 行）
5. mermaid.md SSE 事件流格式（第 269-293 行）+ Handle ID 格式（第 295-303 行）

结构：
```
# Python 后端协议
> 定位：AI 执行器与 Python 推理后端之间的通信协议——接口契约、数据格式、Handle 管理、VRAM 恢复。
## 架构总览 (保留 15§1 的 mermaid 图)
## 接口总览 ← 15§1 表格
## /health ← 15§2
## /node/execute ← 15§3（含 SSE 响应和 multipart 响应）
## /node/cancel ← 15§4
## /handles/release ← 15§5
## Handle 机制 ← 15§7
### Handle ID 格式 ← mermaid.md Handle ID 格式
## SSE 事件流格式 ← mermaid.md SSE 格式
## VRAM 管理 ← 15§8 + mermaid.md VRAM_RECOVERY 子图
## 设计决策
- **D27**: Handle 生命周期 = ResultCache 条目生命周期
- **D28**: VRAM 不足时 Rust 决策释放目标
```

- [ ] **Step 2: 创建 51-python-lifecycle.md**

内容来源：
1. 15§9（第 284-419 行）：进程生命周期 6 状态
2. 15§10（第 422-437 行）：超时机制
3. 15§11（第 439-467 行）：并发模型
4. 15§12（第 469-520 行）：Python 端内部架构
5. mermaid.md LIFECYCLE 子图（第 204-266 行）

结构：
```
# Python 进程生命周期
> 定位：Python 推理后端的运行时管理——进程 6 状态、超时、并发调度、内部架构。
## 架构总览 (mermaid.md LIFECYCLE 子图)
## 进程生命周期 ← 15§9 (含 stateDiagram + 6 状态详细描述 + 状态总结表)
## 超时机制 ← 15§10 (含超时表)
## 并发模型 ← 15§11 (Rust 端 rayon 并发 + Python 端调度)
## Python 端内部架构 ← 15§12 (目录结构 + 各模块职责)
## 设计决策
- **D26**: 协议版本校验
- **D30**: 超时按节点类型差异化
```

- [ ] **Step 3: Commit**

```bash
git add docs/targetv2/50-python-protocol.md docs/targetv2/51-python-lifecycle.md
git commit -m "docs(targetv2): 50/51 — 拆分 Python 协议 + 生命周期"
```

---

### Task 7: 70-decisions.md — 增强决策索引

**Source:** `docs/target/12-decisions.md`

**Files:**
- Create: `docs/targetv2/70-decisions.md`

- [ ] **Step 1: 创建 70-decisions.md**

读取 `docs/target/12-decisions.md`（44 行，一张索引表）。在每一行后加上"详见 [文件名](./文件名.md)"的链接。

决策归属（来自 spec）：

| 决策 | 文件 |
|------|------|
| D01, D02 | 61-error-handling.md |
| D03, D04, D05, D12, D19, D31 | 20-engine.md |
| D06, D07 | 60-concurrency.md |
| D08, D09 | 62-config.md |
| D10 | 30-transport.md |
| D11 | 23-executor-api.md |
| D13-D17 | 63-cross-cutting.md |
| D18, D21, D24 | 24-node-framework.md |
| D20, D32, D33, D34 | 42-app-editor.md |
| D22 | 10-types.md |
| D23, D35 | 41-app-execution.md |
| D25 | 22-executor-ai.md |
| D26, D30 | 51-python-lifecycle.md |
| D27, D28 | 50-python-protocol.md |
| D29 | 21-executor-image.md |
| D36 | 43-app-theme.md |

- [ ] **Step 2: Commit**

```bash
git add docs/targetv2/70-decisions.md
git commit -m "docs(targetv2): 70-decisions — 增强索引加文件链接"
```

---

### Task 8: README.md — 文档体系索引

**Files:**
- Create: `docs/targetv2/README.md`

- [ ] **Step 1: 创建 README.md**

结构（~150 行）：

```
# nodeimg 目标架构 v2

> 本目录是 nodeimg 目标架构的文档体系 v2。按子模块拆分，一个文档一个概念单元。

## 总架构图 (精简版——一张层级总览图，不重复各文档内的详细图)

## Crate → 文档映射

| Crate | 主文档 | 相关文档 |
|-------|--------|---------|
| nodeimg-types | 10-types.md | — |
| nodeimg-graph | 11-graph.md | 10-types.md |
| nodeimg-engine | 20-engine.md | 21/22/23-executor-*, 24-node-framework.md |
| nodeimg-gpu | — | 21-executor-image.md, 24-node-framework.md |
| nodeimg-processing | — | 21-executor-image.md |
| nodeimg-server | 30-transport.md | — |
| nodeimg-app | 40-app-overview.md | 41/42/43 |
| nodeimg-cli | 44-cli.md | — |
| Python 后端 | 50-python-protocol.md | 51-python-lifecycle.md |

## 关键架构约束 (保留 docs/target/README.md 的约束表，更新文档引用)

## 文档索引

| 编号 | 文件 | 主题 |
|------|------|------|
| 00 | 00-context.md | 系统上下文与边界 |
| 01 | 01-overview.md | 调用链总览 + crate 依赖图 |
| ... (26 个文件全部列出) |

## Mermaid 配色规范 (保留 docs/target/README.md 的配色表和语义映射)
```

- [ ] **Step 2: Commit**

```bash
git add docs/targetv2/README.md
git commit -m "docs(targetv2): README — 索引 + crate 映射 + 约束"
```

---

### Task 9: 全局一致性检查

- [ ] **Step 1: 检查所有 26 个文件是否存在**

```bash
ls -la docs/targetv2/*.md | wc -l
# 期望：26
```

- [ ] **Step 2: 检查交叉引用完整性**

在所有 targetv2 文件中 grep markdown 链接，确认没有指向旧文件名（02-service-layer、03-executors、05-graph、06-app、07-node-framework、08-concurrency、09-error-handling、10-config、15-python-backend-protocol、16-node-catalog）的链接。

```bash
grep -rn "02-service\|03-executors\|05-graph\|06-app\|07-node\|08-concurrency\|09-error\|10-config\|15-python\|16-node" docs/targetv2/
# 期望：无输出（如果有，修复链接）
```

- [ ] **Step 3: 检查设计决策覆盖**

grep 所有 D01-D36 在 targetv2 中的出现，确认 36 个都至少出现一次：

```bash
for i in $(seq -w 1 36); do echo -n "D$i: "; grep -rl "D$i" docs/targetv2/ | wc -l; done
# 期望：每个决策��少出现在 1 个文件中（加 70-decisions.md 的索引）
```

- [ ] **Step 4: 检查 mermaid 图的 classDef 一致性**

确认所有 mermaid 图使用的 classDef 都是规范的 8 种颜色之一。

- [ ] **Step 5: 修复发现的问题**

如果上述检查发现问题，逐一修复。

- [ ] **Step 6: Commit**

```bash
git add docs/targetv2/
git commit -m "docs(targetv2): 全局一致性检查 + 修复"
```

---

### Task 10: 最终提交

- [ ] **Step 1: 确认文件数和总行数**

```bash
ls docs/targetv2/*.md | wc -l  # 期望：26
wc -l docs/targetv2/*.md       # 查看各文件行数分布
```

- [ ] **Step 2: Commit（如有未提交的修改）**

```bash
git status
# 如果有未提交的修改：
git add docs/targetv2/
git commit -m "docs(targetv2): 文档体系重构完成 — 26 个文件"
```
