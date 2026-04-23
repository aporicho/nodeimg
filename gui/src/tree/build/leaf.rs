use crate::icon::IconSpec;
use crate::renderer::{Color, PathData, PathStyle, Point, Stroke, TextStyle};
use crate::tree::layout::{BoxStyle, LeafKind, Size, TextLayout};
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

pub fn line(
    id: impl Into<Cow<'static, str>>,
    start: Point,
    end: Point,
    stroke: Stroke,
) -> LeafBuilder {
    leaf(id, LeafKind::Line { start, end, stroke })
}

pub fn curve(id: impl Into<Cow<'static, str>>, points: [Point; 4], stroke: Stroke) -> LeafBuilder {
    leaf(id, LeafKind::Curve { points, stroke })
}

pub fn path(id: impl Into<Cow<'static, str>>, data: PathData, style: PathStyle) -> LeafBuilder {
    leaf(id, LeafKind::Path { data, style })
}

pub fn icon(
    id: impl Into<Cow<'static, str>>,
    icon_id: impl Into<crate::icon::IconId>,
    size: f32,
    color: Color,
) -> LeafBuilder {
    icon_with_spec(id, IconSpec::new(icon_id, color), size)
}

pub fn icon_with_spec(id: impl Into<Cow<'static, str>>, spec: IconSpec, size: f32) -> LeafBuilder {
    let mut builder = leaf(id, LeafKind::Icon { spec });
    builder.style.width = Size::Fixed(size);
    builder.style.height = Size::Fixed(size);
    builder
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, PathStyle};

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn line_builder_uses_stroke_leaf_kind() {
        let stroke = Stroke::new(2.0, Color::WHITE);
        let desc = line("line", p(0.0, 1.0), p(2.0, 3.0), stroke).build();

        let Desc::Leaf { kind, .. } = desc else {
            panic!("line builder should create a leaf");
        };
        assert_eq!(
            kind,
            LeafKind::Line {
                start: p(0.0, 1.0),
                end: p(2.0, 3.0),
                stroke,
            }
        );
    }

    #[test]
    fn path_builder_uses_path_leaf_kind() {
        let data = PathData::line(p(0.0, 0.0), p(4.0, 0.0));
        let style = PathStyle::stroke(Stroke::new(1.0, Color::WHITE));
        let desc = path("path", data.clone(), style).build();

        let Desc::Leaf { kind, .. } = desc else {
            panic!("path builder should create a leaf");
        };
        assert_eq!(kind, LeafKind::Path { data, style });
    }

    #[test]
    fn icon_builder_sets_fixed_size_and_icon_spec() {
        let desc = icon("plus_icon", crate::icon::names::PLUS, 16.0, Color::WHITE).build();

        let Desc::Leaf { kind, style, .. } = desc else {
            panic!("icon builder should create a leaf");
        };
        assert_eq!(style.width, Size::Fixed(16.0));
        assert_eq!(style.height, Size::Fixed(16.0));
        let LeafKind::Icon { spec } = kind else {
            panic!("expected icon kind");
        };
        assert_eq!(spec.id, crate::icon::IconId::from(crate::icon::names::PLUS));
    }
}
