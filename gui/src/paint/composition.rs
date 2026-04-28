use crate::geometry::Affine2D;
use crate::tree::{PaintCache, RepaintBoundaryId};
use std::collections::HashMap;

use super::{ClipId, DisplayList, ResolvedClip, ResolvedPaintCommand};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintCompositionStats {
    pub fragments_flattened: usize,
}

pub fn compose_fragments(
    cache: &PaintCache,
    root: RepaintBoundaryId,
) -> (DisplayList, PaintCompositionStats) {
    let mut list = DisplayList::default();
    let mut stats = PaintCompositionStats::default();
    let mut next_clip_id = 0;
    append_fragment(
        cache,
        root,
        Affine2D::IDENTITY,
        &mut list,
        &mut next_clip_id,
        &mut stats,
    );
    (list, stats)
}

fn append_fragment(
    cache: &PaintCache,
    boundary: RepaintBoundaryId,
    transform: Affine2D,
    list: &mut DisplayList,
    next_clip_id: &mut u32,
    stats: &mut PaintCompositionStats,
) {
    let Some(fragment) = cache.get(boundary) else {
        return;
    };
    stats.fragments_flattened += 1;

    let mut clip_remap = HashMap::with_capacity(fragment.clips.len());
    for clip in &fragment.clips {
        let next = ClipId(*next_clip_id);
        *next_clip_id += 1;
        clip_remap.insert(clip.id, next);
        list.clips.push(ResolvedClip {
            id: next,
            shape: clip.shape.clone(),
            transform: Affine2D::compose(transform, clip.transform),
            parent: clip
                .parent
                .and_then(|parent| clip_remap.get(&parent).copied()),
        });
    }

    list.commands
        .extend(fragment.commands.iter().map(|command| {
            ResolvedPaintCommand {
                command: command.command.clone(),
                transform: Affine2D::compose(transform, command.transform),
                clips: command
                    .clips
                    .iter()
                    .filter_map(|clip| clip_remap.get(clip).copied())
                    .collect(),
            }
        }));

    for child in &fragment.child_boundaries {
        append_fragment(
            cache,
            child.boundary,
            Affine2D::compose(transform, child.local_transform),
            list,
            next_clip_id,
            stats,
        );
    }
}
