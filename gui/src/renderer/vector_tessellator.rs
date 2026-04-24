use std::collections::HashMap;

use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, LineCap as LyonLineCap,
    LineJoin as LyonLineJoin, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};

use crate::geometry::Affine2D;

use super::path::{PathCommand, PathRequest};
use super::path_geometry::{build_lyon_path, lyon_fill_rule};
use super::pipeline::vector::VectorVertex;
use super::style::{Fill, FillRule, LineCap, LineJoin, Stroke};
use super::types::{Color, Point};

#[derive(Default)]
pub struct VectorTessellator {
    cache: HashMap<PathCacheKey, CachedTessellation>,
}

impl VectorTessellator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_path_transformed(
        &mut self,
        req: &PathRequest,
        transform: Affine2D,
        vertices: &mut Vec<VectorVertex>,
        indices: &mut Vec<u32>,
    ) -> u32 {
        let cached = self.cached(req);
        if cached.indices.is_empty() {
            return 0;
        }

        let vertex_offset = vertices.len() as u32;
        let index_count = cached.indices.len() as u32;
        vertices.extend(cached.vertices.iter().map(|vertex| {
            let transformed = transform.transform_point(Point {
                x: vertex.position[0],
                y: vertex.position[1],
            });
            VectorVertex {
                position: [transformed.x, transformed.y],
                color: vertex.color,
            }
        }));
        indices.extend(cached.indices.iter().map(|idx| idx + vertex_offset));
        index_count
    }

    #[cfg(test)]
    pub(crate) fn cache_len(&self) -> usize {
        self.cache.len()
    }

    fn cached(&mut self, req: &PathRequest) -> &CachedTessellation {
        let key = PathCacheKey::from_request(req);
        self.cache.entry(key).or_insert_with(|| tessellate(req))
    }
}

struct CachedTessellation {
    vertices: Vec<VectorVertex>,
    indices: Vec<u32>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct PathCacheKey {
    commands: Vec<PathCommandKey>,
    fill: Option<FillKey>,
    stroke: Option<StrokeKey>,
}

impl PathCacheKey {
    fn from_request(req: &PathRequest) -> Self {
        Self {
            commands: req.data.commands.iter().map(PathCommandKey::from).collect(),
            fill: req.style.fill.map(FillKey::from),
            stroke: req.style.stroke.map(StrokeKey::from),
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum PathCommandKey {
    MoveTo(PointKey),
    LineTo(PointKey),
    QuadTo(PointKey, PointKey),
    CubicTo(PointKey, PointKey, PointKey),
    Close,
}

impl From<&PathCommand> for PathCommandKey {
    fn from(command: &PathCommand) -> Self {
        match *command {
            PathCommand::MoveTo(point) => Self::MoveTo(PointKey::from(point)),
            PathCommand::LineTo(point) => Self::LineTo(PointKey::from(point)),
            PathCommand::QuadTo(ctrl, to) => Self::QuadTo(PointKey::from(ctrl), PointKey::from(to)),
            PathCommand::CubicTo(ctrl1, ctrl2, to) => Self::CubicTo(
                PointKey::from(ctrl1),
                PointKey::from(ctrl2),
                PointKey::from(to),
            ),
            PathCommand::Close => Self::Close,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct PointKey {
    x: u32,
    y: u32,
}

impl From<Point> for PointKey {
    fn from(point: Point) -> Self {
        Self {
            x: point.x.to_bits(),
            y: point.y.to_bits(),
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct FillKey {
    color: ColorKey,
    rule: FillRule,
}

impl From<Fill> for FillKey {
    fn from(fill: Fill) -> Self {
        Self {
            color: ColorKey::from(fill.color),
            rule: fill.rule,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct StrokeKey {
    width: u32,
    color: ColorKey,
    cap: LineCap,
    join: LineJoin,
    miter_limit: u32,
}

impl From<Stroke> for StrokeKey {
    fn from(stroke: Stroke) -> Self {
        Self {
            width: stroke.width.to_bits(),
            color: ColorKey::from(stroke.color),
            cap: stroke.cap,
            join: stroke.join,
            miter_limit: stroke.miter_limit.to_bits(),
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct ColorKey {
    rgba: [u32; 4],
}

impl From<Color> for ColorKey {
    fn from(color: Color) -> Self {
        Self {
            rgba: [
                color.r.to_bits(),
                color.g.to_bits(),
                color.b.to_bits(),
                color.a.to_bits(),
            ],
        }
    }
}

fn tessellate(req: &PathRequest) -> CachedTessellation {
    let path = build_lyon_path(&req.data);
    let mut geometry: VertexBuffers<VectorVertex, u32> = VertexBuffers::new();

    if let Some(fill) = req.style.fill {
        tessellate_fill(&path, fill, &mut geometry);
    }

    if let Some(stroke) = req.style.stroke {
        if stroke.width > 0.0 {
            tessellate_stroke(&path, stroke, &mut geometry);
        }
    }

    CachedTessellation {
        vertices: geometry.vertices,
        indices: geometry.indices,
    }
}

fn tessellate_fill(path: &LyonPath, fill: Fill, geometry: &mut VertexBuffers<VectorVertex, u32>) {
    let color = fill.color.to_array();
    FillTessellator::new()
        .tessellate_path(
            path,
            &FillOptions::default().with_fill_rule(lyon_fill_rule(fill.rule)),
            &mut BuffersBuilder::new(geometry, |vertex: FillVertex| VectorVertex {
                position: vertex.position().to_array(),
                color,
            }),
        )
        .expect("failed to tessellate vector fill");
}

fn tessellate_stroke(
    path: &LyonPath,
    stroke: Stroke,
    geometry: &mut VertexBuffers<VectorVertex, u32>,
) {
    let color = stroke.color.to_array();
    StrokeTessellator::new()
        .tessellate_path(
            path,
            &stroke_options(stroke),
            &mut BuffersBuilder::new(geometry, |vertex: StrokeVertex| VectorVertex {
                position: vertex.position().to_array(),
                color,
            }),
        )
        .expect("failed to tessellate vector stroke");
}

fn stroke_options(stroke: Stroke) -> StrokeOptions {
    StrokeOptions::default()
        .with_line_width(stroke.width)
        .with_line_cap(lyon_line_cap(stroke.cap))
        .with_line_join(lyon_line_join(stroke.join))
        .with_miter_limit(stroke.miter_limit.max(StrokeOptions::MINIMUM_MITER_LIMIT))
}

fn lyon_line_cap(cap: LineCap) -> LyonLineCap {
    match cap {
        LineCap::Butt => LyonLineCap::Butt,
        LineCap::Round => LyonLineCap::Round,
        LineCap::Square => LyonLineCap::Square,
    }
}

fn lyon_line_join(join: LineJoin) -> LyonLineJoin {
    match join {
        LineJoin::Miter => LyonLineJoin::Miter,
        LineJoin::Round => LyonLineJoin::Round,
        LineJoin::Bevel => LyonLineJoin::Bevel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, Fill, PathData, PathStyle};

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn triangle(style: PathStyle) -> PathRequest {
        PathRequest {
            data: PathData::new()
                .move_to(p(0.0, 0.0))
                .line_to(p(10.0, 0.0))
                .line_to(p(10.0, 10.0))
                .close(),
            style,
        }
    }

    #[test]
    fn append_path_tessellates_fill_geometry() {
        let mut tessellator = VectorTessellator::new();
        let req = triangle(PathStyle::fill(Fill::non_zero(Color::WHITE)));
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let index_count = tessellator.append_path_transformed(
            &req,
            Affine2D::IDENTITY,
            &mut vertices,
            &mut indices,
        );

        assert!(index_count > 0);
        assert!(!vertices.is_empty());
        assert_eq!(indices.len(), index_count as usize);
    }

    #[test]
    fn append_path_reuses_cached_tessellation_for_same_request() {
        let mut tessellator = VectorTessellator::new();
        let req = triangle(PathStyle::stroke(Stroke::new(2.0, Color::WHITE)));
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let first_count = tessellator.append_path_transformed(
            &req,
            Affine2D::IDENTITY,
            &mut vertices,
            &mut indices,
        );
        let first_vertices = vertices.len();
        let first_indices = indices.len();
        let second_count = tessellator.append_path_transformed(
            &req,
            Affine2D::IDENTITY,
            &mut vertices,
            &mut indices,
        );

        assert_eq!(tessellator.cache_len(), 1);
        assert_eq!(second_count, first_count);
        assert_eq!(vertices.len(), first_vertices * 2);
        assert_eq!(indices.len(), first_indices * 2);
    }

    #[test]
    fn stroke_style_is_part_of_cache_key() {
        let mut tessellator = VectorTessellator::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let thin = triangle(PathStyle::stroke(Stroke::new(1.0, Color::WHITE)));
        let thick = triangle(PathStyle::stroke(Stroke::new(2.0, Color::WHITE)));

        tessellator.append_path_transformed(&thin, Affine2D::IDENTITY, &mut vertices, &mut indices);
        tessellator.append_path_transformed(
            &thick,
            Affine2D::IDENTITY,
            &mut vertices,
            &mut indices,
        );

        assert_eq!(tessellator.cache_len(), 2);
    }

    #[test]
    fn transformed_append_reuses_local_geometry_cache() {
        let mut tessellator = VectorTessellator::new();
        let req = triangle(PathStyle::fill(Fill::non_zero(Color::WHITE)));
        let mut first_vertices = Vec::new();
        let mut first_indices = Vec::new();
        let mut second_vertices = Vec::new();
        let mut second_indices = Vec::new();

        tessellator.append_path_transformed(
            &req,
            Affine2D::IDENTITY,
            &mut first_vertices,
            &mut first_indices,
        );
        tessellator.append_path_transformed(
            &req,
            Affine2D::translation(20.0, 30.0),
            &mut second_vertices,
            &mut second_indices,
        );

        assert_eq!(tessellator.cache_len(), 1);
        assert_eq!(first_indices, second_indices);
        assert_eq!(first_vertices.len(), second_vertices.len());
        assert!(first_vertices
            .iter()
            .zip(second_vertices.iter())
            .all(|(first, second)| {
                (second.position[0] - first.position[0] - 20.0).abs() < 1e-5
                    && (second.position[1] - first.position[1] - 30.0).abs() < 1e-5
            }));
    }
}
