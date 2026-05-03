use super::data::PanelFrameTemplateData;
use crate::control::templates::mount_control_node;
use crate::control::ControlMetrics;
use crate::template::{TemplateError, TemplateMountCx};
use crate::tree::NodeId;

pub(in crate::panel::retained) fn mount_panel_content(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<(), TemplateError> {
    let metrics = ControlMetrics::from_theme(&data.theme);
    for node in &data.content.nodes {
        mount_control_node(cx, parent, node, &data.theme, metrics)?;
    }
    Ok(())
}
