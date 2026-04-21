use super::OverlayRequest;
use crate::tree::Tree;

#[derive(Debug, Clone, Copy)]
pub enum OverlayPlacement {
    AtPoint { x: f32, y: f32 },
    BelowStart,
}

pub(crate) fn resolve_placement(
    tree: &Tree,
    request: &OverlayRequest,
    fallback: (f32, f32, Option<f32>),
) -> (f32, f32, Option<f32>) {
    match request.placement {
        OverlayPlacement::AtPoint { x, y } => (x + request.offset_x, y + request.offset_y, None),
        OverlayPlacement::BelowStart => anchor_layout(tree, request)
            .map(|(rect_x, rect_y, rect_w, rect_h)| {
                (
                    rect_x + request.offset_x,
                    rect_y + rect_h + request.offset_y,
                    request.match_anchor_width.then_some(rect_w),
                )
            })
            .unwrap_or(fallback),
    }
}

fn anchor_layout(tree: &Tree, request: &OverlayRequest) -> Option<(f32, f32, f32, f32)> {
    let node_id = tree.node_by_str(&request.anchor_id)?;
    let node = tree.get(node_id)?;
    Some((node.rect.x, node.rect.y, node.rect.w, node.rect.h))
}
