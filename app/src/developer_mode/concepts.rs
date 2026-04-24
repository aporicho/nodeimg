use super::catalog::PlaygroundItemId;
use gui::renderer::{Border, TextStyle};
use gui::theme::Theme;
use gui::tree::layout::{Align, Justify, LeafKind, TextAlign, TextLayout, TextOverflow};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};

pub(super) fn concept_sample(item: PlaygroundItemId, theme: &Theme) -> Desc {
    match item {
        PlaygroundItemId::Canvas => canvas_concept(theme),
        PlaygroundItemId::PrimitiveConcept => family_concept(
            "concept_primitives",
            "Primitive layer",
            "Everything visible eventually becomes primitive draw commands.",
            theme,
        ),
        PlaygroundItemId::ContainerConcept => family_concept(
            "concept_containers",
            "Container layer",
            "Containers arrange, clip, group, and host children.",
            theme,
        ),
        PlaygroundItemId::ControlConcept => family_concept(
            "concept_controls",
            "Control layer",
            "Controls own user-facing interaction state.",
            theme,
        ),
        PlaygroundItemId::OverlayConcept => family_concept(
            "concept_overlay",
            "Overlay layer",
            "Overlay is positioned UI outside normal flow.",
            theme,
        ),
        PlaygroundItemId::WorkspaceConcept => family_concept(
            "concept_workspace",
            "Workspace layer",
            "Panels and node cards compose the product UI.",
            theme,
        ),
        PlaygroundItemId::Panel => workspace_panel_concept(theme),
        PlaygroundItemId::NodeCard => workspace_node_card_concept(theme),
        _ => ui::container(format!("concept_empty_{}", item.key())).build(),
    }
}

fn canvas_concept(theme: &Theme) -> Desc {
    ui::container("concept_canvas_stage")
        .relative()
        .fill_width()
        .fill_height()
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
        .child(
            ui::leaf(
                "concept_canvas_grid",
                LeafKind::Grid {
                    spacing: 16.0,
                    dot_color: theme.colors.canvas_grid,
                    dot_size: 1.1,
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(280.0)
            .fixed_height(48.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "concept_canvas_caption",
                "Coordinates, hit testing, painting",
                TextStyle {
                    color: theme.colors.text,
                    ..theme.text_style_label_sm()
                },
                TextLayout {
                    overflow: TextOverflow::Ellipsis,
                    align: TextAlign::Center,
                },
            )
            .absolute_xy(10.0, 17.0)
            .fixed_width(260.0)
            .fixed_height(18.0)
            .build(),
        )
        .build()
}

fn family_concept(
    id: &'static str,
    title: &'static str,
    body: &'static str,
    theme: &Theme,
) -> Desc {
    ui::column(id)
        .fill_width()
        .fill_height()
        .padding_all(8.0)
        .gap(4.0)
        .align_items(Align::Stretch)
        .justify_content(Justify::Center)
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
        .child(text(
            format!("{id}::title"),
            title,
            TextStyle {
                color: theme.colors.accent,
                ..theme.text_style_title_sm()
            },
            24.0,
        ))
        .child(text(
            format!("{id}::body"),
            body,
            TextStyle {
                color: theme.colors.text_muted,
                ..theme.text_style_label_sm()
            },
            32.0,
        ))
        .build()
}

fn workspace_panel_concept(theme: &Theme) -> Desc {
    ui::container("concept_workspace_panel")
        .relative()
        .fill_width()
        .fill_height()
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
        .child(
            ui::container("concept_panel_titlebar")
                .absolute_xy(18.0, 10.0)
                .fixed_width(140.0)
                .fixed_height(20.0)
                .background(theme.colors.surface_hover)
                .radius([6.0, 6.0, 0.0, 0.0])
                .build(),
        )
        .child(
            ui::container("concept_panel_body")
                .absolute_xy(18.0, 30.0)
                .fixed_width(140.0)
                .fixed_height(36.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.border,
                })
                .radius([0.0, 0.0, 6.0, 6.0])
                .build(),
        )
        .build()
}

fn workspace_node_card_concept(theme: &Theme) -> Desc {
    ui::container("concept_workspace_node_card")
        .relative()
        .fill_width()
        .fill_height()
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
        .child(
            ui::container("concept_node_card")
                .absolute_xy(24.0, 12.0)
                .fixed_width(132.0)
                .fixed_height(52.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.border,
                })
                .radius_all(6.0)
                .build(),
        )
        .child(
            ui::container("concept_node_port_in")
                .absolute_xy(18.0, 32.0)
                .fixed_width(9.0)
                .fixed_height(9.0)
                .background(theme.colors.accent)
                .radius_all(4.5)
                .build(),
        )
        .child(
            ui::container("concept_node_port_out")
                .absolute_xy(151.0, 32.0)
                .fixed_width(9.0)
                .fixed_height(9.0)
                .background(theme.colors.accent)
                .radius_all(4.5)
                .build(),
        )
        .build()
}

fn text(id: String, content: &'static str, style: TextStyle, height: f32) -> Desc {
    ui::text_with_layout(
        id,
        content,
        style,
        TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
        },
    )
    .fill_width()
    .fixed_height(height)
    .build()
}
