use super::{
    end_drag, end_resize, export_layouts, import_layouts, move_drag, move_resize, panel_state,
    start_drag, start_resize,
};
use crate::geometry::ResizeEdge;
use crate::panel::{PanelConfig, PanelId};
use crate::renderer::Rect;
use crate::tree::Tree;
use std::borrow::Cow;

fn config(id: &'static str) -> PanelConfig {
    PanelConfig {
        id: PanelId::new(id),
        title: Cow::Borrowed("Panel"),
        default_rect: Rect {
            x: 10.0,
            y: 20.0,
            w: 240.0,
            h: 160.0,
        },
        min_size: [120.0, 80.0],
        titlebar_visible: true,
        draggable: true,
        resizable: true,
        closable: false,
        initially_visible: true,
    }
}

#[test]
fn ensure_panel_initializes_runtime_state_once() {
    let mut tree = Tree::new();
    let first = config("tools");
    super::ensure_panel(&mut tree, &first);
    super::drag::move_by(&mut tree, "tools", 30.0, 40.0);

    let mut changed = config("tools");
    changed.default_rect.x = 1000.0;
    changed.min_size = [260.0, 200.0];
    super::ensure_panel(&mut tree, &changed);

    let state = panel_state(&tree, "tools").expect("panel state");
    assert_eq!(state.rect.x, 40.0);
    assert_eq!(state.rect.y, 60.0);
    assert_eq!(state.min_size, [260.0, 200.0]);
}

#[test]
fn drag_session_moves_panel_and_focuses_it() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));
    super::ensure_panel(&mut tree, &config("tools"));

    start_drag(&mut tree, "preview", 100.0, 100.0);
    move_drag(&mut tree, "preview", 118.0, 93.0);
    end_drag(&mut tree);

    let preview = panel_state(&tree, "preview").expect("preview state");
    let tools = panel_state(&tree, "tools").expect("tools state");
    assert_eq!(preview.rect.x, 28.0);
    assert_eq!(preview.rect.y, 13.0);
    assert!(preview.z_index > tools.z_index);
}

#[test]
fn resize_session_clamps_to_min_size() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::Right, 240.0, 160.0);
    move_resize(&mut tree, "preview", ResizeEdge::Right, 0.0, 160.0);
    end_resize(&mut tree, "preview", ResizeEdge::Right, 0.0, 160.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.w, 120.0);
    assert_eq!(state.rect.h, 160.0);
}

#[test]
fn resize_session_clamps_left_edge_without_moving_right_edge() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::Left, 10.0, 100.0);
    move_resize(&mut tree, "preview", ResizeEdge::Left, 210.0, 100.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 130.0);
    assert_eq!(state.rect.y, 20.0);
    assert_eq!(state.rect.w, 120.0);
    assert_eq!(state.rect.h, 160.0);
    assert_eq!(state.rect.x + state.rect.w, 250.0);
}

#[test]
fn resize_session_expands_left_edge_without_moving_right_edge() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::Left, 10.0, 100.0);
    move_resize(&mut tree, "preview", ResizeEdge::Left, -30.0, 100.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, -30.0);
    assert_eq!(state.rect.y, 20.0);
    assert_eq!(state.rect.w, 280.0);
    assert_eq!(state.rect.h, 160.0);
    assert_eq!(state.rect.x + state.rect.w, 250.0);
}

#[test]
fn resize_session_clamps_top_edge_without_moving_bottom_edge() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::Top, 100.0, 20.0);
    move_resize(&mut tree, "preview", ResizeEdge::Top, 100.0, 140.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 10.0);
    assert_eq!(state.rect.y, 100.0);
    assert_eq!(state.rect.w, 240.0);
    assert_eq!(state.rect.h, 80.0);
    assert_eq!(state.rect.y + state.rect.h, 180.0);
}

#[test]
fn resize_session_expands_top_edge_without_moving_bottom_edge() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::Top, 100.0, 20.0);
    move_resize(&mut tree, "preview", ResizeEdge::Top, 100.0, -40.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 10.0);
    assert_eq!(state.rect.y, -40.0);
    assert_eq!(state.rect.w, 240.0);
    assert_eq!(state.rect.h, 220.0);
    assert_eq!(state.rect.y + state.rect.h, 180.0);
}

#[test]
fn resize_session_clamps_top_left_corner_without_moving_opposite_corner() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::TopLeft, 10.0, 20.0);
    move_resize(&mut tree, "preview", ResizeEdge::TopLeft, 210.0, 140.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 130.0);
    assert_eq!(state.rect.y, 100.0);
    assert_eq!(state.rect.w, 120.0);
    assert_eq!(state.rect.h, 80.0);
    assert_eq!(state.rect.x + state.rect.w, 250.0);
    assert_eq!(state.rect.y + state.rect.h, 180.0);
}

#[test]
fn resize_session_expands_top_left_corner_without_moving_opposite_corner() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::TopLeft, 10.0, 20.0);
    move_resize(&mut tree, "preview", ResizeEdge::TopLeft, -30.0, -40.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, -30.0);
    assert_eq!(state.rect.y, -40.0);
    assert_eq!(state.rect.w, 280.0);
    assert_eq!(state.rect.h, 220.0);
    assert_eq!(state.rect.x + state.rect.w, 250.0);
    assert_eq!(state.rect.y + state.rect.h, 180.0);
}

#[test]
fn resize_session_recomputes_from_start_rect_when_drag_returns_from_min_size() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));

    start_resize(&mut tree, "preview", ResizeEdge::TopLeft, 10.0, 20.0);
    move_resize(&mut tree, "preview", ResizeEdge::TopLeft, 210.0, 140.0);
    move_resize(&mut tree, "preview", ResizeEdge::TopLeft, 90.0, 80.0);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 90.0);
    assert_eq!(state.rect.y, 80.0);
    assert_eq!(state.rect.w, 160.0);
    assert_eq!(state.rect.h, 100.0);
    assert_eq!(state.rect.x + state.rect.w, 250.0);
    assert_eq!(state.rect.y + state.rect.h, 180.0);
}

#[test]
fn panel_layout_roundtrips_known_panels_and_ignores_unknown() {
    let mut tree = Tree::new();
    super::ensure_panel(&mut tree, &config("preview"));
    super::ensure_panel(&mut tree, &config("tools"));

    let mut layouts = export_layouts(&tree);
    assert_eq!(
        layouts
            .iter()
            .map(|layout| layout.id.as_str())
            .collect::<Vec<_>>(),
        vec!["preview", "tools"]
    );

    let preview = layouts
        .iter_mut()
        .find(|layout| layout.id == "preview")
        .expect("preview layout");
    preview.rect.x = 80.0;
    preview.rect.y = 90.0;
    preview.rect.w = 40.0;
    preview.rect.h = 50.0;
    preview.visible = false;
    preview.z_index = 20;
    preview.collapsed = true;

    layouts.push(crate::panel::PanelLayout {
        id: "missing".to_string(),
        rect: Rect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        },
        visible: true,
        z_index: 100,
        collapsed: false,
    });

    import_layouts(&mut tree, &layouts);

    let state = panel_state(&tree, "preview").expect("preview state");
    assert_eq!(state.rect.x, 80.0);
    assert_eq!(state.rect.y, 90.0);
    assert_eq!(state.rect.w, 120.0);
    assert_eq!(state.rect.h, 80.0);
    assert!(!state.visible);
    assert_eq!(state.z_index, 20);
    assert!(state.collapsed);
    assert!(panel_state(&tree, "missing").is_none());

    super::store::bring_to_front(&mut tree, "tools");
    let tools = panel_state(&tree, "tools").expect("tools state");
    assert!(tools.z_index > 20);
}
