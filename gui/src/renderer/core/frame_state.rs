use winit::dpi::PhysicalSize;

pub(super) struct FrameState {
    pub(super) view: wgpu::TextureView,
    pub(super) size: PhysicalSize<u32>,
    pub(super) scale_factor: f64,
}
