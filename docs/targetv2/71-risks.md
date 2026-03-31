# 风险与技术债

> 已识别的风险和已知的技术债务。

## 总览

本文件列出架构设计阶段已识别的风险及缓解措施，帮助在实现过程中提前规避问题。

## 已识别风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| wgpu 跨平台兼容性 | 部分 GPU/驱动不支持 compute shader，CI 环境无 GPU | 提供 CPU 回退路径，CI 使用 wgpu 软件渲染后端 |
| Python 后端进程稳定性 | Python 进程崩溃或 OOM 导致 AI 节点全部失败 | AI 执行器做超时检测和自动重连，提示用户重启后端 |
| 大图性能瓶颈 | 节点数量 >100 时拓扑排序、Cache 失效递归、UI 渲染可能变慢 | 设性能目标，定期 profiling，考虑增量执行 |
| 远端模式网络不稳定 | 执行中断、结果丢失 | Transport 层做重试，执行结果持久化到 ResultCache |
| VRAM 耗尽 | 多个高分辨率纹理同时驻留 GPU | TextureCache LRU 淘汰 + VRAM 上限配置 |

## 已知技术债

以下为当前代码与目标架构之间的已知差距，按迁移优先级排序。

| ID | 技术债 | 当前状态 | 目标状态 | 影响范围 |
|----|--------|---------|---------|---------|
| TD01 | Shader 集中存放 | 30 个 shader 集中在 `nodeimg-gpu/src/shaders/` | 每个 shader 跟随节点文件夹（`builtins/brightness/shader.wgsl`） | nodeimg-gpu、所有 GPU 节点 |
| TD02 | 节点手动注册 | `builtins/mod.rs` 中手动列举 33 个 `register()` 调用 | `inventory` crate 自动注册，`node!` 宏展开时插入 `inventory::submit!` | nodeimg-engine/builtins |
| TD03 | 节点单文件结构 | 每个节点一个 `.rs` 文件（如 `blend.rs`） | 每个节点一个文件夹（`blend/mod.rs` + `shader.wgsl` + 可选 `cpu.rs`） | 所有 33 个内置节点 |
| TD04 | `node!` 宏不存在 | 手动构建 `NodeDef` 结构体注册 | 声明式 `node!` 宏，自动发现 shader/cpu 文件、自动注册 | 节点开发体验 |
| TD05 | `nodeimg-graph` crate 不存在 | `Connection` 在 engine 内部定义，`NodeInstance` 在 types 中，无完整 `Graph` 结构体 | 独立 crate，定义 `Graph`、`Node`、`Connection`、图操作和图查询 API | types、engine、app 的依赖关系 |
| TD06 | AI / API 节点目录不存在 | 无 `ai_nodes/`、`api_nodes/` 目录，所有节点混在 `builtins/` | 三个并列目录：`builtins/`（图像处理）、`ai_nodes/`（AI 自部署）、`api_nodes/`（云端 API） | 节点组织 |
| TD07 | `nodeimg-cli` crate 不存在 | 无 CLI 前端 | 独立 crate，实现 `exec` / `serve` / `batch` 三个子命令 | 新增 crate |

**建议迁移顺序：**

TD05（graph crate）→ TD03 + TD01（文件夹重组 + shader 迁移，可一起做）→ TD04 + TD02（node! 宏 + 自动注册）→ TD06（目录分离）→ TD07（CLI）。

理由：graph crate 是基础依赖，后续所有重构都需要稳定的图数据模型；文件夹重组是 node! 宏的前提（宏依赖文件夹约定发现 shader）；CLI 独立于以上重构，可最后实施。
