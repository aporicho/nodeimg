use types::{Constraint, DataType, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamExpose {
    Control,
    Input,
    Output,
}

/// 参数定义。
pub struct ParamDef {
    pub name: String,
    pub data_type: DataType,
    pub constraint: Option<Constraint>,
    pub default_value: Value,
    pub expose: Vec<ParamExpose>,
}
