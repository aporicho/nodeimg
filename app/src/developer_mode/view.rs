use super::catalog::{item_specs, PlaygroundGroup, PlaygroundItemSpec};
use super::concepts::concept_sample;
use super::containers::container_sample;
use super::ids;
use super::overlay::overlay_sample;
use super::primitives::primitive_sample;
use super::state::DeveloperModeState;
use super::widgets::control_sample;
use gui::canvas::camera::Camera;
use gui::geometry::TransformSpec;
use gui::gesture::Gesture;
use gui::renderer::{Border, Rect, TextStyle};
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;
use gui::tree::layout::{Align, Justify, LeafKind, Overflow, TextAlign, TextLayout, TextOverflow};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};

const CONCEPT_MAP_WIDTH: f32 = 1280.0;
const CONCEPT_MAP_HEIGHT: f32 = 1240.0;
const GRID_SPACING: f32 = 24.0;
const GRID_DOT_SIZE: f32 = 1.2;
const RESIZE_HANDLE_SIZE: f32 = 18.0;

pub(crate) struct DeveloperModeBuildContext<'a> {
    pub(crate) viewport: Rect,
    pub(crate) camera: &'a Camera,
    pub(crate) theme: &'a Theme,
    pub(crate) state: &'a DeveloperModeState,
    pub(crate) image: TextureHandle,
}

pub(crate) fn build_developer_page(ctx: DeveloperModeBuildContext<'_>) -> Desc {
    let map_width = ctx.viewport.w.max(CONCEPT_MAP_WIDTH);
    let map_height = ctx.viewport.h.max(CONCEPT_MAP_HEIGHT);
    let (canvas_min_x, canvas_min_y) = ctx.camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = ctx.camera.screen_to_canvas(ctx.viewport.w, ctx.viewport.h);
    let grid_x = align_grid_start(canvas_min_x, GRID_SPACING);
    let grid_y = align_grid_start(canvas_min_y, GRID_SPACING);
    let grid_w = (canvas_max_x - canvas_min_x).abs() + GRID_SPACING * 4.0;
    let grid_h = (canvas_max_y - canvas_min_y).abs() + GRID_SPACING * 4.0;
    let tiles = item_specs()
        .iter()
        .map(|spec| sample_tile(spec, ctx.theme, ctx.state, ctx.image));

    ui::container(ids::PLAYGROUND_PAGE_ID)
        .fixed_width(ctx.viewport.w)
        .fixed_height(ctx.viewport.h)
        .background(ctx.theme.colors.canvas_bg)
        .child(
            ui::container(ids::PLAYGROUND_CANVAS_ID)
                .absolute_xy(0.0, 0.0)
                .fixed_width(ctx.viewport.w)
                .fixed_height(ctx.viewport.h)
                .transform(TransformSpec::translate_scale(
                    [ctx.camera.x, ctx.camera.y],
                    ctx.camera.zoom,
                ))
                .child(
                    ui::leaf(
                        ids::PLAYGROUND_GRID_ID,
                        LeafKind::Grid {
                            spacing: GRID_SPACING,
                            dot_color: ctx.theme.colors.canvas_grid,
                            dot_size: GRID_DOT_SIZE,
                        },
                    )
                    .absolute_xy(grid_x, grid_y)
                    .fixed_width(grid_w.max(map_width))
                    .fixed_height(grid_h.max(map_height))
                    .z_index(-20)
                    .build(),
                )
                .children(tiles)
                .build(),
        )
        .build()
}

fn sample_tile(
    spec: &PlaygroundItemSpec,
    theme: &Theme,
    state: &DeveloperModeState,
    image: TextureHandle,
) -> Desc {
    let position = state.position(spec.id);
    let size = state.size(spec.id);
    let body = match spec.group {
        PlaygroundGroup::Foundation | PlaygroundGroup::Family | PlaygroundGroup::Workspace => {
            concept_sample(spec.id, theme)
        }
        PlaygroundGroup::Primitive => primitive_sample(spec.id, theme, image),
        PlaygroundGroup::Container => container_sample(spec.id, theme, state.controls()),
        PlaygroundGroup::Control => control_sample(spec.id, theme, state.controls(), image),
        PlaygroundGroup::Overlay => overlay_sample(spec.id),
    };
    let body_width = (size.width - 16.0).max(1.0);
    let body_height = (size.height - 58.0).max(48.0);

    ui::column(ids::tile_id(spec.id))
        .absolute_xy(position.x, position.y)
        .fixed_width(size.width)
        .fixed_height(size.height)
        .padding_all(8.0)
        .gap(5.0)
        .hittable(true)
        .gesture(Gesture::Drag)
        .background(theme.colors.surface)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(8.0)
        .child(
            ui::container(ids::tile_stage_id(spec.id))
                .relative()
                .fixed_width(body_width)
                .fixed_height(body_height)
                .align_items(Align::Center)
                .justify_content(Justify::Center)
                .overflow(Overflow::Hidden)
                .child(body)
                .build(),
        )
        .child(tile_text(
            ids::tile_title_id(spec.id),
            spec.title,
            TextStyle {
                color: theme.colors.text,
                ..theme.text_style_label_sm()
            },
            body_width,
            16.0,
        ))
        .child(tile_text(
            ids::tile_description_id(spec.id),
            spec.description,
            TextStyle {
                color: theme.colors.text_muted,
                ..theme.text_style_label_sm()
            },
            body_width,
            16.0,
        ))
        .child(resize_handle(spec, size.width, size.height, theme))
        .build()
}

fn resize_handle(
    spec: &PlaygroundItemSpec,
    tile_width: f32,
    tile_height: f32,
    theme: &Theme,
) -> Desc {
    ui::container(ids::tile_resize_handle_id(spec.id))
        .absolute_xy(
            (tile_width - RESIZE_HANDLE_SIZE - 2.0).max(0.0),
            (tile_height - RESIZE_HANDLE_SIZE - 2.0).max(0.0),
        )
        .fixed_width(RESIZE_HANDLE_SIZE)
        .fixed_height(RESIZE_HANDLE_SIZE)
        .hittable(true)
        .gesture(Gesture::Drag)
        .background(theme.colors.surface_hover)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(5.0)
        .child(
            ui::line(
                format!("{}::mark_a", ids::tile_resize_handle_id(spec.id)),
                gui::renderer::Point { x: 6.0, y: 13.0 },
                gui::renderer::Point { x: 13.0, y: 6.0 },
                gui::renderer::Stroke::new(1.5, theme.colors.text_muted),
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(RESIZE_HANDLE_SIZE)
            .fixed_height(RESIZE_HANDLE_SIZE)
            .build(),
        )
        .child(
            ui::line(
                format!("{}::mark_b", ids::tile_resize_handle_id(spec.id)),
                gui::renderer::Point { x: 9.0, y: 14.0 },
                gui::renderer::Point { x: 14.0, y: 9.0 },
                gui::renderer::Stroke::new(1.5, theme.colors.text_muted),
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(RESIZE_HANDLE_SIZE)
            .fixed_height(RESIZE_HANDLE_SIZE)
            .build(),
        )
        .build()
}

fn tile_text(id: String, text: &'static str, style: TextStyle, width: f32, height: f32) -> Desc {
    ui::text_with_layout(
        id,
        text,
        style,
        TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
        },
    )
    .fixed_width(width)
    .fixed_height(height)
    .build()
}

pub(crate) fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}
