use crate::renderer::{PathData, PathStyle, Point, Stroke, TextStyle};
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
}
