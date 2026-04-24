use super::catalog::PlaygroundItemId;
use super::ids;
use super::state::DeveloperControlState;
use gui::renderer::Border;
use gui::theme::Theme;
use gui::tree::layout::{Align, Justify};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::collapsible::CollapsibleProps;
use gui::widget::frameworks::group::GroupProps;
use gui::widget::frameworks::list_view::ListViewProps;
use std::borrow::Cow;

pub(super) fn container_sample(
    item: PlaygroundItemId,
    theme: &Theme,
    state: &DeveloperControlState,
) -> Desc {
    match item {
        PlaygroundItemId::ContainerBox => container_box_sample(theme),
        PlaygroundItemId::RowLayout => row_sample(theme),
        PlaygroundItemId::ColumnLayout => column_sample(theme),
        PlaygroundItemId::ScrollArea => scroll_sample(),
        PlaygroundItemId::Group => group_sample(),
        PlaygroundItemId::Collapsible => collapsible_sample(state),
        _ => ui::container(format!("container_empty_{}", item.key())).build(),
    }
}

fn stage(id: impl Into<Cow<'static, str>>, theme: &Theme) -> gui::ui::ContainerBuilder {
    ui::container(id)
        .fill_width()
        .fill_height()
        .align_items(Align::Center)
        .justify_content(Justify::Center)
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(6.0)
}

fn container_box_sample(theme: &Theme) -> Desc {
    stage("container_box_sample", theme)
        .child(
            ui::container("container_box_outer")
                .fixed_width(128.0)
                .fixed_height(48.0)
                .padding_all(8.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.accent,
                })
                .radius_all(8.0)
                .child(
                    ui::container("container_box_inner")
                        .fill_width()
                        .fill_height()
                        .background(theme.colors.accent_soft)
                        .radius_all(4.0)
                        .build(),
                )
                .build(),
        )
        .build()
}

fn row_sample(theme: &Theme) -> Desc {
    stage("container_row_sample", theme)
        .child(
            ui::row("container_row_items")
                .gap(8.0)
                .children([
                    swatch("container_row_a", theme.colors.accent),
                    swatch("container_row_b", theme.colors.surface_hover),
                    swatch("container_row_c", theme.colors.text_muted),
                ])
                .build(),
        )
        .build()
}

fn column_sample(theme: &Theme) -> Desc {
    stage("container_column_sample", theme)
        .child(
            ui::column("container_column_items")
                .gap(6.0)
                .children([
                    bar("container_column_a", 96.0, theme),
                    bar("container_column_b", 72.0, theme),
                    bar("container_column_c", 112.0, theme),
                ])
                .build(),
        )
        .build()
}

fn scroll_sample() -> Desc {
    let items = (1..=8)
        .map(|index| {
            Desc::from(ui::widget(
                format!("container_scroll_item_{index}"),
                LabelProps {
                    text: Cow::Owned(format!("Row {index}")),
                    variant: LabelVariant::Caption,
                    muted: index % 2 == 0,
                },
            ))
        })
        .collect();

    Desc::from(ui::widget(
        Cow::Borrowed("container_scroll_list"),
        ListViewProps {
            height: 58.0,
            items,
        },
    ))
}

fn group_sample() -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed("container_group_sample"),
        GroupProps {
            title: Cow::Borrowed("Group"),
            content: vec![Desc::from(ui::widget(
                Cow::Borrowed("container_group_label"),
                LabelProps {
                    text: Cow::Borrowed("Grouped content"),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            ))],
        },
    ))
}

fn collapsible_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTAINER_COLLAPSIBLE_ID),
        CollapsibleProps {
            title: Cow::Borrowed("Advanced"),
            expanded: state.collapsible_open,
            disabled: false,
            content: vec![Desc::from(ui::widget(
                Cow::Borrowed("container_collapsible_label"),
                LabelProps {
                    text: Cow::Borrowed("Nested content"),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            ))],
        },
    ))
}

fn swatch(id: &'static str, color: gui::renderer::Color) -> Desc {
    ui::container(id)
        .fixed_width(30.0)
        .fixed_height(30.0)
        .background(color)
        .radius_all(6.0)
        .build()
}

fn bar(id: &'static str, width: f32, theme: &Theme) -> Desc {
    ui::container(id)
        .fixed_width(width)
        .fixed_height(12.0)
        .background(theme.colors.accent_soft)
        .radius_all(6.0)
        .build()
}
