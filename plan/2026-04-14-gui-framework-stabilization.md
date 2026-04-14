# GUI 控件框架收口说明

**目标：** 说明为什么当前阶段应该先完善 GUI 控件框架的接口、隔离和职责边界，而不是继续按单个控件零散推进；并给出后续收口方向。

**背景：** 最近一轮我们已经完成 `Button`、`Toggle`、`Slider`、`TextInput`，并且把 `TextInput` 一路补到了桌面级单行输入的基础能力：焦点、选区、IME、剪贴板、拖选、长文本横向滚动与裁剪。这个过程已经把框架层的结构问题提前暴露出来了。

**参考：**
- `docs/target/2.12.0-widget-anatomy.md`
- `docs/target/2.13.0-framework-gaps.md`

---

## 结论

现在应该暂停继续堆复杂控件，先做一轮 **GUI Framework Stabilization**。

原因不是当前实现不可用，而是：

1. `TextInput` 已经触碰到焦点、IME、剪贴板、拖选、滚动、裁剪、平台副作用等一整套框架级问题。
2. 如果继续直接做 `Dropdown`、`NumberInput`、搜索框、参数面板输入控件，这些逻辑会继续堆进 `Context` 和 `tree::paint`，导致后续每个控件都在复制半套框架。
3. 当前代码已经出现“框架职责”和“控件职责”开始混杂的迹象，再往前做会加速技术债积累。

---

## 当前暴露出的主要问题

### 1. `Context` 开始承担过多控件专属逻辑

`gui/src/context.rs` 目前已经同时承担：

- 统一 tree 更新与布局
- 通用交互状态同步
- `TextInput` 的键盘编辑
- `TextInput` 的 IME preedit/commit
- `TextInput` 的拖选 session
- `TextInput` 的剪贴板请求
- 平台侧 `IME` 请求导出

这说明 `Context` 已经从“框架编排层”逐渐变成“第一个复杂控件的总控文件”。

如果继续按这个方向做 `Dropdown`：

- overlay 开关
- outside click dismiss
- Escape
- popup 命中
- 焦点切换

大概率也会继续堆到 `Context`，形成越来越长的 widget-specific `match` 分支。

### 2. `tree::paint` 已经开始堆 widget-specific painter

`gui/src/tree/paint.rs` 目前除了通用 tree 递归绘制，还承担：

- `Button` 视觉 override
- `Toggle` 视觉 override
- `Slider` 视觉 override
- `TextInput` 的 selection/caret/preedit/clip 专用绘制

这意味着 tree core 和 widget painter 的边界已经被打穿。

继续这样做下去，`paint.rs` 会变成一个集中维护所有控件特殊分支的大文件。届时每新增一个复杂控件，都要同时改 tree core，而不是只改控件自身模块。

### 3. 文本编辑已经不是“小控件逻辑”，而是共享框架服务

`gui/src/widget/text_edit.rs` 和 `gui/src/widget/state/text_input.rs` 已经承担：

- 光标与选区状态
- 删除/插入/替换
- 剪贴板语义
- preedit 几何
- scroll-to-caret
- 点击到字符边界的命中

这套能力后续不会只被 `TextInput` 使用，还会被这些场景复用：

- `Dropdown` 搜索框
- `NumberInput`
- 参数过滤框
- 命令面板
- 节点搜索

如果不把这层明确抽成“文字编辑框架能力”，后面每个输入型控件都会重复造轮子。

### 4. 平台副作用边界刚出现，但还没有彻底定型

现在已经形成一个正确趋势：

- `Context` 产出语义结果
- `AppContext` 处理平台副作用

例如：

- `IME` 请求：`Context::ime_request()` -> `AppContext::apply_ime_request()`
- 剪贴板：`Context` 产出 `ClipboardRequest` -> `DemoApp` 调 `AppContext`

这是对的，但还没有抽象成一个统一、稳定的 effect 模型。

如果后续再加 popup、文件对话框、系统菜单、拖放等平台能力，这个边界必须先稳定下来，否则上层 app 和框架层会继续耦合。

### 5. Action 语义还混着“框架内部动作”和“业务动作”

`gui/src/widget/action.rs` 现在同时包含：

- `Click`
- `DoubleClick`
- `TextChange`
- `DragMove`
- `ResizeMove`

这里混合了两类东西：

1. 面向 app 的业务动作
2. 面向框架交互的内部动作

短期可用，长期会让 app 层越来越依赖控件内部实现细节。

### 6. 主题和样式 token 仍未收口

当前 `button/slider/toggle/text_input` 仍然有一批硬编码颜色、圆角、尺寸常量分散在各文件里。

这会带来两个问题：

1. 后续统一改视觉语言成本越来越高
2. 做新控件时容易继续复制当前硬编码

主题不是“最后再做的美化项”，而是控件框架的接口层之一。

### 7. `Dropdown` 的真实前置依赖还没补齐

从控件表面看，`Dropdown` 只是“一个输入框 + 一个列表”。

但真正依赖的是：

- popup/overlay 管理
- outside click dismiss
- Escape
- z-order
- overlay hit test
- 焦点迁移

这些都不是 `Dropdown` 私有逻辑，而是框架缺口。

如果现在直接做 `Dropdown`，本质上是在“以控件名义继续补框架”。

---

## 为什么不能继续“东写一点西写一点”

这种推进方式在基础控件阶段是有效的，因为：

- `Button` 需求简单
- `Toggle` 只要求点击和视觉态
- `Slider` 只要求拖拽与数值变化

但从 `TextInput` 开始，复杂度已经从“控件实现”升级成“框架能力”。

继续零散做的结果会是：

1. `Context` 持续膨胀
2. `paint.rs` 持续膨胀
3. 每个复杂控件都带进一套新特例
4. 后续想做抽象时，需要拆更多已经固化的耦合

也就是说，越晚收口，成本越高。

---

## 应该先收口的框架方向

### 一、明确三层职责边界

建议把 GUI 层明确分成三层：

1. `shell/app`
- 窗口
- cursor
- IME
- 系统剪贴板
- 平台副作用

2. `framework core`
- tree
- reconcile
- layout
- hit test
- interaction/focus
- 事件编排

3. `widget systems`
- text input system
- popup/overlay system
- dropdown system
- panel system

`Context` 应该主要负责编排这些 system，而不是直接长出越来越多控件特化逻辑。

### 二、固化统一输出模型

当前 `EventOutcome { actions, clipboard }` 已经是正确方向，但还可以继续收口。

建议后续统一成显式的语义输出层，例如：

- `UserAction`
- `PlatformEffect`

其中：

- `IME`、剪贴板、未来的系统对话框都属于 `PlatformEffect`
- app 只处理稳定语义，不直接吃框架内部细节

### 三、把 widget-specific runtime 提升为正式 subsystem

当前 `TextInputStore` 已经是第一个 subsystem 的雏形。

下一步不该继续让它作为 `Context` 里的一个特例，而应该正式承认：

- 复杂控件需要自己的 runtime
- runtime 由 subsystem 管理
- `Context` 调 subsystem 的统一入口

### 四、把 widget-specific paint 从 tree core 中抽离

`tree::paint` 应只负责：

- 通用 tree 递归
- transform
- 通用 decoration/leaf
- clip 调度

控件专用外观应该逐步移到 `widget/painters/` 一类的位置。否则 tree core 会持续被各控件的视觉逻辑污染。

### 五、把文字编辑定义成共享服务

这轮 `TextInput` 已经证明：

- IME
- selection
- clipboard
- scroll-to-caret
- 字符边界命中

不是单个控件私有问题，而是输入型控件共享能力。

因此后续应该显式把“文字编辑”作为框架服务维护，而不是继续把它理解成 `TextInput` 的实现细节。

### 六、在做 `Dropdown` 之前补 popup/overlay 框架

`Dropdown` 的真正前置项不是 `ListView`，而是 popup/overlay。

如果不先做这层，`Dropdown` 的实现会不可避免地把 overlay/focus/outside-click 逻辑再次塞进 `Context`。

### 七、尽快引入 theme token

现在加 theme token 仍然是低成本窗口期。

如果继续新增 3 到 5 个控件后再回头收主题，改造面积会明显扩大。

### 八、补齐通用滚动容器闭环

当前框架其实已经有一半滚动基础设施：

- `overflow`
- `scroll_offset`
- `content_height`
- `Tree::scroll()`

但它还没有形成真正的通用闭环。

现在只有 `TextInput` 自己补了横向滚动；而通用容器滚动在事件路由、命中、clip 和视觉反馈层面还没有收口。

如果这层不先补齐，后续这些能力都会继续各写各的：

- 长参数面板
- `ListView`
- `Dropdown` 选项列表
- inspector 区域

所以“通用滚动容器”不该再被视为某个未来控件的附属问题，而应该回到框架主线里处理。

### 九、把 app 层的 stringly routing 收成 controller / route 层

当前 demo 侧仍然大量依赖字符串 id 做路由判断：

- `text_prompt`
- `slider_radius`
- `toggle_grid`

这在 demo 阶段是可接受的，但如果后续接入真正的参数面板、节点编辑器和 popup，app 层会越来越依赖控件内部 id 约定。

需要逐步把这层收成更稳定的 controller / route 边界，让 app 处理业务语义，而不是直接依赖控件树内部节点命名。

### 十、同步文档与实现状态

当前文档和实现已经开始出现漂移：

- 文档里有些“框架缺口”已经部分完成
- 代码里已经出现的新边界和约定，文档还没正式吸收

例如现在代码里已经有：

- `ImeRequest`
- `ClipboardRequest`
- `EventOutcome`
- `TextInput` 的 runtime / preedit / scroll 几何

如果不尽快同步，后续规划会继续基于过时认知做判断，导致“文档上看还没做，代码里其实已经半做了”的状态越来越严重。

---

## 建议的收口顺序

### Phase 1：框架边界收口

- 明确 `Context`、`AppContext`、app 的职责边界
- 把语义输出统一到 outcome/effect 模型
- 稳定平台副作用出口

### Phase 2：抽 subsystem

- 把 `TextInput` 正式收成第一个 widget subsystem
- 为后续 `Dropdown`、popup 提供统一接入点

### Phase 3：抽 painter + 主题 token

- 把 widget-specific visual override 从 `tree::paint` 抽出去
- 同时接入统一主题 token

### Phase 4：补齐通用滚动容器

- 让 `overflow + scroll_offset + clip + scroll event` 形成通用闭环
- 避免后续 `ListView`、参数面板、`Dropdown` 列表重复各写一套滚动逻辑

### Phase 5：popup/overlay 框架

- overlay lifecycle
- outside click dismiss
- focus/escape/z-order

### Phase 6：route / controller 收口

- 减少 app 对字符串 id 和内部节点命名的直接依赖
- 让 app 侧更稳定地处理业务语义

### Phase 7：文档同步

- 更新 framework gaps / widget anatomy / target 文档
- 把已经形成的新边界正式写入文档

### Phase 8：再做 `Dropdown`

到这里，`Dropdown` 才是“建立在框架上的控件”，而不是“披着控件外衣的新一轮框架特例”。

---

## 这轮收口的非目标

这篇说明不是要推翻现有 retained tree 架构，也不是要立即做一个过度抽象的插件系统。

明确不做：

- 重写 tree/reconcile/layout
- 引入过重的 trait-object widget registry
- 一次性重构所有控件
- 在没有 popup 框架前硬做 `Dropdown`

目标是：

- 用最小代价把已经暴露出的耦合点收口
- 让后续控件实现从“继续补框架”变成“真正写控件”

---

## 完成收口后的预期收益

如果先完成这轮框架收口，再继续做控件，会得到这些直接收益：

1. `Context` 不再持续膨胀
2. `tree::paint` 回到 core 调度职责
3. 文本编辑能力可被多个控件复用
4. `Dropdown` 有明确前置能力，不需要再开新特例
5. 视觉样式可以统一收口，不再持续复制硬编码
6. app 层只处理稳定语义，不再绑定过多 widget 内部细节

---

## 一句话结论

我们已经从“做几个基础控件”进入“开始建设控件框架”的阶段。

`TextInput` 已经把未来 `Dropdown`、`NumberInput`、搜索框、参数编辑器会遇到的核心问题提前暴露出来了。现在最合理的选择，不是继续零散做新控件，而是先把框架接口、隔离和职责边界补完整。
