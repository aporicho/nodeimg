use std::time::Instant;

use super::arena::GestureArena;
use super::resize::ResizeRecognizer;
use super::{DragRecognizer, Gesture, GestureRecognizer, LongPressRecognizer, TapRecognizer};
use crate::tree::{HitChain, NodeKind, ResizeHit, Tree};

/// 根据命中链自动创建手势竞技场。
///
/// 规则：
/// - 容器级 draggable/resizable 会注册对应识别器
/// - 显式 gestures 保持兼容，供低层控件直接声明手势
/// - 所有识别器都绑定声明能力的节点本身，业务 owner 由事件层解析
pub(crate) fn arena_from_hit_chain(
    tree: &Tree,
    chain: &HitChain,
    x: f32,
    y: f32,
    last_tap_time: Option<Instant>,
) -> Option<GestureArena> {
    let target_id = default_target_id(tree, chain)?;
    let mut arena = GestureArena::new(target_id);
    let first_widget_index = first_widget_index(tree, chain);

    for (index, node_id) in chain.iter().enumerate() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };
        let target_id = node.id.to_string();
        let gestures = &node.style.gestures;
        let allow_semantic_drag = allows_semantic_drag(index, first_widget_index);
        tracing::debug!(
            target: "gui::gesture",
            node_id = %node.id,
            index,
            draggable = node.style.draggable,
            resizable = node.style.resizable,
            allow_semantic_drag,
            gestures = ?gestures,
            "inspect hit-chain node for gestures"
        );

        if gestures.contains(&Gesture::Tap) || gestures.contains(&Gesture::DoubleTap) {
            let mut rec = TapRecognizer::new(target_id.clone(), last_tap_time);
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if gestures.contains(&Gesture::Drag) || (node.style.draggable && allow_semantic_drag) {
            let mut rec = DragRecognizer::new(target_id.clone());
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if gestures.contains(&Gesture::LongPress) {
            let mut rec = LongPressRecognizer::new(target_id.clone());
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }
    }

    if arena.is_empty() {
        None
    } else {
        Some(arena)
    }
}

pub(crate) fn arena_from_resize_hit(
    tree: &Tree,
    hit: ResizeHit,
    x: f32,
    y: f32,
) -> Option<GestureArena> {
    let node = tree.get(hit.node_id)?;
    let target_id = node.id.to_string();
    let mut arena = GestureArena::new(target_id.clone());
    let mut rec = ResizeRecognizer::new(target_id.clone(), hit.edge);
    if rec.on_pointer_down(x, y) {
        tracing::debug!(
            target: "gui::gesture",
            node_id = %node.id,
            resize_edge = ?hit.edge,
            "add resize recognizer from interaction hit"
        );
        arena.add(Box::new(rec));
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

fn first_widget_index(tree: &Tree, chain: &HitChain) -> Option<usize> {
    chain.iter().enumerate().find_map(|(index, node_id)| {
        tree.get(node_id)
            .is_some_and(|node| matches!(node.kind, NodeKind::Widget(_)))
            .then_some(index)
    })
}

fn allows_semantic_drag(index: usize, first_widget_index: Option<usize>) -> bool {
    first_widget_index
        .map(|widget_index| index <= widget_index)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::geometry::TransformSpec;
    use crate::gesture::{Gesture, GestureSignal};
    use crate::renderer::Rect;
    use crate::theme::{dark_theme, Theme};
    use crate::tree::layout::Overflow;
    use crate::tree::{hit_test, reconcile, resize_hit_at_screen_point, Desc, Tree};
    use crate::ui::{self, StyleBuilder};
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::text_input::TextInputProps;
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
    fn panel_titlebar_drag_binds_to_declaring_node() {
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
    fn panel_root_resize_binds_to_declaring_node() {
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
        let hit = resize_hit_at_screen_point(&tree, root, 10.0, 20.0, None).unwrap();
        let arena = arena_from_resize_hit(&tree, hit, 10.0, 20.0).unwrap();
        assert_eq!(arena.target_id(), "demo_panel");
    }

    #[test]
    fn draggable_container_creates_drag_arena_without_explicit_gesture() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("drag_box")
            .fixed_width(80.0)
            .fixed_height(40.0)
            .hittable(true)
            .draggable(true)
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));
        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 40.0,
        };

        let chain = HitChain::new(vec![root]);
        let arena = arena_from_hit_chain(&tree, &chain, 20.0, 20.0, None).unwrap();
        assert_eq!(arena.target_id(), "drag_box");
    }

    #[test]
    fn resizable_container_creates_resize_arena_without_explicit_gesture() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("resize_box")
            .fixed_width(80.0)
            .fixed_height(40.0)
            .hittable(true)
            .resizable(true)
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));
        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 40.0,
        };

        let hit = resize_hit_at_screen_point(&tree, root, 80.0, 40.0, None).unwrap();
        let arena = arena_from_resize_hit(&tree, hit, 80.0, 40.0).unwrap();
        assert_eq!(arena.target_id(), "resize_box");
    }

    #[test]
    fn semantic_container_gestures_are_skipped_when_widget_control_is_hit() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("node_card")
            .fixed_width(220.0)
            .fixed_height(120.0)
            .hittable(true)
            .draggable(true)
            .resizable(true)
            .child(widget(
                "node_card::body::control",
                TextInputProps {
                    label: None,
                    value: Cow::Borrowed("hello"),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ))
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));

        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 220.0,
            h: 120.0,
        };
        let widget_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "node_card::body::control")
            .unwrap()
            .0;
        let field_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "node_card::body::control::field")
            .unwrap()
            .0;
        tree.get_mut(widget_id).unwrap().rect = Rect {
            x: 8.0,
            y: 8.0,
            w: 204.0,
            h: 24.0,
        };
        tree.get_mut(field_id).unwrap().rect = Rect {
            x: 8.0,
            y: 8.0,
            w: 204.0,
            h: 24.0,
        };

        let chain = HitChain::new(vec![field_id, widget_id, root]);

        assert!(arena_from_hit_chain(&tree, &chain, 100.0, 20.0, None).is_none());
    }

    #[test]
    fn semantic_resize_edge_wins_over_widget_control_body() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("node_card")
            .fixed_width(220.0)
            .fixed_height(120.0)
            .hittable(true)
            .draggable(true)
            .resizable(true)
            .child(widget(
                "node_card::body::control",
                TextInputProps {
                    label: None,
                    value: Cow::Borrowed("hello"),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ))
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));

        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 220.0,
            h: 120.0,
        };
        let widget_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "node_card::body::control")
            .unwrap()
            .0;
        let field_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "node_card::body::control::field")
            .unwrap()
            .0;
        tree.get_mut(widget_id).unwrap().rect = Rect {
            x: 8.0,
            y: 8.0,
            w: 204.0,
            h: 24.0,
        };
        tree.get_mut(field_id).unwrap().rect = Rect {
            x: 8.0,
            y: 8.0,
            w: 204.0,
            h: 24.0,
        };

        let hit = resize_hit_at_screen_point(&tree, root, 212.0, 20.0, None).unwrap();
        let mut arena = arena_from_resize_hit(&tree, hit, 212.0, 20.0).expect("arena");
        let signal = arena.pointer_move(224.0, 20.0).expect("resize start");

        match signal {
            GestureSignal::ResizeStart { id, edge, .. } => {
                assert_eq!(id, "node_card");
                assert_eq!(edge, crate::widget::resize_edge::ResizeEdge::Right);
            }
            other => panic!("expected resize start, got {:?}", other),
        }
    }

    #[test]
    fn semantic_resize_edge_uses_screen_point_under_transform() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("root")
            .fixed_width(500.0)
            .fixed_height(400.0)
            .child(
                ui::container("canvas")
                    .fixed_width(300.0)
                    .fixed_height(200.0)
                    .overflow(Overflow::Visible)
                    .transform(TransformSpec::translate_scale([100.0, -40.0], 2.0))
                    .child(
                        ui::container("node_card")
                            .fixed_width(100.0)
                            .fixed_height(40.0)
                            .hittable(true)
                            .resizable(true)
                            .build(),
                    )
                    .build(),
            )
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));

        let root = tree.root().expect("root");
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 500.0,
            h: 400.0,
        };
        let canvas_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "canvas")
            .expect("canvas")
            .0;
        tree.get_mut(canvas_id).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 200.0,
        };
        let card_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "node_card")
            .expect("card")
            .0;
        tree.get_mut(card_id).unwrap().rect = Rect {
            x: 20.0,
            y: 30.0,
            w: 100.0,
            h: 40.0,
        };

        let x = 340.0;
        let y = 60.0;
        let hit = resize_hit_at_screen_point(&tree, root, x, y, None).unwrap();
        let mut arena = arena_from_resize_hit(&tree, hit, x, y).expect("resize arena");
        let signal = arena.pointer_move(x + 12.0, y).expect("resize start");

        match signal {
            GestureSignal::ResizeStart { id, edge, .. } => {
                assert_eq!(id, "node_card");
                assert_eq!(edge, crate::widget::resize_edge::ResizeEdge::Right);
            }
            other => panic!("expected resize start, got {:?}", other),
        }
    }

    #[test]
    fn explicit_widget_gestures_still_work_inside_semantic_container() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let desc = ui::container("node_card")
            .fixed_width(240.0)
            .fixed_height(120.0)
            .hittable(true)
            .draggable(true)
            .resizable(true)
            .child(widget(
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
            ))
            .build();
        reconcile(&mut tree, desc, build_cx(&theme));

        let root = tree.root().unwrap();
        tree.get_mut(root).unwrap().rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 240.0,
            h: 120.0,
        };
        let slider_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "slider_radius")
            .unwrap()
            .0;
        tree.get_mut(slider_id).unwrap().rect = Rect {
            x: 16.0,
            y: 16.0,
            w: 200.0,
            h: 24.0,
        };
        let track_id = tree
            .iter()
            .find(|(_, n)| n.id.as_ref() == "slider_radius::track")
            .unwrap()
            .0;
        tree.get_mut(track_id).unwrap().rect = Rect {
            x: 56.0,
            y: 18.0,
            w: 120.0,
            h: 18.0,
        };

        let chain = HitChain::new(vec![track_id, slider_id, root]);
        let mut arena = arena_from_hit_chain(&tree, &chain, 100.0, 27.0, None).expect("arena");
        let signal = arena.pointer_move(108.0, 27.0).expect("drag start");

        match signal {
            GestureSignal::DragStart { id, .. } => assert_eq!(id, "slider_radius::track"),
            other => panic!("expected drag start, got {:?}", other),
        }
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
