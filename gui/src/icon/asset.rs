use crate::renderer::svg::SvgSource;

#[derive(Debug, Clone)]
pub(crate) struct IconAsset {
    pub(crate) source: SvgSource,
}

impl IconAsset {
    pub(crate) fn svg(source: SvgSource) -> Self {
        Self { source }
    }
}
