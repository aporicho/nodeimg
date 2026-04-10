pub use nodeimg_types::gpu_texture::*;

use eframe::egui;

/// Extension trait adding egui-dependent methods to GpuTexture.
pub trait GpuTextureEguiExt {
    /// Download this GPU texture as an `egui::ColorImage` (for preview).
    fn to_color_image(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> egui::ColorImage;
}

impl GpuTextureEguiExt for GpuTexture {
    fn to_color_image(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> egui::ColorImage {
        let data = self.download_rgba(device, queue);
        let pixels: Vec<egui::Color32> = data
            .chunks_exact(4)
            .map(|c| egui::Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
            .collect();
        egui::ColorImage::new([self.width as usize, self.height as usize], pixels)
    }
}
