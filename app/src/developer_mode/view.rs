use super::catalog::{item_specs, PlaygroundGroup, PlaygroundItemSpec};
use super::concepts::concept_sample;
use super::containers::container_sample;
use super::ids;
use super::overlay::overlay_sample;
use super::primitives::primitive_sample;
use super::state::DeveloperModeState;
use super::widgets::control_sample;
use gui::gesture::Gesture;
use gui::renderer::{Border, Rect, TextStyle};
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;
use gui::tree::layout::{Align, Justify, LeafKind, Overflow, TextAlign, TextLayout, TextOverflow};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};

const CONCEPT_MAP_WIDTH: f32 = 1280.0;
const CONCEPT_MAP_HEIGHT: f32 = 1240.0;

pub(crate) struct DeveloperModeBuildContext<'a> {
    pub(crate) viewport: Rect,
    pub(crate) theme: &'a Theme,
    pub(crate) state: &'a DeveloperModeState,
    pub(crate) image: TextureHandle,
}

pub(crate) fn build_developer_page(ctx: DeveloperModeBuildContext<'_>) -> Desc {
    let map_width = ctx.viewport.w.max(CONCEPT_MAP_WIDTH);
    let map_height = ctx.viewport.h.max(CONCEPT_MAP_HEIGHT);
    let tiles = item_specs()
        .iter()
        .map(|spec| sample_tile(spec, ctx.theme, ctx.state, ctx.image));

    ui::container(ids::PLAYGROUND_PAGE_ID)
        .fixed_width(ctx.viewport.w)
        .fixed_height(ctx.viewport.h)
        .overflow(Overflow::Scroll)
        .background(ctx.theme.colors.canvas_bg)
        .child(
            ui::container(ids::PLAYGROUND_CANVAS_ID)
                .relative()
                .fixed_width(map_width)
                .fixed_height(map_height)
                .overflow(Overflow::Hidden)
                .background(ctx.theme.colors.canvas_bg)
                .child(
                    ui::leaf(
                        ids::PLAYGROUND_GRID_ID,
                        LeafKind::Grid {
                            spacing: 24.0,
                            dot_color: ctx.theme.colors.canvas_grid,
                            dot_size: 1.2,
                        },
                    )
                    .absolute_xy(0.0, 0.0)
                    .fixed_width(map_width)
                    .fixed_height(map_height)
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
    let body = match spec.group {
        PlaygroundGroup::Foundation | PlaygroundGroup::Family | PlaygroundGroup::Workspace => {
            concept_sample(spec.id, theme)
        }
        PlaygroundGroup::Primitive => primitive_sample(spec.id, theme, image),
        PlaygroundGroup::Container => container_sample(spec.id, theme, state.controls()),
        PlaygroundGroup::Control => control_sample(spec.id, theme, state.controls(), image),
        PlaygroundGroup::Overlay => overlay_sample(spec.id),
    };
    let body_width = (spec.size[0] - 16.0).max(1.0);
    let body_height = (spec.size[1] - 58.0).max(48.0);

    ui::column(ids::tile_id(spec.id))
        .absolute_xy(position.x, position.y)
        .fixed_width(spec.size[0])
        .fixed_height(spec.size[1])
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
