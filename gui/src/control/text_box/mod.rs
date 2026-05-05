mod diagnostics;
mod event;
mod ime;
mod intrinsic;
mod keyboard;
mod layout;
mod model;
mod mouse;
mod number;
mod output;
mod paint_override;
mod painter;
mod preedit;
mod registry;
mod retained_lookup;
mod retained_spec;
mod runtime;
mod store;
mod sync;
mod sync_item;
mod system;

pub(crate) use model::text_box_value_style;
pub use model::{format_number, TextBoxFont, TextBoxMode};
#[cfg(test)]
pub(crate) use model::{TextBoxSpec, TextBoxValueKind};
pub(crate) use paint_override::TextBoxPaintOverride;
pub(crate) use runtime::TextBoxRuntime;
pub(crate) use store::TextBoxStore;
pub use sync_item::ControlTextBoxSyncItem;
pub(crate) use system::TextBoxSystem;

#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;
