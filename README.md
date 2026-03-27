# nodeimg

基于 Rust 的节点式图像处理工具，支持 GPU 加速和 Python AI 后端。

## 功能

- **节点图编辑器** — 通过可视化连线组合图像处理流程
- **32 个内置节点** — 颜色调整、空间变换、滤镜、合成、生成等
- **GPU 加速** — 基于 wgpu 的 compute shader，大部分节点自动走 GPU 路径
- **AI 后端** — 通过 Python FastAPI 集成 SDXL 图像生成管道
- **实时预览** — 参数调整即时更新结果
- **项目文件** — 自动保存/加载节点图（.nis 格式）

## 系统要求

- Rust 1.75+
- 支持 Vulkan/Metal/DX12 的 GPU（用于 wgpu 加速）
- Python 3.10+（仅 AI 功能需要）

## 构建和运行

```bash
# 构建
cargo build --release

# 运行
cargo run --release
```

AI 后端会在需要时自动启动（首次运行会安装 Python 依赖）。也可以手动启动：

```bash
cd python
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn server:app --host 0.0.0.0 --port 8188
```

## 项目结构

```
src/
├── app.rs           # 应用主框架
├── gpu/             # GPU 加速（wgpu + WGSL shaders）
├── node/            # 节点系统（注册表、求值引擎、缓存、后端通信）
│   ├── builtins/    # 32 个内置节点定义
│   └── widget/      # 参数控件
├── processing/      # CPU 图像处理算法
├── theme/           # 主题系统（亮色/暗色）
└── ui/              # UI 组件（节点画布、预览面板）
python/              # AI 后端（FastAPI + SDXL）
docs/                # 设计文档
```

## 设计文档

**当前架构：**
- [领域概念](docs/current/domain.md) — 节点、数据类型、引脚、约束等核心概念
- [系统架构](docs/current/architecture.md) — 组件划分、接口约定、依赖关系
- [节点目录](docs/current/catalog.md) — 所有节点的完整规格
- [GPU 子系统](docs/current/gpu.md) — GPU 加速架构和着色器规范
- [后端协议](docs/current/protocol.md) — 前后端通信协议
- [后端领域](docs/current/backend-domain.md) — AI 后端领域概念
- [后端架构](docs/current/backend-architecture.md) — AI 后端系统结构

**目标架构：**
- [目标架构索引](docs/target/README.md) — 完整目标架构文档体系

**其他：**
- [开发工作流程](docs/开发工作流程.md) — 项目开发规范

## 许可

MIT
