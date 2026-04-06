//! 工具栏 (2.5.0)
//! 悬浮 pill 形状，可拖动。

use crate::panel::{self, FloatingPanel};
use iced::widget::{button, container, mouse_area, row, text};
use iced::{Border, Color, Element, Point, Shadow, Theme, Vector};

pub struct Toolbar {
    pub panel: FloatingPanel,
}

#[derive(Debug, Clone)]
pub enum Message {
    Panel(panel::Event),
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            panel: FloatingPanel::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Panel(event) => self.panel.handle_event(event),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = row![
            text("nodeimg").size(13).color(Color::from_rgb8(0xa1, 0xa1, 0xaa)),
            Self::separator(),
            button(text("New").size(12)).on_press(Message::Panel(panel::Event::DragEnd)).style(Self::btn_style),
            button(text("Open").size(12)).on_press(Message::Panel(panel::Event::DragEnd)).style(Self::btn_style),
            button(text("Save").size(12)).on_press(Message::Panel(panel::Event::DragEnd)).style(Self::btn_style),
            Self::separator(),
            button(text("▶ Run").size(12)).on_press(Message::Panel(panel::Event::DragEnd)).style(Self::run_btn_style),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let pill = container(content)
            .padding(iced::Padding::from([6, 16]))
            .style(Self::pill_style);

        mouse_area(pill)
            .on_press(Message::Panel(panel::Event::DragStart))
            .on_release(Message::Panel(panel::Event::DragEnd))
            .on_move(|point| Message::Panel(panel::Event::DragMove(point)))
            .into()
    }

    fn separator<'a>() -> Element<'a, Message> {
        container(text(""))
            .width(1)
            .height(16)
            .style(|_: &Theme| container::Style {
                background: Some(Color::from_rgb8(0xe4, 0xe4, 0xe7).into()),
                ..Default::default()
            })
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

    fn btn_style(_theme: &Theme, _status: button::Status) -> button::Style {
        button::Style {
            background: None,
            text_color: Color::from_rgb8(0x71, 0x71, 0x7a),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn run_btn_style(_theme: &Theme, _status: button::Status) -> button::Style {
        button::Style {
            background: Some(Color::from_rgb8(0x18, 0x18, 0x1b).into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
