use crate::geometry::Affine2D;
use crate::icon::{IconOpacity, IconStyle};
use crate::paint::{Color, ImageOpacity, LayerPaint, RectStyle};

use super::super::command::BackendCommand;
use super::super::display_resources::DisplayResourceResolver;
use super::api::{DisplayCommandKindCounts, DisplayRenderReport};
use super::lowering::LoweringContext;

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    pub(super) fn lower_layer(&mut self, _index: usize, transform: Affine2D, paint: &LayerPaint) {
        if paint.opacity <= 0.0 {
            return;
        }

        let mut nested = LoweringContext {
            resources: self.resources,
            svg_vector_cache: self.svg_vector_cache,
            active_clips: Vec::new(),
            commands: Vec::new(),
            report: DisplayRenderReport::default(),
            command_kinds: DisplayCommandKindCounts::default(),
            max_clip_depth: 0,
        };

        for (index, command) in paint.content.commands.iter().enumerate() {
            if !nested.sync_clips(index, &command.clips, &paint.content.clips) {
                continue;
            }
            nested.lower_command(index, command);
        }
        nested.pop_all_clips();

        apply_layer_to_commands(&mut nested.commands, transform, paint.opacity);
        self.report.unsupported.extend(nested.report.unsupported);
        self.command_kinds.add(nested.command_kinds);
        self.max_clip_depth = self.max_clip_depth.max(nested.max_clip_depth);
        self.commands.extend(nested.commands);
    }
}

fn apply_layer_to_commands(commands: &mut [BackendCommand], transform: Affine2D, opacity: f32) {
    for command in commands {
        compose_command_transform(command, transform);
        apply_command_opacity(command, opacity);
    }
}

fn compose_command_transform(command: &mut BackendCommand, transform: Affine2D) {
    match command {
        BackendCommand::Shadow(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Rect(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Circle(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Grid(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Text(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Image(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::SvgRaster(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::Path(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::PushClip(req) => {
            req.transform = Affine2D::compose(transform, req.transform);
        }
        BackendCommand::PopClip => {}
    }
}

fn apply_command_opacity(command: &mut BackendCommand, opacity: f32) {
    match command {
        BackendCommand::Shadow(req) => {
            req.shadow.color = color_with_opacity(req.shadow.color, opacity);
        }
        BackendCommand::Rect(req) => {
            apply_rect_style_opacity(&mut req.style, opacity);
        }
        BackendCommand::Circle(req) => {
            req.paint.fill = req
                .paint
                .fill
                .map(|color| color_with_opacity(color, opacity));
            req.paint.stroke = req.paint.stroke.map(|mut stroke| {
                stroke.color = color_with_opacity(stroke.color, opacity);
                stroke
            });
        }
        BackendCommand::Grid(req) => {
            req.paint.dot_color = color_with_opacity(req.paint.dot_color, opacity);
        }
        BackendCommand::Text(req) => {
            req.style.color = color_with_opacity(req.style.color, opacity);
        }
        BackendCommand::Image(req) => {
            req.style = req
                .style
                .with_opacity(ImageOpacity::new(req.style.opacity.get() * opacity));
        }
        BackendCommand::SvgRaster(req) => {
            req.style = icon_style_with_opacity(req.style, opacity);
        }
        BackendCommand::Path(req) => {
            if let Some(mut fill) = req.style.fill {
                fill.color = color_with_opacity(fill.color, opacity);
                req.style.fill = Some(fill);
            }
            if let Some(mut stroke) = req.style.stroke {
                stroke.color = color_with_opacity(stroke.color, opacity);
                req.style.stroke = Some(stroke);
            }
        }
        BackendCommand::PushClip(_) | BackendCommand::PopClip => {}
    }
}

fn apply_rect_style_opacity(style: &mut RectStyle, opacity: f32) {
    style.color = color_with_opacity(style.color, opacity);
    if let Some(mut border) = style.border {
        border.color = color_with_opacity(border.color, opacity);
        style.border = Some(border);
    }
    if let Some(mut shadow) = style.shadow {
        shadow.color = color_with_opacity(shadow.color, opacity);
        style.shadow = Some(shadow);
    }
}

fn icon_style_with_opacity(mut style: IconStyle, opacity: f32) -> IconStyle {
    style.opacity = IconOpacity::new(style.opacity.get() * opacity);
    style
}

fn color_with_opacity(mut color: Color, opacity: f32) -> Color {
    color.a *= opacity.clamp(0.0, 1.0);
    color
}
