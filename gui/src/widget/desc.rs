use std::borrow::Cow;
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use super::node::NodeKind;
use super::props::WidgetProps;

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
}

impl Desc {
    pub fn id(&self) -> &str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Leaf { id, .. } => id,
            Desc::Widget { id, .. } => id,
        }
    }

    /// 消费 Desc，返回 (id, 子描述列表, style, decoration, NodeKind)。
    pub(crate) fn into_parts(self) -> (Cow<'static, str>, Vec<Desc>, BoxStyle, Option<Decoration>, NodeKind) {
        match self {
            Desc::Container { id, style, decoration, children } => {
                (id, children, style, decoration, NodeKind::Container)
            }
            Desc::Leaf { id, style, kind } => {
                (id, Vec::new(), style, None, NodeKind::Leaf(kind))
            }
            Desc::Widget { .. } => {
                unimplemented!("Widget desc should be expanded via build() before reconcile")
            }
        }
    }

}
