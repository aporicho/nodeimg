#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SvgError {
    Parse(String),
    Unsupported(SvgUnsupportedFeature),
}

impl SvgError {
    pub(crate) fn unsupported(feature: SvgUnsupportedFeature) -> Self {
        Self::Unsupported(feature)
    }

    pub(crate) fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SvgUnsupportedFeature {
    NonPathNode(&'static str),
    GroupEffect(&'static str),
    GradientPaint,
    PatternPaint,
    DashArray,
    StrokePaintOrder,
}
