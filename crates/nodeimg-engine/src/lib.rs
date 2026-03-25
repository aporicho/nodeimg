pub mod builtins;
pub(crate) mod internal;
pub mod transport;

// Re-export for backwards compatibility (app and other crates still use these paths)
pub use internal::ai_task;
pub use internal::backend;
pub use internal::cache;
pub use internal::eval;
pub use internal::menu;
pub use internal::registry;

pub use internal::cache::NodeId;
pub use internal::registry::{GpuProcessFn, NodeDef, NodeInstance, NodeRegistry, ProcessFn};
