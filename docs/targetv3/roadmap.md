# 路线图

> 每个里程碑结束时有可 demo 的完整用户场景，引擎与 GUI 穿插推进。

---

## M1 — 画布能跑起来

**可 demo：** 打开 app，添加 LoadImage + Brightness 节点，连线，运行，看到处理结果。

### 引擎
- 基础类型（Value、DataType、Constraint 注册表）
- 节点图控制器（add_node、connect、set_param、undo/redo）
- 节点管理器（inventory 注册）
- 图像执行器（ExecContext、GPU/CPU）
- 内置节点：LoadImage、SaveImage、Brightness、Contrast

### GUI
- 基础画布渲染（节点、连线）
- 基础参数控件（Slider、DragValue）
- 运行按钮 + 预览面板（图像）

**参考文档：** `4.2.0` `4.3.0` `4.4.0` `4.9.0` `2.1.0` `2.1.2` `2.1.3` `2.2.0`

---

## M2 — 图像处理工作流

**可 demo：** 构建多节点图像处理管线，参数实时调整，增量执行。

### 引擎
- 调度器（脏标记、增量执行、auto/manual 模式）
- 缓存管理器
- 更多内置节点（曲线、色调、模糊、锐化、裁剪、缩放、LUT 等）
- 事件系统（执行进度推送）

### GUI
- 完整参数控件（Dropdown、Checkbox、ColorPicker、FilePicker）
- 执行状态显示（节点进度指示器）
- 状态栏（执行进度、GPU 状态）

**参考文档：** `4.1.0` `4.5.0` `4.8.0` `2.6.0`

---

## M3 — AI 节点

**可 demo：** 在画布上运行 KSampler，生成图像，查看产物历史。

### 引擎
- Python 后端（HTTP、Handle 机制、执行调度）
- AI 执行器（AIRuntime trait、PythonHttpRuntime）
- 产物管理器
- AI 内置节点（LoadCheckpoint、KSampler、VAEDecode 等）

### GUI
- 手动执行 AI 节点的触发方式
- 预览面板支持产物历史浏览

**参考文档：** `4.6.0` `4.10.0` `6.0.0`

---

## M4 — API 节点

**可 demo：** 调用 Stability AI 等云端 API 生成图像。

### 引擎
- API 执行器
- Provider 注册机制
- 内置 Provider（OpenAI、Stability AI 等）

**参考文档：** `4.11.0` `4.11.1`

---

## M5 — 项目文件

**可 demo：** 保存项目，重新打开，状态完整恢复。节点库弹窗添加节点。

### 引擎
- 项目管理器（bundle 读写、缩略图生成）

### GUI
- 节点库弹窗（双击画布、连线拖出触发）
- 工具栏（新建/打开/保存）
- 自动保存

**参考文档：** `4.7.0` `5.0.0` `2.3.0` `2.5.0`

---

## M6 — 完整编辑体验

**可 demo：** 流畅的节点编辑操作，快捷键，主题切换，布局管理。

### GUI
- 完整画布交互（框选、复制/粘贴、对齐辅助线、自动滚动）
- 快捷键系统（注册 API）
- 主题系统（亮/暗模式、Token 注册）
- 交互控制器完整子模块（ProjectController、WindowController 等）
- 右键菜单

**参考文档：** `2.1.1` `2.7.0` `2.8.0` `2.10.0`

---

## M7 — AI 对话

**可 demo：** 用自然语言指令操作节点图，AI 实时执行并反馈。

### GUI
- AI 对话面板（流式显示、取消）
- AI 操作员集成

**参考文档：** `2.4.0`

---

## Future

- **CLI**（`3.0.0`）：批处理、无头执行
- **AI 操作员**（`1.0.0`）：完整自然语言操控引擎的能力
