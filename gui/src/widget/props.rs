use std::any::Any;
use std::fmt;
use crate::tree::Desc;
use crate::tree::layout::{BoxStyle, Decoration};

/// build() 的返回值。提供 Widget 节点的根样式、装饰和展开后的子树。
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}

/// 控件配置 trait。每种控件实现此 trait。
pub trait WidgetProps: 'static {
    fn widget_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn WidgetProps>;
    fn props_eq(&self, other: &dyn WidgetProps) -> bool;
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
    fn build(&self, id: &str) -> WidgetBuild;
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
