use crate::panel::state::PanelRuntimeStore;

#[derive(Clone, Debug, Default)]
pub(crate) struct TreeRuntime {
    pub(crate) panels: PanelRuntimeStore,
}
