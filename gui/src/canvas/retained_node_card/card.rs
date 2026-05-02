use super::node_factory::container;
use crate::canvas::node_spec::NodeRenderSpec;
use crate::canvas::node_style::NodeCardMetrics;
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, Overflow, Position, Size,
};
use crate::tree::NodeId;

pub(super) fn mount_card(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    spec: &NodeRenderSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<NodeId, TemplateError> {
    cx.child(
        parent,
        container(
            spec.card_id.clone(),
            BoxStyle {
                position: Position::relative(),
                width: Size::Fixed(metrics.card_width),
                height: Size::Fixed(metrics.card_height),
                padding: Edges::all(metrics.card_padding),
                direction: Direction::Column,
                gap: metrics.row_gap,
                align_items: Align::Start,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                hittable: true,
                resizable: true,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: if spec.selected { 2.0 } else { 1.0 },
                    color: if spec.selected {
                        theme.colors.text
                    } else {
                        theme.colors.border
                    },
                }),
                radius: [metrics.card_radius; 4],
                shadow: None,
            }),
        ),
    )
}
