use super::mount::mount_node_card;
use crate::canvas::node_spec::node_render_spec;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateMountCx, TemplatePayload,
    TemplateRevision, CANVAS_NODE_CARD_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::NodeId;

const REVISION: TemplateRevision = TemplateRevision::new(2);

#[derive(Clone, Debug)]
pub struct CanvasNodeCardTemplateData {
    view: CanvasNodeRenderView,
    theme: Theme,
}

impl CanvasNodeCardTemplateData {
    pub fn new(view: CanvasNodeRenderView, theme: &Theme) -> Self {
        Self {
            view,
            theme: theme.clone(),
        }
    }
}

pub struct CanvasNodeCardRetainedTemplate;

impl RetainedTemplate for CanvasNodeCardRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(CANVAS_NODE_CARD_TEMPLATE)
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
        let TemplatePayload::CanvasNodeCard(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "canvas node card template requires canvas node card payload",
            });
        };
        let spec = node_render_spec(&data.view.template, &data.view.state, &data.theme);
        mount_node_card(cx, parent, &spec, &data.theme)
    }
}
