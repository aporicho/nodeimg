use crate::geometry::Affine2D;

use super::{
    ClipShape, DisplayList, DisplayListBuilder, PaintBuildError, PaintCommand, PaintTarget,
    ResolvedClip, ResolvedPaintCommand, TextStyle,
};

type TextMeasureFn = dyn FnMut(&str, &TextStyle) -> (f32, f32);

pub struct RecordingPaintTarget {
    builder: DisplayListBuilder,
    measure: Box<TextMeasureFn>,
}

impl RecordingPaintTarget {
    pub fn new() -> Self {
        Self::with_measure(|text, style| {
            let width = text.chars().count() as f32 * style.size * 0.5;
            (width, style.size * style.line_height)
        })
    }

    pub fn with_measure(measure: impl FnMut(&str, &TextStyle) -> (f32, f32) + 'static) -> Self {
        Self {
            builder: DisplayListBuilder::new(),
            measure: Box::new(measure),
        }
    }

    pub fn commands(&self) -> &[ResolvedPaintCommand] {
        self.builder.commands()
    }

    pub fn clips(&self) -> &[ResolvedClip] {
        self.builder.clips()
    }

    pub fn display_list(self) -> Result<DisplayList, PaintBuildError> {
        self.builder.finish()
    }
}

impl Default for RecordingPaintTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl PaintTarget for RecordingPaintTarget {
    fn push_transform(&mut self, transform: Affine2D) {
        self.builder.push_transform(transform);
    }

    fn pop_transform(&mut self) {
        self.builder.pop_transform();
    }

    fn push_clip(&mut self, clip: ClipShape) {
        self.builder.push_clip(clip);
    }

    fn pop_clip(&mut self) {
        self.builder.pop_clip();
    }

    fn draw(&mut self, command: PaintCommand) {
        self.builder.draw(command);
    }

    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        (self.measure)(text, style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};
    use crate::paint::{Color, RectPaint, RectStyle};

    fn rect() -> Rect {
        Rect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
        }
    }

    fn style() -> RectStyle {
        RectStyle {
            color: Color::WHITE,
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }
    }

    #[test]
    fn recording_rect_preserves_local_rect() {
        let mut target = RecordingPaintTarget::new();
        target.push_transform(Affine2D::translation(10.0, 20.0));
        target.draw_rect(rect(), style());
        target.pop_transform();

        let list = target.display_list().unwrap();

        assert_eq!(
            list.commands[0].command,
            PaintCommand::Rect(RectPaint {
                rect: rect(),
                style: style(),
            })
        );
        assert_eq!(
            list.commands[0]
                .transform
                .transform_point(Point { x: 0.0, y: 0.0 }),
            Point { x: 10.0, y: 20.0 }
        );
    }

    #[test]
    fn recording_text_uses_measure_callback() {
        let mut target = RecordingPaintTarget::with_measure(|_, _| (42.0, 7.0));

        assert_eq!(
            target.measure_text("ignored", &TextStyle::new(Color::WHITE, 12.0)),
            (42.0, 7.0)
        );
    }

    #[test]
    fn recording_clip_and_transform_are_resolved_once() {
        let mut target = RecordingPaintTarget::new();
        target.push_transform(Affine2D::translation(5.0, 6.0));
        target.push_clip(ClipShape::Rect(rect()));
        target.draw_rect(rect(), style());
        target.pop_clip();
        target.pop_transform();

        let list = target.display_list().unwrap();

        assert_eq!(list.clips.len(), 1);
        assert_eq!(list.commands[0].clips, vec![list.clips[0].id]);
        assert_eq!(list.clips[0].transform, Affine2D::translation(5.0, 6.0));
    }
}
