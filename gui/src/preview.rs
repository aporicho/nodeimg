//! 预览面板 (2.2.0)
//! 浮动面板，显示图片预览。支持缩放/平移、拖拽移动、resize。

use crate::panel::{self, FloatingPanel};
use iced::widget::canvas;
use iced::{mouse, Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme, Vector};

const ZOOM_STEP: f32 = 0.05;
const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 10.0;

pub struct PreviewPanel {
    pub panel: FloatingPanel,
    image: Option<iced::widget::image::Handle>,
    offset: Vector,
    scale: f32,
    drag_origin: Option<Point>,
    starting_offset: Vector,
    cache: canvas::Cache,
}

#[derive(Debug, Clone)]
pub enum Message {
    Panel(panel::Event),
    ImageDragStart(Point),
    ImageDragMove(Point),
    ImageDragEnd,
    ImageZoom(f32),
}

impl PreviewPanel {
    pub fn new() -> Self {
        let path = std::path::Path::new("assets/test/test.png");
        let image = if path.exists() {
            Some(iced::widget::image::Handle::from_path(path))
        } else {
            None
        };

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

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Panel(event) => self.panel.handle_event(event),
            Message::ImageDragStart(pos) => {
                self.drag_origin = Some(pos);
                self.starting_offset = self.offset;
            }
            Message::ImageDragMove(pos) => {
                if let Some(origin) = self.drag_origin {
                    self.offset = self.starting_offset + (pos - origin);
                    self.cache.clear();
                }
            }
            Message::ImageDragEnd => {
                self.drag_origin = None;
            }
            Message::ImageZoom(delta) => {
                let old = self.scale;
                self.scale = if delta > 0.0 {
                    self.scale * (1.0 + ZOOM_STEP)
                } else {
                    self.scale / (1.0 + ZOOM_STEP)
                }
                .clamp(MIN_SCALE, MAX_SCALE);
                if (self.scale - old).abs() > f32::EPSILON {
                    self.cache.clear();
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = canvas::Canvas::new(self).width(Fill).height(Fill).into();
        self.panel.view("Preview", content, Message::Panel)
    }
}

impl canvas::Program<Message> for PreviewPanel {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_over(bounds)?;
                Some(canvas::Action::publish(Message::ImageDragStart(pos)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let pos = cursor.position_over(bounds)?;
                Some(canvas::Action::publish(Message::ImageDragMove(pos)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(canvas::Action::publish(Message::ImageDragEnd))
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let _ = cursor.position_over(bounds)?;
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 20.0,
                };
                Some(canvas::Action::publish(Message::ImageZoom(dy)).and_capture())
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
        let geom = self.cache.draw(renderer, bounds.size(), |frame| {
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                Color::from_rgb8(0xf4, 0xf4, 0xf5),
            );

            if let Some(handle) = &self.image {
                let img_w = bounds.width * self.scale;
                let img_h = bounds.height * self.scale;
                frame.draw_image(
                    Rectangle::new(
                        Point::new(self.offset.x, self.offset.y),
                        Size::new(img_w, img_h),
                    ),
                    handle,
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
            if self.drag_origin.is_some() {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            }
        } else {
            mouse::Interaction::default()
        }
    }
}
