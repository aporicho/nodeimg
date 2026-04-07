//! 工具栏 (2.5.0)
//! 悬浮 pill 形状，可拖动。

use crate::controls::atoms::{icon_button, separator, ButtonVariant};
use crate::panel::{self, FloatingPanel};
use iced::widget::{container, mouse_area, row};
use iced::{Border, Color, Element, Shadow, Size, Theme, Vector};

pub struct Toolbar {
    pub panel: FloatingPanel,
}

#[derive(Debug, Clone)]
pub enum Message {
    Panel(panel::Event),
    Action(ToolbarAction),
}

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    New,
    Open,
    Save,
    Run,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            panel: FloatingPanel::new(Size::ZERO),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Panel(event) => self.panel.handle_event(event),
            Message::Action(_action) => {
                // TODO: 转发给交互控制器
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = row![
            icon_button("New", "plus.svg", ButtonVariant::Default, Message::Action(ToolbarAction::New)),
            icon_button("Open", "folder.svg", ButtonVariant::Default, Message::Action(ToolbarAction::Open)),
            icon_button("Save", "floppy-disk.svg", ButtonVariant::Default, Message::Action(ToolbarAction::Save)),
            separator().map(|_: panel::Event| Message::Panel(panel::Event::DragEnd)),
            icon_button("Run", "play.svg", ButtonVariant::Primary, Message::Action(ToolbarAction::Run)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let pill = container(content)
            .padding(iced::Padding::from([8, 16]))
            .style(Self::pill_style);

        mouse_area(pill)
            .on_press(Message::Panel(panel::Event::DragStart))
            .into()
    }

    fn pill_style(_theme: &Theme) -> container::Style {
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
