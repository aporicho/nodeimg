use types::{NodeId, Vec2};

#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset: Vec2,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { offset: Vec2::new(0.0, 0.0), zoom: 1.0 }
    }
}

impl Viewport {
    pub fn to_screen(&self, canvas_pos: Vec2) -> Vec2 {
        Vec2::new(
            (canvas_pos.x + self.offset.x) * self.zoom,
            (canvas_pos.y + self.offset.y) * self.zoom,
        )
    }

    pub fn to_canvas(&self, screen_pos: Vec2) -> Vec2 {
        Vec2::new(
            screen_pos.x / self.zoom - self.offset.x,
            screen_pos.y / self.zoom - self.offset.y,
        )
    }

    pub fn zoom_at(&mut self, center: Vec2, factor: f32) {
        let canvas_center = self.to_canvas(center);
        self.zoom = (self.zoom * factor).clamp(0.1, 5.0);
        self.offset.x = center.x / self.zoom - canvas_center.x;
        self.offset.y = center.y / self.zoom - canvas_center.y;
    }
}

#[derive(Debug, Clone)]
pub enum ActiveInteraction {
    Dragging { node: NodeId, start_pos: Vec2 },
    Connecting { from_node: NodeId, from_pin: String, cursor: Vec2 },
    Panning { start_offset: Vec2, start_cursor: Vec2 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum HoverTarget {
    Node(NodeId),
    InputPin(NodeId, String),
    OutputPin(NodeId, String),
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct CanvasState {
    pub viewport: Viewport,
    pub interaction: Option<ActiveInteraction>,
    pub hover: Option<HoverTarget>,
}

