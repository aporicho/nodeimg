#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasPortSide {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct CanvasPortView {
    pub name: String,
    pub stable_id: String,
    pub side: CanvasPortSide,
    pub index: usize,
    pub count: usize,
}

impl CanvasPortSide {
    pub fn as_str(self) -> &'static str {
        match self {
            CanvasPortSide::Input => "input",
            CanvasPortSide::Output => "output",
        }
    }
}

pub fn canvas_port_stable_id(owner_id: &str, side: CanvasPortSide, name: &str) -> String {
    format!("canvas_node::{owner_id}::port::{}::{name}", side.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_stable_id_includes_owner_side_and_name() {
        assert_eq!(
            canvas_port_stable_id("engine_node::1", CanvasPortSide::Output, "image"),
            "canvas_node::engine_node::1::port::output::image"
        );
    }
}
