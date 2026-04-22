use gui::canvas::camera::Camera;
use gui::canvas::connection_layer;
use gui::canvas::node_card::{node_card, CanvasNodeView};
use gui::canvas::{CanvasConnectionView, CanvasPendingConnectionView};
use gui::renderer::Rect;
use gui::theme::Theme;
use gui::tree::layout::{BoxStyle, Decoration, LeafKind, Position, Size, Transform};
use gui::tree::Desc;
use std::borrow::Cow;

const GRID_SPACING: f32 = 20.0;
const GRID_DOT_SIZE: f32 = 0.9;
const CANVAS_GRID_Z: i32 = -20;
const CANVAS_CONNECTION_Z: i32 = -10;

pub(crate) fn build_workspace_tree(
    viewport: Rect,
    camera: &Camera,
    theme: &Theme,
    canvas_nodes: &[CanvasNodeView],
    canvas_connections: &[CanvasConnectionView],
    pending_connection: Option<&CanvasPendingConnectionView>,
    panel_root: Desc,
) -> Desc {
    let (canvas_min_x, canvas_min_y) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = camera.screen_to_canvas(viewport.w, viewport.h);
    let grid_x = align_grid_start(canvas_min_x, GRID_SPACING);
    let grid_y = align_grid_start(canvas_min_y, GRID_SPACING);
    let grid_w = (canvas_max_x - canvas_min_x).abs() + GRID_SPACING * 4.0;
    let grid_h = (canvas_max_y - canvas_min_y).abs() + GRID_SPACING * 4.0;

    let mut canvas_children = vec![Desc::Leaf {
        id: Cow::Borrowed("canvas_grid"),
        style: BoxStyle {
            position: Position::absolute_xy(grid_x, grid_y),
            z_index: CANVAS_GRID_Z,
            width: Size::Fixed(grid_w.max(GRID_SPACING)),
            height: Size::Fixed(grid_h.max(GRID_SPACING)),
            ..BoxStyle::default()
        },
        kind: LeafKind::Grid {
            spacing: GRID_SPACING,
            dot_color: theme.colors.canvas_grid,
            dot_size: GRID_DOT_SIZE,
        },
    }];
    canvas_children.push(connection_layer(
        canvas_connections,
        pending_connection,
        CANVAS_CONNECTION_Z,
    ));
    canvas_children.extend(canvas_nodes.iter().map(|node| node_card(node, theme)));

    Desc::Container {
        id: Cow::Borrowed("root"),
        style: BoxStyle {
            width: Size::Fixed(viewport.w),
            height: Size::Fixed(viewport.h),
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.canvas_bg),
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Container {
                id: Cow::Borrowed("canvas_root"),
                style: BoxStyle {
                    position: Position::absolute_xy(0.0, 0.0),
                    width: Size::Fixed(viewport.w),
                    height: Size::Fixed(viewport.h),
                    transform: Some(Transform {
                        translate: [camera.x, camera.y],
                        scale: camera.zoom,
                        rotate: 0.0,
                    }),
                    ..BoxStyle::default()
                },
                decoration: None,
                children: canvas_children,
            },
            panel_root,
        ],
    }
}

pub(crate) fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}

#[cfg(test)]
mod tests {
    use super::{align_grid_start, build_workspace_tree};
    use gui::canvas::camera::Camera;
    use gui::canvas::CanvasPendingConnectionView;
    use gui::renderer::Rect;
    use gui::theme::light_theme;
    use gui::tree::layout::{BoxStyle, Size};
    use gui::tree::Desc;
    use std::borrow::Cow;

    #[test]
    fn grid_alignment_snaps_to_spacing_not_single_units() {
        assert_eq!(align_grid_start(3.0, 20.0), -40.0);
        assert_eq!(align_grid_start(21.0, 20.0), -20.0);
        assert_eq!(align_grid_start(-1.0, 20.0), -60.0);
    }

    #[test]
    fn workspace_tree_includes_pending_connection_layer_item() {
        let theme = light_theme();
        let desc = build_workspace_tree(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &Camera::new(),
            &theme,
            &[],
            &[],
            Some(&CanvasPendingConnectionView {
                from_port_id: "canvas_node::engine_node::1::port::output::image".to_string(),
                cursor_canvas: [100.0, 120.0],
            }),
            Desc::Container {
                id: Cow::Borrowed("panel_root"),
                style: BoxStyle {
                    width: Size::Fixed(0.0),
                    height: Size::Fixed(0.0),
                    ..BoxStyle::default()
                },
                decoration: None,
                children: Vec::new(),
            },
        );

        assert!(contains_desc_id(&desc, "canvas_connection::pending"));
    }

    fn contains_desc_id(desc: &Desc, id: &str) -> bool {
        if desc.id() == id {
            return true;
        }
        match desc {
            Desc::Container { children, .. } => {
                children.iter().any(|child| contains_desc_id(child, id))
            }
            Desc::Widget(_) | Desc::Leaf { .. } => false,
        }
    }
}
