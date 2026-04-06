//! 节点画布 (2.1.0)
//! 自建 Canvas widget：背景网格、节点渲染、连线渲染、叠加层。

pub mod connection;
pub mod controller;
pub mod node;

use controller::CanvasState;
use iced::mouse;
use iced::widget::canvas;
use iced::{Color, Point, Rectangle, Renderer, Theme};

#[derive(Debug, Clone)]
pub enum Message {}

/// 节点画布 Program。
pub struct NodeCanvas;

impl<Message> canvas::Program<Message> for NodeCanvas {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        state.handle_event(event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let grid = state.grid_cache.draw(renderer, bounds.size(), |frame| {
            // zinc-50 #fafafa
            let bg = Color::from_rgb8(0xfa, 0xfa, 0xfa);
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), bg);

            // zinc-300 #d4d4d8
            let dot_color = Color::from_rgb8(0xd4, 0xd4, 0xd8);
            let grid_spacing = 24.0 * state.scale;
            let dot_radius = (1.0 * state.scale).max(0.5);

            if grid_spacing < 4.0 {
                return;
            }

            let offset_x = state.offset.x.rem_euclid(grid_spacing);
            let offset_y = state.offset.y.rem_euclid(grid_spacing);

            let mut x = offset_x;
            while x < bounds.width {
                let mut y = offset_y;
                while y < bounds.height {
                    let dot = canvas::Path::circle(Point::new(x, y), dot_radius);
                    frame.fill(&dot, dot_color);
                    y += grid_spacing;
                }
                x += grid_spacing;
            }
        });

        vec![grid]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !cursor.is_over(bounds) {
            return mouse::Interaction::default();
        }

        if state.is_panning() {
            mouse::Interaction::Grabbing
        } else if state.space_held() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
