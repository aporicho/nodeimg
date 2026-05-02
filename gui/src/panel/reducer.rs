#[cfg(test)]
mod tests {
    use crate::control::ResizeEdge;
    use crate::panel::{PanelConfig, PanelId};
    use crate::renderer::Rect;
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
        let mut tree = crate::tree::Tree::new();
        let first = config("tools");
        tree.ensure_panel(&first);
        tree.move_panel_by("tools", 30.0, 40.0);

        let mut changed = config("tools");
        changed.default_rect.x = 1000.0;
        changed.min_size = [260.0, 200.0];
        tree.ensure_panel(&changed);

        let state = tree.panel_state("tools").expect("panel state");
        assert_eq!(state.rect.x, 40.0);
        assert_eq!(state.rect.y, 60.0);
        assert_eq!(state.min_size, [260.0, 200.0]);
    }

    #[test]
    fn drag_session_moves_panel_and_focuses_it() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));
        tree.ensure_panel(&config("tools"));

        tree.start_panel_drag("preview", 100.0, 100.0);
        tree.move_panel_drag("preview", 118.0, 93.0);
        tree.end_panel_drag();

        let preview = tree.panel_state("preview").expect("preview state");
        let tools = tree.panel_state("tools").expect("tools state");
        assert_eq!(preview.rect.x, 28.0);
        assert_eq!(preview.rect.y, 13.0);
        assert!(preview.z_index > tools.z_index);
    }

    #[test]
    fn resize_session_clamps_to_min_size() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::Right, 240.0, 160.0);
        tree.move_panel_resize("preview", ResizeEdge::Right, 0.0, 160.0);
        tree.end_panel_resize("preview", ResizeEdge::Right, 0.0, 160.0);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 160.0);
    }

    #[test]
    fn resize_session_clamps_left_edge_without_moving_right_edge() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::Left, 10.0, 100.0);
        tree.move_panel_resize("preview", ResizeEdge::Left, 210.0, 100.0);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.x, 130.0);
        assert_eq!(state.rect.y, 20.0);
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 160.0);
        assert_eq!(state.rect.x + state.rect.w, 250.0);
    }

    #[test]
    fn resize_session_clamps_top_edge_without_moving_bottom_edge() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::Top, 100.0, 20.0);
        tree.move_panel_resize("preview", ResizeEdge::Top, 100.0, 140.0);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.x, 10.0);
        assert_eq!(state.rect.y, 100.0);
        assert_eq!(state.rect.w, 240.0);
        assert_eq!(state.rect.h, 80.0);
        assert_eq!(state.rect.y + state.rect.h, 180.0);
    }

    #[test]
    fn resize_session_clamps_top_left_corner_without_moving_opposite_corner() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::TopLeft, 10.0, 20.0);
        tree.move_panel_resize("preview", ResizeEdge::TopLeft, 210.0, 140.0);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.x, 130.0);
        assert_eq!(state.rect.y, 100.0);
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 80.0);
        assert_eq!(state.rect.x + state.rect.w, 250.0);
        assert_eq!(state.rect.y + state.rect.h, 180.0);
    }

    #[test]
    fn resize_session_recomputes_from_start_rect_when_drag_returns_from_min_size() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::TopLeft, 10.0, 20.0);
        tree.move_panel_resize("preview", ResizeEdge::TopLeft, 210.0, 140.0);
        tree.move_panel_resize("preview", ResizeEdge::TopLeft, 90.0, 80.0);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.x, 90.0);
        assert_eq!(state.rect.y, 80.0);
        assert_eq!(state.rect.w, 160.0);
        assert_eq!(state.rect.h, 100.0);
        assert_eq!(state.rect.x + state.rect.w, 250.0);
        assert_eq!(state.rect.y + state.rect.h, 180.0);
    }

    #[test]
    fn panel_layout_roundtrips_known_panels_and_ignores_unknown() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));
        tree.ensure_panel(&config("tools"));

        let mut layouts = tree.export_panel_layouts();
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

        tree.import_panel_layouts(&layouts);

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.x, 80.0);
        assert_eq!(state.rect.y, 90.0);
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 80.0);
        assert!(!state.visible);
        assert_eq!(state.z_index, 20);
        assert!(state.collapsed);
        assert!(tree.panel_state("missing").is_none());

        tree.bring_panel_to_front("tools");
        let tools = tree.panel_state("tools").expect("tools state");
        assert!(tools.z_index > 20);
    }
}
