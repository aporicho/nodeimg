use crate::gpu_texture::GpuTexture;
use std::sync::Arc;

/// Represents any value that flows through the node graph.
#[derive(Clone, Debug)]
pub enum Value {
    Image(Arc<image::DynamicImage>),
    GpuImage(Arc<GpuTexture>),
    Mask(Arc<image::DynamicImage>),
    Float(f32),
    Int(i32),
    Color([f32; 4]),
    Boolean(bool),
    String(String),
}
