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
    pub connection_state: CanvasPortConnectionState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasPortGroupView {
    pub open: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasPortConnectionState {
    #[default]
    Idle,
    Source,
    CompatibleTarget,
    IncompatibleTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasPortRef {
    pub owner_id: String,
    pub side: CanvasPortSide,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasPendingConnectionView {
    pub from_port_id: String,
    pub cursor_canvas: [f32; 2],
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

pub fn parse_canvas_port_id(id: &str) -> Option<CanvasPortRef> {
    let (owner_id, port) = id.strip_prefix("canvas_node::")?.split_once("::port::")?;
    let (side, name) = port.split_once("::")?;
    let side = match side {
        "input" => CanvasPortSide::Input,
        "output" => CanvasPortSide::Output,
        _ => return None,
    };
    Some(CanvasPortRef {
        owner_id: owner_id.to_string(),
        side,
        name: name.to_string(),
    })
}

pub fn canvas_port_event_target_id(id: &str) -> Option<&str> {
    for suffix in ["::pin_item", "::pin_label", "::pin_dot"] {
        if let Some(port_id) = id.strip_suffix(suffix) {
            return parse_canvas_port_id(port_id).is_some().then_some(port_id);
        }
    }
    parse_canvas_port_id(id).is_some().then_some(id)
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
    fn port_stable_id_roundtrips() {
        assert_eq!(
            parse_canvas_port_id("canvas_node::engine_node::1::port::output::image"),
            Some(CanvasPortRef {
                owner_id: "engine_node::1".to_string(),
                side: CanvasPortSide::Output,
                name: "image".to_string(),
            })
        );
        assert_eq!(parse_canvas_port_id("canvas_node::engine_node::1"), None);
    }

    #[test]
    fn port_event_target_accepts_port_parts() {
        assert_eq!(
            canvas_port_event_target_id(
                "canvas_node::engine_node::1::port::output::image::pin_item"
            ),
            Some("canvas_node::engine_node::1::port::output::image")
        );
        assert_eq!(
            canvas_port_event_target_id("canvas_node::engine_node::1"),
            None
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
