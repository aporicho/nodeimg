use crate::tree::layout::{BoxStyle, Decoration, Direction};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Clone)]
pub struct ContainerBuilder {
    pub(crate) id: Cow<'static, str>,
    pub(crate) style: BoxStyle,
    pub(crate) decoration: Option<Decoration>,
    pub(crate) children: Vec<Desc>,
}

pub fn container(id: impl Into<Cow<'static, str>>) -> ContainerBuilder {
    ContainerBuilder {
        id: id.into(),
        style: BoxStyle::default(),
        decoration: None,
        children: Vec::new(),
    }
}

pub fn row(id: impl Into<Cow<'static, str>>) -> ContainerBuilder {
    let mut builder = container(id);
    builder.style.direction = Direction::Row;
    builder
}

pub fn column(id: impl Into<Cow<'static, str>>) -> ContainerBuilder {
    let mut builder = container(id);
    builder.style.direction = Direction::Column;
    builder
}

impl ContainerBuilder {
    pub fn child(mut self, child: impl Into<Desc>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children<I, T>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Desc>,
    {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    pub fn extend_children<I, T>(self, children: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Desc>,
    {
        self.children(children)
    }

    pub fn build(self) -> Desc {
        self.into()
    }

    pub fn build_decoration(self) -> Option<Decoration> {
        self.decoration
    }
}

impl From<ContainerBuilder> for Desc {
    fn from(builder: ContainerBuilder) -> Self {
        Desc::Container {
            id: builder.id,
            style: builder.style,
            decoration: builder.decoration,
            children: builder.children,
        }
    }
}
