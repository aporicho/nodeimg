use super::leaf::paint_leaf;
use super::text_override::TextLeafPaintOverride;
use super::traversal::PaintTraversal;
use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::Affine2D;
use crate::paint::{ClipShape, Color, LayerPaint, PaintCommand, RecordingPaintTarget, RectStyle};
use crate::theme::Theme;
use crate::tree::layout::Overflow;
use crate::tree::node::{NodeId, NodeKind};
use crate::tree::paint_space::PaintSpace;
use crate::tree::paint_target::PaintTarget;
use crate::tree::{RepaintBoundaryId, Tree};

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_to_target(
    tree: &Tree,
    root: NodeId,
    target: &mut dyn PaintTarget,
    text_override: Option<&dyn TextLeafPaintOverride>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    traversal: &mut PaintTraversal<'_>,
) {
    paint_node(
        tree,
        root,
        target,
        PaintSpace::root(),
        text_override,
        animations,
        theme,
        None,
        traversal,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_node(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    current_space: PaintSpace,
    text_override: Option<&dyn TextLeafPaintOverride>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
    traversal: &mut PaintTraversal<'_>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };
    tree.record_paint_node_visited();

    match traversal {
        PaintTraversal::Boundary {
            root,
            boundary_to_screen,
            child_boundaries,
            ..
        } => {
            if node_id != *root && node.paint_meta.boundary.is_some() {
                let node_space = current_space.node_space(node.rect, node.style.transform);
                let local_transform = boundary_to_screen
                    .inverse()
                    .map(|inverse| Affine2D::compose(inverse, node_space.local_to_screen))
                    .unwrap_or_else(|| Affine2D::translation(node.rect.x, node.rect.y));
                child_boundaries.push(crate::paint::FragmentChildRef {
                    boundary: RepaintBoundaryId(node_id),
                    local_transform,
                    clip_stack: Vec::new(),
                    z_index: node.style.z_index,
                });
                return;
            }
        }
        #[cfg(test)]
        PaintTraversal::Full => {}
    }

    if let Some(visual) = animations.and_then(|store| store.visual_for(node.id.as_ref())) {
        if visual.opacity <= 0.0 {
            return;
        }

        let layer = {
            let mut layer_target =
                RecordingPaintTarget::with_measure(|text, style| target.measure_text(text, style));
            paint_node_inner(
                tree,
                node_id,
                &mut layer_target,
                current_space,
                text_override,
                animations,
                theme,
                inherited_text_color,
                traversal,
            );
            match layer_target.display_list() {
                Ok(layer) => layer,
                Err(err) => {
                    tracing::warn!("failed to build animated node layer: {:?}", err);
                    return;
                }
            }
        };

        let pushed_transform = visual_affine(node.rect, visual);
        if let Some(transform) = pushed_transform {
            target.push_transform(transform);
        }
        target.draw(PaintCommand::Layer(LayerPaint::new(
            node.rect,
            visual.opacity,
            layer,
        )));
        if pushed_transform.is_some() {
            target.pop_transform();
        }
        return;
    }

    paint_node_inner(
        tree,
        node_id,
        target,
        current_space,
        text_override,
        animations,
        theme,
        inherited_text_color,
        traversal,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_node_inner(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    current_space: PaintSpace,
    text_override: Option<&dyn TextLeafPaintOverride>,
    animations: Option<&AnimationStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
    traversal: &mut PaintTraversal<'_>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };

    let node_space = current_space.node_space(node.rect, node.style.transform);
    let local_rect = node_space.local_rect;
    let transform = node.style.transform;
    let should_clip_children = matches!(node.style.overflow, Overflow::Hidden | Overflow::Scroll);
    let clip_radius = node
        .decoration
        .as_ref()
        .map(|decoration| decoration.radius)
        .unwrap_or([0.0; 4]);
    let children = tree.children_in_paint_order_cached(node_id, &node.children);
    let child_text_color = inherited_text_color;

    target.push_transform(Affine2D::translation(node.rect.x, node.rect.y));

    if let Some(decoration) = &node.decoration {
        target.draw_rect(
            local_rect,
            RectStyle {
                color: decoration.background.unwrap_or(Color::TRANSPARENT),
                border: decoration.border,
                radius: decoration.radius,
                shadow: decoration.shadow,
            },
        );
    }

    if should_clip_children {
        target.push_clip(ClipShape::RoundedRect {
            rect: local_rect,
            radius: clip_radius,
        });
    }

    if let NodeKind::Leaf(leaf) = &node.kind {
        paint_leaf(
            tree,
            node_id,
            leaf,
            target,
            node.rect,
            node_space,
            current_space,
            text_override,
            theme,
            child_text_color,
        );
    }

    if let Some(transform) = transform {
        target.push_transform(transform.to_affine(local_rect));
        let child_space = node_space.child_space();
        for child_id in children {
            paint_node(
                tree,
                child_id,
                target,
                child_space,
                text_override,
                animations,
                theme,
                child_text_color,
                traversal,
            );
        }
        target.pop_transform();
        if should_clip_children {
            target.pop_clip();
        }
        target.pop_transform();
    } else {
        target.pop_transform();
        for child_id in children {
            paint_node(
                tree,
                child_id,
                target,
                current_space,
                text_override,
                animations,
                theme,
                child_text_color,
                traversal,
            );
        }
        if should_clip_children {
            target.pop_clip();
        }
    }
}
