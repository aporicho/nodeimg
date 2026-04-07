use std::collections::HashMap;
use std::sync::Arc;

use super::types::Color;

pub struct SvgCache {
    cache: HashMap<SvgKey, SvgEntry>,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct SvgKey {
    data_hash: u64,
    size: u32,
    color: [u8; 4],
}

struct SvgEntry {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: Arc<wgpu::TextureView>,
}

impl SvgCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// 加载 SVG，返回 Arc<TextureView>
    pub fn load(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        svg_data: &[u8],
        size: u32,
        color: Color,
    ) -> Arc<wgpu::TextureView> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        svg_data.hash(&mut hasher);
        let data_hash = hasher.finish();

        let color_bytes = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        let key = SvgKey {
            data_hash,
            size,
            color: color_bytes,
        };

        if let Some(entry) = self.cache.get(&key) {
            return Arc::clone(&entry.view);
        }

        let (texture, view) = rasterize_svg(device, queue, svg_data, size, color_bytes);
        let view = Arc::new(view);
        let result = Arc::clone(&view);
        self.cache.insert(key, SvgEntry { texture, view });
        result
    }
}

fn rasterize_svg(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    svg_data: &[u8],
    size: u32,
    color: [u8; 4],
) -> (wgpu::Texture, wgpu::TextureView) {
    let mut svg_str = String::from_utf8_lossy(svg_data).to_string();
    let color_hex = format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]);
    svg_str = svg_str.replace("currentColor", &color_hex);

    let tree = resvg::usvg::Tree::from_str(&svg_str, &resvg::usvg::Options::default())
        .expect("failed to parse SVG");

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("failed to create pixmap");

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("svg_texture"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixmap.data(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * size),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
