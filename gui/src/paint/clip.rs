use crate::geometry::{Affine2D, Rect};

use super::{FillRule, PathData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub enum ClipShape {
    Rect(Rect),
    RoundedRect {
        rect: Rect,
        /// Per-corner radii in local paint units.
        radius: [f32; 4],
    },
    Path {
        data: PathData,
        fill_rule: FillRule,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedClip {
    pub id: ClipId,
    pub shape: ClipShape,
    pub transform: Affine2D,
    pub parent: Option<ClipId>,
}

impl ClipShape {
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::Rect(rect) => Some(*rect),
            Self::RoundedRect { rect, .. } => Some(*rect),
            Self::Path { data, .. } => data.bounds(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn rect_clip_bounds_are_rect() {
        let rect = Rect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        };

        assert_eq!(ClipShape::Rect(rect).bounds(), Some(rect));
    }

    #[test]
    fn path_clip_bounds_use_path_bounds() {
        let clip = ClipShape::Path {
            data: PathData::line(Point { x: 1.0, y: 2.0 }, Point { x: 4.0, y: 6.0 }),
            fill_rule: FillRule::NonZero,
        };

        assert_eq!(
            clip.bounds(),
            Some(Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            })
        );
    }
}
