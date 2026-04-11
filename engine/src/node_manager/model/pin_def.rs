use types::DataType;

/// 引脚定义（输入/输出）。
pub struct PinDef {
    pub name: String,
    pub data_type: DataType,
    pub optional: bool,
}
