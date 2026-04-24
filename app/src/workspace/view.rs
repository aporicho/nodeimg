use gui::canvas::camera::Camera;
use gui::canvas::connection_layer;
use gui::canvas::node_card::node_card_from_render_view;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::canvas::{CanvasConnectionView, CanvasPendingConnectionView};
use gui::geometry::TransformSpec;
use gui::renderer::Rect;
use gui::theme::Theme;
use gui::tree::layout::LeafKind;
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};

const GRID_SPACING: f32 = 20.0;
const GRID_DOT_SIZE: f32 = 0.9;
const CANVAS_GRID_Z: i32 = -20;
const CANVAS_CONNECTION_Z: i32 = -10;

pub(crate) fn build_workspace_tree(
    viewport: Rect,
    camera: &Camera,
    theme: &Theme,
    canvas_nodes: &[CanvasNodeRenderView],
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

    let mut canvas_children = vec![ui::leaf(
        "canvas_grid",
        LeafKind::Grid {
            spacing: GRID_SPACING,
            dot_color: theme.colors.canvas_grid,
            dot_size: GRID_DOT_SIZE,
        },
    )
    .absolute_xy(grid_x, grid_y)
    .z_index(CANVAS_GRID_Z)
    .fixed_width(grid_w.max(GRID_SPACING))
    .fixed_height(grid_h.max(GRID_SPACING))
    .build()];
    canvas_children.push(connection_layer(
        canvas_connections,
        pending_connection,
        CANVAS_CONNECTION_Z,
    ));
    canvas_children.extend(
        canvas_nodes
            .iter()
            .map(|node| node_card_from_render_view(node, theme)),
    );

    ui::container("root")
        .fixed_width(viewport.w)
        .fixed_height(viewport.h)
        .background(theme.colors.canvas_bg)
        .children(vec![
            ui::container("canvas_root")
                .absolute_xy(0.0, 0.0)
                .fixed_width(viewport.w)
                .fixed_height(viewport.h)
                .transform(TransformSpec::translate_scale(
                    [camera.x, camera.y],
                    camera.zoom,
                ))
                .children(canvas_children)
                .build(),
            panel_root,
        ])
        .build()
}

pub(crate) fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}

#[cfg(test)]
mod tests {
    use super::{align_grid_start, build_workspace_tree};
    use gui::canvas::camera::Camera;
    use gui::canvas::CanvasPendingConnectionView;
    use gui::geometry::TransformSpec;
    use gui::renderer::Rect;
    use gui::theme::light_theme;
    use gui::tree::Desc;
    use gui::ui::{self, StyleBuilder};

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
            ui::container("panel_root")
                .fixed_width(0.0)
                .fixed_height(0.0)
                .build(),
        );

        assert!(contains_desc_id(&desc, "canvas_connection::pending"));
    }

    #[test]
    fn workspace_canvas_root_transform_matches_camera() {
        let theme = light_theme();
        let mut camera = Camera::new();
        camera.x = 120.0;
        camera.y = -40.0;
        camera.zoom = 1.75;

        let desc = build_workspace_tree(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &camera,
            &theme,
            &[],
            &[],
            None,
            ui::container("panel_root")
                .fixed_width(0.0)
                .fixed_height(0.0)
                .build(),
        );

        let Desc::Container { children, .. } = desc else {
            panic!("workspace root should be a container");
        };
        let Desc::Container { id, style, .. } = &children[0] else {
            panic!("canvas root should be a container");
        };

        assert_eq!(id.as_ref(), "canvas_root");
        assert_eq!(
            style.transform,
            Some(TransformSpec::translate_scale([120.0, -40.0], 1.75))
        );
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
