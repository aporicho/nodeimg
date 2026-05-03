use std::sync::Arc;

use crate::geometry::{Affine2D, Point};
use crate::paint::{
    ClipShape, Color, DisplayList, DisplayListBuilder, Fill, GridPaint, ImagePaint, LayerPaint,
    PaintCommand, PathData, PathPaint, RectPaint, RectStyle, ShadowPaint, SvgPaint, SvgRasterPaint,
    SvgSourceKey, SvgStyle, TextPaint, TextureHandle,
};
use crate::renderer::command::BackendCommand;
use crate::renderer::display_resources::{DisplayResourceResolver, EmptyDisplayResources};
use crate::renderer::svg::{SvgSource, SvgVectorCache};
use crate::renderer::{PathStyle, Rect, Shadow, Stroke, TextStyle, TextureResource};

use super::api::{DisplayBackendOutput, UnsupportedDisplayCommand, UnsupportedDisplayReason};
use super::lower_display_list;

fn rect() -> Rect {
    Rect {
        x: 1.0,
        y: 2.0,
        w: 3.0,
        h: 4.0,
    }
}

fn rect_style() -> RectStyle {
    RectStyle {
        color: Color::WHITE,
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}

fn lower(list: &DisplayList) -> DisplayBackendOutput {
    let mut svg_cache = SvgVectorCache::new();
    lower_display_list(list, &EmptyDisplayResources, &mut svg_cache)
}

struct SvgOnlyResources {
    source: SvgSource,
}

impl DisplayResourceResolver for SvgOnlyResources {
    fn texture(&self, _handle: TextureHandle) -> Option<TextureResource> {
        None
    }

    fn svg_source(&self, _key: &SvgSourceKey) -> Option<SvgSource> {
        Some(self.source.clone())
    }
}

#[test]
fn lowers_rect_with_translate_scale() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0));
    builder.push_transform(transform);
    builder.draw(PaintCommand::Rect(RectPaint {
        rect: rect(),
        style: rect_style(),
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::Rect(req) => {
            assert_eq!(req.rect, rect());
            assert_eq!(req.transform, transform);
        }
        _ => panic!("expected rect command"),
    }
}

#[test]
fn lowers_grid_to_single_backend_command() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::translation(10.0, 20.0);
    let paint = GridPaint {
        rect: rect(),
        spacing: 8.0,
        dot_color: Color::WHITE,
        dot_size: 1.0,
    };
    builder.push_transform(transform);
    builder.draw(PaintCommand::Grid(paint));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert_eq!(output.commands.len(), 1);
    match &output.commands[0] {
        BackendCommand::Grid(req) => {
            assert_eq!(req.paint, paint);
            assert_eq!(req.transform, transform);
        }
        _ => panic!("expected grid command"),
    }
}

#[test]
fn lowers_rotated_rect_as_affine_command() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::rotation_radians(0.5);
    builder.push_transform(transform);
    builder.draw(PaintCommand::Rect(RectPaint {
        rect: rect(),
        style: rect_style(),
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::Rect(req) => {
            assert_eq!(req.rect, rect());
            assert_eq!(req.transform, transform);
        }
        _ => panic!("expected rect command"),
    }
}

#[test]
fn lowers_path_with_local_geometry_and_affine_transform() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::scale(3.0);
    builder.push_transform(transform);
    builder.draw(PaintCommand::Path(PathPaint {
        data: PathData::line(Point { x: 1.0, y: 1.0 }, Point { x: 2.0, y: 1.0 }),
        style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    match &output.commands[0] {
        BackendCommand::Path(req) => {
            assert_eq!(req.style.stroke.unwrap().width, 2.0);
            assert_eq!(req.transform, transform);
            assert_eq!(
                req.data.commands[0],
                crate::paint::PathCommand::MoveTo(Point { x: 1.0, y: 1.0 })
            );
        }
        _ => panic!("expected path command"),
    }
}

#[test]
fn lowers_clip_stack_to_push_and_pop_commands() {
    let mut builder = DisplayListBuilder::new();
    builder.push_clip(ClipShape::Rect(rect()));
    builder.draw(PaintCommand::Rect(RectPaint {
        rect: rect(),
        style: rect_style(),
    }));
    builder.pop_clip();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    match &output.commands[0] {
        BackendCommand::PushClip(req) => {
            assert_eq!(req.shape, ClipShape::Rect(rect()));
            assert_eq!(req.transform, Affine2D::IDENTITY);
        }
        _ => panic!("expected push clip command"),
    }
    assert!(matches!(output.commands[1], BackendCommand::Rect(_)));
    assert!(matches!(output.commands[2], BackendCommand::PopClip));
}

#[test]
fn reports_missing_texture() {
    let mut builder = DisplayListBuilder::new();
    builder.draw(PaintCommand::Image(ImagePaint {
        rect: rect(),
        texture: TextureHandle(99),
        style: Default::default(),
    }));
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert_eq!(
        output.report.unsupported,
        vec![UnsupportedDisplayCommand {
            index: 0,
            reason: UnsupportedDisplayReason::MissingTexture(TextureHandle(99)),
        }]
    );
}

#[test]
fn reports_missing_svg_source() {
    let mut builder = DisplayListBuilder::new();
    builder.draw(PaintCommand::Svg(SvgPaint {
        rect: rect(),
        source: SvgSourceKey::new("missing"),
        style: SvgStyle::default(),
    }));
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert_eq!(
        output.report.unsupported,
        vec![UnsupportedDisplayCommand {
            index: 0,
            reason: UnsupportedDisplayReason::MissingSvgSource("missing".to_string()),
        }]
    );
}

#[test]
fn lowers_layer_content_with_composed_transform_and_opacity() {
    let mut inner = DisplayListBuilder::new();
    inner.push_transform(Affine2D::translation(3.0, 4.0));
    inner.draw(PaintCommand::Rect(RectPaint {
        rect: rect(),
        style: rect_style(),
    }));
    inner.pop_transform();

    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::rotation_radians(0.25);
    builder.push_transform(transform);
    builder.draw(PaintCommand::Layer(LayerPaint::new(
        rect(),
        0.5,
        inner.finish().unwrap(),
    )));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::Rect(req) => {
            assert_eq!(
                req.transform,
                Affine2D::compose(transform, Affine2D::translation(3.0, 4.0))
            );
            assert_eq!(req.style.color.a, 0.5);
        }
        _ => panic!("expected rect command"),
    }
}

#[test]
fn lowers_filled_path_to_backend_command() {
    let mut builder = DisplayListBuilder::new();
    builder.draw(PaintCommand::Path(PathPaint {
        data: PathData::new()
            .move_to(Point { x: 0.0, y: 0.0 })
            .line_to(Point { x: 10.0, y: 0.0 })
            .line_to(Point { x: 10.0, y: 10.0 })
            .close(),
        style: PathStyle::fill(Fill::non_zero(Color::WHITE)),
    }));
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(matches!(output.commands[0], BackendCommand::Path(_)));
}

#[test]
fn lowers_rotated_text_to_affine_backend_command() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::rotation_radians(0.25);
    builder.push_transform(transform);
    builder.draw(PaintCommand::Text(TextPaint {
        pos: Point { x: 1.0, y: 2.0 },
        text: "hello".to_string(),
        style: TextStyle::new(Color::WHITE, 12.0),
        bounds: Some(rect()),
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::Text(req) => {
            assert_eq!(req.pos, Point { x: 1.0, y: 2.0 });
            assert_eq!(req.transform, transform);
            assert_eq!(req.style.size, 12.0);
        }
        _ => panic!("expected text command"),
    }
}

#[test]
fn lowers_rotated_shadow_to_affine_backend_command() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::rotation_radians(0.25);
    builder.push_transform(transform);
    builder.draw(PaintCommand::Shadow(ShadowPaint {
        rect: rect(),
        radius: [2.0; 4],
        shadow: Shadow {
            color: Color::BLACK,
            offset: [1.0, 2.0],
            blur: 3.0,
            spread: 4.0,
        },
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::Shadow(req) => {
            assert_eq!(req.rect, rect());
            assert_eq!(req.radius, [2.0; 4]);
            assert_eq!(req.transform, transform);
        }
        _ => panic!("expected shadow command"),
    }
}

#[test]
fn lowers_rotated_svg_raster_to_affine_backend_command() {
    let mut builder = DisplayListBuilder::new();
    let transform = Affine2D::rotation_radians(0.25);
    builder.push_transform(transform);
    builder.draw(PaintCommand::SvgRaster(SvgRasterPaint {
        rect: rect(),
        source: SvgSourceKey::new("test-icon"),
        color: Some(Color::WHITE),
    }));
    builder.pop_transform();
    let list = builder.finish().unwrap();
    let mut svg_cache = SvgVectorCache::new();
    let resources = SvgOnlyResources {
        source: SvgSource::new("test-icon", Arc::<[u8]>::from(&b"<svg/>"[..])),
    };

    let output = lower_display_list(&list, &resources, &mut svg_cache);

    assert!(output.report.unsupported.is_empty());
    match &output.commands[0] {
        BackendCommand::SvgRaster(req) => {
            assert_eq!(req.rect, rect());
            assert_eq!(req.transform, transform);
        }
        _ => panic!("expected svg raster command"),
    }
}

#[test]
fn display_lowering_trace_counts_command_kinds() {
    let mut builder = DisplayListBuilder::new();
    builder.draw(PaintCommand::Rect(RectPaint {
        rect: rect(),
        style: rect_style(),
    }));
    builder.draw(PaintCommand::Text(TextPaint {
        pos: Point { x: 1.0, y: 2.0 },
        text: "hello".to_string(),
        style: TextStyle::new(Color::WHITE, 12.0),
        bounds: Some(rect()),
    }));
    let list = builder.finish().unwrap();

    let output = lower(&list);

    assert_eq!(output.report.stats.input_commands, 2);
    assert_eq!(output.report.stats.backend_commands, 2);
    assert_eq!(output.report.stats.command_kinds.rect, 1);
    assert_eq!(output.report.stats.command_kinds.text, 1);
    assert_eq!(output.report.stats.unsupported, 0);
}
