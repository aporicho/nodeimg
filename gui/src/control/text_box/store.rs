use std::collections::HashMap;

use super::diagnostics::log_text_box_sizing;
use super::layout::fallback_height;
use super::model::{TextBoxSpec, TextBoxValueKind};
use super::registry::TextBoxRegistry;
use super::retained_lookup::{find_rect, text_box_owner_from_retained_node};
use super::runtime::TextBoxRuntime;
use crate::control::{parse_color_hex, text_box_value_style, ControlIntrinsic};
use crate::renderer::{Rect, TextMeasurer};
use crate::text::TextLayoutCache;
use crate::theme::Theme;
use crate::tree::{NodeId, Tree};

pub(crate) struct TextBoxStore {
    pub(super) runtimes: HashMap<String, TextBoxRuntime>,
    registry: TextBoxRegistry,
    layout_cache: TextLayoutCache,
}

impl TextBoxStore {
    pub(crate) fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
            registry: TextBoxRegistry::default(),
            layout_cache: TextLayoutCache::default(),
        }
    }

    pub(crate) fn take_dirty_intrinsics(&mut self) -> std::collections::BTreeSet<String> {
        self.registry.take_dirty_intrinsics()
    }

    pub(crate) fn sync_text_box(
        &mut self,
        tree: &Tree,
        measurer: &mut TextMeasurer,
        theme: &Theme,
        control_id: String,
        spec: TextBoxSpec,
        focused_control_id: Option<&str>,
    ) {
        self.registry
            .register_instance(control_id.clone(), spec.external_text.clone());
        let is_new_runtime = !self.runtimes.contains_key(&control_id);
        let mut runtime = self
            .runtimes
            .remove(&control_id)
            .unwrap_or_else(|| TextBoxRuntime::new(&spec));

        runtime.sync_spec(&spec);
        let allow_override = Some(control_id.as_str()) != focused_control_id;
        let external_text_changed = runtime.sync_external_text(&spec.external_text, allow_override);
        self.registry
            .update_external_value(&control_id, spec.external_text.clone());

        let field_rect = find_rect(tree, &format!("{control_id}::field")).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: fallback_height(spec.mode, spec.tokens),
        });
        let value_rect = find_rect(tree, &format!("{control_id}::value"));
        let value_style = text_box_value_style(theme, spec.tokens, spec.font);
        tree.record_text_layout_request();
        let layout_sync = runtime.sync_layout_with_style(
            field_rect,
            value_rect,
            measurer,
            &spec,
            value_style,
            &mut self.layout_cache,
        );
        if layout_sync.cache_hit {
            tree.record_text_layout_cache_hit();
        }
        if runtime.is_multiline() && (is_new_runtime || layout_sync.desired_height_changed) {
            self.registry
                .handle_editor_change(&control_id, runtime.editor().text().to_string());
            self.registry.mark_dirty_intrinsic(&control_id);
        }
        log_text_box_sizing(
            control_id.as_str(),
            &runtime,
            &spec,
            Some(control_id.as_str()) == focused_control_id,
            external_text_changed,
            value_rect,
        );
        self.runtimes.insert(control_id, runtime);
    }

    pub(crate) fn dirty_intrinsic_ids(&self) -> Vec<String> {
        self.registry
            .dirty_intrinsics()
            .map(str::to_string)
            .collect()
    }

    pub(crate) fn has_dirty_intrinsics(&self) -> bool {
        self.registry.has_dirty_intrinsics()
    }

    pub(crate) fn take_dirty_control_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.take_dirty_intrinsics()
            .into_iter()
            .filter_map(|control_id| self.control_intrinsic(&control_id))
            .collect()
    }

    pub(crate) fn text_box(&self, control_id: &str) -> Option<&TextBoxRuntime> {
        self.runtimes.get(control_id)
    }

    pub(crate) fn text_box_mut(&mut self, control_id: &str) -> Option<&mut TextBoxRuntime> {
        self.runtimes.get_mut(control_id)
    }

    pub(crate) fn clear_unfocused_preedit(&mut self, focused_control_id: Option<&str>) {
        for (control_id, runtime) in &mut self.runtimes {
            if Some(control_id.as_str()) != focused_control_id {
                runtime.clear_preedit();
            }
        }
    }

    pub(crate) fn revert_unfocused_commit_sensitive_values(
        &mut self,
        focused_control_id: Option<&str>,
    ) {
        for (control_id, runtime) in &mut self.runtimes {
            if Some(control_id.as_str()) == focused_control_id {
                continue;
            }
            if should_revert_unfocused(runtime) {
                runtime.revert_to_external();
            }
        }
    }

    pub(crate) fn focused_control_id(
        &self,
        tree: &Tree,
        focused: Option<NodeId>,
    ) -> Option<String> {
        let focused = focused?;
        let node = tree.get(focused)?;
        text_box_owner_from_retained_node(tree, node.id.as_ref())
    }
}

fn should_revert_unfocused(runtime: &TextBoxRuntime) -> bool {
    match runtime.value_kind() {
        TextBoxValueKind::Number { .. } => runtime.editor().text() != runtime.external_text(),
        TextBoxValueKind::Color { rgba } => {
            runtime.editor().text() != runtime.external_text()
                && parse_color_hex(runtime.editor().text(), rgba[3]).is_none()
        }
        TextBoxValueKind::Text | TextBoxValueKind::FilePath => false,
    }
}

impl Default for TextBoxStore {
    fn default() -> Self {
        Self::new()
    }
}
