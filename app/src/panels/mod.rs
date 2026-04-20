use crate::demo_gallery::GalleryState;
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;

#[allow(dead_code)]
pub(crate) struct PanelBuildContext<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) gallery: &'a GalleryState,
    pub(crate) image: TextureHandle,
}

include!(concat!(env!("OUT_DIR"), "/panels_generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_gallery::GalleryState;
    use gui::theme::light_theme;

    #[test]
    fn generated_panels_include_initial_workspace_panels() {
        let theme = light_theme();
        let gallery = GalleryState::default();
        let panels = collect_panels(&PanelBuildContext {
            theme: &theme,
            gallery: &gallery,
            image: TextureHandle(1),
        });
        let ids = panels
            .iter()
            .map(|panel| panel.config.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"preview"));
        assert!(ids.contains(&"toolbar"));
    }
}
