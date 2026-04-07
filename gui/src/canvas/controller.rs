//! 画布控制器 (2.1.1)
//! 维护 CanvasState，处理画布内所有交互事件。

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;
use iced::{Point, Rectangle, Vector};

const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 5.0;
const ZOOM_STEP: f32 = 0.03;

/// 画布状态：视口变换 + 交互状态。
#[derive(Debug)]
pub struct CanvasState {
    /// 视口偏移
    pub offset: Vector,
    /// 视口缩放
    pub scale: f32,
    /// 拖拽中：记录鼠标按下时的光标位置
    grab_origin: Option<Point>,
    /// 拖拽开始时的 offset 快照
    starting_offset: Vector,
    /// Space 键是否按下（Space+左键拖拽平移）
    space_held: bool,
    /// 当前修饰键状态
    modifiers: keyboard::Modifiers,
    /// 网格缓存
    pub grid_cache: canvas::Cache,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            offset: Vector::ZERO,
            scale: 1.0,
            grab_origin: None,
            starting_offset: Vector::ZERO,
            space_held: false,
            modifiers: keyboard::Modifiers::empty(),
            grid_cache: canvas::Cache::new(),
        }
    }
}

impl CanvasState {
    /// 处理画布事件。
    pub fn handle_event<Message>(
        &mut self,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            // ── 键盘：跟踪 Space 和修饰键 ──
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) => {
                self.space_held = true;
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) => {
                self.space_held = false;
                if self.grab_origin.is_some() {
                    self.grab_origin = None;
                }
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                self.modifiers = *modifiers;
                None
            }

            // +/= 键 → 放大
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "+" || ch.as_str() == "=" => {
                let center = Point::new(
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                self.zoom_at(center, bounds, 1.0);
                Some(canvas::Action::request_redraw().and_capture())
            }
            // - 键 → 缩小
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "-" => {
                let center = Point::new(
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                self.zoom_at(center, bounds, -1.0);
                Some(canvas::Action::request_redraw().and_capture())
            }
            // Cmd+0 → 重置视口
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "0" && self.modifiers.command() => {
                self.offset = Vector::ZERO;
                self.scale = 1.0;
                self.grid_cache.clear();
                Some(canvas::Action::request_redraw().and_capture())
            }

            // ── 鼠标按下 ──
            canvas::Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                let pos = cursor.position_over(bounds)?;

                let should_pan = match button {
                    mouse::Button::Middle => true,
                    mouse::Button::Left if self.space_held => true,
                    _ => false,
                };

                if should_pan {
                    self.grab_origin = Some(pos);
                    self.starting_offset = self.offset;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // ── 鼠标释放 ──
            canvas::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                if self.grab_origin.is_some()
                    && matches!(button, mouse::Button::Middle | mouse::Button::Left)
                {
                    self.grab_origin = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // ── 光标移动 → 拖拽中更新偏移 ──
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(origin) = self.grab_origin {
                    let pos = cursor.position_over(bounds)?;
                    let delta = pos - origin;
                    self.offset = self.starting_offset + delta;
                    self.grid_cache.clear();
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                None
            }

            // ── 滚轮 ──
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let cursor_position = cursor.position_over(bounds)?;

                match delta {
                    // x = INFINITY → 触控板捏合（patch 标记），直接缩放
                    mouse::ScrollDelta::Lines { x, y } if x.is_infinite() => {
                        self.zoom_at(cursor_position, bounds, *y * 20.0);
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                    // 滚轮 → 缩放
                    _ => {
                        let dy = match delta {
                            mouse::ScrollDelta::Lines { y, .. } => *y * 20.0,
                            mouse::ScrollDelta::Pixels { y, .. } => *y,
                        };
                        self.zoom_at(cursor_position, bounds, dy);
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                }
            }

            _ => None,
        }
    }

    /// 以指定位置为中心缩放。
    fn zoom_at(&mut self, cursor_position: Point, bounds: Rectangle, scroll_y: f32) {
        if (scroll_y < 0.0 && self.scale <= MIN_SCALE)
            || (scroll_y > 0.0 && self.scale >= MAX_SCALE)
        {
            return;
        }

        let old_scale = self.scale;
        self.scale = if scroll_y > 0.0 {
            self.scale * (1.0 + ZOOM_STEP)
        } else {
            self.scale / (1.0 + ZOOM_STEP)
        }
        .clamp(MIN_SCALE, MAX_SCALE);

        let cursor_in_bounds =
            Vector::new(cursor_position.x - bounds.x, cursor_position.y - bounds.y);
        let factor = self.scale / old_scale;
        self.offset = Vector::new(
            cursor_in_bounds.x - (cursor_in_bounds.x - self.offset.x) * factor,
            cursor_in_bounds.y - (cursor_in_bounds.y - self.offset.y) * factor,
        );

        self.grid_cache.clear();
    }

    pub fn is_panning(&self) -> bool {
        self.grab_origin.is_some()
    }

    pub fn space_held(&self) -> bool {
        self.space_held
    }
}
