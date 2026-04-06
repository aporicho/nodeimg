pub mod canvas;
pub mod render;

use iced::{Element, Task, Theme};

pub struct App;

#[derive(Debug, Clone)]
pub enum Message {}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (App, Task::none())
    }

    pub fn title(&self) -> String {
        "nodeimg".to_string()
    }

    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("nodeimg").into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }
}
