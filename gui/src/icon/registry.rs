use std::collections::HashMap;
use std::sync::Arc;

use crate::renderer::svg::SvgSource;

use super::{builtin, IconAsset, IconId};

#[derive(Debug, Clone)]
pub(crate) struct IconRegistry {
    icons: HashMap<IconId, IconAsset>,
}

impl IconRegistry {
    pub(crate) fn new() -> Self {
        Self {
            icons: HashMap::new(),
        }
    }

    pub(crate) fn with_builtin_icons() -> Self {
        let mut registry = Self::new();
        builtin::register_builtin_icons(&mut registry);
        registry
    }

    pub(crate) fn register_svg(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        let id = id.into();
        let source = SvgSource::new(id.as_str(), Arc::<[u8]>::from(svg.as_ref()));
        self.icons.insert(id, IconAsset::svg(source));
    }

    pub(crate) fn resolve(&self, id: &IconId) -> Option<&IconAsset> {
        self.icons.get(id)
    }
}

impl Default for IconRegistry {
    fn default() -> Self {
        Self::with_builtin_icons()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_icons_are_registered() {
        let registry = IconRegistry::with_builtin_icons();

        assert!(registry.resolve(&IconId::from("plus")).is_some());
        assert!(registry.resolve(&IconId::from("close")).is_some());
        assert!(registry.resolve(&IconId::from("chevron_down")).is_some());
        assert!(registry.resolve(&IconId::from("chevron_right")).is_some());
        assert!(registry.resolve(&IconId::from("play")).is_some());
        assert!(registry.resolve(&IconId::from("search")).is_some());
        assert!(registry.resolve(&IconId::from("settings")).is_some());
    }

    #[test]
    fn custom_svg_icon_can_be_registered() {
        let mut registry = IconRegistry::new();
        registry.register_svg(
            "custom",
            br#"<svg width="1" height="1" viewBox="0 0 1 1"><path d="M0 0H1V1Z"/></svg>"#,
        );

        let asset = registry
            .resolve(&IconId::from("custom"))
            .expect("custom icon should resolve");
        assert_eq!(asset.source.key().id(), "custom");
    }
}
