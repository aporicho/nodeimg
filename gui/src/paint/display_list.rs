use crate::geometry::Affine2D;

use super::state::PaintState;
use super::{ClipId, ClipShape, PaintCommand, ResolvedClip};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    pub commands: Vec<ResolvedPaintCommand>,
    pub clips: Vec<ResolvedClip>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPaintCommand {
    pub command: PaintCommand,
    pub transform: Affine2D,
    pub clips: Vec<ClipId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintBuildError {
    TransformStackUnderflow,
    ClipStackUnderflow,
    UnbalancedTransformStack { depth: usize },
    UnbalancedClipStack { depth: usize },
}

#[derive(Debug, Clone)]
pub struct DisplayListBuilder {
    state: PaintState,
    commands: Vec<ResolvedPaintCommand>,
    clips: Vec<ResolvedClip>,
    error: Option<PaintBuildError>,
    next_clip_id: u32,
}

impl DisplayListBuilder {
    pub fn new() -> Self {
        Self {
            state: PaintState::new(),
            commands: Vec::new(),
            clips: Vec::new(),
            error: None,
            next_clip_id: 0,
        }
    }

    pub fn push_transform(&mut self, transform: Affine2D) {
        self.state.push_transform(transform);
    }

    pub fn pop_transform(&mut self) {
        if let Err(err) = self.state.pop_transform() {
            self.set_error(err);
        }
    }

    pub fn push_clip(&mut self, shape: ClipShape) -> ClipId {
        let id = ClipId(self.next_clip_id);
        self.next_clip_id += 1;
        let parent = self.state.current_clip();
        self.clips.push(ResolvedClip {
            id,
            shape,
            transform: self.state.current_transform(),
            parent,
        });
        self.state.push_clip(id);
        id
    }

    pub fn pop_clip(&mut self) {
        if let Err(err) = self.state.pop_clip() {
            self.set_error(err);
        }
    }

    pub fn draw(&mut self, command: PaintCommand) {
        self.commands.push(ResolvedPaintCommand {
            command,
            transform: self.state.current_transform(),
            clips: self.state.clip_stack().to_vec(),
        });
    }

    pub fn commands(&self) -> &[ResolvedPaintCommand] {
        &self.commands
    }

    pub fn clips(&self) -> &[ResolvedClip] {
        &self.clips
    }

    pub fn finish(self) -> Result<DisplayList, PaintBuildError> {
        if let Some(err) = self.error {
            return Err(err);
        }
        if self.state.transform_depth() != 1 {
            return Err(PaintBuildError::UnbalancedTransformStack {
                depth: self.state.transform_depth(),
            });
        }
        if self.state.clip_depth() != 0 {
            return Err(PaintBuildError::UnbalancedClipStack {
                depth: self.state.clip_depth(),
            });
        }
        Ok(DisplayList {
            commands: self.commands,
            clips: self.clips,
        })
    }

    fn set_error(&mut self, err: PaintBuildError) {
        if self.error.is_none() {
            self.error = Some(err);
        }
    }
}

impl Default for DisplayListBuilder {
    fn default() -> Self {
        Self::new()
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

    fn rect_command() -> PaintCommand {
        PaintCommand::Rect(RectPaint {
            rect: rect(),
            style: RectStyle {
                color: Color::WHITE,
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
        })
    }

    #[test]
    fn draw_records_local_command_and_current_transform() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::translation(10.0, 20.0));
        builder.draw(rect_command());
        builder.pop_transform();

        let list = builder.finish().unwrap();

        assert_eq!(list.commands[0].command, rect_command());
        assert_eq!(
            list.commands[0]
                .transform
                .transform_point(Point { x: 1.0, y: 2.0 }),
            Point { x: 11.0, y: 22.0 }
        );
    }

    #[test]
    fn nested_transform_stack_composes_parent_child_order() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::translation(10.0, 0.0));
        builder.push_transform(Affine2D::scale(2.0));
        builder.draw(rect_command());
        builder.pop_transform();
        builder.pop_transform();

        let list = builder.finish().unwrap();

        assert_eq!(
            list.commands[0]
                .transform
                .transform_point(Point { x: 3.0, y: 4.0 }),
            Point { x: 16.0, y: 8.0 }
        );
    }

    #[test]
    fn push_clip_records_shape_transform_and_parent() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::translation(5.0, 6.0));
        let first = builder.push_clip(ClipShape::Rect(rect()));
        let second = builder.push_clip(ClipShape::RoundedRect {
            rect: rect(),
            radius: [1.0; 4],
        });
        builder.pop_clip();
        builder.pop_clip();
        builder.pop_transform();

        let list = builder.finish().unwrap();

        assert_eq!(list.clips[0].id, first);
        assert_eq!(list.clips[0].parent, None);
        assert_eq!(list.clips[1].id, second);
        assert_eq!(list.clips[1].parent, Some(first));
        assert_eq!(list.clips[0].transform, Affine2D::translation(5.0, 6.0));
    }

    #[test]
    fn draw_under_nested_clips_records_full_clip_stack() {
        let mut builder = DisplayListBuilder::new();
        let first = builder.push_clip(ClipShape::Rect(rect()));
        let second = builder.push_clip(ClipShape::Rect(rect()));
        builder.draw(rect_command());
        builder.pop_clip();
        builder.pop_clip();

        let list = builder.finish().unwrap();

        assert_eq!(list.commands[0].clips, vec![first, second]);
    }

    #[test]
    fn pop_transform_underflow_returns_error_on_finish() {
        let mut builder = DisplayListBuilder::new();
        builder.pop_transform();

        assert_eq!(
            builder.finish(),
            Err(PaintBuildError::TransformStackUnderflow)
        );
    }

    #[test]
    fn pop_clip_underflow_returns_error_on_finish() {
        let mut builder = DisplayListBuilder::new();
        builder.pop_clip();

        assert_eq!(builder.finish(), Err(PaintBuildError::ClipStackUnderflow));
    }

    #[test]
    fn unbalanced_transform_stack_returns_error_on_finish() {
        let mut builder = DisplayListBuilder::new();
        builder.push_transform(Affine2D::translation(1.0, 2.0));

        assert_eq!(
            builder.finish(),
            Err(PaintBuildError::UnbalancedTransformStack { depth: 2 })
        );
    }

    #[test]
    fn unbalanced_clip_stack_returns_error_on_finish() {
        let mut builder = DisplayListBuilder::new();
        builder.push_clip(ClipShape::Rect(rect()));

        assert_eq!(
            builder.finish(),
            Err(PaintBuildError::UnbalancedClipStack { depth: 1 })
        );
    }
}
