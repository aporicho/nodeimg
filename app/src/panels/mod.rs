#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EnginePanelState {
    pub(crate) node_count: usize,
    pub(crate) connection_count: usize,
    pub(crate) node_def_count: usize,
    pub(crate) graph_version: u64,
    pub(crate) dirty: bool,
    pub(crate) execution_status: String,
    pub(crate) last_action: String,
}
