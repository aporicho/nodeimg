use std::sync::Arc;

use crate::icon::{IconOpacity, IconSpec};
use crate::renderer::{Color, LineCap, PathCommand, Point, Rect};

use super::super::{SvgError, SvgSource, SvgUnsupportedFeature};
use super::model::SvgSize;
use super::{resolve_svg_icon_paths, SvgVectorCache};

fn source(svg: &str) -> SvgSource {
    SvgSource::new("test", Arc::<[u8]>::from(svg.as_bytes()))
}

fn rect() -> Rect {
    Rect {
        x: 10.0,
        y: 20.0,
        w: 48.0,
        h: 48.0,
    }
}

#[test]
fn svg_vector_cache_parses_path_segments() {
    let mut cache = SvgVectorCache::new();
    let source = source(
        r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M2 3L4 5Q6 7 8 9C10 11 12 13 14 15Z" fill="#ff0000"/></svg>"##,
    );

    let document = cache.get_or_parse(&source).expect("svg should parse");

    assert_eq!(
        document.source_size,
        SvgSize {
            width: 24.0,
            height: 24.0
        }
    );
    assert_eq!(
        document.paths[0].data.commands,
        vec![
            PathCommand::MoveTo(Point { x: 2.0, y: 3.0 }),
            PathCommand::LineTo(Point { x: 4.0, y: 5.0 }),
            PathCommand::QuadTo(Point { x: 6.0, y: 7.0 }, Point { x: 8.0, y: 9.0 }),
            PathCommand::CubicTo(
                Point { x: 10.0, y: 11.0 },
                Point { x: 12.0, y: 13.0 },
                Point { x: 14.0, y: 15.0 },
            ),
            PathCommand::Close,
        ]
    );
}

#[test]
fn svg_vector_cache_reuses_successful_documents() {
    let mut cache = SvgVectorCache::new();
    let source = source(
        r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0H24V24Z" fill="#fff"/></svg>"##,
    );

    let first = cache.get_or_parse(&source).expect("svg should parse");
    let second = cache.get_or_parse(&source).expect("svg should parse");

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn svg_icon_resolve_replaces_stroke_color_and_scales_width() {
    let mut cache = SvgVectorCache::new();
    let source = source(
        r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M6 12H18" fill="none" stroke="#000000" stroke-width="2" stroke-linecap="round"/></svg>"##,
    );
    let document = cache.get_or_parse(&source).expect("svg should parse");
    let style = IconSpec::new(
        "plus",
        Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 1.0,
        },
    )
    .style
    .with_opacity(IconOpacity::new(0.5));

    let paths = resolve_svg_icon_paths(&document, rect(), style);

    assert_eq!(paths.len(), 1);
    let stroke = paths[0].style.stroke.expect("stroke should resolve");
    assert_eq!(stroke.cap, LineCap::Round);
    assert_eq!(stroke.width, 4.0);
    assert_eq!(
        stroke.color,
        Color {
            r: 0.2,
            g: 0.4,
            b: 0.6,
            a: 0.5,
        }
    );
}

#[test]
fn unsupported_dasharray_returns_explicit_error() {
    let mut cache = SvgVectorCache::new();
    let source = source(
        r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0H24" fill="none" stroke="#fff" stroke-width="2" stroke-dasharray="2 2"/></svg>"##,
    );

    let err = cache
        .get_or_parse(&source)
        .expect_err("dasharray should not vectorize");

    assert!(matches!(
        err,
        SvgError::Unsupported(SvgUnsupportedFeature::DashArray)
    ));
}

#[test]
fn svg_vector_cache_reuses_unsupported_errors() {
    let mut cache = SvgVectorCache::new();
    let source = source(
        r##"<svg width="24" height="24" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M0 0H24" fill="none" stroke="#fff" stroke-dasharray="2 2"/></svg>"##,
    );

    let first = cache
        .get_or_parse(&source)
        .expect_err("dasharray should not vectorize");
    let second = cache
        .get_or_parse(&source)
        .expect_err("dasharray should not vectorize");

    assert_eq!(first, second);
}
