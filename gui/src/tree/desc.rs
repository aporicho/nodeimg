use super::layout::{BoxStyle, Decoration, LeafKind};
use crate::widget::props::WidgetProps;
use std::borrow::Cow;

/// 视图描述：view() 函数返回的轻量描述树。
pub enum Desc {
    Container {
        id: Cow<'static, str>,
        style: BoxStyle,
        decoration: Option<Decoration>,
        children: Vec<Desc>,
    },
    Leaf {
        id: Cow<'static, str>,
        style: BoxStyle,
        kind: LeafKind,
    },
    Widget {
        id: Cow<'static, str>,
        props: Box<dyn WidgetProps>,
    },
    WidgetContainer {
        id: Cow<'static, str>,
        props: Box<dyn WidgetProps>,
        children: Vec<Desc>,
    },
}

impl Desc {
    pub fn id(&self) -> &str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Leaf { id, .. } => id,
            Desc::Widget { id, .. } | Desc::WidgetContainer { id, .. } => id,
        }
    }
}
