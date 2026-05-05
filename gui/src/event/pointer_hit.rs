use crate::animation::AnimationStore;
use crate::shell::{AppEvent, MouseButton};
use crate::tree::{
    hit_test_with_animations, resize_hit_at_screen_point, HitChain, ResizeHit, Tree,
};

#[derive(Debug, Clone)]
pub(crate) struct PointerHitSnapshot {
    x: f32,
    y: f32,
    chain: HitChain,
    resize_hit: Option<ResizeHit>,
}

impl PointerHitSnapshot {
    pub(crate) fn from_event(
        tree: &Tree,
        animations: Option<&AnimationStore>,
        event: &AppEvent,
    ) -> Option<Self> {
        let (x, y, include_resize_hit) = pointer_event_hit_request(event)?;
        let Some(root) = tree.root() else {
            return Some(Self {
                x,
                y,
                chain: HitChain::empty(),
                resize_hit: None,
            });
        };
        let resize_hit = include_resize_hit
            .then(|| resize_hit_at_screen_point(tree, root, x, y, animations))
            .flatten();
        Some(Self {
            x,
            y,
            chain: hit_test_with_animations(tree, root, x, y, animations),
            resize_hit,
        })
    }

    pub(crate) fn matches_point(&self, x: f32, y: f32) -> bool {
        self.x == x && self.y == y
    }

    pub(crate) fn chain(&self) -> &HitChain {
        &self.chain
    }

    pub(crate) fn resize_hit(&self) -> Option<ResizeHit> {
        self.resize_hit
    }
}

fn pointer_event_hit_request(event: &AppEvent) -> Option<(f32, f32, bool)> {
    match *event {
        AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        } => Some((x, y, true)),
        AppEvent::MouseMove { x, y }
        | AppEvent::MouseRelease { x, y, .. }
        | AppEvent::ScrollLine { x, y, .. }
        | AppEvent::ScrollPixel { x, y, .. }
        | AppEvent::PinchZoom { x, y, .. } => Some((x, y, false)),
        AppEvent::MousePress { .. } => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::tree::layout::BoxStyle;
    use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

    fn hittable_node(id: &'static str, rect: Rect) -> TreeNode {
        TreeNodeBuilder::container(
            id,
            BoxStyle {
                hittable: true,
                ..Default::default()
            },
        )
        .rect(rect)
        .build()
    }

    #[test]
    fn pointer_snapshot_reuses_hit_chain_for_event_point() {
        let mut tree = Tree::new();
        let root = tree.insert(hittable_node(
            "root",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let snapshot =
            PointerHitSnapshot::from_event(&tree, None, &AppEvent::MouseMove { x: 20.0, y: 30.0 })
                .expect("pointer snapshot");

        assert!(snapshot.matches_point(20.0, 30.0));
        assert_eq!(snapshot.chain().leaf(), Some(root));
        assert!(snapshot.resize_hit().is_none());
    }
}
