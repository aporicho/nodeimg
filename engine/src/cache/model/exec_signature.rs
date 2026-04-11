#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecSignature {
    pub sig_schema_version: u16,
    pub node_version: u16,
    pub params_hash: u64,
    pub upstream_hash: u64,
}

impl ExecSignature {
    pub fn new(
        sig_schema_version: u16,
        node_version: u16,
        params_hash: u64,
        upstream_hash: u64,
    ) -> Self {
        Self {
            sig_schema_version,
            node_version,
            params_hash,
            upstream_hash,
        }
    }
}
