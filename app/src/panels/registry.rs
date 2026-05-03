#[path = "engine.rs"]
mod engine;
#[path = "preview.rs"]
mod preview;
#[path = "toolbar.rs"]
mod toolbar;

use std::collections::{BTreeMap, BTreeSet};

use gui::layout::TextureHandle;
use gui::panel::{PanelConfig, PanelContentTemplate};
use gui::theme::Theme;

use super::EnginePanelState;

type PanelFactory = for<'a> fn(PanelRenderInput<'a>) -> PanelSpec;

const PANEL_FACTORIES: &[PanelFactory] = &[toolbar::spec, preview::spec, engine::spec];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelWorkspaceMode {
    CleanRoom,
    Full,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelModeSet {
    CleanRoom,
    Full,
    Both,
}

impl PanelModeSet {
    pub(crate) fn contains(self, mode: PanelWorkspaceMode) -> bool {
        matches!(
            (self, mode),
            (Self::Both, _)
                | (Self::CleanRoom, PanelWorkspaceMode::CleanRoom)
                | (Self::Full, PanelWorkspaceMode::Full)
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PanelRenderInput<'a> {
    pub(crate) preview_image: TextureHandle,
    pub(crate) engine_panel: &'a EnginePanelState,
}

#[derive(Clone, Debug)]
pub(crate) struct PanelSpec {
    pub(crate) modes: PanelModeSet,
    pub(crate) config: PanelConfig,
    pub(crate) content: PanelContentTemplate,
    pub(crate) applied: PanelAppliedSnapshot,
}

impl PanelSpec {
    pub(crate) fn runtime_config(&self, theme: &Theme) -> PanelConfig {
        let content_min_height = self.content.panel_min_height(&self.config, theme);
        let mut config = self.config.clone();
        config.min_size[1] = config.min_size[1].max(content_min_height);
        config.default_rect.w = config.default_rect.w.max(config.min_size[0]);
        config.default_rect.h = config.default_rect.h.max(config.min_size[1]);
        config
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PanelAppliedSnapshot {
    pub(crate) texts: BTreeMap<String, String>,
}

impl PanelAppliedSnapshot {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_text(mut self, id: impl Into<String>, value: impl Into<String>) -> Self {
        self.texts.insert(id.into(), value.into());
        self
    }
}

pub(crate) fn registered_panels(input: PanelRenderInput<'_>) -> Vec<PanelSpec> {
    let panels = PANEL_FACTORIES
        .iter()
        .map(|factory| factory(input))
        .collect::<Vec<_>>();
    assert_unique_panel_ids(&panels);
    panels
}

fn assert_unique_panel_ids(panels: &[PanelSpec]) {
    let mut ids = BTreeSet::new();
    for panel in panels {
        let id = panel.config.id.as_str();
        assert!(ids.insert(id.to_string()), "duplicate panel id '{id}'");
    }
}

#[cfg(test)]
mod tests {
    use gui::layout::TextureHandle;
    use gui::theme::light_theme;

    use super::*;
    use crate::panels::EnginePanelState;

    fn input() -> PanelRenderInput<'static> {
        PanelRenderInput {
            preview_image: TextureHandle(1),
            engine_panel: Box::leak(Box::new(EnginePanelState {
                node_count: 1,
                connection_count: 2,
                node_def_count: 3,
                graph_version: 4,
                dirty: false,
                execution_status: "Idle".to_string(),
                last_action: "Ready".to_string(),
            })),
        }
    }

    #[test]
    fn registered_panel_ids_are_unique() {
        let panels = registered_panels(input());
        let ids = panels
            .iter()
            .map(|panel| panel.config.id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(ids.len(), panels.len());
    }

    #[test]
    fn clean_room_filters_to_toolbar_and_preview() {
        let panels = registered_panels(input())
            .into_iter()
            .filter(|panel| panel.modes.contains(PanelWorkspaceMode::CleanRoom))
            .map(|panel| panel.config.id.as_str().to_string())
            .collect::<Vec<_>>();

        assert_eq!(panels, vec!["toolbar", "preview"]);
    }

    #[test]
    fn full_mode_filters_to_all_registered_panels() {
        let panels = registered_panels(input())
            .into_iter()
            .filter(|panel| panel.modes.contains(PanelWorkspaceMode::Full))
            .map(|panel| panel.config.id.as_str().to_string())
            .collect::<Vec<_>>();

        assert_eq!(panels, vec!["toolbar", "preview", "engine"]);
    }

    #[test]
    fn runtime_config_clamps_height_to_panel_content() {
        let theme = light_theme();
        let toolbar = registered_panels(input())
            .into_iter()
            .find(|panel| panel.config.id.as_str() == "toolbar")
            .expect("toolbar panel should be registered");

        let config = toolbar.runtime_config(&theme);
        let content_min_height = toolbar.content.panel_min_height(&toolbar.config, &theme);

        assert!(config.min_size[1] >= content_min_height);
        assert!(config.default_rect.h >= content_min_height);
    }
}
