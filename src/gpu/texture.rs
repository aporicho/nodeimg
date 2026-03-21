use std::fmt;

use eframe::egui;
use image::DynamicImage;

/// A GPU-resident RGBA image backed by a `wgpu::Texture`.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl fmt::Debug for GpuTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpuTexture")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

/// Common texture usage flags for compute-shader-accessible textures.
const TEXTURE_USAGES: wgpu::TextureUsages = wgpu::TextureUsages::STORAGE_BINDING
    .union(wgpu::TextureUsages::COPY_SRC)
    .union(wgpu::TextureUsages::COPY_DST)
    .union(wgpu::TextureUsages::TEXTURE_BINDING);

impl GpuTexture {
    /// Create an empty GPU texture (e.g. for compute shader output).
    pub fn create_empty(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GpuTexture::empty"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TEXTURE_USAGES,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            width,
            height,
        }
    }

    /// Upload a CPU `DynamicImage` to the GPU.
    pub fn from_dynamic_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &DynamicImage,
    ) -> Self {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let tex = Self::create_empty(device, width, height);

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        tex
    }

    /// Download this GPU texture to a CPU `DynamicImage`.
    pub fn to_dynamic_image(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> DynamicImage {
        let data = self.download_rgba(device, queue);
        let buf = image::RgbaImage::from_raw(self.width, self.height, data)
            .expect("downloaded data should match texture dimensions");
        DynamicImage::ImageRgba8(buf)
    }

    /// Download this GPU texture as an `egui::ColorImage` (for preview).
    pub fn to_color_image(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> egui::ColorImage {
        let data = self.download_rgba(device, queue);
        let pixels: Vec<egui::Color32> = data
            .chunks_exact(4)
            .map(|c| egui::Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]))
            .collect();
        egui::ColorImage::new([self.width as usize, self.height as usize], pixels)
    }

    /// Internal helper: read back RGBA bytes from GPU using a staging buffer.
    fn download_rgba(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = bytes_per_pixel * self.width;
        // wgpu requires rows to be aligned to 256 bytes
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuTexture::download staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        // Map the staging buffer and read back.
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU poll failed");
        rx.recv()
            .expect("GPU readback channel closed")
            .expect("GPU readback failed");

        let mapped = slice.get_mapped_range();
        // Strip row padding
        let mut result = Vec::with_capacity((unpadded_bytes_per_row * self.height) as usize);
        for row in 0..self.height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            result.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        staging.unmap();

        result
    }
}
