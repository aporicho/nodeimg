pub mod checkbox;
pub mod color_picker;
pub mod dropdown;
pub mod file_picker;
pub mod number_input;
pub mod radio_group;
pub mod slider;
pub mod text_input;

use nodeimg_types::constraint::{Constraint, ConstraintType};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use eframe::egui;
use std::collections::HashMap;

pub use nodeimg_types::widget_id::WidgetId;

/// Key for matching data type + constraint to widget(s).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetMatchKey {
    pub data_type: DataTypeId,
    pub constraint_type: ConstraintType,
}

/// Render function signature: (ui, value, constraint, param_name, disabled) -> changed
pub type WidgetRenderFn = fn(&mut egui::Ui, &mut Value, &Constraint, &str, bool) -> bool;

pub struct WidgetEntry {
    pub id: WidgetId,
    pub is_default: bool,
    pub render: WidgetRenderFn,
}

pub struct WidgetRegistry {
    /// Maps (DataType + Constraint) → list of available widgets.
    mappings: HashMap<WidgetMatchKey, Vec<WidgetEntry>>,
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetRegistry {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        data_type: DataTypeId,
        constraint_type: ConstraintType,
        widget_id: WidgetId,
        is_default: bool,
        render: WidgetRenderFn,
    ) {
        let key = WidgetMatchKey {
            data_type,
            constraint_type,
        };
        let entries = self.mappings.entry(key).or_default();
        entries.push(WidgetEntry {
            id: widget_id,
            is_default,
            render,
        });
    }

    /// Render a parameter using the specified widget (or default).
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        widget_id: Option<&WidgetId>,
        data_type: &DataTypeId,
        constraint_type: &ConstraintType,
        ui: &mut egui::Ui,
        value: &mut Value,
        constraint: &Constraint,
        param_name: &str,
        disabled: bool,
    ) -> bool {
        let key = WidgetMatchKey {
            data_type: data_type.clone(),
            constraint_type: constraint_type.clone(),
        };
        if let Some(entries) = self.mappings.get(&key) {
            let entry = if let Some(wid) = widget_id {
                entries.iter().find(|e| &e.id == wid)
            } else {
                entries.iter().find(|e| e.is_default)
            };
            if let Some(entry) = entry {
                return (entry.render)(ui, value, constraint, param_name, disabled);
            }
        }
        false
    }

    /// Returns the default widget for a given type+constraint combo.
    pub fn default_widget(
        &self,
        data_type: &DataTypeId,
        constraint_type: &ConstraintType,
    ) -> Option<&WidgetId> {
        let key = WidgetMatchKey {
            data_type: data_type.clone(),
            constraint_type: constraint_type.clone(),
        };
        self.mappings
            .get(&key)
            .and_then(|entries| entries.iter().find(|e| e.is_default).map(|e| &e.id))
    }

    /// Returns all available widgets for a given type+constraint combo.
    pub fn available_widgets(
        &self,
        data_type: &DataTypeId,
        constraint_type: &ConstraintType,
    ) -> Vec<&WidgetId> {
        let key = WidgetMatchKey {
            data_type: data_type.clone(),
            constraint_type: constraint_type.clone(),
        };
        self.mappings
            .get(&key)
            .map(|entries| entries.iter().map(|e| &e.id).collect())
            .unwrap_or_default()
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();

        // Float + Range → Slider (default), NumberInput
        reg.register(
            DataTypeId::new("float"),
            ConstraintType::Range,
            WidgetId::new("slider"),
            true,
            slider::render_slider,
        );
        reg.register(
            DataTypeId::new("float"),
            ConstraintType::Range,
            WidgetId::new("number_input"),
            false,
            number_input::render_number_input,
        );

        // Int + Range → IntSlider (default), NumberInput
        reg.register(
            DataTypeId::new("int"),
            ConstraintType::Range,
            WidgetId::new("int_slider"),
            true,
            slider::render_int_slider,
        );
        reg.register(
            DataTypeId::new("int"),
            ConstraintType::Range,
            WidgetId::new("number_input"),
            false,
            number_input::render_number_input,
        );

        // Boolean + None → Checkbox (default)
        reg.register(
            DataTypeId::new("boolean"),
            ConstraintType::None,
            WidgetId::new("checkbox"),
            true,
            checkbox::render_checkbox,
        );

        // Color + None → ColorPicker (default)
        reg.register(
            DataTypeId::new("color"),
            ConstraintType::None,
            WidgetId::new("color_picker"),
            true,
            color_picker::render_color_picker,
        );

        // String + Enum → Dropdown (default), RadioGroup
        reg.register(
            DataTypeId::new("string"),
            ConstraintType::Enum,
            WidgetId::new("dropdown"),
            true,
            dropdown::render_dropdown,
        );
        reg.register(
            DataTypeId::new("string"),
            ConstraintType::Enum,
            WidgetId::new("radio_group"),
            false,
            radio_group::render_radio_group,
        );

        // String + FilePath → FilePicker (default)
        reg.register(
            DataTypeId::new("string"),
            ConstraintType::FilePath,
            WidgetId::new("file_picker"),
            true,
            file_picker::render_file_picker,
        );

        // Float + None → NumberInput (default)
        reg.register(
            DataTypeId::new("float"),
            ConstraintType::None,
            WidgetId::new("number_input"),
            true,
            number_input::render_number_input,
        );

        // Int + None → NumberInput (default)
        reg.register(
            DataTypeId::new("int"),
            ConstraintType::None,
            WidgetId::new("number_input"),
            true,
            number_input::render_number_input,
        );

        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_widget_float_range() {
        let reg = WidgetRegistry::with_builtins();
        let w = reg.default_widget(&DataTypeId::new("float"), &ConstraintType::Range);
        assert_eq!(w, Some(&WidgetId::new("slider")));
    }

    #[test]
    fn test_available_widgets_float_range() {
        let reg = WidgetRegistry::with_builtins();
        let ws = reg.available_widgets(&DataTypeId::new("float"), &ConstraintType::Range);
        assert_eq!(ws.len(), 2);
    }

    #[test]
    fn test_default_widget_string_enum() {
        let reg = WidgetRegistry::with_builtins();
        let w = reg.default_widget(&DataTypeId::new("string"), &ConstraintType::Enum);
        assert_eq!(w, Some(&WidgetId::new("dropdown")));
    }
}
