mod content;
mod data;
mod frame;
mod node_factory;
mod template;
mod titlebar;

#[cfg(test)]
mod tests;

pub use data::{PanelContentTemplate, PanelFrameTemplateData};
pub(crate) use template::PanelFrameRetainedTemplate;
