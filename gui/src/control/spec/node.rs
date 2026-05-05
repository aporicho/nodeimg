use super::model::ControlSpec;

#[derive(Clone, Debug, PartialEq)]
pub struct ControlNode {
    id: String,
    spec: ControlSpec,
}

impl ControlNode {
    pub fn new(id: impl Into<String>, spec: ControlSpec) -> Self {
        Self {
            id: id.into(),
            spec,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn spec(&self) -> &ControlSpec {
        &self.spec
    }
}
