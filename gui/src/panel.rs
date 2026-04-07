//! 通用浮动面板组件。
//! 所有面板浮在画布上方，可拖拽移动、可 resize。
//! 提供统一的面板框架：标题栏 + 内容区 + resize handle。

use iced::widget::{column, container, mouse_area, text};
use iced::{Border, Color, Element, Fill, Point, Shadow, Size, Theme, Vector};

const TITLE_BAR_HEIGHT: f32 = 28.0;
const RESIZE_HANDLE_SIZE: f32 = 16.0;

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

#[derive(Debug, Clone)]
pub enum Event {
    DragStart,
    DragMove(Point),
    DragEnd,
    ResizeStart,
    ResizeMove(Point),
    ResizeEnd,
}

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

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::DragStart => {
                self.dragging = true;
                self.last_cursor = None;
            }
            Event::DragMove(cursor) => {
                if !self.dragging {
                    return;
                }
                if let Some(last) = self.last_cursor {
                    self.offset = self.offset + Vector::new(cursor.x - last.x, cursor.y - last.y);
                }
                self.last_cursor = Some(cursor);
            }
            Event::DragEnd => {
                self.dragging = false;
                self.last_cursor = None;
            }
            Event::ResizeStart => {
                self.resizing = true;
                self.last_cursor = None;
            }
            Event::ResizeMove(cursor) => {
                if !self.resizing {
                    return;
                }
                if let Some(last) = self.last_cursor {
                    self.size = Size::new(
                        (self.size.width + cursor.x - last.x).max(120.0),
                        (self.size.height + cursor.y - last.y).max(80.0),
                    );
                }
                self.last_cursor = Some(cursor);
            }
            Event::ResizeEnd => {
                self.resizing = false;
                self.last_cursor = None;
            }
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn is_resizing(&self) -> bool {
        self.resizing
    }

    pub fn is_interacting(&self) -> bool {
        self.dragging || self.resizing
    }

    /// 通用面板框架：标题栏 + 内容区 + resize handle。
    /// `content` 是面板内部的内容，`map` 将 `Event` 映射到外部消息类型。
    pub fn view<'a, M: 'a + Clone>(
        &self,
        title: &str,
        content: Element<'a, M>,
        map: impl Fn(Event) -> M + 'a,
    ) -> Element<'a, M> {
        // 标题栏
        let title_bar = mouse_area(
            container(text(title.to_owned()).size(12).color(Color::from_rgb8(0x71, 0x71, 0x7a)))
                .width(Fill)
                .height(TITLE_BAR_HEIGHT)
                .padding(iced::Padding::from([6, 10]))
                .style(Self::title_bar_style),
        )
        .on_press((map)(Event::DragStart));

        // resize handle
        let resize_handle = mouse_area(
            container(text("\u{25E2}").size(10).color(Color::from_rgb8(0xa1, 0xa1, 0xaa)))
                .width(RESIZE_HANDLE_SIZE)
                .height(RESIZE_HANDLE_SIZE)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom),
        )
        .on_press((map)(Event::ResizeStart));

        let body = column![title_bar, content]
            .width(self.size.width)
            .height(self.size.height);

        container(iced::widget::stack![
            body,
            container(resize_handle)
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom),
        ])
        .style(Self::panel_style)
        .into()
    }

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
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}
