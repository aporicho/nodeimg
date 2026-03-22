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
├── nodeimg-processing/  — CPU 图像算法（color、filter、transform、composite、generate）
├── nodeimg-engine/      — 执��引擎（NodeRegistry、EvalEngine、Cache、BackendClient、32 个内置节点）
├── nodeimg-app/         — 编辑器 UI（eframe/egui、节点画布、预览面板、参数控件、主题）
└── nodeimg-server/      — HTTP 服务（空壳，待实现）
python/                  — AI 后端（FastAPI + SDXL）
```

���赖方向：types ← gpu/processing ← engine ← app/server

## 设计文档

设计文档在 `docs/` 目录，是项目的规范参考。文档可以先行于代码实现：
- `domain.md` — 领域概念（节点、数据类型、引脚、约束）
- `architecture.md` — 系统架构（组件、接口、依赖）
- `catalog.md` — 节点目录（所有节点的完整规格）
- `gpu.md` — GPU 子系统设计
- `protocol.md` / `backend-*.md` — AI 后端协议和架构

## 开发工作流程

遵循 `docs/开发工作流程.md` 中的规范：
- 主分支为 `main`
- 每个功能/修复 = 一个独立分支 + 一个 PR
- 分支命名：`feature/功能名`、`fix/问题描述`、`hotfix/紧急修复`、`refactor/重构描述`、`docs/文档描述`
- Commit 信息使用规范前缀（feat/fix/docs/style/refactor/test/chore）
- PR 描述中用 `Closes #N` / `Fixes #N` 关联 Issue
- 新功能/架构变更需要先走 brainstorming 设计流程，再写实现计划，再开发
- Code Review 分级：简单 PR 快速扫描，复杂 PR 使用 superpowers:requesting-code-review

## 开发约定

- 新增节点：在 `crates/nodeimg-engine/src/builtins/` 创建文件，实现 `register()` 注册 NodeDef，在 `builtins/mod.rs` 中调用
- GPU 加速：在 `crates/nodeimg-gpu/src/shaders/` 添加 WGSL shader，在 `shaders.rs` 中导出常量，在节点中通过 `nodeimg_gpu::shaders::XXX` 引用
- CPU 算法：放在 `crates/nodeimg-processing/src/` 对应模块中，由 builtin 的 `process` 函数调用
- 所有 compute shader 使用 16x16 workgroup size
