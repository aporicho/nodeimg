use super::GuiAction;

pub const NODE_LIBRARY_ADD_PREFIX: &str = "node_library::add::";

pub fn dispatch_widget_click(id: &str) -> GuiAction {
    if let Some(type_id) = node_library_add_type_id(id) {
        return GuiAction::AddNode {
            type_id: type_id.to_string(),
        };
    }

    GuiAction::WidgetClicked { id: id.to_string() }
}

pub fn node_library_add_type_id(id: &str) -> Option<&str> {
    id.strip_prefix(NODE_LIBRARY_ADD_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_node_library_add_click() {
        assert_eq!(
            dispatch_widget_click("node_library::add::image_gen"),
            GuiAction::AddNode {
                type_id: "image_gen".to_string()
            }
        );
    }

    #[test]
    fn dispatches_plain_widget_click_as_fallback() {
        assert_eq!(
            dispatch_widget_click("toolbar::save"),
            GuiAction::WidgetClicked {
                id: "toolbar::save".to_string()
            }
        );
    }
}
