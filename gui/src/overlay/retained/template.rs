use super::overlay::mount_dropdown_overlay;
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateMountCx, TemplatePayload,
    TemplateRevision, DROPDOWN_OVERLAY_TEMPLATE,
};
use crate::tree::NodeId;

const REVISION: TemplateRevision = TemplateRevision::new(1);

pub(crate) struct DropdownOverlayRetainedTemplate;

impl RetainedTemplate for DropdownOverlayRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(DROPDOWN_OVERLAY_TEMPLATE)
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
        let TemplatePayload::DropdownOverlay(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "dropdown overlay template requires dropdown overlay payload",
            });
        };
        mount_dropdown_overlay(cx, parent, &data)
    }
}
