#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecSignature {
    pub sig_schema_version: u16,
    pub node_version: u16,
    pub params_hash: u64,
    pub upstream_hash: u64,
    pub cooking_context_hash: u64,
    pub capability_version: u32,
}

impl ExecSignature {
    pub fn new(
        sig_schema_version: u16,
        node_version: u16,
        params_hash: u64,
        upstream_hash: u64,
    ) -> Self {
        Self::with_context(
            sig_schema_version,
            node_version,
            params_hash,
            upstream_hash,
            0,
            0,
        )
    }

    pub fn with_context(
        sig_schema_version: u16,
        node_version: u16,
        params_hash: u64,
        upstream_hash: u64,
        cooking_context_hash: u64,
        capability_version: u32,
    ) -> Self {
        Self {
            sig_schema_version,
            node_version,
            params_hash,
            upstream_hash,
            cooking_context_hash,
            capability_version,
        }
    }
}
