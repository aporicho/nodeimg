use crate::renderer::Rect;
use crate::tree::{NodeId, Revision};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl LayoutConstraints {
    pub fn from_available(rect: Rect) -> Self {
        Self {
            min_width: 0.0,
            max_width: rect.w.max(0.0),
            min_height: 0.0,
            max_height: rect.h.max(0.0),
        }
    }

    pub fn key(self) -> LayoutConstraintsKey {
        LayoutConstraintsKey {
            min_width: LayoutScalar::from_f32(self.min_width),
            max_width: LayoutScalar::from_f32(self.max_width),
            min_height: LayoutScalar::from_f32(self.min_height),
            max_height: LayoutScalar::from_f32(self.max_height),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutInput {
    pub node: NodeId,
    pub constraints: LayoutConstraints,
    pub available_content_width: f32,
    pub layout_dependency_revision: Revision,
}

impl LayoutInput {
    pub fn key(self) -> LayoutCacheKey {
        LayoutCacheKey {
            node: self.node,
            constraints: self.constraints.key(),
            available_content_width: LayoutScalar::from_f32(self.available_content_width),
            layout_dependency_revision: self.layout_dependency_revision,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOutput {
    pub rect: Rect,
    pub content_rect: Rect,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
    pub baseline: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutScalar(i32);

impl LayoutScalar {
    const SCALE: f32 = 100.0;

    pub fn from_f32(value: f32) -> Self {
        if value.is_infinite() {
            return Self(i32::MAX);
        }
        if value.is_nan() {
            return Self(0);
        }
        Self(
            (value * Self::SCALE)
                .round()
                .clamp(i32::MIN as f32, i32::MAX as f32) as i32,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutConstraintsKey {
    pub min_width: LayoutScalar,
    pub max_width: LayoutScalar,
    pub min_height: LayoutScalar,
    pub max_height: LayoutScalar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LayoutCacheKey {
    pub node: NodeId,
    pub constraints: LayoutConstraintsKey,
    pub available_content_width: LayoutScalar,
    pub layout_dependency_revision: Revision,
}
