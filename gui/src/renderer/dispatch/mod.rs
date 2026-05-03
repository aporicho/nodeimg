mod affine_text;
mod api;
mod blit;
mod deferred;
mod execute;
mod load_ops;
mod ops_pass;
mod render_pass;
mod render_steps;
mod text_pass;
mod trace;
mod upload;

#[cfg(test)]
mod tests;

pub(in crate::renderer) use api::{DispatchFrame, DispatchPipelines};
pub(in crate::renderer) use execute::dispatch;
