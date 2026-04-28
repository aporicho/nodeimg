use super::layout::Overflow;
use super::node::{NodeId, TreeNode};
use super::paint_space::PaintSpace;
use super::shape::ContainerShape;
use super::stacking::children_in_hit_order;
use super::tree::Tree;
use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::{Point, Rect};
use crate::gesture::Gesture;
use crate::widget::resize_edge::ResizeEdge;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct InteractionHit {
    pub(crate) node_id: NodeId,
    pub(crate) kind: InteractionHitKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum InteractionHitKind {
    Resize { edge: ResizeEdge },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResizeHit {
    pub(crate) node_id: NodeId,
    pub(crate) edge: ResizeEdge,
}

impl InteractionHit {
    pub(crate) fn resize(node_id: NodeId, edge: ResizeEdge) -> Self {
        Self {
            node_id,
            kind: InteractionHitKind::Resize { edge },
        }
    }

    pub(crate) fn as_resize(self) -> ResizeHit {
        match self.kind {
            InteractionHitKind::Resize { edge } => ResizeHit {
                node_id: self.node_id,
                edge,
            },
        }
    }
}

pub(crate) fn interaction_hit_at_screen_point(
    tree: &Tree,
    root: NodeId,
    x: f32,
    y: f32,
    animations: Option<&AnimationStore>,
) -> Option<InteractionHit> {
    interaction_hit_recursive(tree, root, Point { x, y }, PaintSpace::root(), animations)
}

pub(crate) fn resize_hit_at_screen_point(
    tree: &Tree,
    root: NodeId,
    x: f32,
    y: f32,
    animations: Option<&AnimationStore>,
) -> Option<ResizeHit> {
    interaction_hit_at_screen_point(tree, root, x, y, animations).map(InteractionHit::as_resize)
}

fn interaction_hit_recursive(
    tree: &Tree,
    node_id: NodeId,
    screen: Point,
    current_space: PaintSpace,
    animations: Option<&AnimationStore>,
) -> Option<InteractionHit> {
    let node = tree.get(node_id)?;

    let visual = animations.and_then(|store| store.visual_for(node.id.as_ref()));
    let current_space = visual
        .and_then(|visual| visual_affine(node.rect, visual))
        .map(|transform| current_space.transformed(transform))
        .unwrap_or(current_space);
    let node_space = current_space.node_space(node.rect, node.style.transform);
    let local_point = current_space.point_to_local(node.rect, screen)?;
    let inside_bounds = node_space.contains_local_bounds(local_point);

    let child_interactions_allowed =
        child_interactions_may_overflow(node, local_point, inside_bounds);
    if child_interactions_allowed {
        let child_space = if node_space.children_are_local {
            node_space.child_space()
        } else {
            current_space
        };

        for child_id in children_in_hit_order(tree, &node.children) {
            if let Some(hit) =
                interaction_hit_recursive(tree, child_id, screen, child_space, animations)
            {
                return Some(hit);
            }
        }
    } else {
        tracing::trace!(
            target: "gui::tree::interaction_hit",
            node_id = %node.id,
            screen_x = screen.x,
            screen_y = screen.y,
            local_x = local_point.x,
            local_y = local_point.y,
            overflow = ?node.style.overflow,
            inside_bounds,
            shape_contains = container_shape(node).contains(local_point),
            "skip child interaction traversal because parent clips this point"
        );
    }

    let resize_edge = resize_edge_for_node(node, local_point);
    if resize_enabled(node) {
        tracing::trace!(
            target: "gui::tree::interaction_hit",
            node_id = %node.id,
            screen_x = screen.x,
            screen_y = screen.y,
            local_x = local_point.x,
            local_y = local_point.y,
            rect_x = node.rect.x,
            rect_y = node.rect.y,
            rect_w = node.rect.w,
            rect_h = node.rect.h,
            radius = ?node.decoration.as_ref().map(|decoration| decoration.radius).unwrap_or([0.0; 4]),
            threshold = node.style.resize_edge_threshold,
            overflow = ?node.style.overflow,
            inside_bounds,
            child_interactions_allowed,
            edge = ?resize_edge,
            "inspect resizable container interaction hit"
        );
    }

    resize_edge.map(|edge| {
        tracing::debug!(
            target: "gui::tree::interaction_hit",
            node_id = %node.id,
            edge = ?edge,
            local_x = local_point.x,
            local_y = local_point.y,
            rect_x = node.rect.x,
            rect_y = node.rect.y,
            rect_w = node.rect.w,
            rect_h = node.rect.h,
            threshold = node.style.resize_edge_threshold,
            "container resize interaction hit"
        );
        InteractionHit::resize(node_id, edge)
    })
}

fn resize_edge_for_node(node: &TreeNode, local_point: Point) -> Option<ResizeEdge> {
    if !resize_enabled(node) {
        return None;
    }

    let shape = container_shape(node);
    shape.resize_edge(local_point, node.style.resize_edge_threshold)
}

fn child_interactions_may_overflow(
    node: &TreeNode,
    local_point: Point,
    inside_bounds: bool,
) -> bool {
    match node.style.overflow {
        Overflow::Visible => true,
        Overflow::Hidden | Overflow::Scroll => {
            inside_bounds && container_shape(node).contains(local_point)
        }
    }
}

fn container_shape(node: &TreeNode) -> ContainerShape {
    let radius = node
        .decoration
        .as_ref()
        .map(|decoration| decoration.radius)
        .unwrap_or([0.0; 4]);
    ContainerShape::rounded_rect(
        Rect {
            x: 0.0,
            y: 0.0,
            w: node.rect.w,
            h: node.rect.h,
        },
        radius,
    )
}

fn resize_enabled(node: &TreeNode) -> bool {
    node.style.hittable != Some(false)
        && (node.style.resizable || node.style.gestures.contains(&Gesture::Resize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::TransformSpec;
    use crate::renderer::{Color, Rect};
    use crate::tree::layout::{BoxStyle, Decoration};
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;

    fn container(id: &'static str, style: BoxStyle, rect: Rect) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style,
            decoration: None,
            kind: NodeKind::Container,
            rect,
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn resizable_style() -> BoxStyle {
        BoxStyle {
            resizable: true,
            ..Default::default()
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn rounded_decoration(radius: [f32; 4]) -> Decoration {
        Decoration {
            background: Some(Color::WHITE),
            border: None,
            radius,
            shadow: None,
        }
    }

    #[test]
    fn resize_hit_uses_transformed_parent_space() {
        let mut tree = Tree::new();
        let card_id = tree.insert(container(
            "card",
            resizable_style(),
            rect(20.0, 30.0, 100.0, 40.0),
        ));
        let mut canvas = container(
            "canvas",
            BoxStyle {
                overflow: Overflow::Visible,
                transform: Some(TransformSpec::translate_scale([100.0, -40.0], 2.0)),
                ..Default::default()
            },
            rect(0.0, 0.0, 300.0, 200.0),
        );
        canvas.children = vec![card_id];
        let canvas_id = tree.insert(canvas);
        let mut root = container("root", BoxStyle::default(), rect(0.0, 0.0, 500.0, 400.0));
        root.children = vec![canvas_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let hit =
            resize_hit_at_screen_point(&tree, root_id, 340.0, 60.0, None).expect("resize hit");

        assert_eq!(hit.node_id, card_id);
        assert_eq!(hit.edge, ResizeEdge::Right);
    }

    #[test]
    fn resize_hit_respects_rounded_container_shape() {
        let mut tree = Tree::new();
        let mut card = container("card", resizable_style(), rect(0.0, 0.0, 100.0, 100.0));
        card.decoration = Some(rounded_decoration([40.0; 4]));
        let card_id = tree.insert(card);
        tree.set_root(card_id);

        assert_eq!(
            resize_hit_at_screen_point(&tree, card_id, 100.0, 100.0, None),
            None
        );

        let hit =
            resize_hit_at_screen_point(&tree, card_id, 88.0, 88.0, None).expect("arc resize hit");
        assert_eq!(hit.node_id, card_id);
        assert_eq!(hit.edge, ResizeEdge::BottomRight);
    }

    #[test]
    fn resize_hit_includes_edge_band_outside_container_bounds() {
        let mut tree = Tree::new();
        let card_id = tree.insert(container(
            "card",
            resizable_style(),
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        tree.set_root(card_id);

        let hit = resize_hit_at_screen_point(&tree, card_id, 106.0, 50.0, None)
            .expect("outside edge band should resize");

        assert_eq!(hit.node_id, card_id);
        assert_eq!(hit.edge, ResizeEdge::Right);
    }

    #[test]
    fn resize_hit_prefers_topmost_overlapping_child() {
        let mut tree = Tree::new();
        let lower_id = tree.insert(container(
            "lower",
            BoxStyle {
                z_index: 0,
                ..resizable_style()
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        let upper_id = tree.insert(container(
            "upper",
            BoxStyle {
                z_index: 10,
                ..resizable_style()
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        let mut root = container("root", BoxStyle::default(), rect(0.0, 0.0, 200.0, 200.0));
        root.children = vec![lower_id, upper_id];
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let hit =
            resize_hit_at_screen_point(&tree, root_id, 100.0, 50.0, None).expect("resize hit");

        assert_eq!(hit.node_id, upper_id);
        assert_eq!(hit.edge, ResizeEdge::Right);
    }
}
