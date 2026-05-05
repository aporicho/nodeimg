mod interaction;
mod intrinsic;
mod kind;
pub(crate) mod kinds;
mod layout;
pub(crate) mod mount;
mod resize_edge;
mod role;
mod spec;
mod system_context;
pub(crate) mod text_box;
mod value;

pub(crate) use interaction::{ControlInteractionSpec, ControlInteractionSystem};
pub use intrinsic::ControlIntrinsic;
pub use kind::ControlKind;
pub use layout::{
    control_kind, control_layout_policy, control_list_min_height, control_min_height,
    control_node_min_height, ControlHeight, ControlLayoutPolicy, ControlMetrics,
};
#[cfg(test)]
pub(crate) use resize_edge::detect_resize_edge;
pub use resize_edge::{ResizeEdge, DEFAULT_RESIZE_EDGE_THRESHOLD};
pub use role::ControlRole;
pub use spec::{ControlNode, ControlSpec};
pub(crate) use system_context::SystemCx;
pub(crate) use text_box::painter::paint_text_leaf_override;
pub use text_box::{format_number, ControlTextBoxSyncItem, TextBoxFont, TextBoxMode};
pub(crate) use text_box::{text_box_value_style, TextBoxStore, TextBoxSystem};
pub use value::ControlValue;
