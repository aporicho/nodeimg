use gui::canvas::camera::Camera;
use gui::renderer::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorkspaceCursorRefreshKey {
    x: f32,
    y: f32,
    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,
    viewport: Rect,
    panning: bool,
}

impl WorkspaceCursorRefreshKey {
    pub(crate) fn new(x: f32, y: f32, camera: &Camera, viewport: Rect, panning: bool) -> Self {
        Self {
            x,
            y,
            camera_x: camera.x,
            camera_y: camera.y,
            camera_zoom: camera.zoom,
            viewport,
            panning,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceCursorDirtyReason {
    Initial,
    Scene,
    Navigation,
    Animation,
    ExternalInput,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkspaceCursorRefreshDecision {
    pub(crate) should_refresh: bool,
    pub(crate) reasons: Vec<WorkspaceCursorDirtyReason>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkspaceCursorRefreshGate {
    last_refreshed_key: Option<WorkspaceCursorRefreshKey>,
    dirty_reasons: Vec<WorkspaceCursorDirtyReason>,
}

impl WorkspaceCursorRefreshGate {
    pub(crate) fn new() -> Self {
        Self {
            last_refreshed_key: None,
            dirty_reasons: vec![WorkspaceCursorDirtyReason::Initial],
        }
    }

    pub(crate) fn mark(&mut self, reason: WorkspaceCursorDirtyReason) {
        if !self.dirty_reasons.contains(&reason) {
            self.dirty_reasons.push(reason);
        }
    }

    pub(crate) fn mark_if(&mut self, changed: bool, reason: WorkspaceCursorDirtyReason) {
        if changed {
            self.mark(reason);
        }
    }

    pub(crate) fn begin_frame(
        &self,
        key: WorkspaceCursorRefreshKey,
    ) -> WorkspaceCursorRefreshDecision {
        let mut reasons = self.dirty_reasons.clone();
        if self.last_refreshed_key != Some(key) {
            reasons.push(WorkspaceCursorDirtyReason::ExternalInput);
        }
        dedupe_reasons(&mut reasons);
        WorkspaceCursorRefreshDecision {
            should_refresh: !reasons.is_empty(),
            reasons,
        }
    }

    pub(crate) fn finish_refreshed(&mut self, key: WorkspaceCursorRefreshKey) {
        self.last_refreshed_key = Some(key);
        self.dirty_reasons.clear();
    }
}

impl Default for WorkspaceCursorRefreshGate {
    fn default() -> Self {
        Self::new()
    }
}

fn dedupe_reasons(reasons: &mut Vec<WorkspaceCursorDirtyReason>) {
    let mut deduped = Vec::with_capacity(reasons.len());
    for reason in reasons.drain(..) {
        if !deduped.contains(&reason) {
            deduped.push(reason);
        }
    }
    *reasons = deduped;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(x: f32, camera_x: f32, panning: bool) -> WorkspaceCursorRefreshKey {
        let mut camera = Camera::new();
        camera.x = camera_x;
        WorkspaceCursorRefreshKey::new(
            x,
            20.0,
            &camera,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            panning,
        )
    }

    #[test]
    fn initial_frame_requires_refresh() {
        let gate = WorkspaceCursorRefreshGate::new();
        let decision = gate.begin_frame(key(10.0, 0.0, false));

        assert!(decision.should_refresh);
        assert!(decision
            .reasons
            .contains(&WorkspaceCursorDirtyReason::Initial));
    }

    #[test]
    fn clean_same_key_skips_refresh() {
        let mut gate = WorkspaceCursorRefreshGate::new();
        let key = key(10.0, 0.0, false);

        gate.finish_refreshed(key);

        assert!(!gate.begin_frame(key).should_refresh);
    }

    #[test]
    fn pointer_camera_and_panning_changes_refresh() {
        let mut gate = WorkspaceCursorRefreshGate::new();
        gate.finish_refreshed(key(10.0, 0.0, false));

        assert!(gate.begin_frame(key(12.0, 0.0, false)).should_refresh);
        assert!(gate.begin_frame(key(10.0, 3.0, false)).should_refresh);
        assert!(gate.begin_frame(key(10.0, 0.0, true)).should_refresh);
    }

    #[test]
    fn mark_if_tracks_scene_dirty() {
        let mut gate = WorkspaceCursorRefreshGate::new();
        let key = key(10.0, 0.0, false);
        gate.finish_refreshed(key);

        gate.mark_if(false, WorkspaceCursorDirtyReason::Scene);
        assert!(!gate.begin_frame(key).should_refresh);

        gate.mark_if(true, WorkspaceCursorDirtyReason::Scene);
        let decision = gate.begin_frame(key);
        assert!(decision.should_refresh);
        assert!(decision
            .reasons
            .contains(&WorkspaceCursorDirtyReason::Scene));
    }
}
