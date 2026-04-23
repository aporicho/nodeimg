use std::collections::HashMap;

use crate::icon::{IconId, IconRegistry};
use crate::paint::{SvgSourceKey as PaintSvgSourceKey, TextureHandle};

use super::svg::SvgSource;
use super::TextureResource;

pub(crate) trait DisplayResourceResolver {
    fn texture(&self, handle: TextureHandle) -> Option<TextureResource>;
    fn svg_source(&self, key: &PaintSvgSourceKey) -> Option<SvgSource>;
}

pub(crate) struct RegistryDisplayResources<'a> {
    textures: &'a HashMap<TextureHandle, TextureResource>,
    icons: &'a IconRegistry,
}

impl<'a> RegistryDisplayResources<'a> {
    pub(crate) fn new(
        textures: &'a HashMap<TextureHandle, TextureResource>,
        icons: &'a IconRegistry,
    ) -> Self {
        Self { textures, icons }
    }
}

impl DisplayResourceResolver for RegistryDisplayResources<'_> {
    fn texture(&self, handle: TextureHandle) -> Option<TextureResource> {
        self.textures.get(&handle).cloned()
    }

    fn svg_source(&self, key: &PaintSvgSourceKey) -> Option<SvgSource> {
        self.icons
            .resolve(&IconId::from(key.id.as_str()))
            .map(|asset| asset.source.clone())
    }
}

#[cfg(test)]
pub(crate) struct EmptyDisplayResources;

#[cfg(test)]
impl DisplayResourceResolver for EmptyDisplayResources {
    fn texture(&self, _handle: TextureHandle) -> Option<TextureResource> {
        None
    }

    fn svg_source(&self, _key: &PaintSvgSourceKey) -> Option<SvgSource> {
        None
    }
}
