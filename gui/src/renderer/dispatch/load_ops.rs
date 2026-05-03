use super::super::types::Color;

pub(super) fn color_load_op(first_pass: bool, clear_color: Color) -> wgpu::LoadOp<wgpu::Color> {
    if first_pass {
        wgpu::LoadOp::Clear(wgpu::Color {
            r: clear_color.r as f64,
            g: clear_color.g as f64,
            b: clear_color.b as f64,
            a: clear_color.a as f64,
        })
    } else {
        wgpu::LoadOp::Load
    }
}

pub(super) fn color_store_op() -> wgpu::StoreOp {
    wgpu::StoreOp::Store
}

pub(super) fn stencil_load_op(first_pass: bool) -> wgpu::LoadOp<u32> {
    if first_pass {
        wgpu::LoadOp::Clear(0)
    } else {
        wgpu::LoadOp::Load
    }
}
