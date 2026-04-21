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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasPortGroupView {
    pub open: bool,
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

pub fn canvas_port_group_stable_id(owner_id: &str, side: CanvasPortSide) -> String {
    format!("canvas_node::{owner_id}::port_group::{}", side.as_str())
}

pub fn canvas_port_group_trigger_id(owner_id: &str, side: CanvasPortSide) -> String {
    format!("{}::trigger", canvas_port_group_stable_id(owner_id, side))
}

pub fn parse_canvas_port_group_trigger_id(id: &str) -> Option<(&str, CanvasPortSide)> {
    let group_id = id.strip_suffix("::trigger")?;
    parse_canvas_port_group_id(group_id)
}

pub fn parse_canvas_port_group_id(id: &str) -> Option<(&str, CanvasPortSide)> {
    let (owner_id, side) = id
        .strip_prefix("canvas_node::")?
        .rsplit_once("::port_group::")?;
    let side = match side {
        "input" => CanvasPortSide::Input,
        "output" => CanvasPortSide::Output,
        _ => return None,
    };
    Some((owner_id, side))
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

    #[test]
    fn port_group_trigger_id_roundtrips() {
        let id = canvas_port_group_trigger_id("engine_node::1", CanvasPortSide::Input);

        assert_eq!(
            parse_canvas_port_group_trigger_id(&id),
            Some(("engine_node::1", CanvasPortSide::Input))
        );
        assert_eq!(parse_canvas_port_group_trigger_id("slider"), None);
    }
}
