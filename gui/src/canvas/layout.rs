use crate::renderer::Rect;

#[derive(Clone, Debug)]
pub struct CanvasNodeIdentity {
    pub owner_id: String,
    pub default_rect: Rect,
}

#[derive(Clone, Debug)]
pub struct CanvasNodeLayout {
    pub owner_id: String,
    pub rect: Rect,
    pub z_index: i32,
    pub collapsed: bool,
    pub user_min_height: Option<f32>,
}

pub fn canvas_node_stable_id(owner_id: &str) -> String {
    format!("canvas_node::{owner_id}")
}

pub fn canvas_node_owner_id(stable_id: &str) -> Option<&str> {
    stable_id.strip_prefix("canvas_node::")
}

pub fn canvas_node_event_owner_id(stable_id: &str) -> Option<&str> {
    let raw = stable_id.strip_prefix("canvas_node::")?;
    let marker_index = [
        "::card",
        "::body",
        "::label",
        "::label_dot",
        "::label_text",
        "::pin_column::",
        "::port::",
        "::port_group::",
    ]
    .iter()
    .filter_map(|marker| raw.find(marker))
    .min();

    Some(match marker_index {
        Some(index) => &raw[..index],
        None => raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_node_event_owner_id_accepts_nested_node_parts() {
        assert_eq!(
            canvas_node_event_owner_id("canvas_node::showcase_node::text_area_control::card"),
            Some("showcase_node::text_area_control")
        );
        assert_eq!(
            canvas_node_event_owner_id(
                "canvas_node::engine_node::7::body::param::0::control::content"
            ),
            Some("engine_node::7")
        );
        assert_eq!(
            canvas_node_event_owner_id("canvas_node::engine_node::7"),
            Some("engine_node::7")
        );
        assert_eq!(canvas_node_event_owner_id("slider"), None);
    }
}
