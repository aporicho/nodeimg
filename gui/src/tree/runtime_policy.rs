#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSlotPolicy {
    pub retention: RuntimeRetention,
    pub persistence: PersistenceClass,
    pub undo: UndoClass,
}

impl RuntimeSlotPolicy {
    pub const EPHEMERAL: Self = Self {
        retention: RuntimeRetention::DropWhenNodeMissing,
        persistence: PersistenceClass::Ephemeral,
        undo: UndoClass::NonUndoable,
    };
}

impl Default for RuntimeSlotPolicy {
    fn default() -> Self {
        Self::EPHEMERAL
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeRetention {
    DropWhenNodeMissing,
    KeepWhileStableNodeExists,
    KeepWhileOwnerExists(String),
    KeepForSession,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceClass {
    Ephemeral,
    ProjectLayout,
    SessionOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndoClass {
    Undoable,
    NonUndoable,
}
