mod intrinsic;
mod layout;
mod resize_edge;
mod role;
mod spec;
mod system_context;
pub(crate) mod templates;
pub(crate) mod text_box;

pub use intrinsic::ControlIntrinsic;
pub use layout::{
    control_kind, control_layout_policy, control_list_min_height, control_min_height,
    control_node_min_height, ControlHeight, ControlKind, ControlLayoutPolicy, ControlMetrics,
};
#[cfg(test)]
pub(crate) use resize_edge::detect_resize_edge;
pub use resize_edge::{ResizeEdge, DEFAULT_RESIZE_EDGE_THRESHOLD};
pub use role::ControlRole;
pub use spec::{ControlNode, ControlSpec, ControlSpecMap};
pub(crate) use system_context::SystemCx;
pub(crate) use text_box::painter::paint_text_leaf_override;
pub use text_box::{format_number, TextBoxFont, TextBoxMode};
pub(crate) use text_box::{text_box_value_style, TextBoxStore, TextBoxSystem};
