use crate::tree::Desc;
use crate::widget::props::WidgetProps;
use crate::widget::WidgetDesc;
use std::borrow::Cow;

pub struct WidgetBuilder {
    id: Cow<'static, str>,
    props: Box<dyn WidgetProps>,
    children: Vec<Desc>,
}

pub fn widget(id: impl Into<Cow<'static, str>>, props: impl WidgetProps) -> WidgetBuilder {
    WidgetBuilder {
        id: id.into(),
        props: Box::new(props),
        children: Vec::new(),
    }
}

impl WidgetBuilder {
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
}

impl From<WidgetBuilder> for Desc {
    fn from(builder: WidgetBuilder) -> Self {
        Desc::Widget(WidgetDesc::from_boxed(
            builder.id,
            builder.props,
            builder.children,
        ))
    }
}
