# Core Widget Visual Baseline Plan

Date: 2026-04-26

## Goal

先把最常用、最影响节点参数区质感的控件做扎实，再统一节点卡片外观。

第一批控件只覆盖当前业务和节点参数高频路径：

```text
TextInput / NumberInput
-> Slider
-> Dropdown
-> Toggle
-> Button
-> Label / TruncatedText
-> shared visual tokens
-> node card integration
```

## Scope

第一批控件：

- `TextInput`
- `NumberInput`
- `Slider`
- `Dropdown`
- `Toggle`
- `Button`
- `Label`
- `TruncatedText`

暂缓控件：

- `ColorSwatch`
- `PathInput`
- `Checkbox`
- `Radio`
- `ImageViewer`
- `Separator`
- `Panel`
- `Group`
- `ScrollArea`
- `ListView`
- `Collapsible`

暂缓项不是不重要，而是等核心控件的视觉语言稳定后再统一。

## Rules

- 控件缺陷优先修在 `gui/src/widget/atoms/*`、`gui/src/widget/painters/*`、`gui/src/theme/*`。
- 节点内参数控件走 `gui/src/widget/param_control.rs` 的紧凑形态，不在节点卡片里手写控件外观。
- 节点卡片参数行只提供控件槽位，不统一渲染 `param.name` 或 `default_value`；需要 label/value 时由控件自身决定位置。
- 不为了某个节点临时绕过通用控件。
- 每个控件必须覆盖普通、hover、pressed/focused、disabled 的关键状态；没有对应状态时要显式说明。
- 文本必须在紧凑宽度下正确裁剪或省略，不允许撑破节点卡片。
- 图标按钮优先使用已有 icon registry，不新增一次性 SVG。
- 做完单个控件后先验证，再进入下一个控件。

## Visual Baseline Checklist

所有第一批控件共享以下基线：

- [ ] 高度体系统一：节点内 compact 控件高度稳定，普通面板控件高度稳定。
- [ ] 圆角体系统一：输入框、按钮、下拉、滑块 thumb 的圆角来自 theme tokens。
- [ ] 边框体系统一：normal / hover / focus / disabled 的边框颜色和宽度一致。
- [ ] 背景体系统一：normal / hover / pressed / disabled 的背景层次清晰。
- [ ] 字体体系统一：控件文字、控件内 label、状态文字使用合适的 text style。
- [ ] 间距体系统一：label、icon、value、control 之间的 gap 不靠局部魔法数扩散。
- [ ] Focus 表现统一：键盘焦点和文本编辑焦点可见但不过重。
- [ ] Disabled 表现统一：不可交互控件降低对比度但仍可读。
- [ ] Node compact 表现统一：控件在 128px 左右宽度内仍专业、可读、不溢出。

## Control Checklist

### 1. TextInput / NumberInput

代码入口：

- `gui/src/widget/atoms/text_input.rs`
- `gui/src/widget/atoms/number_input.rs`
- `gui/src/widget/painters/text_input.rs`
- `gui/src/widget/systems/text_input.rs`
- `gui/src/widget/state/text_input.rs`

Checklist：

- [ ] 普通态：边框、背景、文字、内边距稳定。
- [ ] Hover 态：可感知但不抢视觉。
- [ ] Focus 态：caret、selection、focus border 清晰。
- [ ] Disabled 态：不可编辑且视觉降级一致。
- [ ] Number formatting：精度、step、min/max 显示稳定。
- [ ] 长文本：横向滚动、caret 可见、省略策略不破布局。
- [ ] 节点内：`ParamControlSpec::Text` 和 `ParamControlSpec::Number` 在节点卡片内不撑宽。
- [ ] 节点内：TextInput-only showcase 节点只有一个可点击输入框，不出现节点层参数名/默认值。
- [ ] 测试：现有 text input / number input 单测通过，并补足必要的结构断言。

完成条件：

- [ ] `cargo test -p gui text_input`
- [ ] `cargo test -p gui number_input`
- [ ] 手动看节点 showcase 中 Text / Number 参数。

### 2. Slider

代码入口：

- `gui/src/widget/atoms/slider.rs`
- `gui/src/widget/painters/slider.rs`
- `gui/src/gesture/factory.rs`

Checklist：

- [ ] Track、fill、thumb 的尺寸和颜色统一。
- [ ] Hover / drag 态明确。
- [ ] Disabled 态明确。
- [ ] Thumb 命中范围足够，视觉尺寸不显笨重。
- [ ] Min/max/value 映射稳定，极值不越界。
- [ ] 节点内：亮度、对比度、强度类参数在 compact 宽度下可读可拖。
- [ ] 测试：保留现有行为测试，补结构/状态断言。

完成条件：

- [ ] `cargo test -p gui slider`
- [ ] 手动看节点 showcase 中 Slider 参数。

### 3. Dropdown

代码入口：

- `gui/src/widget/atoms/dropdown.rs`
- `gui/src/widget/painters/dropdown.rs`
- `gui/src/widget/systems/dropdown.rs`
- `gui/src/overlay/*`

Checklist：

- [ ] Field 普通态、hover、focus、disabled 统一。
- [ ] Chevron 图标大小、位置、颜色稳定。
- [ ] 选中项文本在窄宽度下省略正确。
- [ ] Popup 边框、阴影、圆角、背景清晰。
- [ ] Option hover / highlighted / selected 状态清晰。
- [ ] 键盘选择和鼠标选择视觉一致。
- [ ] 节点内：enum 参数和 provider/model 参数不撑破卡片。

完成条件：

- [ ] `cargo test -p gui dropdown`
- [ ] 手动看节点 showcase 中 Select 参数。

### 4. Toggle

代码入口：

- `gui/src/widget/atoms/toggle.rs`
- `gui/src/widget/painters/toggle.rs`
- `gui/src/gesture/factory.rs`

Checklist：

- [ ] Off / On 状态一眼可分。
- [ ] Hover / pressed 状态不改变布局。
- [ ] Disabled 状态清楚。
- [ ] Knob 动画或位置稳定，不出现跳动。
- [ ] 节点内：Bool 参数在 compact 宽度下清晰。

完成条件：

- [ ] `cargo test -p gui toggle`
- [ ] 手动看节点 showcase 中 Toggle 参数。

### 5. Button

代码入口：

- `gui/src/widget/atoms/button.rs`
- `gui/src/widget/painters/button.rs`

Checklist：

- [ ] Text button 普通、hover、pressed、disabled 状态统一。
- [ ] Icon button 尺寸、命中区、图标颜色统一。
- [ ] Leading icon + label 的 gap 稳定。
- [ ] 长 label 不撑破容器。
- [ ] Toolbar / node palette / panel close button 三类场景都成立。

完成条件：

- [ ] `cargo test -p gui button`
- [ ] 手动看 toolbar、node palette、panel close button。

### 6. Label / TruncatedText

代码入口：

- `gui/src/widget/atoms/label.rs`
- `gui/src/widget/atoms/truncated_text.rs`
- `gui/src/tree/text_layout.rs`

Checklist：

- [ ] Title / body / caption 的层级清楚。
- [ ] Muted 文本可读但不抢视觉。
- [ ] Ellipsis 在固定宽度下稳定。
- [ ] Start / center / end 对齐正确。
- [ ] 节点标题、控件内 label、状态文字都能复用同一套规则。

完成条件：

- [ ] `cargo test -p gui label`
- [ ] `cargo test -p gui truncated_text`
- [ ] `cargo test -p gui text_layout`
- [ ] 手动看 engine panel、node palette、节点卡片标题和参数名。

## Integration Checklist

单个控件完成后：

- [ ] 更新对应 widget / painter / theme 代码。
- [ ] 跑该控件相关单测。
- [ ] 手动看 `showcase_node` 或真实 workspace 场景。
- [ ] 记录有意延后的边界问题。

第一批控件全部完成后：

- [ ] 整理重复颜色、圆角、高度、间距到 theme tokens。
- [ ] 更新 `ParamControlMetrics`，让节点内控件走统一 compact 规格。
- [ ] 回到 `gui/src/canvas/node_card.rs` 做节点卡片外壳视觉统一。
- [ ] 复查 `docs/todo/2026-04-24-gui-visual-audit-plan.md` 中 basic widgets 相关行。

## Suggested Execution Order

1. `TextInput / NumberInput`
2. `Slider`
3. `Dropdown`
4. `Toggle`
5. `Button`
6. `Label / TruncatedText`
7. shared visual tokens
8. node card integration

## Final Acceptance

- [ ] 第一批控件 checklist 全部完成。
- [ ] 节点 showcase 中第一批控件视觉一致。
- [ ] Toolbar、node palette、engine panel 没有明显视觉回退。
- [ ] `cargo fmt --check`
- [ ] `cargo test -p gui`
- [ ] `cargo test -p app`
- [ ] 记录下一批控件：`ColorSwatch / PathInput / Panel / Group / ScrollArea`。
