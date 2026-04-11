use types::DataType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExposedPinKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExposedPinSource {
    Pin,
    Param,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExposedPinDef {
    pub name: String,
    pub data_type: DataType,
    pub optional: bool,
    pub kind: ExposedPinKind,
    pub source: ExposedPinSource,
}
