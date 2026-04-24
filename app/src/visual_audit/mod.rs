mod overlay;
mod page;
mod primitives;
mod section;
mod state;
mod widgets;

pub(crate) use overlay::build_visual_audit_popup;
pub(crate) use page::{build_visual_audit_page, VisualAuditBuildContext};
pub(crate) use state::{
    VisualAuditState, POPUP_CLOSE_ID, POPUP_TRIGGER_ID, SLIDER_RADIUS_ID, TOGGLE_GRID_ID,
};

#[cfg(test)]
mod tests {
    use super::*;
    use gui::theme::light_theme;
    use gui::tree::layout::{LeafKind, TextureHandle};
    use gui::tree::Desc;

    #[test]
    fn visual_audit_contains_expected_sections() {
        let sections: Vec<section::VisualAuditSection> = page::build_visual_audit_sections(
            &light_theme(),
            &VisualAuditState::default(),
            TextureHandle(1),
        );
        let ids: Vec<&str> = sections.iter().map(|section| section.id).collect();

        assert!(ids.contains(&"overview"));
        assert!(ids.contains(&"primitives"));
        assert!(ids.contains(&"typography"));
        assert!(ids.contains(&"buttons"));
        assert!(ids.contains(&"inputs"));
        assert!(ids.contains(&"selection"));
        assert!(ids.contains(&"media"));
        assert!(ids.contains(&"containers"));
        assert!(ids.contains(&"overlay"));
    }

    #[test]
    fn primitive_section_covers_direct_primitive_leaf_kinds() {
        let theme = light_theme();
        let section = primitives::primitive_section(&theme, TextureHandle(1));
        let content = &section.content;

        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Circle { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Line { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Curve { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Path { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Image { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Icon { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Grid { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Connection { .. }
        )));
    }

    fn contains_leaf(descs: &[Desc], predicate: impl Fn(&LeafKind) -> bool + Copy) -> bool {
        descs.iter().any(|desc| match desc {
            Desc::Leaf { kind, .. } => predicate(kind),
            Desc::Container { children, .. } => contains_leaf(children, predicate),
            Desc::Widget(_) => false,
        })
    }
}
