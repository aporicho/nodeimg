mod compiled;
mod generated;
mod id;
mod mount;
mod payload;
mod registry;
mod revision;
mod slots;
mod template;

pub(crate) use compiled::{BoundaryDeclarations, CompiledNode, CompiledTemplate};
pub use generated::{
    CANVAS_CONNECTION_TEMPLATE, CANVAS_GRID_TEMPLATE, CANVAS_NODE_CARD_TEMPLATE,
    CANVAS_PENDING_CONNECTION_TEMPLATE, CANVAS_ROOT_TEMPLATE, NODE_PALETTE_CATEGORY_TEMPLATE,
    NODE_PALETTE_EMPTY_TEMPLATE, NODE_PALETTE_ITEM_TEMPLATE, NODE_PALETTE_TEMPLATE,
    PANEL_FRAME_TEMPLATE, PARAM_CONTROL_TEMPLATE, TEXT_BOX_TEMPLATE, WORKSPACE_ROOT_TEMPLATE,
};
pub use id::{InstanceId, TemplateId};
pub(crate) use mount::TemplateMountCx;
pub use payload::TemplatePayload;
pub use registry::TemplateRegistry;
pub use revision::TemplateRevision;
pub use slots::{SlotBinding, SlotTarget, SlotValue, SlotValues, TemplateSlots};
pub(crate) use template::RetainedTemplate;
pub use template::{TemplateError, TemplateInstance};
