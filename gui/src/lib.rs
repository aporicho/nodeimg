mod canvas;
mod controller;
mod mock;
mod preview;
mod state;
mod statusbar;
mod theme;
mod toolbar;

use iced::widget::canvas as iced_canvas;
use iced::{Element, Fill, Task, Theme};

use canvas::NodeCanvas;

pub struct App {
    canvas: NodeCanvas,
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(canvas::Message),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (App { canvas: NodeCanvas }, Task::none())
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced_canvas::Canvas::new(&self.canvas)
            .width(Fill)
            .height(Fill)
            .into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }
}
