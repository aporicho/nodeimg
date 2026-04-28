use std::collections::HashMap;

use crate::renderer::{Point, Rect};
use crate::tree::StylePatch;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlotValues {
    values: HashMap<String, SlotValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SlotValue {
    Text(String),
    Rect(Rect),
    Style(StylePatch),
    Visible(bool),
    ZIndex(i32),
    Connection {
        from_port: String,
        to_port: String,
    },
    PendingConnection {
        from_port: String,
        cursor_canvas: Point,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemplateSlots {
    bindings: Vec<SlotBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotBinding {
    pub name: &'static str,
    pub node_suffix: &'static str,
    pub target: SlotTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotTarget {
    TextContent,
    Rect,
    Style,
    Visible,
    ZIndex,
    ConnectionEndpoints,
    PendingConnection,
}

impl SlotValues {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: SlotValue) {
        self.values.insert(name.into(), value);
    }

    pub fn with(mut self, name: impl Into<String>, value: SlotValue) -> Self {
        self.insert(name, value);
        self
    }

    pub fn get(&self, name: &str) -> Option<&SlotValue> {
        self.values.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &SlotValue)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl TemplateSlots {
    pub fn new(bindings: impl Into<Vec<SlotBinding>>) -> Self {
        Self {
            bindings: bindings.into(),
        }
    }

    pub fn bindings(&self) -> &[SlotBinding] {
        &self.bindings
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl SlotBinding {
    pub const fn new(name: &'static str, node_suffix: &'static str, target: SlotTarget) -> Self {
        Self {
            name,
            node_suffix,
            target,
        }
    }
}
