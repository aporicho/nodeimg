# nodeimg

基于 Rust 的节点式图像处理工具，支持 GPU 加速和 Python AI 后端。

## 构建

```bash
cargo build --release
cargo run --release
cargo test
```

## 项目结构

- `src/app.rs` — 应用入口，管理生命周期、后端启动、项目 I/O
- `src/gpu/` — GPU 加速（GpuContext、GpuTexture、pipeline 辅助、WGSL shaders）
- `src/node/` — 节点系统核心（registry、types、eval、cache、viewer、backend）
- `src/node/builtins/` — 32 个内置节点，每个文件一个 `register()` 函数
- `src/node/widget/` — 参数控件（slider、checkbox、dropdown 等）
- `src/processing/` — CPU 图像处理算法（被 builtins 的 process 函数调用）
- `src/theme/` — 亮色/暗色主题
- `src/ui/` — 节点画布、预览面板
- `python/` — AI 后端（FastAPI + SDXL）

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
- 分支命名：`feature/功能名`、`fix/问题描述`、`hotfix/紧急修复`
- Commit 信息使用规范前缀（feat/fix/docs/style/refactor/test/chore）
- PR 描述中用 `Closes #N` / `Fixes #N` 关联 Issue

## 开发约定

- 新增节点：在 `builtins/` 创建文件，实现 `register()` 注册 NodeDef，在 `builtins/mod.rs` 中调用
- GPU 加速：在 `gpu/shaders/` 添加 WGSL shader，在节点的 `gpu_process` 中通过 `include_str!` 引用
- CPU 算法：放在 `processing/` 对应模块中，由 builtin 的 `process` 函数调用
- 所有 compute shader 使用 16x16 workgroup size
