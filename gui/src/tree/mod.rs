pub mod build;
mod connection_endpoint;
mod desc;
mod diff;
mod dirty;
mod frame_stats;
mod hit;
mod hit_order;
mod hit_shape;
mod id;
mod index;
mod interaction_hit;
pub mod layout;
mod layout_adapter;
pub(crate) mod legacy_desc;
mod mutation;
mod node;
mod paint;
mod paint_cache;
pub(crate) mod paint_helpers;
mod paint_order;
mod paint_space;
pub mod paint_target;
mod props;
mod repaint;
mod retained_runtime;
mod revision;
mod runtime_policy;
mod runtime_slots;
mod scroll;
mod shape;
mod stacking;
pub(crate) mod text_layout;
#[allow(clippy::module_inception)]
mod tree;

pub use desc::Desc;
pub use dirty::{DirtyFlags, DirtyQueues};
pub use frame_stats::FrameStats;
pub(crate) use hit::screen_to_node_layout_point;
pub use hit::{hit_test, hit_test_with_animations, HitChain};
pub use id::{NodeId, StableId, TreeNodeId};
pub use index::{TreeIndex, TreeIndexError};
pub(crate) use interaction_hit::{resize_hit_at_screen_point, ResizeHit};
pub use layout::layout;
#[cfg(test)]
pub use legacy_desc::reconcile;
pub use mutation::{Invalidation, MutationError, StylePatch, TreeMutation};
pub use node::{
    AnimationRuntime, LayoutRuntime, NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodePaintMeta,
    TreeNode,
};
#[cfg(test)]
pub(crate) use paint::build_display_list;
pub(crate) use paint::PaintCx;
pub use paint_cache::{PaintCache, PaintCacheError};
pub use props::NodeProps;
pub use repaint::{PaintDirtyQueues, PaintDirtyReason, RepaintBoundaryId, RepaintBoundaryReason};
pub(crate) use retained_runtime::RetainedRuntimeStore;
pub use revision::Revision;
pub use runtime_policy::{PersistenceClass, RuntimeRetention, RuntimeSlotPolicy, UndoClass};
pub use runtime_slots::{RuntimeSlot, RuntimeSlots};
pub use tree::Tree;
