# 手势识别系统 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Flutter 竞技场模式的手势识别系统，替换现有手写状态机

**Architecture:** 分层路由 + 竞技场。事件按层级路由（面板→画布），同一节点上多手势通过 GestureArena 裁决。三个识别器：Tap（点击/双击）、Drag（拖拽）、LongPress（长按）。

**Tech Stack:** Rust, std::time::Instant

**Spec:** `docs/superpowers/specs/2026-04-09-gesture-system-design.md`

---

### Task 1: Action 枚举 + InteractionState

**Files:**
- Modify: `gui/src/widget/action.rs`
- Modify: `gui/src/widget/state.rs`
- Modify: `gui/src/widget/mod.rs`

- [ ] **Step 1: 实现 Action 枚举**

```rust
// gui/src/widget/action.rs

/// 手势识别后产出的动作。
#[derive(Debug, Clone)]
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart { id: String, x: f32, y: f32 },
    DragMove { id: String, x: f32, y: f32 },
    DragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
}
```

- [ ] **Step 2: 实现 InteractionState 枚举**

```rust
// gui/src/widget/state.rs

/// 控件的交互状态。不存在节点上，从竞技场和 hit test 结果推导。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
```

- [ ] **Step 3: 更新 widget/mod.rs 导出**

```rust
// gui/src/widget/mod.rs — 在现有 mod 声明下面改为 pub mod：
pub mod action;
pub mod state;
// （其余不变）
```

- [ ] **Step 4: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过，warning 可忽略

- [ ] **Step 5: Commit**

```bash
git add gui/src/widget/action.rs gui/src/widget/state.rs gui/src/widget/mod.rs
git commit -m "feat: Action 枚举 + InteractionState"
```

---

### Task 2: GestureRecognizer trait + GestureDisposition

**Files:**
- Create: `gui/src/gesture/mod.rs`
- Create: `gui/src/gesture/recognizer.rs`
- Modify: `gui/src/lib.rs`

- [ ] **Step 1: 创建 gesture/recognizer.rs**

```rust
// gui/src/gesture/recognizer.rs

use crate::widget::action::Action;

/// 识别器的裁决结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureDisposition {
    /// 还在观察，不确定
    Pending,
    /// 确认是我的手势
    Accepted,
    /// 不是我的手势
    Rejected,
}

/// 手势识别器 trait。每种手势（点击、拖拽、长按）实现此 trait。
pub trait GestureRecognizer {
    /// 指针按下，是否对这个手势感兴趣
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool;

    /// 指针移动
    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition;

    /// 指针松开
    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition;

    /// 你赢了，返回对应的 Action
    fn accept(&mut self) -> Action;

    /// 你输了，清理状态
    fn reject(&mut self);
}
```

- [ ] **Step 2: 创建 gesture/mod.rs**

```rust
// gui/src/gesture/mod.rs

mod recognizer;

pub use recognizer::{GestureDisposition, GestureRecognizer};
```

- [ ] **Step 3: 更新 lib.rs**

在 `gui/src/lib.rs` 的 mod 列表中添加：

```rust
pub mod gesture;
```

- [ ] **Step 4: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add gui/src/gesture/ gui/src/lib.rs
git commit -m "feat: GestureRecognizer trait + GestureDisposition"
```

---

### Task 3: GestureArena

**Files:**
- Create: `gui/src/gesture/arena.rs`
- Modify: `gui/src/gesture/mod.rs`

- [ ] **Step 1: 实现 GestureArena**

```rust
// gui/src/gesture/arena.rs

use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

/// 手势竞技场。收集识别器，分发事件，裁决胜负。
pub struct GestureArena {
    members: Vec<Box<dyn GestureRecognizer>>,
    target_id: String,
    resolved: bool,
    winner_action: Option<Action>,
}

impl GestureArena {
    pub fn new(target_id: String) -> Self {
        Self {
            members: Vec::new(),
            target_id,
            resolved: false,
            winner_action: None,
        }
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved
    }

    pub fn add(&mut self, recognizer: Box<dyn GestureRecognizer>) {
        self.members.push(recognizer);
    }

    /// 分发 pointer_move。返回赢家产出的 Action（如果有）。
    pub fn pointer_move(&mut self, x: f32, y: f32) -> Option<Action> {
        if self.resolved {
            return None;
        }

        // 已经有赢家在持续产出 Action（如 DragMove）
        if let Some(ref mut action) = self.winner_action {
            // 赢家是唯一剩余成员
            if let Some(member) = self.members.first_mut() {
                let disp = member.on_pointer_move(x, y);
                if disp == GestureDisposition::Accepted {
                    return Some(member.accept());
                }
            }
            return None;
        }

        let mut accepted_idx = None;

        // 逆序遍历以便安全删除
        let mut i = self.members.len();
        while i > 0 {
            i -= 1;
            let disp = self.members[i].on_pointer_move(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => {
                    accepted_idx = Some(i);
                    break;
                }
                GestureDisposition::Pending => {}
            }
        }

        // 有人主动宣布获胜
        if let Some(idx) = accepted_idx {
            return Some(self.resolve_winner(idx));
        }

        // 只剩一个，自动赢
        self.try_auto_resolve()
    }

    /// 分发 pointer_up。返回赢家产出的 Action（如果有）。
    pub fn pointer_up(&mut self, x: f32, y: f32) -> Option<Action> {
        if self.resolved {
            return None;
        }

        // 已有赢家（如 Drag），通知松开
        if self.winner_action.is_some() {
            if let Some(member) = self.members.first_mut() {
                member.on_pointer_up(x, y);
                let action = member.accept();
                self.resolved = true;
                return Some(action);
            }
            return None;
        }

        let mut accepted_idx = None;

        let mut i = self.members.len();
        while i > 0 {
            i -= 1;
            let disp = self.members[i].on_pointer_up(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => {
                    accepted_idx = Some(i);
                    break;
                }
                GestureDisposition::Pending => {}
            }
        }

        if let Some(idx) = accepted_idx {
            let action = self.resolve_winner(idx);
            self.resolved = true;
            return Some(action);
        }

        // pointer_up 后只剩一个 → 自动赢
        if let Some(action) = self.try_auto_resolve() {
            self.resolved = true;
            return Some(action);
        }

        // 全部 rejected
        if self.members.is_empty() {
            self.resolved = true;
        }

        None
    }

    /// 指定索引的识别器获胜，其余全部 reject。
    fn resolve_winner(&mut self, winner_idx: usize) -> Action {
        // reject 所有非赢家
        for (i, member) in self.members.iter_mut().enumerate() {
            if i != winner_idx {
                member.reject();
            }
        }
        let winner = self.members.swap_remove(winner_idx);
        self.members.clear();
        self.members.push(winner);
        let action = self.members[0].accept();
        self.winner_action = Some(action.clone());
        action
    }

    /// 只剩一个成员时自动获胜。
    fn try_auto_resolve(&mut self) -> Option<Action> {
        if self.members.len() == 1 {
            let action = self.members[0].accept();
            self.winner_action = Some(action.clone());
            Some(action)
        } else {
            None
        }
    }
}
```

- [ ] **Step 2: 更新 gesture/mod.rs**

```rust
// gui/src/gesture/mod.rs

mod arena;
mod recognizer;

pub use arena::GestureArena;
pub use recognizer::{GestureDisposition, GestureRecognizer};
```

- [ ] **Step 3: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add gui/src/gesture/
git commit -m "feat: GestureArena 竞技场裁决"
```

---

### Task 4: TapRecognizer

**Files:**
- Create: `gui/src/gesture/tap.rs`
- Modify: `gui/src/gesture/mod.rs`

- [ ] **Step 1: 实现 TapRecognizer**

```rust
// gui/src/gesture/tap.rs

use std::time::Instant;

use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;
const DOUBLE_TAP_TIMEOUT_MS: u128 = 300;

pub struct TapRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    down_time: Instant,
    last_tap_time: Option<Instant>,
    rejected: bool,
}

impl TapRecognizer {
    pub fn new(target_id: String, last_tap_time: Option<Instant>) -> Self {
        Self {
            target_id,
            down_x: 0.0,
            down_y: 0.0,
            down_time: Instant::now(),
            last_tap_time,
            rejected: false,
        }
    }
}

impl GestureRecognizer for TapRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.down_time = Instant::now();
        true
    }

    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        let dx = x - self.down_x;
        let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.rejected = true;
            GestureDisposition::Rejected
        } else {
            GestureDisposition::Pending
        }
    }

    fn on_pointer_up(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        if self.rejected {
            GestureDisposition::Rejected
        } else {
            GestureDisposition::Accepted
        }
    }

    fn accept(&mut self) -> Action {
        if let Some(last) = self.last_tap_time {
            if self.down_time.duration_since(last).as_millis() < DOUBLE_TAP_TIMEOUT_MS {
                return Action::DoubleClick(self.target_id.clone());
            }
        }
        Action::Click(self.target_id.clone())
    }

    fn reject(&mut self) {
        self.rejected = true;
    }
}
```

- [ ] **Step 2: 更新 gesture/mod.rs 导出**

```rust
// gui/src/gesture/mod.rs

mod arena;
mod recognizer;
mod tap;

pub use arena::GestureArena;
pub use recognizer::{GestureDisposition, GestureRecognizer};
pub use tap::TapRecognizer;
```

- [ ] **Step 3: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add gui/src/gesture/
git commit -m "feat: TapRecognizer（点击/双击）"
```

---

### Task 5: DragRecognizer

**Files:**
- Create: `gui/src/gesture/drag.rs`
- Modify: `gui/src/gesture/mod.rs`

- [ ] **Step 1: 实现 DragRecognizer**

```rust
// gui/src/gesture/drag.rs

use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;

pub struct DragRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    current_x: f32,
    current_y: f32,
    dragging: bool,
    done: bool,
}

impl DragRecognizer {
    pub fn new(target_id: String) -> Self {
        Self {
            target_id,
            down_x: 0.0,
            down_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            dragging: false,
            done: false,
        }
    }
}

impl GestureRecognizer for DragRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.current_x = x;
        self.current_y = y;
        true
    }

    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;

        if self.dragging {
            // 已经在拖拽中，持续 Accepted
            return GestureDisposition::Accepted;
        }

        let dx = x - self.down_x;
        let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.dragging = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Pending
        }
    }

    fn on_pointer_up(&mut self, x: f32, y: f32) -> GestureDisposition {
        self.current_x = x;
        self.current_y = y;
        if self.dragging {
            self.done = true;
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Rejected
        }
    }

    fn accept(&mut self) -> Action {
        if self.done {
            Action::DragEnd {
                id: self.target_id.clone(),
                x: self.current_x,
                y: self.current_y,
            }
        } else if self.dragging {
            Action::DragMove {
                id: self.target_id.clone(),
                x: self.current_x,
                y: self.current_y,
            }
        } else {
            // 首次 accept = DragStart
            self.dragging = true;
            Action::DragStart {
                id: self.target_id.clone(),
                x: self.current_x,
                y: self.current_y,
            }
        }
    }

    fn reject(&mut self) {}
}
```

- [ ] **Step 2: 更新 gesture/mod.rs 导出**

```rust
// gui/src/gesture/mod.rs

mod arena;
mod drag;
mod recognizer;
mod tap;

pub use arena::GestureArena;
pub use drag::DragRecognizer;
pub use recognizer::{GestureDisposition, GestureRecognizer};
pub use tap::TapRecognizer;
```

- [ ] **Step 3: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add gui/src/gesture/
git commit -m "feat: DragRecognizer（拖拽识别）"
```

---

### Task 6: LongPressRecognizer

**Files:**
- Create: `gui/src/gesture/long_press.rs`
- Modify: `gui/src/gesture/mod.rs`

- [ ] **Step 1: 实现 LongPressRecognizer**

```rust
// gui/src/gesture/long_press.rs

use std::time::Instant;

use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

const MOVE_THRESHOLD: f32 = 3.0;
const LONG_PRESS_DURATION_MS: u128 = 500;

pub struct LongPressRecognizer {
    target_id: String,
    down_x: f32,
    down_y: f32,
    down_time: Instant,
    triggered: bool,
    rejected: bool,
}

impl LongPressRecognizer {
    pub fn new(target_id: String) -> Self {
        Self {
            target_id,
            down_x: 0.0,
            down_y: 0.0,
            down_time: Instant::now(),
            triggered: false,
            rejected: false,
        }
    }
}

impl GestureRecognizer for LongPressRecognizer {
    fn on_pointer_down(&mut self, x: f32, y: f32) -> bool {
        self.down_x = x;
        self.down_y = y;
        self.down_time = Instant::now();
        true
    }

    fn on_pointer_move(&mut self, x: f32, y: f32) -> GestureDisposition {
        let dx = x - self.down_x;
        let dy = y - self.down_y;
        if dx * dx + dy * dy > MOVE_THRESHOLD * MOVE_THRESHOLD {
            self.rejected = true;
            return GestureDisposition::Rejected;
        }

        // 检查时间
        if Instant::now().duration_since(self.down_time).as_millis() >= LONG_PRESS_DURATION_MS {
            self.triggered = true;
            return GestureDisposition::Accepted;
        }

        GestureDisposition::Pending
    }

    fn on_pointer_up(&mut self, _x: f32, _y: f32) -> GestureDisposition {
        if self.triggered {
            GestureDisposition::Accepted
        } else {
            GestureDisposition::Rejected
        }
    }

    fn accept(&mut self) -> Action {
        Action::LongPress(self.target_id.clone())
    }

    fn reject(&mut self) {
        self.rejected = true;
    }
}
```

- [ ] **Step 2: 更新 gesture/mod.rs 导出**

```rust
// gui/src/gesture/mod.rs

mod arena;
mod drag;
mod long_press;
mod recognizer;
mod tap;

pub use arena::GestureArena;
pub use drag::DragRecognizer;
pub use long_press::LongPressRecognizer;
pub use recognizer::{GestureDisposition, GestureRecognizer};
pub use tap::TapRecognizer;
```

- [ ] **Step 3: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add gui/src/gesture/
git commit -m "feat: LongPressRecognizer（长按识别）"
```

---

### Task 7: 改造 panel/drag.rs 和 panel/resize.rs

**Files:**
- Modify: `gui/src/panel/drag.rs`
- Modify: `gui/src/panel/resize.rs`
- Modify: `gui/src/panel/mod.rs`

- [ ] **Step 1: 简化 drag.rs 为纯业务逻辑**

drag.rs 不再管手势状态机，只提供位置更新函数：

```rust
// gui/src/panel/drag.rs

use super::layer::PanelLayer;

/// 根据 Action 中的坐标更新面板位置。
pub fn apply_drag_move(layer: &mut PanelLayer, panel_id: &str, x: f32, y: f32, last_x: f32, last_y: f32) {
    let dx = x - last_x;
    let dy = y - last_y;
    if let Some(frame) = layer.get_mut(panel_id) {
        frame.x += dx;
        frame.y += dy;
    }
}
```

- [ ] **Step 2: 简化 resize.rs**

保留 `detect_edge` 和 resize 计算逻辑，删除状态机：

```rust
// gui/src/panel/resize.rs

use crate::shell::CursorStyle;
use super::frame::PanelFrame;
use super::layer::PanelLayer;

const EDGE_THRESHOLD: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
pub enum ResizeEdge {
    Top, Bottom, Left, Right,
    TopLeft, TopRight, BottomLeft, BottomRight,
}

impl ResizeEdge {
    pub fn cursor_style(self) -> CursorStyle {
        match self {
            Self::Top => CursorStyle::ResizeN,
            Self::Bottom => CursorStyle::ResizeS,
            Self::Left => CursorStyle::ResizeW,
            Self::Right => CursorStyle::ResizeE,
            Self::TopLeft => CursorStyle::ResizeNW,
            Self::TopRight => CursorStyle::ResizeNE,
            Self::BottomLeft => CursorStyle::ResizeSW,
            Self::BottomRight => CursorStyle::ResizeSE,
        }
    }
}

/// 检测鼠标在面板的哪个边缘。返回 None 表示不在边缘。
pub fn detect_edge(frame: &PanelFrame, x: f32, y: f32) -> Option<ResizeEdge> {
    let r = frame.rect();
    let near_left = (x - r.x).abs() < EDGE_THRESHOLD;
    let near_right = (x - (r.x + r.w)).abs() < EDGE_THRESHOLD;
    let near_top = (y - r.y).abs() < EDGE_THRESHOLD;
    let near_bottom = (y - (r.y + r.h)).abs() < EDGE_THRESHOLD;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
    }
}

/// 根据 Action 中的坐标执行 resize。
pub fn apply_resize(layer: &mut PanelLayer, panel_id: &str, edge: ResizeEdge, x: f32, y: f32, last_x: f32, last_y: f32) {
    let dx = x - last_x;
    let dy = y - last_y;

    let Some(frame) = layer.get_mut(panel_id) else { return };

    match edge {
        ResizeEdge::Right => {
            frame.w = (frame.w + dx).max(frame.min_w);
        }
        ResizeEdge::Bottom => {
            frame.h = (frame.h + dy).max(frame.min_h);
        }
        ResizeEdge::Left => {
            let new_w = (frame.w - dx).max(frame.min_w);
            frame.x += frame.w - new_w;
            frame.w = new_w;
        }
        ResizeEdge::Top => {
            let new_h = (frame.h - dy).max(frame.min_h);
            frame.y += frame.h - new_h;
            frame.h = new_h;
        }
        ResizeEdge::TopLeft => {
            let new_w = (frame.w - dx).max(frame.min_w);
            let new_h = (frame.h - dy).max(frame.min_h);
            frame.x += frame.w - new_w;
            frame.y += frame.h - new_h;
            frame.w = new_w;
            frame.h = new_h;
        }
        ResizeEdge::TopRight => {
            frame.w = (frame.w + dx).max(frame.min_w);
            let new_h = (frame.h - dy).max(frame.min_h);
            frame.y += frame.h - new_h;
            frame.h = new_h;
        }
        ResizeEdge::BottomLeft => {
            let new_w = (frame.w - dx).max(frame.min_w);
            frame.x += frame.w - new_w;
            frame.w = new_w;
            frame.h = (frame.h + dy).max(frame.min_h);
        }
        ResizeEdge::BottomRight => {
            frame.w = (frame.w + dx).max(frame.min_w);
            frame.h = (frame.h + dy).max(frame.min_h);
        }
    }
}
```

- [ ] **Step 3: 更新 panel/mod.rs 导出**

```rust
// gui/src/panel/mod.rs

mod drag;
mod frame;
mod hit;
mod layer;
mod resize;
pub mod tree;

pub use drag::apply_drag_move;
pub use frame::PanelFrame;
pub use hit::hit_test_panel;
pub use layer::PanelLayer;
pub use resize::{ResizeEdge, apply_resize, detect_edge};
```

- [ ] **Step 4: 编译验证**

Run: `cargo build -p gui`
Expected: 编译错误（demo.rs 还在用旧的 DragState/ResizeState），下一个 task 修

- [ ] **Step 5: Commit**

```bash
git add gui/src/panel/drag.rs gui/src/panel/resize.rs gui/src/panel/mod.rs
git commit -m "refactor: panel drag/resize 去除状态机，改为纯函数"
```

---

### Task 8: 改造 demo.rs 接入手势系统

**Files:**
- Modify: `gui/src/demo.rs`

- [ ] **Step 1: 重写 demo.rs**

```rust
// gui/src/demo.rs

use std::borrow::Cow;
use std::time::Instant;

use crate::canvas::Canvas;
use crate::gesture::{GestureArena, TapRecognizer, DragRecognizer};
use crate::widget::action::Action;
use crate::widget::atoms::button::ButtonProps;
use crate::widget::atoms::slider::SliderProps;
use crate::widget::layout::{BoxStyle, Direction, resolve};
use crate::panel::{PanelFrame, PanelLayer, ResizeEdge, apply_drag_move, apply_resize, detect_edge, hit_test_panel};
use crate::panel::tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Rect, Renderer};
use crate::shell::{App, AppContext, AppEvent, MouseButton};

const PADDING: f32 = 16.0;

fn build_view(active: Option<&str>, slider_value: f32) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("__root"),
        style: BoxStyle {
            direction: Direction::Column,
            gap: 8.0,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            Desc::Widget {
                id: Cow::Borrowed("btn_a"),
                props: Box::new(ButtonProps {
                    label: "Button A".into(),
                    icon: None,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("btn_b"),
                props: Box::new(ButtonProps {
                    label: "Button B".into(),
                    icon: None,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("slider_radius"),
                props: Box::new(SliderProps {
                    label: "Radius".into(),
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    value: slider_value,
                    disabled: false,
                }),
            },
        ],
    }
}

/// 面板交互的额外上下文（拖拽/resize 需要记住上一帧坐标和边缘）
struct PanelInteraction {
    drag_panel: Option<&'static str>,
    resize_panel: Option<&'static str>,
    resize_edge: Option<ResizeEdge>,
    last_x: f32,
    last_y: f32,
}

impl PanelInteraction {
    fn new() -> Self {
        Self {
            drag_panel: None,
            resize_panel: None,
            resize_edge: None,
            last_x: 0.0,
            last_y: 0.0,
        }
    }
}

pub struct DemoApp {
    canvas: Canvas,
    layer: PanelLayer,
    tree: PanelTree,
    arena: Option<GestureArena>,
    panel_interaction: PanelInteraction,
    active_button: Option<String>,
    slider_value: f32,
    last_tap_time: Option<Instant>,
    mouse_x: f32,
    mouse_y: f32,
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        let mut layer = PanelLayer::new();
        layer.add(PanelFrame::new("demo", 100.0, 100.0, 300.0, 200.0));

        Self {
            canvas: Canvas::new(),
            layer,
            tree: PanelTree::new(),
            arena: None,
            panel_interaction: PanelInteraction::new(),
            active_button: None,
            slider_value: 5.0,
            last_tap_time: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn event(&mut self, event: AppEvent, ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::debug!("{:?}", event);
        }

        match &event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                // 竞技场已有活跃 → 忽略
                if self.arena.is_some() {
                    return;
                }

                self.mouse_x = *x;
                self.mouse_y = *y;

                // 分层路由：面板层
                if let Some(panel_id) = hit_test_panel(&self.layer, *x, *y) {
                    let frame = self.layer.get(panel_id).unwrap();

                    if let Some(edge) = detect_edge(frame, *x, *y) {
                        // 面板边缘 → resize 拖拽
                        let mut arena = GestureArena::new(format!("panel_resize:{panel_id}"));
                        let mut rec = DragRecognizer::new(format!("panel_resize:{panel_id}"));
                        rec.on_pointer_down(*x, *y);
                        arena.add(Box::new(rec));
                        self.arena = Some(arena);
                        self.panel_interaction.resize_panel = Some(panel_id);
                        self.panel_interaction.resize_edge = Some(edge);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    } else if let Some(root) = self.tree.root() {
                        if let Some(widget_id) = hit_test(&self.tree, root, *x, *y) {
                            // 命中控件 → 根据控件类型创建识别器
                            let mut arena = GestureArena::new(widget_id.to_string());
                            let mut tap = TapRecognizer::new(widget_id.to_string(), self.last_tap_time);
                            tap.on_pointer_down(*x, *y);
                            arena.add(Box::new(tap));

                            // Slider 额外加 DragRecognizer
                            if widget_id.contains("slider") || widget_id.contains("track") || widget_id.contains("fill") || widget_id.contains("spacer") {
                                let mut drag = DragRecognizer::new(widget_id.to_string());
                                drag.on_pointer_down(*x, *y);
                                arena.add(Box::new(drag));
                            }

                            self.arena = Some(arena);
                        } else {
                            // 命中面板但没命中控件 → 面板拖拽
                            let mut arena = GestureArena::new(format!("panel_drag:{panel_id}"));
                            let mut rec = DragRecognizer::new(format!("panel_drag:{panel_id}"));
                            rec.on_pointer_down(*x, *y);
                            arena.add(Box::new(rec));
                            self.arena = Some(arena);
                            self.panel_interaction.drag_panel = Some(panel_id);
                            self.panel_interaction.last_x = *x;
                            self.panel_interaction.last_y = *y;
                        }
                    } else {
                        // 没有控件树 → 面板拖拽
                        let mut arena = GestureArena::new(format!("panel_drag:{panel_id}"));
                        let mut rec = DragRecognizer::new(format!("panel_drag:{panel_id}"));
                        rec.on_pointer_down(*x, *y);
                        arena.add(Box::new(rec));
                        self.arena = Some(arena);
                        self.panel_interaction.drag_panel = Some(panel_id);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    }

                    self.layer.bring_to_front(panel_id);
                    return;
                }

                // 没命中面板 → 画布
                self.canvas.event(&event);
            }

            AppEvent::MouseMove { x, y } => {
                self.mouse_x = *x;
                self.mouse_y = *y;

                if let Some(arena) = &mut self.arena {
                    if let Some(action) = arena.pointer_move(*x, *y) {
                        self.handle_action(action);
                    }
                } else {
                    // 光标样式
                    if let Some(panel_id) = hit_test_panel(&self.layer, *x, *y) {
                        if let Some(frame) = self.layer.get(panel_id) {
                            if let Some(edge) = detect_edge(frame, *x, *y) {
                                ctx.cursor.set(edge.cursor_style());
                            }
                        }
                    }
                    self.canvas.event(&event);
                }
            }

            AppEvent::MouseRelease { x, y, button } if *button == MouseButton::Left => {
                if let Some(mut arena) = self.arena.take() {
                    if let Some(action) = arena.pointer_up(*x, *y) {
                        self.handle_action(action);
                    }
                    // 清理面板交互状态
                    self.panel_interaction.drag_panel = None;
                    self.panel_interaction.resize_panel = None;
                    self.panel_interaction.resize_edge = None;
                } else {
                    self.canvas.event(&event);
                }
            }

            _ => {
                self.canvas.event(&event);
            }
        }
    }

    fn update(&mut self, _renderer: &mut Renderer, _ctx: &mut AppContext) {
        let desc = build_view(self.active_button.as_deref(), self.slider_value);
        reconcile(&mut self.tree, desc);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        if let (Some(frame), Some(root)) = (self.layer.get("demo"), self.tree.root()) {
            let content_rect = Rect {
                x: frame.x + PADDING,
                y: frame.y + PADDING,
                w: frame.w - PADDING * 2.0,
                h: frame.h - PADDING * 2.0,
            };
            resolve::resolve(&mut self.tree, renderer.text_measurer());
            layout(&mut self.tree, root, content_rect);
        }
        let viewport_w = ctx.size.width as f32 / ctx.scale_factor as f32;
        let viewport_h = ctx.size.height as f32 / ctx.scale_factor as f32;

        self.canvas.render(renderer, viewport_w, viewport_h);

        let tree = &self.tree;
        self.layer.render(renderer, |_frame, renderer| {
            if let Some(root) = tree.root() {
                paint(tree, root, renderer);
            }
        });
    }
}

impl DemoApp {
    fn handle_action(&mut self, action: Action) {
        tracing::debug!("Action: {:?}", action);
        match &action {
            Action::Click(id) => {
                self.last_tap_time = Some(Instant::now());
                self.active_button = Some(id.clone());
            }
            Action::DoubleClick(id) => {
                self.last_tap_time = None;
                // Slider 双击复位
                if id.contains("slider") {
                    self.slider_value = 5.0;
                }
            }
            Action::DragMove { id, x, y } => {
                if let Some(panel_id) = self.panel_interaction.drag_panel {
                    apply_drag_move(&mut self.layer, panel_id, *x, *y, self.panel_interaction.last_x, self.panel_interaction.last_y);
                    self.panel_interaction.last_x = *x;
                    self.panel_interaction.last_y = *y;
                } else if let Some(panel_id) = self.panel_interaction.resize_panel {
                    if let Some(edge) = self.panel_interaction.resize_edge {
                        apply_resize(&mut self.layer, panel_id, edge, *x, *y, self.panel_interaction.last_x, self.panel_interaction.last_y);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    }
                } else if id.contains("slider") {
                    // Slider 拖拽：根据 x 坐标计算新值
                    // 简单实现：暂时用线性映射
                    tracing::debug!("Slider drag at ({}, {})", x, y);
                }
            }
            Action::DragStart { .. } | Action::DragEnd { .. } => {}
            Action::LongPress(id) => {
                tracing::debug!("LongPress: {}", id);
            }
        }
    }
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build -p gui`
Expected: 编译通过

- [ ] **Step 3: 运行验证**

Run: `cargo run -p gui`
Expected:
- Button 点击打印 Action::Click
- Slider 拖拽打印 Action::DragStart/DragMove/DragEnd
- 面板可拖拽移动
- 面板边缘可 resize
- 画布平移缩放正常

- [ ] **Step 4: Commit**

```bash
git add gui/src/demo.rs
git commit -m "feat: demo 接入手势系统（竞技场模式）"
```
