mod compiled;
mod generated;
mod id;
mod payload;
mod registry;
mod revision;
mod slots;
mod template;

pub use compiled::{BoundaryDeclarations, CompiledNode, CompiledNodeKind, CompiledTemplate};
pub use generated::{
    canvas_connection_template, canvas_grid_template, canvas_node_card_template,
    canvas_pending_connection_template, canvas_root_template, node_palette_category_template,
    node_palette_empty_template, node_palette_item_template, node_palette_template,
    panel_frame_template, param_control_template, register_builtin_templates, text_box_template,
    workspace_root_template, CANVAS_CONNECTION_TEMPLATE, CANVAS_GRID_TEMPLATE,
    CANVAS_NODE_CARD_TEMPLATE, CANVAS_PENDING_CONNECTION_TEMPLATE, CANVAS_ROOT_TEMPLATE,
    NODE_PALETTE_CATEGORY_TEMPLATE, NODE_PALETTE_EMPTY_TEMPLATE, NODE_PALETTE_ITEM_TEMPLATE,
    NODE_PALETTE_TEMPLATE, PANEL_FRAME_TEMPLATE, PARAM_CONTROL_TEMPLATE, TEXT_BOX_TEMPLATE,
    WORKSPACE_ROOT_TEMPLATE,
};
pub use id::{InstanceId, TemplateId};
pub use payload::TemplatePayload;
pub use registry::TemplateRegistry;
pub use revision::TemplateRevision;
pub use slots::{SlotBinding, SlotTarget, SlotValue, SlotValues, TemplateSlots};
pub use template::{RetainedTemplate, TemplateError, TemplateInstance};
