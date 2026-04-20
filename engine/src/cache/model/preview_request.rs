use types::NodeId;

use super::{ExecSignature, GenerationId, OutputPin};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreviewRequest {
    pub node_id: NodeId,
    pub output_pin: OutputPin,
    pub exec_signature: ExecSignature,
    pub generation: GenerationId,
}

impl PreviewRequest {
    pub fn new(
        node_id: NodeId,
        output_pin: impl Into<OutputPin>,
        exec_signature: ExecSignature,
        generation: GenerationId,
    ) -> Self {
        Self {
            node_id,
            output_pin: output_pin.into(),
            exec_signature,
            generation,
        }
    }
}
