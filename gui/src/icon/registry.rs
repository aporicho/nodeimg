use std::collections::HashMap;
use std::sync::Arc;

use crate::renderer::svg::SvgSource;

use super::{aliases, builtin, IconAsset, IconId};

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
        for asset in builtin::assets() {
            registry.register_static_svg(asset.id, asset.svg);
        }
        registry
    }

    pub(crate) fn register_svg(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        let id = id.into();
        let source = SvgSource::new(id.as_str(), Arc::<[u8]>::from(svg.as_ref()));
        self.icons.insert(id, IconAsset::svg(source));
    }

    pub(crate) fn register_static_svg(&mut self, id: &'static str, svg: &'static [u8]) {
        let source = SvgSource::new(id, Arc::<[u8]>::from(svg));
        self.icons.insert(IconId::from(id), IconAsset::svg(source));
    }

    pub(crate) fn resolve(&self, id: &IconId) -> Option<&IconAsset> {
        self.icons.get(id).or_else(|| {
            aliases::canonical_id(id.as_str()).and_then(|canonical| self.icons.get(canonical))
        })
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.icons.len()
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

        assert_eq!(registry.len(), builtin::assets().len());
        assert!(registry.len() > 1_000);
        assert!(registry.resolve(&IconId::from("plus")).is_some());
        assert!(registry.resolve(&IconId::from("xmark")).is_some());
        assert!(registry.resolve(&IconId::from("nav-arrow-down")).is_some());
        assert!(registry.resolve(&IconId::from("nav-arrow-right")).is_some());
        assert!(registry.resolve(&IconId::from("play")).is_some());
        assert!(registry.resolve(&IconId::from("search")).is_some());
        assert!(registry.resolve(&IconId::from("settings")).is_some());
        assert!(registry.resolve(&IconId::from("check")).is_some());
    }

    #[test]
    fn icon_aliases_resolve_to_canonical_assets() {
        let registry = IconRegistry::with_builtin_icons();

        assert_eq!(
            registry
                .resolve(&IconId::from("close"))
                .expect("close alias")
                .source
                .key()
                .id(),
            "xmark"
        );
        assert_eq!(
            registry
                .resolve(&IconId::from("chevron_down"))
                .expect("chevron_down alias")
                .source
                .key()
                .id(),
            "nav-arrow-down"
        );
        assert_eq!(
            registry
                .resolve(&IconId::from("chevron_right"))
                .expect("chevron_right alias")
                .source
                .key()
                .id(),
            "nav-arrow-right"
        );
    }

    #[test]
    fn generated_icon_names_match_asset_ids() {
        assert_eq!(builtin::names::PLUS.as_str(), "plus");
        assert_eq!(builtin::names::XMARK.as_str(), "xmark");
        assert_eq!(builtin::names::NAV_ARROW_DOWN.as_str(), "nav-arrow-down");
        assert_eq!(builtin::names::CHECK.as_str(), "check");
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
