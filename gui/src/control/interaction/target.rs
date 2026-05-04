use super::spec::ControlInteractionSpec;
use crate::control::SystemCx;
use crate::tree::{HitChain, NodeId};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ControlInteractionTarget {
    pub(super) node_id: NodeId,
    pub(super) id: String,
    pub(super) spec: ControlInteractionSpec,
}

pub(super) fn target_at(cx: &SystemCx<'_>, x: f32, y: f32) -> Option<ControlInteractionTarget> {
    let chain = cx.hit_chain(x, y);
    target_in_chain(cx, &chain)
}

pub(super) fn target_by_id(cx: &SystemCx<'_>, id: &str) -> Option<ControlInteractionTarget> {
    let node_id = cx.node_id_by_name(id)?;
    target_for_node(cx, node_id)
}

fn target_in_chain(cx: &SystemCx<'_>, chain: &HitChain) -> Option<ControlInteractionTarget> {
    chain
        .iter()
        .find_map(|node_id| target_for_node(cx, node_id))
}

fn target_for_node(cx: &SystemCx<'_>, node_id: NodeId) -> Option<ControlInteractionTarget> {
    let node = cx.tree().get(node_id)?;
    if !node.props.enabled {
        return None;
    }
    let spec = *node.runtime_slots.get::<ControlInteractionSpec>()?;
    if !spec.is_interactive() {
        return None;
    }
    Some(ControlInteractionTarget {
        node_id,
        id: node.id.as_ref().to_string(),
        spec,
    })
}
