use crate::renderer::TextStyle;
use crate::tree::layout::{BoxStyle, LeafKind, TextLayout};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct LeafBuilder {
    pub(crate) id: Cow<'static, str>,
    pub(crate) style: BoxStyle,
    pub(crate) kind: LeafKind,
}

pub fn leaf(id: impl Into<Cow<'static, str>>, kind: LeafKind) -> LeafBuilder {
    LeafBuilder {
        id: id.into(),
        style: BoxStyle::default(),
        kind,
    }
}

pub fn text(
    id: impl Into<Cow<'static, str>>,
    content: impl Into<String>,
    style: TextStyle,
) -> LeafBuilder {
    text_with_layout(id, content, style, TextLayout::default())
}

pub fn text_with_layout(
    id: impl Into<Cow<'static, str>>,
    content: impl Into<String>,
    style: TextStyle,
    layout: TextLayout,
) -> LeafBuilder {
    leaf(
        id,
        LeafKind::Text {
            content: content.into(),
            style,
            layout,
        },
    )
}

impl LeafBuilder {
    pub fn build(self) -> Desc {
        self.into()
    }
}

impl From<LeafBuilder> for Desc {
    fn from(builder: LeafBuilder) -> Self {
        Desc::Leaf {
            id: builder.id,
            style: builder.style,
            kind: builder.kind,
        }
    }
}
