# nodeimg

基于 Rust 的节点式图像处理工具，支持 GPU 加速和 Python AI 后端。

## 构建

```bash
cargo build --release
cargo run -p nodeimg-app --release
cargo test --workspace
```

## 项目结构（Workspace）

```
crates/
├── nodeimg-types/       — 基础数据类型（Value、DataType、Constraint、GpuTexture…）
├── nodeimg-gpu/         — GPU 计算（GpuContext、pipeline、WGSL shaders）
├── nodeimg-processing/  — CPU 辅助算法（histogram 计算/渲染、LUT 文件解析）
├── nodeimg-engine/      — 执��引擎（NodeRegistry、EvalEngine、Cache、BackendClient、32 个内置节点）
├── nodeimg-app/         — 编辑器 UI（eframe/egui、节点画布、预览面板、参数控件、主题）
└── nodeimg-server/      — HTTP 服务（空壳，待实现）
python/                  — AI 后端（FastAPI + SDXL）
```

���赖方向：types ← gpu/processing ← engine ← app/server

## 设计文档

设计文档在 `docs/` 目录，是项目的规范参考。文档可以先行于代码实现：

**当前架构（描述代码实际状态）：**
- `docs/current/domain.md` — 领域概念（节点、数据类型、引脚、约束）
- `docs/current/architecture.md` — 当前系统架构（组件、接口、依赖）
- `docs/current/catalog.md` — 节点目录（所有节点的完整规格）
- `docs/current/gpu.md` — GPU 子系统设计
- `docs/current/protocol.md` / `docs/current/backend-*.md` — AI 后端协议和架构

**目标架构（描述理想状态）：**
- `docs/target/roadmap.md` — 实现路线图（7 个里程碑）
- `docs/target/spec.md` — 文档规范
- `docs/target/0.x.x` — 全局文档（架构总览、数据模型）
- `docs/target/2.x.x` — GUI 文档
- `docs/target/4.x.x` — 引擎文档
- `docs/target/5.x.x` — 项目文件
- `docs/target/6.x.x` — Python 后端

## 开发工作流程

遵循 `docs/开发工作流程.md` 中的规范：
- 主分支为 `main`
- 每个功能/修复 = 一个独立分支 + 一个 PR
- 分支命名：`feature/功能名`、`fix/问题描述`、`hotfix/紧急修复`、`refactor/重构描述`、`docs/文档描述`
- Commit 信息使用规范前缀（feat/fix/docs/style/refactor/test/chore）
- PR 描述中用 `Closes #N` / `Fixes #N` 关联 Issue
- 新功能/架构变更需要先走 brainstorming 设计流程，再写实现计划，再开发
- Code Review 分级：简单 PR 快速扫描，复杂 PR 使用 superpowers:requesting-code-review
- 所有 commit 必须在功能分支上，绝不在 main 上直接 commit
- 用 `git pull --rebase` 同步 main，避免产生 merge commit

## 开发约定

- 新增节点：在 `crates/nodeimg-engine/src/builtins/` 创建文件，实现 `register()` 注册 NodeDef，在 `builtins/mod.rs` 中调用
- GPU 优先：像素级运算只写 WGSL shader，通过统一的 `execute(ctx)` 函数调用 `ctx.gpu()`
- GPU shader：在 `crates/nodeimg-gpu/src/shaders/` 添加 WGSL shader，在 `shaders.rs` 中导出常量，在节点中通过 `nodeimg_gpu::shaders::XXX` 引用
- CPU 仅用于 GPU 做不到的事：文件 I/O（load_image、save_image）、文件解析（LUT）
- AI 节点判据：executor 类型为 AI（通过 `NodeDef::is_ai_node()` 方法）
- 所有 compute shader 使用 16x16 workgroup size
