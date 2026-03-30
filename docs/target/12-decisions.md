# 设计决策表

> 所有架构决策的索引，记录"选了什么"和"为什么"。

本文件是全局决策索引。各决策的详细上下文在对应主题文档中内联说明。

| ID | 主题 | 决策 | 理由 |
|----|------|------|------|
| D01 | 错误处理 | 分层 Error enum（NodeError → ExecutionError → TransportError） | 每层独立演进，thiserror 生态天然支持 |
| D02 | 错误展示 | 多位置同时展示（节点红框 + 进度区 + 全局通知） | 各层独立补充展示信息，不互斥 |
| D03 | Cache 架构 | 分离为 ResultCache + TextureCache | 执行结果和渲染纹理关注点不同，淘汰逻辑不应纠缠 |
| D04 | ResultCache 淘汰 | 失效驱动 + 可选 LRU 上限，Handle 豁免 LRU | Handle 淘汰意味着释放 Python GPU 对象，重算代价高 |
| D05 | TextureCache 淘汰 | LRU + VRAM 上限 | 预览纹理丢了可重新上传，代价低 |
| D06 | 并发模型 | 后台执行线程 + rayon 同层并行 | CPU/GPU 密集型任务用线程池最自然，不引入 async 复杂度 |
| D07 | AI HTTP 调用 | reqwest::blocking 在 rayon 线程中阻塞 | AI 节点通常串行依赖，同层并行 AI 节点场景少 |
| D08 | 配置格式 | TOML，CLI > 环境变量 > 文件 > 默认值 | Rust 生态标准做法 |
| D09 | 配置加载 | 启动时加载，不热更新 | 简单，配置变更频率极低 |
| D10 | server 接口 | 抽象服务接口与 Transport trait 对齐 | 不绑定具体传输协议，允许后期自由切换 |
| D11 | API 执行器 | 统一 ApiProvider trait | 认证/速率限制/重试逻辑每个 provider 都需要，统一避免重复 |
| D12 | ResultEnvelope | Transport 层决定 Local/Remote 路径 | 前端不感知模式差异 |
| D13 | 安全性 | 预留扩展点，当前不实现 | 做好接口，后期直接接入 |
| D14 | 可观测性 | tracing crate + 关键观测点 | Rust 生态标准，不需要架构级特殊设计 |
| D15 | 节点回归测试 | perceptual hash 比对 | 像素级比对在 GPU 浮点误差下 flaky |
| D16 | 部署模式 | 全本地 / 半本地 / 远端计算 | AI 推理太重，即使本机有好 GPU 也值得远端执行 |
| D17 | 版本兼容 | 项目文件 version + 迁移，Transport 接口版本协商 | 允许增量升级 |
| D18 | 插件系统 | 移除 | 当前阶段不需要，inventory 静态注册足够 |
| D19 | engine 内部 | 不拆 crate，mod 文件夹组织 | 避免 workspace 复杂度，内部模块图保证清晰 |
| D20 | WidgetRegistry | 移到 App 逻辑层 | engine 不应知道控件概念，行业标准做法（Blender、ComfyUI） |
| D21 | nodeimg-gpu 定位 | GPU 运行时基础设施（不含 shader） | shader 跟随节点文件夹 |
| D22 | Node 结构体归属 | 放 nodeimg-graph | types 保持轻量原子类型，Node 和 Graph 内聚在一起 |
| D23 | ExecutionManager | channel 通信 + 进度事件 + AtomicBool 取消 | 简单有效，不引入复杂异步机制 |
| D24 | 控件覆写 | node! 宏中 widget(...) 选择预置控件 | 新增控件和新增节点独立扩展 |
| D25 | Python 后端协议 | 单节点执行 + SSE 流式 + Handle 机制 | 细粒度缓存、Handle 跨步骤复用、实时进度反馈 |
| D26 | Python 进程管理 | App 自动启动（可配置关闭） | 开箱即用体验，python_auto_launch 配置项控制 |
| D27 | Python 并发 | Rust 并发发送，Python 队列调度 | 轻量节点可并行利用 GPU，重型节点独占 GPU 避免争抢 |
| D28 | VRAM 释放策略 | 缓存失效驱动 + VRAM 不足被动恢复 | 两条互补路径，Rust 端统一决策保证状态一致 |
| D29 | 节点自适应 | 一个节点适配所有模型架构，不做模型专属节点 | 减少节点数量，用户无需关心模型架构差异 |
| D30 | 节点粒度 | 与 ComfyUI 对齐 | 用户可无缝迁移 ComfyUI 工作流 |
| D31 | 节点命名 | 重新设计，不照搬 ComfyUI | 更清晰的分类体系，不受历史包袱影响 |
| D32 | Undo/Redo | 不可变状态 + 结构共享（Arc\<Node\>） | 兼顾内存效率和实现可靠性，新增操作零 undo 代码 |
| D33 | 快捷键 | 逻辑层 KeybindingMap，config.toml 可覆盖 | 渲染层不硬编码快捷键，用户可自定义 |
| D34 | 多项目 Tab | 每 tab 独立 GraphController/UndoManager，Cache 共享 | tab 间互不干扰，VRAM 不重复占用 |
| D35 | 自动执行策略 | GPU/CPU 节点 200ms 防抖自动执行，AI/API 节点 Ctrl+Enter 手动触发 | 图像处理实时预览，AI 生成用户可控；过期结果保留旧预览不提示（ComfyUI 风格） |
| D36 | 主题系统 | Theme trait 语义化颜色接口 + DarkTheme/LightTheme 实现 + tokens 布局常量 | 渲染层不硬编码颜色，新增主题只需实现 trait；颜色按分类（节点头部）和数据类型（引脚）语义映射 |
