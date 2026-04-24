use super::value::Lerp;
use crate::geometry::{Affine2D, Rect, TransformSpec};

const VISUAL_EPSILON: f32 = 0.0001;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AnimationProps {
    pub opacity: Option<f32>,
    pub translate: Option<[f32; 2]>,
    pub scale: Option<[f32; 2]>,
    pub rotate: Option<f32>,
}

impl AnimationProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn translate(mut self, translate: [f32; 2]) -> Self {
        self.translate = Some(translate);
        self
    }

    pub fn scale(mut self, scale: [f32; 2]) -> Self {
        self.scale = Some(scale);
        self
    }

    pub fn scale_uniform(self, scale: f32) -> Self {
        self.scale([scale, scale])
    }

    pub fn rotate(mut self, rotate: f32) -> Self {
        self.rotate = Some(rotate);
        self
    }

    pub fn is_empty(self) -> bool {
        self.opacity.is_none()
            && self.translate.is_none()
            && self.scale.is_none()
            && self.rotate.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimatedVisual {
    pub opacity: f32,
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotate: f32,
}

impl AnimatedVisual {
    pub const IDENTITY: Self = Self {
        opacity: 1.0,
        translate: [0.0, 0.0],
        scale: [1.0, 1.0],
        rotate: 0.0,
    };

    pub fn apply(&mut self, props: AnimationProps) {
        if let Some(opacity) = props.opacity {
            self.opacity = normalized_opacity(opacity);
        }
        if let Some(translate) = props.translate {
            self.translate = translate;
        }
        if let Some(scale) = props.scale {
            self.scale = scale;
        }
        if let Some(rotate) = props.rotate {
            self.rotate = rotate;
        }
    }

    pub fn sample_to(self, to: AnimationProps, t: f32) -> AnimationProps {
        AnimationProps {
            opacity: to.opacity.map(|value| self.opacity.lerp(value, t)),
            translate: to.translate.map(|value| self.translate.lerp(value, t)),
            scale: to.scale.map(|value| self.scale.lerp(value, t)),
            rotate: to.rotate.map(|value| self.rotate.lerp(value, t)),
        }
    }

    pub fn transform_is_identity(self) -> bool {
        near(self.translate[0], 0.0)
            && near(self.translate[1], 0.0)
            && near(self.scale[0], 1.0)
            && near(self.scale[1], 1.0)
            && near(self.rotate, 0.0)
    }

    pub fn is_identity(self) -> bool {
        near(self.opacity, 1.0) && self.transform_is_identity()
    }
}

impl Default for AnimatedVisual {
    fn default() -> Self {
        Self::IDENTITY
    }
}

pub fn compose_transform(
    base: Option<TransformSpec>,
    visual: Option<AnimatedVisual>,
) -> Option<TransformSpec> {
    let Some(visual) = visual else {
        return base;
    };
    if visual.transform_is_identity() {
        return base;
    }

    let mut transform = base.unwrap_or_default();
    transform.translate[0] += visual.translate[0];
    transform.translate[1] += visual.translate[1];
    transform.scale[0] *= visual.scale[0];
    transform.scale[1] *= visual.scale[1];
    transform.rotate += visual.rotate;
    Some(transform)
}

pub fn visual_affine(rect: Rect, visual: AnimatedVisual) -> Option<Affine2D> {
    if visual.transform_is_identity() {
        return None;
    }

    let local_rect = Rect {
        x: 0.0,
        y: 0.0,
        w: rect.w,
        h: rect.h,
    };
    let visual_transform = TransformSpec {
        translate: visual.translate,
        scale: visual.scale,
        rotate: visual.rotate,
        ..TransformSpec::default()
    }
    .to_affine(local_rect);
    Some(Affine2D::compose(
        Affine2D::translation(rect.x, rect.y),
        Affine2D::compose(visual_transform, Affine2D::translation(-rect.x, -rect.y)),
    ))
}

fn normalized_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= VISUAL_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_builder_sets_partial_fields() {
        let props = AnimationProps::new()
            .opacity(0.5)
            .translate([2.0, 3.0])
            .scale_uniform(2.0)
            .rotate(0.25);

        assert_eq!(props.opacity, Some(0.5));
        assert_eq!(props.translate, Some([2.0, 3.0]));
        assert_eq!(props.scale, Some([2.0, 2.0]));
        assert_eq!(props.rotate, Some(0.25));
    }

    #[test]
    fn visual_clamps_opacity() {
        let mut visual = AnimatedVisual::default();
        visual.apply(AnimationProps::new().opacity(2.0));

        assert_eq!(visual.opacity, 1.0);
    }

    #[test]
    fn compose_transform_merges_visual_transform() {
        let merged = compose_transform(
            Some(TransformSpec::translate_scale([4.0, 5.0], 2.0)),
            Some(AnimatedVisual {
                translate: [10.0, -2.0],
                scale: [0.5, 3.0],
                rotate: 0.25,
                ..AnimatedVisual::default()
            }),
        )
        .expect("merged transform");

        assert_eq!(merged.translate, [14.0, 3.0]);
        assert_eq!(merged.scale, [1.0, 6.0]);
        assert_eq!(merged.rotate, 0.25);
    }

    #[test]
    fn visual_affine_transforms_around_node_origin() {
        let affine = visual_affine(
            Rect {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
            },
            AnimatedVisual {
                scale: [2.0, 2.0],
                ..AnimatedVisual::default()
            },
        )
        .expect("affine");

        assert_eq!(
            affine.transform_point(crate::geometry::Point { x: 10.0, y: 20.0 }),
            crate::geometry::Point { x: 10.0, y: 20.0 }
        );
        assert_eq!(
            affine.transform_point(crate::geometry::Point { x: 20.0, y: 20.0 }),
            crate::geometry::Point { x: 30.0, y: 20.0 }
        );
    }
}
