use super::model::CanvasInteractionRuntime;
use super::{
    begin_pending_connection, bring_node_to_front, cancel_pending_connection, clear_selection,
    export_node_layouts, hovered_port_id, import_node_layouts, is_node_selected, move_node_by,
    node_layout, pending_connection, port_group_view, resize_node_by, resize_node_from,
    select_node, set_hovered_port, set_node_rect, sync_node_layouts, toggle_port_group,
    update_pending_connection,
};
use crate::canvas::{CanvasNodeIdentity, CanvasNodeLayout, CanvasPortSide};
use crate::geometry::ResizeEdge;
use crate::renderer::Rect;
use crate::tree::Tree;

const OUTPUT_PORT: &str = "canvas_node::node::1::port::output::image";

#[test]
fn hovered_port_update_is_idempotent() {
    let mut runtime = CanvasInteractionRuntime::default();

    assert!(runtime.set_hovered_port(Some(OUTPUT_PORT)));
    assert!(!runtime.set_hovered_port(Some(OUTPUT_PORT)));
    assert!(runtime.set_hovered_port(None));
    assert!(!runtime.set_hovered_port(None));
}

#[test]
fn pending_connection_update_is_idempotent() {
    let mut runtime = CanvasInteractionRuntime::default();

    assert!(runtime.begin_pending_connection(OUTPUT_PORT, [1.0, 2.0]));
    assert!(!runtime.update_pending_connection([1.0, 2.0]));
    assert!(runtime.update_pending_connection([2.0, 3.0]));
}

#[test]
fn canvas_node_layout_sync_uses_owner_identity() {
    let mut tree = Tree::new();
    let first = CanvasNodeIdentity {
        owner_id: "engine_node::1".to_string(),
        default_rect: Rect {
            x: 10.0,
            y: 20.0,
            w: 220.0,
            h: 96.0,
        },
    };

    let layouts = sync_node_layouts(&mut tree, std::slice::from_ref(&first));
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].owner_id, "engine_node::1");
    assert_eq!(layouts[0].rect.x, 10.0);

    import_node_layouts(
        &mut tree,
        &[CanvasNodeLayout {
            owner_id: "engine_node::1".to_string(),
            rect: Rect {
                x: 80.0,
                y: 90.0,
                w: 260.0,
                h: 120.0,
            },
            z_index: 7,
            collapsed: true,
            user_min_height: Some(120.0),
        }],
    );

    let layouts = sync_node_layouts(&mut tree, &[first]);
    assert_eq!(layouts[0].rect.x, 80.0);
    assert_eq!(layouts[0].rect.y, 90.0);
    assert_eq!(layouts[0].z_index, 7);
    assert!(layouts[0].collapsed);
    assert_eq!(layouts[0].user_min_height, Some(120.0));

    assert!(move_node_by(&mut tree, "engine_node::1", 10.0, -5.0));
    let layouts = export_node_layouts(&tree);
    assert_eq!(layouts[0].rect.x, 90.0);
    assert_eq!(layouts[0].rect.y, 85.0);

    assert!(resize_node_by(
        &mut tree,
        "engine_node::1",
        ResizeEdge::Right,
        80.0,
        20.0
    ));
    let layouts = export_node_layouts(&tree);
    assert_eq!(layouts[0].rect.w, 340.0);
    assert_eq!(layouts[0].rect.h, 132.0);

    let stale = sync_node_layouts(&mut tree, &[]);
    assert!(stale.is_empty());
    assert!(export_node_layouts(&tree).is_empty());
}

#[test]
fn canvas_port_group_toggle_is_runtime_state() {
    let mut tree = Tree::new();

    assert!(!port_group_view(&tree, "engine_node::1", CanvasPortSide::Input).open);
    assert!(toggle_port_group(
        &mut tree,
        "engine_node::1",
        CanvasPortSide::Input
    ));
    assert!(port_group_view(&tree, "engine_node::1", CanvasPortSide::Input).open);
    assert!(!toggle_port_group(
        &mut tree,
        "engine_node::1",
        CanvasPortSide::Input
    ));
}

#[test]
fn canvas_interaction_state_tracks_selection_and_prunes_stale_nodes() {
    let mut tree = Tree::new();
    let first = CanvasNodeIdentity {
        owner_id: "engine_node::1".to_string(),
        default_rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 80.0,
        },
    };
    let second = CanvasNodeIdentity {
        owner_id: "engine_node::2".to_string(),
        default_rect: Rect {
            x: 120.0,
            y: 0.0,
            w: 100.0,
            h: 80.0,
        },
    };

    sync_node_layouts(&mut tree, &[first.clone(), second.clone()]);
    assert!(select_node(&mut tree, "engine_node::1"));
    assert!(!select_node(&mut tree, "engine_node::1"));
    assert!(is_node_selected(&tree, "engine_node::1"));
    assert!(!is_node_selected(&tree, "engine_node::2"));
    assert!(toggle_port_group(
        &mut tree,
        "engine_node::1",
        CanvasPortSide::Input
    ));
    assert!(port_group_view(&tree, "engine_node::1", CanvasPortSide::Input).open);
    assert!(!begin_pending_connection(
        &mut tree,
        "canvas_node::engine_node::1::port::input::prompt",
        [2.0, 3.0],
    ));
    assert!(begin_pending_connection(
        &mut tree,
        "canvas_node::engine_node::1::port::output::image",
        [2.0, 3.0],
    ));
    assert_eq!(
        pending_connection(&tree).map(|pending| pending.cursor_canvas),
        Some([2.0, 3.0])
    );
    assert!(update_pending_connection(&mut tree, [4.0, 5.0]));
    assert!(set_hovered_port(
        &mut tree,
        Some("canvas_node::engine_node::1::port::input::prompt")
    ));
    assert_eq!(
        hovered_port_id(&tree),
        Some("canvas_node::engine_node::1::port::input::prompt".to_string())
    );
    assert!(cancel_pending_connection(&mut tree));
    assert!(pending_connection(&tree).is_none());
    assert!(hovered_port_id(&tree).is_none());
    assert!(!cancel_pending_connection(&mut tree));
    assert!(begin_pending_connection(
        &mut tree,
        "canvas_node::engine_node::1::port::output::image",
        [2.0, 3.0],
    ));
    assert!(set_hovered_port(
        &mut tree,
        Some("canvas_node::engine_node::1::port::input::prompt")
    ));

    sync_node_layouts(&mut tree, &[second]);

    assert!(!is_node_selected(&tree, "engine_node::1"));
    assert!(!port_group_view(&tree, "engine_node::1", CanvasPortSide::Input).open);
    assert!(pending_connection(&tree).is_none());
    assert!(hovered_port_id(&tree).is_none());
}

#[test]
fn canvas_node_frame_api_sets_rect_and_brings_nodes_to_front() {
    let mut tree = Tree::new();
    sync_node_layouts(
        &mut tree,
        &[
            CanvasNodeIdentity {
                owner_id: "engine_node::1".to_string(),
                default_rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 304.0,
                    h: 132.0,
                },
            },
            CanvasNodeIdentity {
                owner_id: "engine_node::2".to_string(),
                default_rect: Rect {
                    x: 20.0,
                    y: 20.0,
                    w: 304.0,
                    h: 132.0,
                },
            },
        ],
    );

    assert_eq!(node_layout(&tree, "engine_node::1").unwrap().z_index, 0);
    assert_eq!(node_layout(&tree, "engine_node::2").unwrap().z_index, 1);
    assert!(bring_node_to_front(&mut tree, "engine_node::1"));
    assert_eq!(node_layout(&tree, "engine_node::1").unwrap().z_index, 2);
    assert!(!bring_node_to_front(&mut tree, "engine_node::1"));

    let rect = Rect {
        x: 40.0,
        y: 50.0,
        w: 380.0,
        h: 190.0,
    };
    assert!(set_node_rect(&mut tree, "engine_node::1", rect));
    assert!(!set_node_rect(&mut tree, "engine_node::1", rect));
    assert_eq!(node_layout(&tree, "engine_node::1").unwrap().rect, rect);
}

#[test]
fn canvas_node_resize_from_start_rect_keeps_opposite_corner_fixed_after_clamp() {
    let mut tree = Tree::new();
    let identity = CanvasNodeIdentity {
        owner_id: "engine_node::1".to_string(),
        default_rect: Rect {
            x: 10.0,
            y: 20.0,
            w: 420.0,
            h: 220.0,
        },
    };
    sync_node_layouts(&mut tree, &[identity]);
    let start = node_layout(&tree, "engine_node::1").unwrap().rect;
    let fixed_right = start.x + start.w;
    let fixed_bottom = start.y + start.h;

    assert!(resize_node_from(
        &mut tree,
        "engine_node::1",
        start,
        ResizeEdge::TopLeft,
        500.0,
        500.0,
    ));
    let clamped = node_layout(&tree, "engine_node::1").unwrap().rect;
    assert_eq!(clamped.x + clamped.w, fixed_right);
    assert_eq!(clamped.y + clamped.h, fixed_bottom);
    assert_eq!(clamped.w, 304.0);
    assert_eq!(clamped.h, 132.0);

    assert!(resize_node_from(
        &mut tree,
        "engine_node::1",
        start,
        ResizeEdge::TopLeft,
        -40.0,
        -30.0,
    ));
    let expanded = node_layout(&tree, "engine_node::1").unwrap().rect;
    assert_eq!(expanded.x, start.x - 40.0);
    assert_eq!(expanded.y, start.y - 30.0);
    assert_eq!(expanded.w, start.w + 40.0);
    assert_eq!(expanded.h, start.h + 30.0);
    assert_eq!(expanded.x + expanded.w, fixed_right);
    assert_eq!(expanded.y + expanded.h, fixed_bottom);
}

#[test]
fn clear_canvas_selection_reports_only_real_changes() {
    let mut tree = Tree::new();
    sync_node_layouts(
        &mut tree,
        &[CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 80.0,
            },
        }],
    );

    assert!(!clear_selection(&mut tree));
    assert!(select_node(&mut tree, "engine_node::1"));
    assert!(clear_selection(&mut tree));
    assert!(!is_node_selected(&tree, "engine_node::1"));
    assert!(!clear_selection(&mut tree));
}
