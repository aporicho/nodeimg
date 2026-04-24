mod catalog;
mod concepts;
mod containers;
mod controller;
mod ids;
mod overlay;
mod primitives;
mod state;
mod view;
mod widgets;

pub(crate) use controller::DeveloperModeController;
pub(crate) use view::{build_developer_page, DeveloperModeBuildContext};

#[cfg(test)]
mod tests {
    use super::*;
    use gui::canvas::camera::Camera;
    use gui::theme::light_theme;
    use gui::tree::layout::{LeafKind, TextureHandle};
    use gui::tree::Desc;

    #[test]
    fn playground_catalog_uses_spatial_concept_levels() {
        let specs = catalog::item_specs();
        let family_y = specs
            .iter()
            .find(|spec| spec.id == catalog::PlaygroundItemId::PrimitiveConcept)
            .expect("primitive concept")
            .default_position[1];

        assert_eq!(
            specs.first().map(|spec| spec.id),
            Some(catalog::PlaygroundItemId::Canvas)
        );
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Family)
            .all(|spec| spec.default_position[1] == family_y));
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Primitive)
            .all(|spec| spec.default_position[1] > family_y));
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Container)
            .all(|spec| spec.default_position[1] > family_y));
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Control)
            .all(|spec| spec.default_position[1] > family_y));
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Overlay)
            .all(|spec| spec.default_position[1] > family_y));
        assert!(specs
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Workspace)
            .all(|spec| spec.default_position[1] > family_y));
    }

    #[test]
    fn developer_page_contains_canvas_root() {
        let theme = light_theme();
        let state = state::DeveloperModeState::default();
        let page = view::build_developer_page(view::DeveloperModeBuildContext {
            viewport: gui::renderer::Rect {
                x: 0.0,
                y: 0.0,
                w: 1280.0,
                h: 800.0,
            },
            camera: &Camera::new(),
            theme: &theme,
            state: &state,
            image: TextureHandle(1),
        });

        assert!(contains_id(&page, ids::PLAYGROUND_CANVAS_ID));
        assert!(contains_leaf(&[page], |kind| matches!(
            kind,
            LeafKind::Grid { .. }
        )));
    }

    #[test]
    fn developer_canvas_root_transform_matches_camera() {
        let theme = light_theme();
        let state = state::DeveloperModeState::default();
        let mut camera = Camera::new();
        camera.x = 80.0;
        camera.y = -24.0;
        camera.zoom = 1.5;

        let page = view::build_developer_page(view::DeveloperModeBuildContext {
            viewport: gui::renderer::Rect {
                x: 0.0,
                y: 0.0,
                w: 1280.0,
                h: 800.0,
            },
            camera: &camera,
            theme: &theme,
            state: &state,
            image: TextureHandle(1),
        });
        let Some(transform) = find_container_transform(&page, ids::PLAYGROUND_CANVAS_ID) else {
            panic!("expected developer canvas root transform");
        };

        assert_eq!(
            transform,
            gui::geometry::TransformSpec::translate_scale([80.0, -24.0], 1.5)
        );
    }

    #[test]
    fn primitive_samples_cover_direct_leaf_kinds() {
        let theme = light_theme();
        let descs: Vec<Desc> = catalog::item_specs()
            .iter()
            .filter(|spec| spec.group == catalog::PlaygroundGroup::Primitive)
            .map(|spec| primitives::primitive_sample(spec.id, &theme, TextureHandle(1)))
            .collect();

        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Text { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Image { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Icon { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Circle { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Line { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Curve { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Path { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Grid { .. }
        )));
        assert!(contains_leaf(&descs, |kind| matches!(
            kind,
            LeafKind::Connection { .. }
        )));
    }

    #[test]
    fn dragging_tile_moves_only_that_item() {
        let mut state = state::DeveloperModeState::default();
        let item = catalog::PlaygroundItemId::Surface;
        let original = state.position(item);
        let tile_id = ids::tile_id(item);

        assert!(state.start_drag(&tile_id, 10.0, 10.0));
        assert!(state.drag(&tile_id, 42.0, 31.0));
        assert!(state.end_drag(&tile_id, 42.0, 31.0));

        let moved = state.position(item);
        assert_eq!(moved.x, original.x + 32.0);
        assert_eq!(moved.y, original.y + 21.0);
        assert_eq!(
            state.position(catalog::PlaygroundItemId::Text),
            catalog::spec_for(catalog::PlaygroundItemId::Text)
                .expect("text spec")
                .default_position
                .into()
        );
    }

    #[test]
    fn resizing_tile_changes_only_that_item_size() {
        let mut state = state::DeveloperModeState::default();
        let item = catalog::PlaygroundItemId::Surface;
        let original = state.size(item);
        let handle_id = ids::tile_resize_handle_id(item);

        assert!(state.start_resize(&handle_id, 10.0, 10.0));
        assert!(state.resize(&handle_id, 42.0, 31.0));
        assert!(state.end_resize(&handle_id, 42.0, 31.0));

        let resized = state.size(item);
        assert_eq!(resized.width, original.width + 32.0);
        assert_eq!(resized.height, original.height + 21.0);
        assert_eq!(
            state.size(catalog::PlaygroundItemId::Text),
            catalog::spec_for(catalog::PlaygroundItemId::Text)
                .expect("text spec")
                .size
                .into()
        );
    }

    #[test]
    fn widget_state_updates_are_centralized() {
        let mut state = state::DeveloperModeState::default();

        assert!(state.apply_click(ids::CONTROL_TOGGLE_ID));
        assert!(!state.controls().toggle_enabled);
        assert!(state.apply_text_change(ids::CONTROL_TEXT_INPUT_ID, "updated".to_string()));
        assert_eq!(state.controls().text_value, "updated");
        assert!(state.apply_selection_change(ids::CONTROL_DROPDOWN_ID, 2));
        assert_eq!(state.controls().dropdown_selected, 2);
    }

    fn contains_id(desc: &Desc, expected: &str) -> bool {
        if desc.id() == expected {
            return true;
        }
        match desc {
            Desc::Container { children, .. } => {
                children.iter().any(|child| contains_id(child, expected))
            }
            Desc::Widget(widget) => widget
                .children()
                .iter()
                .any(|child| contains_id(child, expected)),
            Desc::Leaf { .. } => false,
        }
    }

    fn contains_leaf(descs: &[Desc], predicate: impl Fn(&LeafKind) -> bool + Copy) -> bool {
        descs.iter().any(|desc| match desc {
            Desc::Leaf { kind, .. } => predicate(kind),
            Desc::Container { children, .. } => contains_leaf(children, predicate),
            Desc::Widget(widget) => contains_leaf(widget.children(), predicate),
        })
    }

    fn find_container_transform(
        desc: &Desc,
        expected: &str,
    ) -> Option<gui::geometry::TransformSpec> {
        match desc {
            Desc::Container {
                id,
                style,
                children,
                ..
            } => {
                if id.as_ref() == expected {
                    return style.transform;
                }
                children
                    .iter()
                    .find_map(|child| find_container_transform(child, expected))
            }
            Desc::Widget(widget) => widget
                .children()
                .iter()
                .find_map(|child| find_container_transform(child, expected)),
            Desc::Leaf { .. } => None,
        }
    }
}
