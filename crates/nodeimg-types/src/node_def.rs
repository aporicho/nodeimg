use crate::constraint::Constraint;
use crate::data_type::DataTypeId;
use crate::value::Value;
use crate::widget_id::WidgetId;

/// Describes an input or output pin.
#[derive(Clone, Debug)]
pub struct PinDef {
    pub name: String,
    pub data_type: DataTypeId,
    pub required: bool,
}

/// Describes a node parameter.
#[derive(Clone, Debug)]
pub struct ParamDef {
    pub name: String,
    pub data_type: DataTypeId,
    pub constraint: Constraint,
    pub default: Value,
    pub widget_override: Option<WidgetId>,
}
