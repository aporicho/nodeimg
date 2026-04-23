mod aliases;
mod asset;
mod builtin;
mod id;
mod name;
mod registry;
mod spec;

pub use builtin::names;
pub use id::IconId;
pub use name::IconName;
pub use spec::{IconFit, IconOpacity, IconPaintOverride, IconSpec, IconStrokeWidth, IconStyle};

pub(crate) use asset::IconAsset;
pub(crate) use registry::IconRegistry;
