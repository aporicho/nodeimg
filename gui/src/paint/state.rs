use crate::geometry::Affine2D;

use super::{ClipId, PaintBuildError};

#[derive(Debug, Clone)]
pub struct PaintState {
    transform_stack: Vec<Affine2D>,
    clip_stack: Vec<ClipId>,
}

impl PaintState {
    pub fn new() -> Self {
        Self {
            transform_stack: vec![Affine2D::IDENTITY],
            clip_stack: Vec::new(),
        }
    }

    pub fn current_transform(&self) -> Affine2D {
        self.transform_stack
            .last()
            .copied()
            .unwrap_or(Affine2D::IDENTITY)
    }

    pub fn current_clip(&self) -> Option<ClipId> {
        self.clip_stack.last().copied()
    }

    pub fn clip_stack(&self) -> &[ClipId] {
        &self.clip_stack
    }

    pub fn transform_depth(&self) -> usize {
        self.transform_stack.len()
    }

    pub fn clip_depth(&self) -> usize {
        self.clip_stack.len()
    }

    pub fn push_transform(&mut self, relative: Affine2D) {
        let composed = Affine2D::compose(self.current_transform(), relative);
        self.transform_stack.push(composed);
    }

    pub fn pop_transform(&mut self) -> Result<(), PaintBuildError> {
        if self.transform_stack.len() <= 1 {
            return Err(PaintBuildError::TransformStackUnderflow);
        }
        self.transform_stack.pop();
        Ok(())
    }

    pub fn push_clip(&mut self, id: ClipId) {
        self.clip_stack.push(id);
    }

    pub fn pop_clip(&mut self) -> Result<(), PaintBuildError> {
        self.clip_stack
            .pop()
            .map(|_| ())
            .ok_or(PaintBuildError::ClipStackUnderflow)
    }
}

impl Default for PaintState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn transform_stack_starts_at_identity() {
        assert_eq!(PaintState::new().current_transform(), Affine2D::IDENTITY);
    }

    #[test]
    fn push_transform_composes_with_current_transform() {
        let mut state = PaintState::new();

        state.push_transform(Affine2D::translation(10.0, 0.0));
        state.push_transform(Affine2D::scale(2.0));

        assert_eq!(
            state
                .current_transform()
                .transform_point(Point { x: 3.0, y: 4.0 }),
            Point { x: 16.0, y: 8.0 }
        );
    }

    #[test]
    fn pop_transform_rejects_root_underflow() {
        assert_eq!(
            PaintState::new().pop_transform(),
            Err(PaintBuildError::TransformStackUnderflow)
        );
    }

    #[test]
    fn clip_stack_tracks_current_clip() {
        let mut state = PaintState::new();
        state.push_clip(ClipId(7));

        assert_eq!(state.current_clip(), Some(ClipId(7)));
        assert_eq!(state.clip_stack(), &[ClipId(7)]);
    }
}
