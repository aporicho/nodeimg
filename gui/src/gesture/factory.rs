use std::time::Instant;

use super::arena::GestureArena;
use super::resize::ResizeRecognizer;
use super::{DragRecognizer, Gesture, GestureRecognizer, LongPressRecognizer, TapRecognizer};
use crate::tree::{HitChain, Tree};

/// 根据命中链自动创建手势竞技场。
///
/// 规则：
/// - Tap / DoubleTap 共用点击识别器（TapRecognizer）
/// - Drag / Resize 优先绑定最近的 Panel widget
/// - 其余手势绑定声明该手势的节点本身
pub(crate) fn arena_from_hit_chain(
    tree: &Tree,
    chain: &HitChain,
    x: f32,
    y: f32,
    last_tap_time: Option<Instant>,
) -> Option<GestureArena> {
    let target_id = default_target_id(tree, chain)?;
    let mut arena = GestureArena::new(target_id);

    for node_id in chain.iter() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };
        for gesture in &node.style.gestures {
            match gesture {
                Gesture::Tap | Gesture::DoubleTap => {
                    let mut rec = TapRecognizer::new(node.id.to_string(), last_tap_time);
                    if rec.on_pointer_down(x, y) {
                        arena.add(Box::new(rec));
                    }
                }
                Gesture::Drag => {
                    let target_id = resolve_drag_target_id(tree, chain, node_id);
                    let mut rec = DragRecognizer::new(target_id);
                    if rec.on_pointer_down(x, y) {
                        arena.add(Box::new(rec));
                    }
                }
                Gesture::LongPress => {
                    let mut rec = LongPressRecognizer::new(node.id.to_string());
                    if rec.on_pointer_down(x, y) {
                        arena.add(Box::new(rec));
                    }
                }
                Gesture::Resize => {
                    let Some((target_id, rect)) = resolve_resize_target(tree, chain, node_id)
                    else {
                        continue;
                    };
                    let mut rec = ResizeRecognizer::new(target_id, rect);
                    if rec.on_pointer_down(x, y) {
                        arena.add(Box::new(rec));
                    }
                }
            }
        }
    }

    if arena.is_empty() {
        None
    } else {
        Some(arena)
    }
}

fn default_target_id(tree: &Tree, chain: &HitChain) -> Option<String> {
    chain
        .leaf()
        .and_then(|id| tree.get(id))
        .map(|node| node.id.to_string())
}

fn resolve_drag_target_id(tree: &Tree, chain: &HitChain, fallback_node_id: usize) -> String {
    if is_panel_titlebar_drag(tree, fallback_node_id) {
        nearest_panel_id(tree, chain).unwrap_or_else(|| {
            tree.get(fallback_node_id)
                .map(|node| node.id.to_string())
                .unwrap_or_default()
        })
    } else {
        tree.get(fallback_node_id)
            .map(|node| node.id.to_string())
            .unwrap_or_default()
    }
}

fn is_panel_titlebar_drag(tree: &Tree, node_id: usize) -> bool {
    tree.get(node_id)
        .map(|node| node.id.as_ref().ends_with("::titlebar"))
        .unwrap_or(false)
}

fn resolve_resize_target(
    tree: &Tree,
    chain: &HitChain,
    fallback_node_id: usize,
) -> Option<(String, crate::renderer::Rect)> {
    if let Some(panel_id) = nearest_panel_node_id(tree, chain) {
        let node = tree.get(panel_id)?;
        return Some((node.id.to_string(), node.rect));
    }

    let node = tree.get(fallback_node_id)?;
    Some((node.id.to_string(), node.rect))
}

fn nearest_panel_id(tree: &Tree, chain: &HitChain) -> Option<String> {
    nearest_panel_node_id(tree, chain)
        .and_then(|id| tree.get(id))
        .map(|node| node.id.to_string())
}

fn nearest_panel_node_id(tree: &Tree, chain: &HitChain) -> Option<usize> {
    chain.iter().find(|&node_id| {
        tree.get(node_id)
            .and_then(|node| match &node.kind {
                crate::tree::NodeKind::Widget(props) => Some(props.widget_type() == "Panel"),
                _ => None,
            })
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::gesture::{Gesture, GestureSignal};
    use crate::renderer::Rect;
    use crate::theme::{dark_theme, Theme};
    use crate::tree::{hit_test, reconcile, Desc, Tree};
    use crate::ui::{self, StyleBuilder};
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::toggle::ToggleProps;
    use crate::widget::frameworks::panel::PanelProps;
    use crate::widget::props::{WidgetBuildCx, WidgetProps};

    fn build_cx<'a>(theme: &'a Theme) -> WidgetBuildCx<'a> {
        WidgetBuildCx {
            theme,
            force_rebuild: false,
        }
    }

    fn widget(id: impl Into<Cow<'static, str>>, props: impl WidgetProps) -> Desc {
        ui::widget(id, props).build()
    }

    fn test_panel_desc() -> Desc {
        widget(
            "demo_panel",
            PanelProps {
                title: Cow::Borrowed("Demo"),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 200.0,
                    h: 120.0,
                },
                z_index: 0,
                min_size: [120.0, 80.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                content: vec![],
            },
        )
    }

    #[test]
    fn factory_returns_none_for_empty_chain() {
        let tree = Tree::new();
        let chain = HitChain::empty();
        assert!(arena_from_hit_chain(&tree, &chain, 0.0, 0.0, None).is_none());
    }

    #[test]
    fn drag_and_resize_bind_to_panel_widget() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        reconcile(&mut tree, test_panel_desc(), build_cx(&theme));

        let root = tree.root().unwrap();
        let root_node = tree.get_mut(root).unwrap();
        root_node.rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 120.0,
        };

        let titlebar_id = root_node.children[0];
        let titlebar = tree.get_mut(titlebar_id).unwrap();
        titlebar.rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 32.0,
        };

        let chain = HitChain::new(vec![titlebar_id, root]);
        let arena = arena_from_hit_chain(&tree, &chain, 12.0, 22.0, None).unwrap();
        assert_eq!(arena.target_id(), "demo_panel::titlebar");
    }

    #[test]
    fn resize_uses_panel_rect_and_survives_titlebar_hit() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        reconcile(&mut tree, test_panel_desc(), build_cx(&theme));

        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 120.0,
        };
        let chain = HitChain::new(vec![root]);
        let arena = arena_from_hit_chain(&tree, &chain, 10.0, 20.0, None);
        assert!(arena.is_some());
    }

    #[test]
    fn tap_declared_leaf_creates_arena() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::text(
            "tap_target",
            "x",
            crate::renderer::TextStyle::new(crate::renderer::Color::WHITE, 12.0),
        )
        .fixed_width(20.0)
        .fixed_height(20.0)
        .hittable(true)
        .gesture(Gesture::Tap)
        .build();
        reconcile(&mut tree, desc, build_cx(&theme));
        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 20.0,
        };

        let chain = hit_test(&tree, root, 5.0, 5.0);
        let arena = arena_from_hit_chain(&tree, &chain, 5.0, 5.0, None);
        assert!(arena.is_some());
        assert_eq!(arena.unwrap().target_id(), "tap_target");
    }

    #[test]
    fn toggle_track_click_emits_click_signal() {
        let theme = dark_theme();
        let desc = widget(
            "toggle_grid",
            ToggleProps {
                label: Some(Cow::Borrowed("Grid")),
                value: true,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        );
        let mut tree = Tree::new();
        reconcile(&mut tree, desc, build_cx(&theme));
        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 20.0,
        };
        let track_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "toggle_grid::track")
            .unwrap()
            .0;
        tree.get_mut(track_id).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 32.0,
            h: 18.0,
        };

        let chain = hit_test(&tree, root, 10.0, 9.0);
        let mut arena = arena_from_hit_chain(&tree, &chain, 10.0, 9.0, None).expect("arena");
        let signal = arena.pointer_up(10.0, 9.0).expect("click signal");
        match signal {
            GestureSignal::Click(id) => assert_eq!(id, "toggle_grid::track"),
            other => panic!("expected click, got {:?}", other),
        }
    }

    #[test]
    fn slider_track_drag_emits_slider_target_not_panel() {
        let theme = dark_theme();
        let desc = widget(
            "slider_radius",
            SliderProps {
                label: Some(Cow::Borrowed("Radius")),
                min: 0.0,
                max: 10.0,
                step: 0.1,
                value: 5.0,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        );
        let mut tree = Tree::new();
        reconcile(&mut tree, desc, build_cx(&theme));
        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 24.0,
        };
        let track_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "slider_radius::track")
            .unwrap()
            .0;
        tree.get_mut(track_id).unwrap().rect = Rect {
            x: 40.0,
            y: 2.0,
            w: 120.0,
            h: 18.0,
        };

        let chain = hit_test(&tree, root, 100.0, 10.0);
        let mut arena = arena_from_hit_chain(&tree, &chain, 100.0, 10.0, None).expect("arena");
        let signal = arena.pointer_move(110.0, 10.0).expect("drag start");
        match signal {
            GestureSignal::DragStart { id, .. } => assert_eq!(id, "slider_radius::track"),
            other => panic!("expected drag start, got {:?}", other),
        }

        let signal = arena.pointer_move(120.0, 10.0).expect("drag move");
        match signal {
            GestureSignal::DragMove { id, .. } => assert_eq!(id, "slider_radius::track"),
            other => panic!("expected drag move, got {:?}", other),
        }
    }
}
