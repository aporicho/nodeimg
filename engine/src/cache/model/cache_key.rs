use types::NodeId;

use super::exec_signature::ExecSignature;

pub type OutputPin = String;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub node_id: NodeId,
    pub output_pin: OutputPin,
    pub exec_signature: ExecSignature,
}

impl CacheKey {
    pub fn new(
        node_id: NodeId,
        output_pin: impl Into<OutputPin>,
        exec_signature: ExecSignature,
    ) -> Self {
        Self {
            node_id,
            output_pin: output_pin.into(),
            exec_signature,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureKey {
    pub node_id: NodeId,
    pub output_pin: OutputPin,
}

impl TextureKey {
    pub fn new(node_id: NodeId, output_pin: impl Into<OutputPin>) -> Self {
        Self {
            node_id,
            output_pin: output_pin.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use types::NodeId;

    use crate::cache::model::ExecSignature;

    use super::{CacheKey, TextureKey};

    #[test]
    fn cache_key_distinguishes_signature() {
        let a = CacheKey::new(NodeId(1), "image", ExecSignature::new(1, 1, 10, 20));
        let b = CacheKey::new(NodeId(1), "image", ExecSignature::new(1, 1, 11, 20));

        assert_ne!(a, b);
    }

    #[test]
    fn texture_key_ignores_signature() {
        let a = TextureKey::new(NodeId(1), "image");
        let b = TextureKey::new(NodeId(1), "image");

        assert_eq!(a, b);
    }
}
