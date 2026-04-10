# Preview Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a floating preview panel that displays an image with zoom/pan, draggable movement, and resizable edges.

**Architecture:** Extend `FloatingPanel` with resize support, then build `PreviewPanel` using iced `canvas::Program` for image rendering with zoom/pan. Integrate into `App` with a unified interaction overlay layer.

**Tech Stack:** Rust, iced 0.14 (canvas + image features)

**Spec:** `docs/superpowers/specs/2026-04-07-preview-panel-design.md`

---

## File Structure

| File | Role |
|------|------|
| `gui/Cargo.toml` | Add `"image"` to iced features |
| `gui/src/panel.rs` | FloatingPanel: add `size`, resize events, `is_interacting()` |
| `gui/src/preview.rs` | PreviewPanel: canvas Program for image zoom/pan, title bar, resize handle |
| `gui/src/lib.rs` | Integrate preview into layout, unify interaction overlay for toolbar + preview |

---

### Task 1: Add iced `image` feature

**Files:**
- Modify: `gui/Cargo.toml:11`

- [ ] **Step 1: Add `"image"` feature to iced dependency**

```toml
iced = { version = "0.14", features = ["canvas", "image", "wgpu", "tokio"] }
```

- [ ] **Step 2: Verify it compiles**

Run: `./scripts/windows_run.sh` (will build then launch; Ctrl+C after build succeeds)
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add gui/Cargo.toml Cargo.lock
git commit -m "chore: add iced image feature for preview panel"
```

---

### Task 2: Extend FloatingPanel with resize

**Files:**
- Modify: `gui/src/panel.rs`

- [ ] **Step 1: Add `Size` import and `size` / resize state fields**

```rust
use iced::{Point, Size, Vector};

#[derive(Debug)]
pub struct FloatingPanel {
    /// 相对于默认布局位置的偏移
    pub offset: Vector,
    /// 面板尺寸
    pub size: Size,
    /// 拖拽状态
    dragging: bool,
    /// resize 状态
    resizing: bool,
    /// 上一帧光标位置（窗口坐标系）
    last_cursor: Option<Point>,
}
```

- [ ] **Step 2: Add resize events to `Event` enum**

```rust
#[derive(Debug, Clone)]
pub enum Event {
    DragStart,
    DragMove(Point),
    DragEnd,
    ResizeStart,
    ResizeMove(Point),
    ResizeEnd,
}
```

- [ ] **Step 3: Update `new()` to accept initial size**

```rust
impl FloatingPanel {
    pub fn new(size: Size) -> Self {
        Self {
            offset: Vector::ZERO,
            size,
            dragging: false,
            resizing: false,
            last_cursor: None,
        }
    }
```

- [ ] **Step 4: Handle resize events in `handle_event()`**

Add these arms to the match:

```rust
Event::ResizeStart => {
    self.resizing = true;
    self.last_cursor = None;
}
Event::ResizeMove(cursor) => {
    if !self.resizing {
        return;
    }
    if let Some(last) = self.last_cursor {
        let dw = cursor.x - last.x;
        let dh = cursor.y - last.y;
        self.size = Size::new(
            (self.size.width + dw).max(120.0),
            (self.size.height + dh).max(80.0),
        );
    }
    self.last_cursor = Some(cursor);
}
Event::ResizeEnd => {
    self.resizing = false;
    self.last_cursor = None;
}
```

- [ ] **Step 5: Add `is_resizing()` and `is_interacting()` methods**

```rust
pub fn is_resizing(&self) -> bool {
    self.resizing
}

pub fn is_interacting(&self) -> bool {
    self.dragging || self.resizing
}
```

- [ ] **Step 6: Update Toolbar to use `new(size)` — pass a dummy size since toolbar doesn't use it**

In `gui/src/toolbar.rs`, change:
```rust
// old
panel: FloatingPanel::new(),
// new
panel: FloatingPanel::new(Size::ZERO),
```

Add `Size` to the iced import line.

- [ ] **Step 7: Verify it compiles**

Run: `./scripts/windows_run.sh`
Expected: compiles, app runs, toolbar still works

- [ ] **Step 8: Commit**

```bash
git add gui/src/panel.rs gui/src/toolbar.rs
git commit -m "feat: extend FloatingPanel with resize support"
```

---

### Task 3: Implement PreviewPanel

**Files:**
- Modify: `gui/src/preview.rs`

- [ ] **Step 1: Define PreviewPanel struct and Message enum**

```rust
//! 预览面板 (2.2.0)
//! 浮动面板，显示图片预览。支持图片缩放/平移、面板拖拽移动、面板 resize。

use crate::panel::{self, FloatingPanel};
use iced::widget::{canvas, column, container, mouse_area, row, text};
use iced::{
    mouse, Border, Color, Element, Fill, Length, Point, Rectangle, Renderer, Shadow, Size, Theme,
    Vector,
};

const TITLE_BAR_HEIGHT: f32 = 28.0;
const RESIZE_HANDLE_SIZE: f32 = 16.0;
const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 10.0;
const ZOOM_STEP: f32 = 0.05;

pub struct PreviewPanel {
    pub panel: FloatingPanel,
    image: Option<image::Handle>,
    /// 图片平移偏移
    offset: Vector,
    /// 图片缩放
    scale: f32,
    /// 图片内拖拽
    drag_origin: Option<Point>,
    starting_offset: Vector,
    /// canvas 缓存
    cache: canvas::Cache,
}
```

Note: `image::Handle` here is `iced::widget::image::Handle`.

- [ ] **Step 2: Implement `new()` and image loading**

```rust
impl PreviewPanel {
    pub fn new() -> Self {
        let image = Self::load_test_image();
        Self {
            panel: FloatingPanel::new(Size::new(300.0, 250.0)),
            image,
            offset: Vector::ZERO,
            scale: 1.0,
            drag_origin: None,
            starting_offset: Vector::ZERO,
            cache: canvas::Cache::new(),
        }
    }

    fn load_test_image() -> Option<iced::widget::image::Handle> {
        let path = std::path::Path::new("assets/test/test.png");
        if path.exists() {
            Some(iced::widget::image::Handle::from_path(path))
        } else {
            None
        }
    }
}
```

- [ ] **Step 3: Define Message enum**

```rust
#[derive(Debug, Clone)]
pub enum Message {
    Panel(panel::Event),
    CanvasEvent(CanvasMessage),
}

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    DragStart(Point),
    DragMove(Point),
    DragEnd,
    Zoom(f32, Point),
}
```

- [ ] **Step 4: Implement `update()`**

```rust
impl PreviewPanel {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Panel(event) => self.panel.handle_event(event),
            Message::CanvasEvent(msg) => match msg {
                CanvasMessage::DragStart(pos) => {
                    self.drag_origin = Some(pos);
                    self.starting_offset = self.offset;
                }
                CanvasMessage::DragMove(pos) => {
                    if let Some(origin) = self.drag_origin {
                        let delta = pos - origin;
                        self.offset = self.starting_offset + delta;
                        self.cache.clear();
                    }
                }
                CanvasMessage::DragEnd => {
                    self.drag_origin = None;
                }
                CanvasMessage::Zoom(delta, _cursor) => {
                    let old_scale = self.scale;
                    self.scale = if delta > 0.0 {
                        self.scale * (1.0 + ZOOM_STEP)
                    } else {
                        self.scale / (1.0 + ZOOM_STEP)
                    }
                    .clamp(MIN_SCALE, MAX_SCALE);
                    if (self.scale - old_scale).abs() > f32::EPSILON {
                        self.cache.clear();
                    }
                }
            },
        }
    }
}
```

- [ ] **Step 5: Implement canvas Program for image drawing**

```rust
struct PreviewCanvas {
    image: Option<iced::widget::image::Handle>,
    offset: Vector,
    scale: f32,
}

impl canvas::Program<CanvasMessage> for PreviewCanvas {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<CanvasMessage>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_over(bounds)?;
                Some(canvas::Action::publish(CanvasMessage::DragStart(pos)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(canvas::Action::publish(CanvasMessage::DragEnd))
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_over(bounds) {
                    Some(canvas::Action::publish(CanvasMessage::DragMove(pos)).and_capture())
                } else {
                    None
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let cursor_pos = cursor.position_over(bounds)?;
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 20.0,
                };
                Some(canvas::Action::publish(CanvasMessage::Zoom(dy, cursor_pos)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let content = canvas::Cache::new(); // 注意：实际使用外部 cache
        let geom = content.draw(renderer, bounds.size(), |frame| {
            // 背景
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgb8(0xf4, 0xf4, 0xf5),
            );

            // 绘制图片
            if let Some(handle) = &self.image {
                frame.draw_image(
                    iced::advanced::image::Image::new(handle.clone())
                        .filter_method(iced::advanced::image::FilterMethod::Nearest),
                    Rectangle::new(
                        Point::new(self.offset.x, self.offset.y),
                        Size::new(bounds.width * self.scale, bounds.height * self.scale),
                    ),
                );
            }
        });
        vec![geom]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
```

注意：`draw()` 中不能直接用 `self.cache`（借用问题），需要用 `PreviewPanel` 持有的 cache。这里有两种解法：
- 方案 A：让 `PreviewCanvas` 不实现 `Program`，改为 `PreviewPanel` 直接实现 `Program`
- 方案 B：用独立的 `Cache` 实例

实际实现时用方案 A 更简洁：让 `PreviewPanel` 自身实现 `canvas::Program`。

- [ ] **Step 6: Implement `view()`**

```rust
impl PreviewPanel {
    pub fn view(&self) -> Element<'_, Message> {
        // 标题栏
        let title_bar = mouse_area(
            container(
                text("Preview").size(12).color(Color::from_rgb8(0x71, 0x71, 0x7a)),
            )
            .width(Fill)
            .height(TITLE_BAR_HEIGHT)
            .padding(iced::Padding::from([6, 10]))
            .style(Self::title_bar_style),
        )
        .on_press(Message::Panel(panel::Event::DragStart));

        // Canvas 内容区
        let preview_canvas = canvas::Canvas::new(self)
            .width(Fill)
            .height(Fill);

        // resize handle（右下角）
        let resize_handle = mouse_area(
            container(text("⟋").size(10).color(Color::from_rgb8(0xa1, 0xa1, 0xaa)))
                .width(RESIZE_HANDLE_SIZE)
                .height(RESIZE_HANDLE_SIZE)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom),
        )
        .on_press(Message::Panel(panel::Event::ResizeStart));

        // 组合
        let panel_content = column![title_bar, preview_canvas]
            .width(self.panel.size.width)
            .height(self.panel.size.height);

        // 外层容器 + resize handle overlay
        container(
            iced::widget::stack![
                panel_content,
                container(resize_handle)
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Bottom),
            ]
        )
        .style(Self::panel_style)
        .into()
    }
}
```

- [ ] **Step 7: Add style functions**

```rust
impl PreviewPanel {
    fn panel_style(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                color: Color::from_rgb8(0xe4, 0xe4, 0xe7),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        }
    }

    fn title_bar_style(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb8(0xfa, 0xfa, 0xfa).into()),
            border: Border {
                color: Color::from_rgb8(0xe4, 0xe4, 0xe7),
                width: 0.0,
                radius: [8.0, 8.0, 0.0, 0.0].into(),
            },
            ..Default::default()
        }
    }
}
```

- [ ] **Step 8: Verify it compiles (may need iteration on imports and canvas Program)**

Run: `./scripts/windows_run.sh`

- [ ] **Step 9: Commit**

```bash
git add gui/src/preview.rs
git commit -m "feat: implement PreviewPanel with image zoom/pan"
```

---

### Task 4: Integrate PreviewPanel into App

**Files:**
- Modify: `gui/src/lib.rs`

- [ ] **Step 1: Add Preview to App struct and Message**

```rust
use preview::PreviewPanel;

pub struct App {
    canvas: NodeCanvas,
    toolbar: Toolbar,
    preview: PreviewPanel,
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(canvas::Message),
    Toolbar(toolbar::Message),
    Preview(preview::Message),
}
```

- [ ] **Step 2: Update `App::new()`**

```rust
pub fn new() -> (Self, Task<Message>) {
    (
        App {
            canvas: NodeCanvas,
            toolbar: Toolbar::new(),
            preview: PreviewPanel::new(),
        },
        Task::none(),
    )
}
```

- [ ] **Step 3: Update `App::update()`**

```rust
pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Canvas(_) => {}
        Message::Toolbar(msg) => self.toolbar.update(msg),
        Message::Preview(msg) => self.preview.update(msg),
    }
    Task::none()
}
```

- [ ] **Step 4: Update `App::view()` — add preview panel + unified interaction overlay**

```rust
pub fn view(&self) -> Element<'_, Message> {
    let canvas = iced_canvas::Canvas::new(&self.canvas)
        .width(Fill)
        .height(Fill);

    // 工具栏
    let tb_offset = self.toolbar.panel.offset;
    let toolbar = float(
        center(self.toolbar.view().map(Message::Toolbar))
            .width(Fill)
            .height(Length::Shrink)
            .padding(iced::Padding { top: 16.0, right: 0.0, bottom: 0.0, left: 0.0 }),
    )
    .translate(move |_bounds, _viewport| tb_offset);

    // 预览面板：右上角定位
    let pv_offset = self.preview.panel.offset;
    let preview = float(
        container(self.preview.view().map(Message::Preview))
            .width(Fill)
            .height(Length::Shrink)
            .align_x(iced::alignment::Horizontal::Right)
            .padding(iced::Padding { top: 60.0, right: 16.0, bottom: 0.0, left: 0.0 }),
    )
    .translate(move |_bounds, _viewport| pv_offset);

    // 统一交互覆盖层（拖拽 / resize）
    let any_interacting = self.toolbar.panel.is_dragging()
        || self.preview.panel.is_interacting();

    if any_interacting {
        let drag_layer = mouse_area(
            container("").width(Fill).height(Fill),
        )
        .on_move(|p| {
            // 路由到正在交互的面板
            // 注意：同一时间只有一个面板在交互
            if self.toolbar.panel.is_dragging() {
                Message::Toolbar(toolbar::Message::Panel(panel::Event::DragMove(p)))
            } else if self.preview.panel.is_dragging() {
                Message::Preview(preview::Message::Panel(panel::Event::DragMove(p)))
            } else if self.preview.panel.is_resizing() {
                Message::Preview(preview::Message::Panel(panel::Event::ResizeMove(p)))
            } else {
                // fallback — 不应该到达这里
                Message::Toolbar(toolbar::Message::Panel(panel::Event::DragEnd))
            }
        })
        .on_release({
            if self.toolbar.panel.is_dragging() {
                Message::Toolbar(toolbar::Message::Panel(panel::Event::DragEnd))
            } else if self.preview.panel.is_dragging() {
                Message::Preview(preview::Message::Panel(panel::Event::DragEnd))
            } else {
                Message::Preview(preview::Message::Panel(panel::Event::ResizeEnd))
            }
        });

        stack![canvas, preview, toolbar, drag_layer].into()
    } else {
        stack![canvas, preview, toolbar].into()
    }
}
```

注意：`on_move` 和 `on_release` 闭包中引用 `self` 会有借用问题（`view()` 已经借了 `&self`）。解决方法：在闭包前用局部变量缓存状态。

```rust
let tb_dragging = self.toolbar.panel.is_dragging();
let pv_dragging = self.preview.panel.is_dragging();
let pv_resizing = self.preview.panel.is_resizing();
```

然后闭包中用这些 bool。

- [ ] **Step 5: Verify it compiles and runs**

Run: `./scripts/windows_run.sh`
Expected: 右上角出现预览面板，显示测试图片，可拖拽/resize/缩放

- [ ] **Step 6: Commit**

```bash
git add gui/src/lib.rs
git commit -m "feat: integrate preview panel into app layout"
```

---

### Task 5: Manual testing and polish

- [ ] **Step 1: Test panel drag** — 拖拽标题栏移动面板
- [ ] **Step 2: Test panel resize** — 拖拽右下角调整大小
- [ ] **Step 3: Test image zoom** — 在面板内滚轮缩放图片
- [ ] **Step 4: Test image pan** — 在面板内左键拖拽平移图片
- [ ] **Step 5: Test toolbar still works** — 工具栏拖拽不受影响
- [ ] **Step 6: Fix any issues found during testing**
- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "feat: preview panel complete"
```
