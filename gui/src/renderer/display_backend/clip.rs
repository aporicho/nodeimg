use crate::paint::ResolvedClip;

use super::super::command::{AffineClipRequest, BackendCommand};
use super::super::display_resources::DisplayResourceResolver;
use super::api::UnsupportedDisplayReason;
use super::lowering::LoweringContext;

impl<R: DisplayResourceResolver> LoweringContext<'_, R> {
    pub(super) fn sync_clips(
        &mut self,
        index: usize,
        desired: &[crate::paint::ClipId],
        clips: &[ResolvedClip],
    ) -> bool {
        let common = self
            .active_clips
            .iter()
            .zip(desired)
            .take_while(|(active, wanted)| active == wanted)
            .count();

        for _ in common..self.active_clips.len() {
            self.commands.push(BackendCommand::PopClip);
        }
        self.active_clips.truncate(common);

        let mut pending = Vec::new();
        for clip_id in &desired[common..] {
            let Some(clip) = clips.iter().find(|clip| clip.id == *clip_id) else {
                self.report.record(
                    index,
                    UnsupportedDisplayReason::UnsupportedClip("missing clip"),
                );
                return false;
            };
            let Some(command) = self.lower_clip(index, clip) else {
                return false;
            };
            pending.push((*clip_id, command));
        }

        for (clip_id, command) in pending {
            self.commands.push(command);
            self.active_clips.push(clip_id);
        }
        self.max_clip_depth = self.max_clip_depth.max(self.active_clips.len());

        true
    }

    fn lower_clip(&mut self, _index: usize, clip: &ResolvedClip) -> Option<BackendCommand> {
        Some(BackendCommand::PushClip(AffineClipRequest {
            shape: clip.shape.clone(),
            transform: clip.transform,
        }))
    }

    pub(super) fn pop_all_clips(&mut self) {
        for _ in 0..self.active_clips.len() {
            self.commands.push(BackendCommand::PopClip);
        }
        self.active_clips.clear();
    }
}
