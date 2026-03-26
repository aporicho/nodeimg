use std::collections::HashMap;

use crate::value::Value;

/// Runtime node instance stored in the graph.
#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub type_id: String,
    pub params: HashMap<String, Value>,
}
