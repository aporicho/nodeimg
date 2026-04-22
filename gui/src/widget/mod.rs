pub mod anatomy;
pub mod atoms;
pub mod desc;
pub mod frameworks;
pub mod props;

mod mapping;
pub(crate) mod painters;
pub mod resize_edge;
pub mod state;
pub(crate) mod systems;
pub(crate) mod text_edit;

pub use desc::WidgetDesc;
pub(crate) use text_edit::TextEditState;
