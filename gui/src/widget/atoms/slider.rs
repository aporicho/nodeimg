use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    pub label: Cow<'static, str>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub disabled: bool,
}

impl WidgetProps for SliderProps {
    fn widget_type(&self) -> &'static str { "Slider" }
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn WidgetProps> { Box::new(self.clone()) }
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |o| self == o)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, _id: &str) -> WidgetBuild {
        todo!("SliderProps::build")
    }
}
