# 原型

交互式 HTML 原型，用浏览器直接打开。

## 文件

- `m1-node-design.html` — 节点视觉设计原型（2026-04-06）

## 设计决策

### D1: 画布即主窗口

节点画布占据整个窗口，是唯一的底层组件。工具栏、预览面板、状态栏、节点库、AI 对话等均为浮动面板，叠加在画布之上。

```
┌─────────────────────────────────────────────┐
│ ┌─ toolbar ──────────────────────────────┐  │
│ └────────────────────────────────────────┘  │
│                                             │
│              节点画布（全窗口）               │
│                                    ┌──────┐ │
│                                    │preview│ │
│                                    │      │ │
│                                    └──────┘ │
│ ┌─ statusbar ────────────────────────────┐  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

不存在固定的分栏布局。所有面板可显隐、可拖拽，画布始终铺满。

## 节点设计决策

### 节点外观建模

一个节点的外观由 NodeDef 唯一确定，无需额外信息。

```
NodeDef {
    name, category,              → 标签（名字 + 分类色点）
    inputs, outputs,             → 引脚（⊕ 按钮 → 展开引脚列表）
    params,                      → 控件（DataType + Constraint 自动映射）
    inline_pins: Vec<String>,    → 哪些 pin 作为控件显示在卡片内
    panel: Option<PanelHint>,    → 哪些 param 移到弹出面板里
}
```

卡片内的控件 = inline_pins 的预览控件 + params 的自动映射控件。不存在"模式"区分，所有节点统一处理，视觉差异由控件组合自然产生。

### 两种控件数据来源

| 来源 | 说明 | 例子 |
|------|------|------|
| param | 用户输入/调整的参数值 | Slider、Dropdown、TextArea |
| pin | 执行结果的预览 | ImagePreview（输出图片） |

### 控件映射表（12 种）

| DataType | Constraint | 控件 |
|----------|-----------|------|
| Float | Range(min, max) | Slider |
| Float | 无 | DragValue |
| Int | Range(小跨度) | SliderInt |
| Int | Range(大跨度) | DragValue |
| String | Enum(>4 项) | Dropdown |
| String | Enum(≤4 项) | RadioGroup |
| String | FilePath | FilePicker |
| String | Multiline | TextArea |
| String | 无 | TextInput |
| Bool | — | Checkbox |
| Color | — | ColorPicker |
| Image（pin） | — | ImagePreview |
| — | action | ActionButton |

### 引脚交互：两级展开 + Safe Triangle

1. Hover 节点 → 两侧出现 ⊕ 按钮
2. Hover ⊕ → 展开引脚列表（彩色圆点 + 名称）
3. 鼠标从 ⊕ 移向列表时，Safe Triangle 算法保证列表不会闪退

### 弹出面板

- 可选，任何节点都可以有（不限于特定类型）
- 选中节点后在节点底部弹出
- 用于放不适合在卡片内显示的参数
- PanelHint 描述面板布局

```
struct PanelHint {
    params: Vec<String>,        // 哪些 param 放面板里
    layout: PanelLayout,        // Auto（自动排列）或 Toolbar（横排选择器+文本框）
    action: Option<String>,     // 动作按钮文字
}
```

### 文本编辑交互

- 双击进入编辑模式
- 单击/拖拽是移动节点
- 点击外部退出编辑

### 图片节点交互

- 点击图片区域 → 弹出系统文件对话框

### 视觉风格

- 白色极简风，shadcn 风格
- zinc 色系（#fafafa 背景、#e4e4e7 边框、#18181b 文字/强调）
- 节点名称在卡片外上方（小标签 + 分类色点）
- 选中态：深色边框 + ring 阴影
- 圆点网格背景，间距 24px

### 布局

- 画布全屏
- 工具栏：悬浮 pill 形状，可拖动
- 预览面板：左侧悬浮，可拖动、可调整大小
- 所有面板除画布外均可悬浮拖动

### 需同步更新的目标文档

| 文档 | 变更 |
|------|------|
| 2.0.0-gui.md | 布局改为画布全屏 + 悬浮面板 |
| 2.1.0-canvas.draft.md | 画布占满窗口 |
| 2.1.2-node.draft.md | 白色极简风；去掉折叠；节点外观建模更新；引脚改为 ⊕ 两级展开 |
| 2.1.3-param-widgets.draft.md | 补充 TextInput、TextArea 控件映射 |
| 2.2.0-preview.draft.md | 预览面板改为左侧悬浮 |
| 2.5.0-toolbar.draft.md | 工具栏改为悬浮 pill |
| 2.7.0-theme.draft.md | 色彩基调从暗色改为白色极简风 |
| 0.2.0-file-architecture.draft.md | gui/ 新增 widgets/floating.rs |
