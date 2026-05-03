pub(in crate::renderer) const MSAA_SAMPLE_COUNT: u32 = 4;
pub(super) const DEFAULT_RENDER_SCALE: f32 = 2.0;

pub(super) fn msaa_multisample_state() -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: MSAA_SAMPLE_COUNT,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}
