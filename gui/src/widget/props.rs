use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration};
use crate::tree::Desc;
use std::any::Any;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetRole {
    #[default]
    Generic,
    Button,
    Checkbox,
    Collapsible,
    Dropdown,
    NumberInput,
    Panel,
    Radio,
    Slider,
    TextArea,
    TextInput,
    Toggle,
}

impl WidgetRole {
    pub fn is_focusable(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::Checkbox
                | Self::Collapsible
                | Self::Dropdown
                | Self::NumberInput
                | Self::Radio
                | Self::Slider
                | Self::TextArea
                | Self::TextInput
                | Self::Toggle
        )
    }

    pub fn is_panel(self) -> bool {
        matches!(self, Self::Panel)
    }

    pub fn is_text_input(self) -> bool {
        matches!(self, Self::TextInput)
    }

    pub fn is_text_area(self) -> bool {
        matches!(self, Self::TextArea)
    }
}

/// build() 的返回值。提供 Widget 节点的根样式、装饰和展开后的子树。
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}

#[derive(Clone, Copy)]
pub struct WidgetBuildCx<'a> {
    pub theme: &'a Theme,
    pub force_rebuild: bool,
}

/// Legacy `Desc` widget expansion trait.
///
/// Retained production code mounts `CompiledTemplate` instances and patches
/// state through `TreeMutation`; it must not call `build()` as glue.
pub trait WidgetProps: 'static {
    fn widget_type(&self) -> &'static str;
    fn role(&self) -> WidgetRole {
        WidgetRole::Generic
    }
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn WidgetProps>;
    fn props_eq(&self, other: &dyn WidgetProps) -> bool;
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild;
}

impl Clone for Box<dyn WidgetProps> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn WidgetProps> {
    fn eq(&self, other: &Self) -> bool {
        self.props_eq(other.as_ref())
    }
}

impl fmt::Debug for Box<dyn WidgetProps> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_fmt(f)
    }
}
