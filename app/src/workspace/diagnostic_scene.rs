use gui::canvas::node_template::{
    CanvasNodeInstanceState, CanvasNodeRenderView, CanvasNodeTemplate,
};
use gui::canvas::{CanvasNodeIdentity, CanvasNodeLayout};
use gui::renderer::Rect;

pub(crate) const DIAGNOSTIC_NODE_OWNER_ID: &str = "diagnostic_node::retained_clean_room";

pub(crate) fn diagnostic_node_identity() -> CanvasNodeIdentity {
    CanvasNodeIdentity {
        owner_id: DIAGNOSTIC_NODE_OWNER_ID.to_string(),
        default_rect: Rect {
            x: 80.0,
            y: 80.0,
            w: 304.0,
            h: 132.0,
        },
    }
}

pub(crate) fn diagnostic_render_view_for_layout(layout: CanvasNodeLayout) -> CanvasNodeRenderView {
    let template = CanvasNodeTemplate {
        type_id: "diagnostic::retained_clean_room".to_string(),
        title: "Retained Node".to_string(),
        subtitle: "clean room".to_string(),
        category: "diagnostic".to_string(),
        params: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
    };
    let state = CanvasNodeInstanceState {
        owner_id: layout.owner_id.clone(),
        layout,
        selected: false,
        input_group: Default::default(),
        output_group: Default::default(),
        port_states: Vec::new(),
    };
    CanvasNodeRenderView { template, state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_node_has_stable_clean_room_identity() {
        let identity = diagnostic_node_identity();
        assert_eq!(identity.owner_id, DIAGNOSTIC_NODE_OWNER_ID);

        let view = diagnostic_render_view_for_layout(CanvasNodeLayout {
            owner_id: identity.owner_id.clone(),
            rect: identity.default_rect,
            z_index: 0,
            collapsed: false,
            user_min_height: None,
        });

        assert_eq!(view.state.owner_id, DIAGNOSTIC_NODE_OWNER_ID);
        assert_eq!(view.template.title, "Retained Node");
        assert!(view.template.params.is_empty());
        assert!(view.state.port_states.is_empty());
    }
}
