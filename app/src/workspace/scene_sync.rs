use gui::canvas::camera::Camera;
use gui::renderer::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorkspaceSceneSyncKey {
    viewport: Rect,
    camera_x: f32,
    camera_y: f32,
    camera_zoom: f32,
    composition: &'static str,
}

impl WorkspaceSceneSyncKey {
    pub(crate) fn new(viewport: Rect, camera: &Camera, composition: &'static str) -> Self {
        Self {
            viewport,
            camera_x: camera.x,
            camera_y: camera.y,
            camera_zoom: camera.zoom,
            composition,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceSceneDirtyReason {
    Initial,
    Composition,
    EngineGraph,
    CanvasRuntime,
    PanelRuntime,
    Overlay,
    ControlIntrinsic,
    ExternalInput,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkspaceSceneSyncDecision {
    pub(crate) should_sync: bool,
    pub(crate) reasons: Vec<WorkspaceSceneDirtyReason>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkspaceSceneSyncGate {
    last_synced_key: Option<WorkspaceSceneSyncKey>,
    dirty_reasons: Vec<WorkspaceSceneDirtyReason>,
}

impl WorkspaceSceneSyncGate {
    pub(crate) fn new() -> Self {
        Self {
            last_synced_key: None,
            dirty_reasons: vec![WorkspaceSceneDirtyReason::Initial],
        }
    }

    pub(crate) fn mark(&mut self, reason: WorkspaceSceneDirtyReason) {
        if !self.dirty_reasons.contains(&reason) {
            self.dirty_reasons.push(reason);
        }
    }

    pub(crate) fn mark_if(&mut self, changed: bool, reason: WorkspaceSceneDirtyReason) {
        if changed {
            self.mark(reason);
        }
    }

    pub(crate) fn begin_frame(
        &self,
        key: WorkspaceSceneSyncKey,
        control_intrinsics_dirty: bool,
    ) -> WorkspaceSceneSyncDecision {
        let mut reasons = self.dirty_reasons.clone();
        if self.last_synced_key != Some(key) {
            reasons.push(WorkspaceSceneDirtyReason::ExternalInput);
        }
        if control_intrinsics_dirty {
            reasons.push(WorkspaceSceneDirtyReason::ControlIntrinsic);
        }
        dedupe_reasons(&mut reasons);
        WorkspaceSceneSyncDecision {
            should_sync: !reasons.is_empty(),
            reasons,
        }
    }

    pub(crate) fn finish_synced(&mut self, key: WorkspaceSceneSyncKey) {
        self.last_synced_key = Some(key);
        self.dirty_reasons.clear();
    }
}

impl Default for WorkspaceSceneSyncGate {
    fn default() -> Self {
        Self::new()
    }
}

fn dedupe_reasons(reasons: &mut Vec<WorkspaceSceneDirtyReason>) {
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

    fn key(x: f32, composition: &'static str) -> WorkspaceSceneSyncKey {
        let mut camera = Camera::new();
        camera.x = x;
        WorkspaceSceneSyncKey::new(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 800.0,
                h: 600.0,
            },
            &camera,
            composition,
        )
    }

    #[test]
    fn initial_frame_requires_sync() {
        let gate = WorkspaceSceneSyncGate::new();
        let decision = gate.begin_frame(key(0.0, "user"), false);

        assert!(decision.should_sync);
        assert!(decision
            .reasons
            .contains(&WorkspaceSceneDirtyReason::Initial));
    }

    #[test]
    fn clean_same_key_skips_sync() {
        let mut gate = WorkspaceSceneSyncGate::new();
        let key = key(0.0, "user");

        gate.finish_synced(key);

        assert!(!gate.begin_frame(key, false).should_sync);
    }

    #[test]
    fn key_change_requires_sync() {
        let mut gate = WorkspaceSceneSyncGate::new();
        gate.finish_synced(key(0.0, "user"));

        let decision = gate.begin_frame(key(10.0, "user"), false);

        assert!(decision.should_sync);
        assert!(decision
            .reasons
            .contains(&WorkspaceSceneDirtyReason::ExternalInput));
    }

    #[test]
    fn mark_if_controls_dirty_reasons() {
        let mut gate = WorkspaceSceneSyncGate::new();
        let key = key(0.0, "user");
        gate.finish_synced(key);

        gate.mark_if(false, WorkspaceSceneDirtyReason::CanvasRuntime);
        assert!(!gate.begin_frame(key, false).should_sync);

        gate.mark_if(true, WorkspaceSceneDirtyReason::CanvasRuntime);
        let decision = gate.begin_frame(key, false);
        assert!(decision.should_sync);
        assert!(decision
            .reasons
            .contains(&WorkspaceSceneDirtyReason::CanvasRuntime));
    }

    #[test]
    fn control_intrinsic_dirty_requires_sync() {
        let mut gate = WorkspaceSceneSyncGate::new();
        let key = key(0.0, "user");
        gate.finish_synced(key);

        let decision = gate.begin_frame(key, true);

        assert!(decision.should_sync);
        assert!(decision
            .reasons
            .contains(&WorkspaceSceneDirtyReason::ControlIntrinsic));
    }
}
