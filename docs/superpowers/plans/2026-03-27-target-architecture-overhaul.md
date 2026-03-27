# 目标架构文档体系重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将单文件 target-architecture.md 重构为 docs/current/ + docs/target/ 的多文件架构文档体系，统一模板和 Mermaid 配色。

**Architecture:** 现有 7 个架构文档原样迁入 docs/current/。docs/target/ 下新建 16 个文件（README + 15 个主题文档），内容来源于现有 target-architecture.md 的拆分改写 + spec 中定义的新章节。所有 mermaid 图使用统一的柔和专业配色方案。

**Tech Stack:** Markdown, Mermaid diagrams

**Spec:** `docs/superpowers/specs/2026-03-27-target-architecture-overhaul-design.md`

---

### Task 1: 脚手架 — 创建目录、迁移现有文档

**Files:**
- Create: `docs/current/` (directory)
- Create: `docs/target/` (directory)
- Move: `docs/architecture.md` → `docs/current/architecture.md`
- Move: `docs/domain.md` → `docs/current/domain.md`
- Move: `docs/catalog.md` → `docs/current/catalog.md`
- Move: `docs/gpu.md` → `docs/current/gpu.md`
- Move: `docs/protocol.md` → `docs/current/protocol.md`
- Move: `docs/backend-architecture.md` → `docs/current/backend-architecture.md`
- Move: `docs/backend-domain.md` → `docs/current/backend-domain.md`
- Delete: `docs/target-architecture.md`
- Delete: `docs/target-architecture-review.md`

- [ ] **Step 1: 创建目录并移动文件**

```bash
mkdir -p docs/current docs/target
git mv docs/architecture.md docs/current/
git mv docs/domain.md docs/current/
git mv docs/catalog.md docs/current/
git mv docs/gpu.md docs/current/
git mv docs/protocol.md docs/current/
git mv docs/backend-architecture.md docs/current/
git mv docs/backend-domain.md docs/current/
git rm docs/target-architecture.md
git rm docs/target-architecture-review.md
```

- [ ] **Step 2: 提交**

```bash
git add -A docs/current/ docs/target/
git commit -m "docs: 创建 current/target 目录结构，迁移现有文档"
```

---

### Task 2: README.md — 索引 + 配色规范 + 模板说明

**Files:**
- Create: `docs/target/README.md`

**参考:** spec §2（目录结构）、§3（文档模板）、§4（Mermaid 风格规范）

- [ ] **Step 1: 编写 README.md**

内容包含三个部分：

1. **文档索引** — 列出所有 15 个文件的标题和一句话定位，带相对链接
2. **文档模板** — 复制 spec §3 的模板定义
3. **Mermaid 配色规范** — 复制 spec §4 的完整 classDef 定义、语义映射表、图表规则

注意：README.md 不使用通用模板，它本身就是定义模板的文件。

- [ ] **Step 2: 提交**

```bash
git add docs/target/README.md
git commit -m "docs(target): 添加 README — 索引、模板、配色规范"
```

---

### Task 3: 00-context.md — 系统上下文

**Files:**
- Create: `docs/target/00-context.md`

**参考:** spec §5 → 00-context.md（全新内容，来源 arc42 §3）

- [ ] **Step 1: 编写 00-context.md**

按统一模板：
- 一句话定位：nodeimg 系统与外部世界的边界和交互对象
- 总览：一张上下文 mermaid 图（nodeimg 系统居中，外部参与���环绕）
  - 外部参与者：用户（GUI/CLI）、本地文件系统、Python 推理后端、云端模型 API（OpenAI/Stability/通义）、项目文件（.nodeimg）
  - 使用统一 classDef 配色
- 主题小节：
  - 系统边界（包含什么：节点编辑、图像处理、执行调度、GPU 计算；不包含什么：AI 模型训练、视频处理）
  - 外部交互说明（每个参与者的数据流向和交互方式）

- [ ] **Step 2: 提交**

```bash
git add docs/target/00-context.md
git commit -m "docs(target): 添加 00-context — 系统上下文"
```

---

### Task 4: 01-overview.md — 调用链总览 + crate 依赖

**Files:**
- Create: `docs/target/01-overview.md`

**参考:** spec §5 → 01-overview.md；来源：现有 target-architecture.md §1 + §3

- [ ] **Step 1: 编写 01-overview.md**

按统一模板：
- 一句话定位：系统顶层架构，模块间如何协作，crate 如何依赖
- 总览：调用链总览 mermaid 图（从现有 §1 改写，换用新 classDef 配色）
- 主题小节：
  - **交互服务 vs 计算服务** — 解释分离原因
  - **交互服务流程图**（新增）— 始终 LocalTransport 直调 Registry/Menu/Serializer 的简单流程
  - **计算服务协议选择** — Local/HTTP/gRPC 可替换
  - **Crate 依赖图** — 从现有 §3 改写，换用新 classDef 配色，更新 nodeimg-gpu 定位为"GPU 运行时基础设施"

所有 mermaid 图换用 spec §4 定义的统一 classDef。

- [ ] **Step 2: 提交**

```bash
git add docs/target/01-overview.md
git commit -m "docs(target): 添加 01-overview — 调用链总览与 crate 依赖"
```

---

### Task 5: 02-service-layer.md — 服务层内部

**Files:**
- Create: `docs/target/02-service-layer.md`

**参考:** spec §5 → 02-service-layer.md；来源：现有 target-architecture.md §2 + spec 新增内容

- [ ] **Step 1: 编写 02-service-layer.md**

按统一模板：
- 一句话定位：EvalEngine 如何调度执行，服务层内部模块如何组织
- 总览：服务层内部结构 mermaid 图（从现有 §2 改写，换配色，移除插件加载框）
- 主题小节：
  - **EvalEngine 分发逻辑** — 拓扑排序后按节点类型路由到三路执行器
  - **engine 内部模块图**（新增）— 8 个 mod 文件夹及依赖关系的 mermaid 图
  - **数据类型边界与 Handle 机制** — 从现有 §2 的 Handle 部分改写
  - **Cache 架构**（新增）— ResultCache（失效驱动 + LRU，Handle 豁免）+ TextureCache（LRU + VRAM 上限）+ 并发访问策略（RwLock）
  - **ResultEnvelope**（新增）— enum 定义、Transport 层决定路径、SerializedValue 说明

- [ ] **Step 2: 提交**

```bash
git add docs/target/02-service-layer.md
git commit -m "docs(target): 添加 02-service-layer — 服务层内部架构"
```

---

### Task 6: 03-executors.md — 三路执行器

**Files:**
- Create: `docs/target/03-executors.md`

**参考:** spec §5 → 03-executors.md；来源：现有 target-architecture.md §2 执行器部分 + §5 AI 执行器 + spec 新增 API 执行器

- [ ] **Step 1: 编写 03-executors.md**

按统一模板：
- 一句话定位：图像处理、AI 推理、模型 API 三路执行器的详细设计
- 总览：三路执行器分发 mermaid 图（简化版服务层图，聚焦三路分发）
- 主题小节：
  - **图像处理执行器** — CPU/GPU 选择逻辑，shader 加载方式
  - **AI 执行器** — 从现有 §5 完整改写（Rust/Python 双侧 mermaid 流程图、SSE 协议、Handle 存储/释放、进度反馈）。换用新 classDef 配色
  - **混合图执行示例** — 从现有 §4 的 sequence diagram 改写
  - **模型 API 执行器**（新增）— 与 AI 执行器的对比表、ApiProvider trait 定义（`authenticate`/`execute`/`rate_limit_status`）、无状态调用模式

- [ ] **Step 2: 提交**

```bash
git add docs/target/03-executors.md
git commit -m "docs(target): 添加 03-executors — 三路执行器详细设计"
```

---

### Task 7: 04-transport.md — Transport + server 接口

**Files:**
- Create: `docs/target/04-transport.md`

**参考:** spec §5 → 04-transport.md；来源：现有 target-architecture.md §1 Transport 部分 + spec 新增 server 接口

- [ ] **Step 1: 编写 04-transport.md**

按统一模板：
- 一句话定位：Transport trait 实现协议透明性，server 接口定义远端通信契约
- 总览：Transport trait + 实现层 mermaid 图
- 主题小节：
  - **Transport trait** — 统一接口设计，Local/HTTP/gRPC 可替换
  - **LocalTransport** — in-process 直调，零拷贝
  - **nodeimg-server 抽象接口**（新增）— 交互服务接口（4 个）+ 计算服务接口（5 个）+ 核心数据类型（TaskId、ProgressEvent、FileId 的 Rust struct 定义）
  - **协议无关性** — 接口不绑定具体传输协议，当前实现可用 HTTP/axum

- [ ] **Step 2: 提交**

```bash
git add docs/target/04-transport.md
git commit -m "docs(target): 添加 04-transport — Transport trait 与 server 接口"
```

---

### Task 8: 05-graph.md — nodeimg-graph 数据模型

**Files:**
- Create: `docs/target/05-graph.md`

**参考:** spec §5 → 05-graph.md；来源：现有 target-architecture.md §7（大幅扩充）

- [ ] **Step 1: 编写 05-graph.md**

按统一模板：
- 一句话定位：节点图的数据结构和操作 API，nodeimg-graph crate 的完整设计
- 总览：Graph 数据模型 mermaid 图（从现有 §7 改写，换配色）
- 主题小节：
  - **与 nodeimg-types 的边界** — types 只保留原子类型（NodeId、Position、Value），Node 在 graph
  - **核心数据结构** — Graph、Node、Connection 的完整 Rust struct 定义
  - **图操作 API** — add_node、remove_node、connect、disconnect、set_param
  - **图查询 API** — topo_sort、upstream、downstream、validate
  - **序列化** — to_json / from_json，项目文件格式

- [ ] **Step 2: 提交**

```bash
git add docs/target/05-graph.md
git commit -m "docs(target): 添加 05-graph — nodeimg-graph 数据模型与 API"
```

---

### Task 9: 06-app.md — App 层

**Files:**
- Create: `docs/target/06-app.md`

**参考:** spec §5 → 06-app.md；来源：现有 target-architecture.md §6（修改+补充）

- [ ] **Step 1: 编写 06-app.md**

按统一模板：
- 一句话定位：App 层的逻辑/渲染分离架构，框架无关的核心逻辑设计
- 总览：App 层架构 mermaid 图（从现有 §6 改写，换配色）
- 主题小节：
  - **逻辑层/渲染层分离** — 保持现有设计，说明迁移优势
  - **WidgetRegistry 归属**（修改）— 明确在 App 逻辑层，engine 只提供 DataType+Constraint，映射逻辑在 App
  - **ExecutionManager 工作机制**（新增）— submit→TaskId、channel 通信、ProgressEvent 类型、AtomicBool 取消、结果在 ResultCache
  - **节点渲染器** — 从现有 §6 的控件映射表改写

注意：现有 §6 的 widget.rs 覆写 mermaid 图需要更新，去掉 widget.rs，改为 `widget(...)` 宏字段。

- [ ] **Step 2: 提交**

```bash
git add docs/target/06-app.md
git commit -m "docs(target): 添加 06-app — App 层逻辑/渲染分离架构"
```

---

### Task 10: 07-node-framework.md — 节点架构

**Files:**
- Create: `docs/target/07-node-framework.md`

**参考:** spec §5 → 07-node-framework.md；来源：现有 target-architecture.md §8（修改+补充）

- [ ] **Step 1: 编写 07-node-framework.md**

按统一模板：
- 一句话定位：节点开发者体验——如何用最少的代码定义一个新节点
- 总览：节点框架 mermaid 图（从现有 §8 改写，换配色，移除插件）
- 主题小节：
  - **node! 宏语法** — 图像处理节点示例（brightness）+ AI 节点示例（ksampler with executor: AI）
  - **文件夹约定** — 文件夹 mermaid 图（从现有改写，换配色）
  - **自动发现规则** — shader.wgsl → gpu_process, cpu.rs → process, 都有 → GPU 优先, 都没有 → executor 字段路由
  - **控件覆写机制**（新增）— 预置控件库列表、WidgetRegistry 默认映射、node! 中 widget(...) 覆写、废弃 widget.rs
  - **Shader 位置** — 跟随节点文件夹，nodeimg-gpu 变为纯运行时
  - **目录结构** — builtins/ + ai_nodes/ + api_nodes/ 三目录布局
  - **Rust/Python 职责分工** — Rust 定义元信息，Python 只执行
  - **inventory 自动注册** — 编译时收集，启动时注入

- [ ] **Step 2: 提交**

```bash
git add docs/target/07-node-framework.md
git commit -m "docs(target): 添加 07-node-framework — 节点架构与开发体验"
```

---

### Task 11: 08-concurrency.md — 并发模型

**Files:**
- Create: `docs/target/08-concurrency.md`

**参考:** spec §5 → 08-concurrency.md（全新章节）

- [ ] **Step 1: 编写 08-concurrency.md**

按统一模板：
- 一句话定位：系统的线程模型、并行策略、前端异步交互方式
- 总览：线程模型 mermaid 图（App 主线程 ↔ channel ↔ 后台执行线程 → rayon 线程池）
- 主题小节：
  - **执行模型** — 后台执行线程 + rayon 同层并行，内联决策理由（D06）
  - **各类节点的并行行为** — GPU 节点（Arc<Mutex> GpuContext，串行提交 queue），CPU 节点（rayon 直接并行），AI/API 节点（reqwest::blocking，内联决策理由 D07）
  - **前端不阻塞策略** — channel 提交、每帧读取进度、AtomicBool 取消

- [ ] **Step 2: 提交**

```bash
git add docs/target/08-concurrency.md
git commit -m "docs(target): 添加 08-concurrency — 并发模型"
```

---

### Task 12: 09-error-handling.md — 错误处理

**Files:**
- Create: `docs/target/09-error-handling.md`

**参考:** spec §5 → 09-error-handling.md（全新章节）

- [ ] **Step 1: 编写 09-error-handling.md**

按统一模板：
- 一句话定位：分层 Error 类型、传播路径、前端展示策略
- 总览：错误传播链路 mermaid 图（NodeError → ExecutionError → TransportError → 前端）
- 主题小节：
  - **分层 Error enum** — 三层类型定义，thiserror crate，内联决策理由（D01）
  - **错误产生位置** — 表格：6 个位置 × 错误类型 × 示例
  - **前端展示策略** — 多位置同时展示（节点红框 + 进度区 + 全局通知），展示规则表，内联决策理由（D02）

- [ ] **Step 2: 提交**

```bash
git add docs/target/09-error-handling.md
git commit -m "docs(target): 添加 09-error-handling — 错误处理体系"
```

---

### Task 13: 10-config.md — 配置系统

**Files:**
- Create: `docs/target/10-config.md`

**参考:** spec §5 → 10-config.md（全新章节）

- [ ] **Step 1: 编写 10-config.md**

按统一模板：
- 一句话定位：配置源、配置项清单、加载时机
- 总览：配置加载流程 mermaid 图（CLI → 环境变量 → TOML 文件 → 默认值，逐层覆盖）
- 主题小节：
  - **配置源优先级** — CLI > 环境变量 > 配置文件 > 默认值，内联决策理由（D08）
  - **配置文件** — TOML 格式，路径 `~/.config/nodeimg/config.toml`，示例配置文件内容
  - **配置项清单** — 6 项的完整表格（transport、server_url、python_backend_url、python_auto_launch、result_cache_limit_mb、texture_cache_limit_mb）
  - **加载时机** — 启动时加载不热更新，内联决策理由（D09）

- [ ] **Step 2: 提交**

```bash
git add docs/target/10-config.md
git commit -m "docs(target): 添加 10-config — 配置系统"
```

---

### Task 14: 11-cross-cutting.md — 横切关注点

**Files:**
- Create: `docs/target/11-cross-cutting.md`

**参考:** spec §5 → 11-cross-cutting.md（全新章节，6 个子主题）

- [ ] **Step 1: 编写 11-cross-cutting.md**

按统一模板：
- 一句话定位：跨层面的约束和策略——安全、日志、测试、性能、部署、版本
- 总览：横切关注点总览（简要列出 6 个子主题及其核心思想）
- 主题小节（6 个）：
  - **安全性** — 扩展点表格（3 行：认证/路径校验/后端认证），内联决策理由（D13）
  - **可观测性/日志** — tracing crate，关键观测点表格（6 行），内联决策理由（D14）
  - **测试策略** — 按层测试类型表格（6 行），内联决策理由（D15）
  - **性能目标** — 量化指标表格（8 行），AI 节点不设目标
  - **部署拓扑** — 三种部署模式表格（全本地/半本地/远端计算），含 mermaid 部署图，内联决策理由（D16）
  - **版本兼容/迁移** — 项目文件 version + 迁移规则，Transport 接口版本协商，内联决策理由（D17）

- [ ] **Step 2: 提交**

```bash
git add docs/target/11-cross-cutting.md
git commit -m "docs(target): 添加 11-cross-cutting — 横切关注点"
```

---

### Task 15: 12-decisions.md — 设计决策表

**Files:**
- Create: `docs/target/12-decisions.md`

**参考:** spec §5 → 12-decisions.md

- [ ] **Step 1: 编写 12-decisions.md**

不使用通用模板，专用格式：
- 标题 + 一句话定位
- D01-D24 的完整决策表格（ID、主题、决策、理由），直接复制 spec 中的表格
- 表格前加简短说明：本文件是所有架构决策的索引，各决策的详细上下文在对应主题文档中内联说明

- [ ] **Step 2: 提交**

```bash
git add docs/target/12-decisions.md
git commit -m "docs(target): 添加 12-decisions — 设计决策表"
```

---

### Task 16: 13-risks.md — 风险与技术债

**Files:**
- Create: `docs/target/13-risks.md`

**参考:** spec §5 → 13-risks.md

- [ ] **Step 1: 编写 13-risks.md**

按统一模板：
- 一句话定位：已识别的风险和已知的技术债务
- 总览：风险概述（5 个已识别风险）
- 主题小节：
  - **已识别风险** — 表格（5 行：wgpu 兼容性、Python 稳定性、大图性能、网络不稳定、VRAM 耗尽），每行含影响和缓解措施
  - **已知技术债** — 占位，待实现时补充

- [ ] **Step 2: 提交**

```bash
git add docs/target/13-risks.md
git commit -m "docs(target): 添加 13-risks — 风险与技术债"
```

---

### Task 17: 14-glossary.md — 术语表

**Files:**
- Create: `docs/target/14-glossary.md`

**参考:** spec §5 → 14-glossary.md；来源：现有 domain.md + spec 新增概念

- [ ] **Step 1: 编写 14-glossary.md**

按统一模板：
- 一句话定位：项目中使用的领域术语定义
- 内容：16 个核心术语的定义表格（直接使用 spec 中的术语清单），基于现有 domain.md 扩充目标架构新增概念

注意：glossary 是参考文档，不需要 mermaid 图。

- [ ] **Step 2: 提交**

```bash
git add docs/target/14-glossary.md
git commit -m "docs(target): 添加 14-glossary — 术语表"
```

---

### Task 18: 更新 CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: 更新设计文档引用路径**

CLAUDE.md 中 `## 设计文档` 节需要更新：

```markdown
## 设计文档

设计文档在 `docs/` 目录，是项目的规范参考。文档可以先行于代码实现：

**当前架构（描述代码实际状态）：**
- `docs/current/domain.md` — 领域概念（节点、数据类型、引脚、约束）
- `docs/current/architecture.md` — 当前系统架构（组件、接口、依赖）
- `docs/current/catalog.md` — 节点目录（所有节点的完整规格）
- `docs/current/gpu.md` — GPU 子系统设计
- `docs/current/protocol.md` / `docs/current/backend-*.md` — AI 后端协议和架构

**目标架构（描述理想状态）：**
- `docs/target/README.md` — 目标架构索引（含所有子文档链接）
```

- [ ] **Step 2: 提交**

```bash
git add CLAUDE.md
git commit -m "docs: 更新 CLAUDE.md 文档引用路径"
```

---

### Task 19: 最终验证

- [ ] **Step 1: 验证文件结构**

```bash
find docs/current docs/target -name '*.md' | sort
```

预期输出 23 个文件：current/ 7 个 + target/ 16 个（README + 00 到 14）。

- [ ] **Step 2: 验证所有 mermaid 图使用统一 classDef**

在 target/ 中搜索旧配色（#4A90D9、#9B59B6、#2ECC71、#E74C3C 等），确保已��部替换为新配色（#6C9BCF、#A78BCA、#6DBFA0、#E88B8B 等）。

```bash
grep -r '#4A90D9\|#9B59B6\|#2ECC71\|#E74C3C\|#E67E22\|#F39C12\|#45B39D\|#BDC3C7' docs/target/
```

预期：无结果（旧配色已全部替换）。

- [ ] **Step 3: 验证无残留引用**

搜索项目中是否还有引用旧路径 `docs/architecture.md`、`docs/domain.md` 等的地方。

```bash
grep -r 'docs/architecture\.md\|docs/domain\.md\|docs/catalog\.md\|docs/gpu\.md\|docs/protocol\.md\|docs/backend-' --include='*.md' --include='*.toml' --include='*.rs' .
```

如有残留引用，更新为 `docs/current/` 路径。

- [ ] **Step 4: 确认完成**

所有文件就位，所有 mermaid 图使用统一配色，所有引用指向正确路径。
