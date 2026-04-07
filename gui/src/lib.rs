mod canvas;
mod controller;
mod mock;
mod panel;
mod preview;
mod state;
mod statusbar;
mod theme;
mod toolbar;

use iced::widget::{canvas as iced_canvas, center, container, float, mouse_area, stack};
use iced::{Element, Fill, Length, Task, Theme};

use canvas::NodeCanvas;
use preview::PreviewPanel;
use toolbar::Toolbar;

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

impl App {
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(_) => {}
            Message::Toolbar(msg) => self.toolbar.update(msg),
            Message::Preview(msg) => self.preview.update(msg),
        }
        Task::none()
    }

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
                .padding(iced::Padding {
                    top: 16.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
        )
        .translate(move |_bounds, _viewport| tb_offset);

        // 预览面板：右上角（仅可见时显示）
        let pv_visible = self.preview.panel.visible;
        let pv_offset = self.preview.panel.offset;

        // 统一交互覆盖层
        let tb_dragging = self.toolbar.panel.is_dragging();
        let pv_dragging = pv_visible && self.preview.panel.is_dragging();
        let pv_resizing = pv_visible && self.preview.panel.is_resizing();
        let any_interacting = tb_dragging || pv_dragging || pv_resizing;

        let mut layers: Vec<Element<'_, Message>> = vec![canvas.into(), toolbar.into()];

        if pv_visible {
            let preview = float(
                container(self.preview.view().map(Message::Preview))
                    .width(Fill)
                    .height(Length::Shrink)
                    .align_x(iced::alignment::Horizontal::Right)
                    .padding(iced::Padding {
                        top: 60.0,
                        right: 16.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
            )
            .translate(move |_bounds, _viewport| pv_offset);
            layers.push(preview.into());
        }

        if any_interacting {
            let drag_layer = mouse_area(container("").width(Fill).height(Fill))
                .on_move(move |p| {
                    if tb_dragging {
                        Message::Toolbar(toolbar::Message::Panel(panel::Event::DragMove(p)))
                    } else if pv_dragging {
                        Message::Preview(preview::Message::Panel(panel::Event::DragMove(p)))
                    } else {
                        Message::Preview(preview::Message::Panel(panel::Event::ResizeMove(p)))
                    }
                })
                .on_release(if tb_dragging {
                    Message::Toolbar(toolbar::Message::Panel(panel::Event::DragEnd))
                } else if pv_dragging {
                    Message::Preview(preview::Message::Panel(panel::Event::DragEnd))
                } else {
                    Message::Preview(preview::Message::Panel(panel::Event::ResizeEnd))
                });
            layers.push(drag_layer.into());
        }

        stack(layers).into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }
}
