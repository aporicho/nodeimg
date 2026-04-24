use super::primitives::primitive_section;
use super::section::VisualAuditSection;
use super::state::VisualAuditState;
use super::widgets::{
    button_section, container_section, input_section, media_section, overlay_section,
    overview_section, selection_section, typography_section,
};
use gui::renderer::{Border, Rect};
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::list_view::ListViewProps;
use std::borrow::Cow;

pub(crate) fn build_visual_audit_sections(
    theme: &Theme,
    state: &VisualAuditState,
    image: TextureHandle,
) -> Vec<VisualAuditSection> {
    vec![
        overview_section(),
        primitive_section(theme, image),
        typography_section(theme),
        button_section(),
        input_section(state),
        selection_section(state),
        media_section(state, image),
        container_section(state),
        overlay_section(),
    ]
}

pub(crate) struct VisualAuditBuildContext<'a> {
    pub(crate) viewport: Rect,
    pub(crate) theme: &'a Theme,
    pub(crate) state: &'a VisualAuditState,
    pub(crate) image: TextureHandle,
}

pub(crate) fn build_visual_audit_page(ctx: VisualAuditBuildContext<'_>) -> Desc {
    let items = build_visual_audit_sections(ctx.theme, ctx.state, ctx.image)
        .into_iter()
        .map(|section| section_card(ctx.theme, section))
        .collect();

    ui::column("visual_audit_page")
        .fixed_width(ctx.viewport.w)
        .fixed_height(ctx.viewport.h)
        .gap(ctx.theme.spacing.md)
        .padding_all(ctx.theme.spacing.lg)
        .background(ctx.theme.colors.canvas_bg)
        .child(Desc::from(ui::widget(
            Cow::Borrowed("visual_audit_title"),
            LabelProps {
                text: Cow::Borrowed("Visual Audit"),
                variant: LabelVariant::Title,
                muted: false,
            },
        )))
        .child(Desc::from(ui::widget(
            Cow::Borrowed("visual_audit_caption"),
            LabelProps {
                text: Cow::Borrowed(
                    "Developer page: audit primitives first, then controls, panels, node cards, workspace.",
                ),
                variant: LabelVariant::Caption,
                muted: true,
            },
        )))
        .child(Desc::from(ui::widget(
            Cow::Borrowed("visual_audit_page_list"),
            ListViewProps {
                height: (ctx.viewport.h - 108.0).max(240.0),
                items,
            },
        )))
        .build()
}

fn section_card(theme: &Theme, section: VisualAuditSection) -> Desc {
    ui::column(format!("visual_audit_section_{}", section.id))
        .fill_width()
        .auto_height()
        .gap(theme.spacing.sm)
        .padding_all(theme.spacing.md)
        .background(theme.colors.surface)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(theme.radii.md.min(8.0))
        .child(Desc::from(ui::widget(
            Cow::Owned(format!("visual_audit_section_{}::title", section.id)),
            LabelProps {
                text: Cow::Borrowed(section.title),
                variant: LabelVariant::Title,
                muted: false,
            },
        )))
        .children(section.content)
        .build()
}
