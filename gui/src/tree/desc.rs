use super::layout::{BoxStyle, Decoration, LeafKind};
use crate::widget::WidgetDesc;
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
    Widget(WidgetDesc),
}

impl Clone for Desc {
    fn clone(&self) -> Self {
        match self {
            Desc::Container {
                id,
                style,
                decoration,
                children,
            } => Desc::Container {
                id: id.clone(),
                style: style.clone(),
                decoration: decoration.clone(),
                children: children.clone(),
            },
            Desc::Leaf { id, style, kind } => Desc::Leaf {
                id: id.clone(),
                style: style.clone(),
                kind: kind.clone(),
            },
            Desc::Widget(widget) => Desc::Widget(widget.clone()),
        }
    }
}

impl Desc {
    pub fn id(&self) -> &str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Leaf { id, .. } => id,
            Desc::Widget(widget) => widget.id(),
        }
    }
}
