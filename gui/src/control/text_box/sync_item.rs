use crate::control::ControlSpec;

#[derive(Clone)]
pub struct ControlTextBoxSyncItem<'a> {
    pub control_id: String,
    pub control: &'a ControlSpec,
}

impl<'a> ControlTextBoxSyncItem<'a> {
    pub fn new(control_id: impl Into<String>, control: &'a ControlSpec) -> Self {
        Self {
            control_id: control_id.into(),
            control,
        }
    }
}
