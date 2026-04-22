use crate::tree::layout::{BoxStyle, Decoration, Direction};
use crate::tree::Desc;
use crate::widget::props::WidgetBuild;

#[derive(Default)]
pub struct WidgetBuildBuilder {
    pub(crate) style: BoxStyle,
    pub(crate) decoration: Option<Decoration>,
    pub(crate) children: Vec<Desc>,
}

pub fn root() -> WidgetBuildBuilder {
    WidgetBuildBuilder::default()
}

pub fn row() -> WidgetBuildBuilder {
    let mut builder = root();
    builder.style.direction = Direction::Row;
    builder
}

pub fn column() -> WidgetBuildBuilder {
    let mut builder = root();
    builder.style.direction = Direction::Column;
    builder
}

impl WidgetBuildBuilder {
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

    pub fn build(self) -> WidgetBuild {
        WidgetBuild {
            style: self.style,
            decoration: self.decoration,
            children: self.children,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Border, Color, TextStyle};
    use crate::tree::layout::{Align, Direction, Size};
    use crate::ui::{self, DecorationBuilder, StyleBuilder};

    #[test]
    fn root_builds_empty_widget_build() {
        let build = root().build();

        assert_eq!(build.style, BoxStyle::default());
        assert!(build.decoration.is_none());
        assert!(build.children.is_empty());
    }

    #[test]
    fn row_and_column_set_root_direction() {
        assert_eq!(row().build().style.direction, Direction::Row);
        assert_eq!(column().build().style.direction, Direction::Column);
    }

    #[test]
    fn style_methods_write_widget_build_style() {
        let build = row()
            .fixed_width(120.0)
            .fixed_height(24.0)
            .align_items(Align::Center)
            .gap(8.0)
            .build();

        assert_eq!(build.style.width, Size::Fixed(120.0));
        assert_eq!(build.style.height, Size::Fixed(24.0));
        assert_eq!(build.style.align_items, Align::Center);
        assert_eq!(build.style.gap, 8.0);
    }

    #[test]
    fn decoration_methods_write_widget_build_decoration() {
        let build = root()
            .background(Color::WHITE)
            .border(Border {
                width: 1.0,
                color: Color::BLACK,
            })
            .radius_all(6.0)
            .build();

        let decoration = build.decoration.expect("decoration");
        assert_eq!(decoration.background, Some(Color::WHITE));
        assert_eq!(decoration.border.expect("border").width, 1.0);
        assert_eq!(decoration.radius, [6.0; 4]);
    }

    #[test]
    fn children_accept_desc_and_ui_builders() {
        let build = column()
            .child(ui::container("container_child"))
            .child(ui::text(
                "text_child",
                "Text",
                TextStyle::new(Color::WHITE, 12.0),
            ))
            .build();

        assert_eq!(build.children.len(), 2);
    }
}
