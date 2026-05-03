use crate::geometry::Affine2D;
use crate::paint::{
    CirclePaint, GridPaint, ImagePaint, PathPaint, RectPaint, RectStyle, ShadowPaint, TextPaint,
};

use super::super::command::{
    AffineCircleRequest, AffineGridRequest, AffineImageRequest, AffinePathRequest,
    AffineRectRequest, AffineShadowRequest, AffineTextRequest, BackendCommand,
};
use super::super::display_resources::DisplayResourceResolver;
use super::api::UnsupportedDisplayReason;
use super::lowering::LoweringContext;

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    pub(super) fn lower_rect(&mut self, _index: usize, transform: Affine2D, paint: &RectPaint) {
        if let Some(shadow) = paint.style.shadow {
            self.commands
                .push(BackendCommand::Shadow(AffineShadowRequest {
                    rect: paint.rect,
                    radius: paint.style.radius,
                    shadow,
                    transform,
                }));
        }
        self.commands.push(BackendCommand::Rect(AffineRectRequest {
            rect: paint.rect,
            style: rect_style_without_shadow(&paint.style),
            transform,
        }));
    }

    pub(super) fn lower_path(&mut self, _index: usize, transform: Affine2D, paint: &PathPaint) {
        self.commands.push(BackendCommand::Path(AffinePathRequest {
            data: paint.data.clone(),
            style: paint.style,
            transform,
        }));
    }

    pub(super) fn lower_circle(&mut self, _index: usize, transform: Affine2D, paint: CirclePaint) {
        self.commands
            .push(BackendCommand::Circle(AffineCircleRequest {
                paint,
                transform,
            }));
    }

    pub(super) fn lower_grid(&mut self, _index: usize, transform: Affine2D, paint: GridPaint) {
        if paint.spacing <= 0.0 || paint.dot_size <= 0.0 {
            return;
        }
        self.commands
            .push(BackendCommand::Grid(AffineGridRequest { paint, transform }));
    }

    pub(super) fn lower_image(&mut self, index: usize, transform: Affine2D, paint: ImagePaint) {
        let Some(resource) = self.resources.texture(paint.texture) else {
            self.report.record(
                index,
                UnsupportedDisplayReason::MissingTexture(paint.texture),
            );
            return;
        };
        self.commands
            .push(BackendCommand::Image(AffineImageRequest {
                rect: paint.rect,
                transform,
                view: resource.view,
                size: resource.size,
                style: paint.style,
            }));
    }

    pub(super) fn lower_text(&mut self, _index: usize, transform: Affine2D, paint: &TextPaint) {
        self.commands.push(BackendCommand::Text(AffineTextRequest {
            pos: paint.pos,
            text: paint.text.clone(),
            style: paint.style,
            bounds: paint.bounds,
            transform,
        }));
    }

    pub(super) fn lower_shadow(&mut self, _index: usize, transform: Affine2D, paint: ShadowPaint) {
        self.commands
            .push(BackendCommand::Shadow(AffineShadowRequest {
                rect: paint.rect,
                radius: paint.radius,
                shadow: paint.shadow,
                transform,
            }));
    }
}

fn rect_style_without_shadow(style: &RectStyle) -> RectStyle {
    RectStyle {
        color: style.color,
        border: style.border,
        radius: style.radius,
        shadow: None,
    }
}
