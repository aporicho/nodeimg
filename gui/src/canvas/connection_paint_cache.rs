use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanvasConnectionPaintCache {
    dirty_connections: BTreeSet<String>,
    whole_layer_dirty: bool,
}

impl CanvasConnectionPaintCache {
    pub fn mark_layer_dirty(&mut self) {
        self.whole_layer_dirty = true;
    }

    pub fn mark_connection_dirty(&mut self, id: impl Into<String>) {
        self.dirty_connections.insert(id.into());
    }

    pub fn take_dirty(&mut self) -> CanvasConnectionPaintDirty {
        CanvasConnectionPaintDirty {
            whole_layer_dirty: std::mem::take(&mut self.whole_layer_dirty),
            connections: std::mem::take(&mut self.dirty_connections),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanvasConnectionPaintDirty {
    pub whole_layer_dirty: bool,
    pub connections: BTreeSet<String>,
}
