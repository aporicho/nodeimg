# 手势识别系统设计

## 概述

实现 Flutter 竞技场模式的手势识别系统，作为 `gui/src/gesture/` 顶层模块。采用分层路由 + 竞技场方案：层级之间简单路由（消费即停止），同一节点上的多手势通过竞技场裁决。

## 文件结构

```
gui/src/
├── gesture/
│   ├── mod.rs            — 导出
│   ├── recognizer.rs     — GestureRecognizer trait + GestureDisposition 枚举
│   ├── arena.rs          — GestureArena（收集识别器、分发事件、裁决胜负）
│   ├── tap.rs            — TapRecognizer（单击 + 双击）
│   ├── drag.rs           — DragRecognizer（方向 + 阈值）
│   └── long_press.rs     — LongPressRecognizer（时间阈值）
├── widget/
│   ├── state.rs          — InteractionState 枚举（从竞技场结果推导）
│   ├── action.rs         — Action 枚举
│   └── ...
```

## 核心类型

### GestureRecognizer trait（recognizer.rs）

```rust
pub trait GestureRecognizer {
    /// 指针按下，是否对这个手势感兴趣
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool;

    /// 指针移动
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition;

    /// 指针松开
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition;

    /// 你赢了，执行手势对应的 Action
    fn accept(&mut self) -> Action;

    /// 你输了，清理状态
    fn reject(&mut self);
}

pub enum GestureDisposition {
    Pending,   // 还在观察
    Accepted,  // 确认是我的手势
    Rejected,  // 不是我的手势
}
```

### GestureArena（arena.rs）

```rust
pub struct GestureArena {
    members: Vec<Box<dyn GestureRecognizer>>,
    target_id: String,  // 命中的节点 id
}

impl GestureArena {
    pub fn new(target_id: String) -> Self;
    pub fn add(&mut self, recognizer: Box<dyn GestureRecognizer>);
    pub fn pointer_move(&mut self, x: f32, y: f32) -> Option<Action>;
    pub fn pointer_up(&mut self, x: f32, y: f32) -> Option<Action>;
    pub fn is_resolved(&self) -> bool;
    pub fn target_id(&self) -> &str;
}
```

竞技场规则：
1. 识别器返回 Rejected → 移出竞技场
2. 只剩一个识别器 → 自动赢，调 accept()
3. 识别器返回 Accepted → 宣布获胜，其他全部 reject()

### InteractionState（widget/state.rs）

```rust
pub enum InteractionState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
```

不存在节点上，从当前状态推导：
- 竞技场活跃且目标是我 → Pressed
- 鼠标在我身上（hit test）→ Hovered
- 有键盘焦点 → Focused
- 控件标记 disabled → Disabled
- 以上都不是 → Normal

### Action（widget/action.rs）

```rust
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart(String, f32, f32),
    DragMove(String, f32, f32),
    DragEnd(String, f32, f32),
    LongPress(String),
}
```

## 三个识别器

### TapRecognizer（tap.rs）

```
on_pointer_down → 记录位置和时间，返回 true
on_pointer_move → 移动超过 3px → Rejected
on_pointer_up   → Accepted
accept          → 距上次 tap < 300ms → DoubleClick，否则 → Click
```

双击检测需要记录上次 tap 的时间，由 TapRecognizer 自身持有 `last_tap_time: Option<Instant>`。

### DragRecognizer（drag.rs）

```
on_pointer_down → 记录起始位置，返回 true
on_pointer_move → 移动超过 3px → Accepted（首次返回 DragStart）
                  已 Accepted → 直接返回 Accepted（后续 DragMove 在 accept 时处理）
on_pointer_up   → 如果已 Accepted → Accepted（DragEnd），否则 → Rejected
```

DragRecognizer 一旦 Accepted，后续 move 直接产出 DragMove Action，不再经过竞技场裁决。

### LongPressRecognizer（long_press.rs）

```
on_pointer_down → 记录时间，返回 true
on_pointer_move → 移动超过 3px → Rejected
                  未移动且经过 500ms → Accepted
on_pointer_up   → 如果 Pending → Rejected（松开太早）
```

定时器实现：不用系统定时器。在 on_pointer_move 中用 Instant::now() - down_time 判断是否超过阈值。需要外部每帧调一次 tick 或在 move 事件中顺带检查。

## 事件流（分层路由）

```
AppEvent 进入
  │
  ├── MousePress { x, y, Left }
  │     ① 竞技场已有活跃成员？→ 忽略（一次只处理一个指针）
  │     ② 面板层 hit test
  │       → 命中面板边缘？→ DragRecognizer(resize) → 进竞技场
  │       → 命中面板标题？→ DragRecognizer(move) → 进竞技场
  │       → 命中控件节点？→ 按控件类型创建识别器 → 进竞技场
  │           Button → TapRecognizer + LongPressRecognizer
  │           Slider → TapRecognizer + DragRecognizer
  │       → 没命中面板？→ 交给画布处理
  │
  ├── MouseMove { x, y }
  │     → 竞技场活跃？→ 分发给竞技场 → 返回 Option<Action>
  │     → 竞技场不活跃？→ 更新 hovered（hit test）
  │
  └── MouseRelease { x, y }
        → 竞技场活跃？→ 分发给竞技场 → 返回 Option<Action>
        → 清空竞技场
```

## 现有代码改造

| 文件 | 改造 |
|------|------|
| `panel/drag.rs` | 删除 start/update/end 状态机，保留位置更新逻辑，改为响应 DragMove Action |
| `panel/resize.rs` | 删除 start/update/end 状态机，保留 detect_edge + resize 计算，改为响应 DragMove Action |
| `demo.rs` event() | 不再手写 if/match，改为：创建竞技场 → 分发事件 → 处理返回的 Action |
| `widget/state.rs` | 实现 InteractionState 枚举 |
| `widget/action.rs` | 实现 Action 枚举 |
| `lib.rs` | 新增 `pub mod gesture;` |
| `canvas/event.rs` | 不改造（画布手势独立于控件系统） |

## 验证

```bash
cargo build -p gui
cargo run -p gui
```

验证场景：
1. Button 点击产出 Click Action
2. Slider 拖拽产出 DragStart/DragMove/DragEnd
3. Slider 快速点击不触发拖拽
4. 面板拖拽移动正常
5. 面板边缘 resize 正常
6. 画布平移缩放不受影响
