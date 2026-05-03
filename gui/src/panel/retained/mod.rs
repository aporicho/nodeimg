mod content;
mod controls;
mod data;
mod frame;
mod node_factory;
mod template;
mod text;
mod titlebar;

#[cfg(test)]
mod tests;

pub use data::{PanelContentTemplate, PanelFrameTemplateData};
pub(crate) use template::PanelFrameRetainedTemplate;
