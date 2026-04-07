//! 通用浮动面板组件。
//! 所有面板浮在画布上方，可拖拽移动、可 resize。
//! 提供统一的面板框架：标题栏 + 内容区 + resize handle。

use crate::controls::composites::title_bar;
use iced::widget::{column, container, mouse_area, text};
use iced::{Border, Color, Element, Fill, Point, Shadow, Size, Theme, Vector};

const RESIZE_HANDLE_SIZE: f32 = 16.0;

#[derive(Debug)]
pub struct FloatingPanel {
    /// 相对于默认布局位置的偏移
    pub offset: Vector,
    /// 面板尺寸
    pub size: Size,
    /// 是否可见
    pub visible: bool,
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
    Close,
    Open,
}

impl FloatingPanel {
    pub fn new(size: Size) -> Self {
        Self {
            offset: Vector::ZERO,
            size,
            visible: true,
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
            Event::Close => {
                self.visible = false;
            }
            Event::Open => {
                self.visible = true;
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
        // 标题栏：使用 TitleBar 组合控件
        let tb = mouse_area(title_bar(title, (map)(Event::Close)))
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

        let body = column![tb, content]
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
        .clip(true)
        .style(Self::panel_style)
        .into()
    }

    fn panel_style(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                color: Color::from_rgb8(0xe4, 0xe4, 0xe7),
                width: 1.0,
                radius: 20.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..Default::default()
        }
    }

}
