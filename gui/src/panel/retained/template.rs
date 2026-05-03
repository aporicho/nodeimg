use super::frame::mount_panel;
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateMountCx, TemplatePayload,
    TemplateRevision, PANEL_FRAME_TEMPLATE,
};
use crate::tree::NodeId;

const REVISION: TemplateRevision = TemplateRevision::new(2);

pub(crate) struct PanelFrameRetainedTemplate;

impl RetainedTemplate for PanelFrameRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(PANEL_FRAME_TEMPLATE)
    }

    fn revision(&self) -> TemplateRevision {
        REVISION
    }

    fn instantiate(
        &self,
        cx: &mut TemplateMountCx<'_>,
        _instance: &InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<NodeId, TemplateError> {
        let TemplatePayload::PanelFrame(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "panel frame template requires panel frame payload",
            });
        };
        mount_panel(cx, parent, &data)
    }
}
