use crate::renderer::FillRule;

use super::super::{SvgError, SvgSource, SvgUnsupportedFeature};
use super::convert::{convert_fill, convert_fill_rule, convert_path_data, convert_stroke};
use super::model::{SvgSize, SvgVectorDocument, SvgVectorPath};

pub(super) fn parse_svg_vector(source: &SvgSource) -> Result<SvgVectorDocument, SvgError> {
    let tree = resvg::usvg::Tree::from_data(source.bytes(), &resvg::usvg::Options::default())
        .map_err(|err| SvgError::Parse(err.to_string()))?;
    if !tree.linear_gradients().is_empty() || !tree.radial_gradients().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GradientPaint));
    }
    if !tree.patterns().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::PatternPaint));
    }
    if !tree.clip_paths().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "clip_path",
        )));
    }
    if !tree.masks().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "mask",
        )));
    }
    if !tree.filters().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "filter",
        )));
    }
    let size = tree.size();
    let mut paths = Vec::new();
    collect_group(tree.root(), &mut paths)?;
    Ok(SvgVectorDocument {
        source_size: SvgSize {
            width: size.width(),
            height: size.height(),
        },
        paths,
    })
}

fn collect_group(
    group: &resvg::usvg::Group,
    paths: &mut Vec<SvgVectorPath>,
) -> Result<(), SvgError> {
    if group.opacity().get() < 0.999 {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "opacity",
        )));
    }
    if group.blend_mode() != resvg::usvg::BlendMode::Normal {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "blend_mode",
        )));
    }
    if group.isolate() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "isolation",
        )));
    }
    if group.clip_path().is_some() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "clip_path",
        )));
    }
    if group.mask().is_some() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "mask",
        )));
    }
    if !group.filters().is_empty() {
        return Err(SvgError::unsupported(SvgUnsupportedFeature::GroupEffect(
            "filter",
        )));
    }

    for child in group.children() {
        match child {
            resvg::usvg::Node::Group(group) => collect_group(group, paths)?,
            resvg::usvg::Node::Path(path) => {
                if let Some(path) = convert_path(path)? {
                    paths.push(path);
                }
            }
            resvg::usvg::Node::Image(_) => {
                return Err(SvgError::unsupported(SvgUnsupportedFeature::NonPathNode(
                    "image",
                )));
            }
            resvg::usvg::Node::Text(_) => {
                return Err(SvgError::unsupported(SvgUnsupportedFeature::NonPathNode(
                    "text",
                )));
            }
        }
    }

    Ok(())
}

fn convert_path(path: &resvg::usvg::Path) -> Result<Option<SvgVectorPath>, SvgError> {
    if !path.is_visible() {
        return Ok(None);
    }
    if path.paint_order() == resvg::usvg::PaintOrder::StrokeAndFill {
        return Err(SvgError::unsupported(
            SvgUnsupportedFeature::StrokePaintOrder,
        ));
    }

    Ok(Some(SvgVectorPath {
        data: convert_path_data(path.data(), path.abs_transform()),
        fill: match path.fill() {
            Some(fill) => Some(convert_fill(fill)?),
            None => None,
        },
        fill_rule: path
            .fill()
            .map(convert_fill_rule)
            .unwrap_or(FillRule::NonZero),
        stroke: match path.stroke() {
            Some(stroke) => Some(convert_stroke(stroke)?),
            None => None,
        },
    }))
}
