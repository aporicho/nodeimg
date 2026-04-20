use types::NodeId;

use super::{GenerationId, OutputPin};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheError {
    HandleMemoryPressure {
        node_id: NodeId,
        output_pin: OutputPin,
        generation: GenerationId,
    },
}
