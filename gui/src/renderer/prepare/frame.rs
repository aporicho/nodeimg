use std::sync::Arc;

use super::super::command::{AffineShadowRequest, AffineSvgRasterRequest, AffineTextRequest};
use super::super::pipeline::circle::CircleVertex;
use super::super::pipeline::grid::GridVertex;
use super::super::pipeline::image::PreparedImageDraw;
use super::super::pipeline::quad::QuadVertex;
use super::super::pipeline::stencil::StencilVertex;
use super::super::pipeline::text::TextRequest;
use super::super::pipeline::vector::VectorVertex;
use super::super::scene_prepare::RendererPrepareStats;

pub(in crate::renderer) enum DrawOp {
    Quad {
        index_start: u32,
        index_count: u32,
    },
    Circle {
        index_start: u32,
        index_count: u32,
    },
    Grid {
        index_start: u32,
        index_count: u32,
    },
    Vector {
        index_start: u32,
        index_count: u32,
    },
    Shadow(AffineShadowRequest),
    Image {
        view: Arc<wgpu::TextureView>,
        draw: PreparedImageDraw,
    },
    SvgRaster(AffineSvgRasterRequest),
    AffineText(AffineTextRequest),
    Noop,
    Text {
        index: usize,
    },
    StencilWrite {
        index_start: u32,
        index_count: u32,
    },
    StencilClear {
        index_start: u32,
        index_count: u32,
    },
}

pub(in crate::renderer) struct PreparedFrame {
    pub(in crate::renderer) stats: RendererPrepareStats,
    pub(in crate::renderer) ops: Vec<DrawOp>,
    pub(in crate::renderer) text_requests: Vec<TextRequest>,

    pub(in crate::renderer) quad_vertices: Vec<QuadVertex>,
    pub(in crate::renderer) quad_indices: Vec<u32>,

    pub(in crate::renderer) vector_vertices: Vec<VectorVertex>,
    pub(in crate::renderer) vector_indices: Vec<u32>,

    pub(in crate::renderer) circle_vertices: Vec<CircleVertex>,
    pub(in crate::renderer) circle_indices: Vec<u32>,

    pub(in crate::renderer) grid_vertices: Vec<GridVertex>,
    pub(in crate::renderer) grid_indices: Vec<u32>,

    pub(in crate::renderer) stencil_vertices: Vec<StencilVertex>,
    pub(in crate::renderer) stencil_indices: Vec<u32>,
}

impl PreparedFrame {
    pub(super) fn new(backend_commands: usize) -> Self {
        Self {
            stats: RendererPrepareStats {
                backend_commands,
                ..RendererPrepareStats::default()
            },
            ops: Vec::new(),
            text_requests: Vec::new(),
            quad_vertices: Vec::new(),
            quad_indices: Vec::new(),
            vector_vertices: Vec::new(),
            vector_indices: Vec::new(),
            circle_vertices: Vec::new(),
            circle_indices: Vec::new(),
            grid_vertices: Vec::new(),
            grid_indices: Vec::new(),
            stencil_vertices: Vec::new(),
            stencil_indices: Vec::new(),
        }
    }
}
