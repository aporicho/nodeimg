pub(crate) mod builtins;
pub(crate) mod internal;
pub mod transport;

// Crate-internal short aliases so builtins can `use crate::registry::…`
pub(crate) use internal::cache;
pub(crate) use internal::registry;

// Only export types that app layer still needs directly
pub use internal::cache::NodeId;
pub use internal::registry::{NodeInstance, NodeRegistry};

// Hidden re-exports for integration tests (tests/ dir is outside the crate)
#[doc(hidden)]
pub mod _test_support {
    pub use crate::builtins::register_all;
    pub use crate::internal::cache::Cache;
    pub use crate::internal::eval::{Connection, EvalEngine};
    pub use crate::internal::registry::NodeDef;
}
