mod canvas;
mod controller;
mod mock;
mod panel;
mod preview;
mod state;
mod statusbar;
mod theme;
mod toolbar;

use iced::widget::{canvas as iced_canvas, center, float, mouse_area, stack};
use iced::{Element, Fill, Length, Task, Theme, Vector};

use canvas::NodeCanvas;
use toolbar::Toolbar;

pub struct App {
    canvas: NodeCanvas,
    toolbar: Toolbar,
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(canvas::Message),
    Toolbar(toolbar::Message),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            App {
                canvas: NodeCanvas,
                toolbar: Toolbar::new(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(_) => {}
            Message::Toolbar(msg) => self.toolbar.update(msg),
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let canvas = iced_canvas::Canvas::new(&self.canvas)
            .width(Fill)
            .height(Fill);

        // 工具栏：居中布局 + offset 偏移
        let offset = self.toolbar.panel.offset;
        let toolbar = float(
            center(self.toolbar.view().map(Message::Toolbar))
                .width(Fill)
                .height(Length::Shrink)
                .padding(iced::Padding { top: 16.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        )
        .translate(move |_bounds, _viewport| offset);

        if self.toolbar.panel.is_dragging() {
            let drag_layer = mouse_area(
                iced::widget::container("")
                    .width(Fill)
                    .height(Fill),
            )
            .on_move(|p| Message::Toolbar(toolbar::Message::Panel(panel::Event::DragMove(p))))
            .on_release(Message::Toolbar(toolbar::Message::Panel(panel::Event::DragEnd)));

            stack![canvas, toolbar, drag_layer].into()
        } else {
            stack![canvas, toolbar].into()
        }
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }
}
