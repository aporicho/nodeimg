use crate::tree::RuntimeSlot;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DropdownRuntime {
    pub(crate) open: Option<OpenDropdown>,
}

impl RuntimeSlot for DropdownRuntime {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenDropdown {
    pub(crate) id: String,
    pub(crate) highlighted: usize,
}
