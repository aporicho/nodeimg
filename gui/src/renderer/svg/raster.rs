use std::collections::HashMap;

use crate::icon::IconFit;
use crate::renderer::{Color, TextureResource, TextureSize};

use super::{SvgError, SvgSource, SvgSourceKey};

#[derive(Debug, Clone)]
pub(crate) struct SvgRasterRequest {
    pub(crate) source: SvgSource,
    pub(crate) pixel_size: TextureSize,
    pub(crate) color: Color,
    pub(crate) fit: IconFit,
}

impl SvgRasterRequest {
    pub(crate) fn new(
        source: SvgSource,
        pixel_size: TextureSize,
        color: Color,
        fit: IconFit,
    ) -> Self {
        Self {
            source,
            pixel_size,
            color,
            fit,
        }
    }
}

#[derive(Default)]
pub(crate) struct SvgRasterCache {
    cache: HashMap<SvgRasterKey, SvgRasterEntry>,
}

struct SvgRasterEntry {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    resource: TextureResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SvgRasterKey {
    source: SvgSourceKey,
    width: u32,
    height: u32,
    color: [u8; 4],
    fit: IconFit,
}

impl SvgRasterCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_or_rasterize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        request: SvgRasterRequest,
    ) -> Result<TextureResource, SvgError> {
        let color = color_bytes(request.color);
        let key = SvgRasterKey {
            source: request.source.key().clone(),
            width: request.pixel_size.width,
            height: request.pixel_size.height,
            color,
            fit: request.fit,
        };

        if let Some(entry) = self.cache.get(&key) {
            return Ok(entry.resource.clone());
        }

        let (texture, resource) = rasterize_svg(
            device,
            queue,
            request.source.bytes(),
            request.pixel_size,
            color,
            request.fit,
        )?;
        let result = resource.clone();
        self.cache.insert(key, SvgRasterEntry { texture, resource });
        Ok(result)
    }
}

fn rasterize_svg(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    svg_data: &[u8],
    size: TextureSize,
    color: [u8; 4],
    fit: IconFit,
) -> Result<(wgpu::Texture, TextureResource), SvgError> {
    let tree = resvg::usvg::Tree::from_data(svg_data, &resvg::usvg::Options::default())
        .map_err(|err| SvgError::Parse(err.to_string()))?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width.max(1), size.height.max(1))
        .ok_or_else(|| SvgError::Parse("failed to create SVG pixmap".to_string()))?;

    let svg_size = tree.size();
    let transform = raster_transform(svg_size.width(), svg_size.height(), size, fit);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let [r, g, b, a] = color;
    for px in pixmap.data_mut().chunks_exact_mut(4) {
        if px[3] > 0 {
            px[0] = r;
            px[1] = g;
            px[2] = b;
            px[3] = ((px[3] as u16 * a as u16) / 255) as u8;
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("svg_raster_texture"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
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
            bytes_per_row: Some(4 * size.width.max(1)),
            rows_per_image: Some(size.height.max(1)),
        },
        wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
    );

    let view = std::sync::Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
    let resource = TextureResource::new(view, size);
    Ok((texture, resource))
}

fn raster_transform(
    source_w: f32,
    source_h: f32,
    size: TextureSize,
    fit: IconFit,
) -> resvg::tiny_skia::Transform {
    let target_w = size.width.max(1) as f32;
    let target_h = size.height.max(1) as f32;
    let source_w = source_w.max(1.0);
    let source_h = source_h.max(1.0);

    match fit {
        IconFit::Stretch => resvg::tiny_skia::Transform::from_row(
            target_w / source_w,
            0.0,
            0.0,
            target_h / source_h,
            0.0,
            0.0,
        ),
        IconFit::Contain => {
            let scale = (target_w / source_w).min(target_h / source_h);
            let dx = (target_w - source_w * scale) * 0.5;
            let dy = (target_h - source_h * scale) * 0.5;
            resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy)
        }
    }
}

fn color_bytes(color: Color) -> [u8; 4] {
    [
        float_to_u8(color.r),
        float_to_u8(color.g),
        float_to_u8(color.b),
        float_to_u8(color.a),
    ]
}

fn float_to_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 255;
    }
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn svg_raster_source_key_includes_bytes_hash() {
        let a = SvgSource::new("same", Arc::<[u8]>::from(&b"<svg/>"[..]));
        let b = SvgSource::new("same", Arc::<[u8]>::from(&b"<svg><path/></svg>"[..]));

        assert_ne!(a.key(), b.key());
        assert_eq!(a.key().id(), "same");
    }

    #[test]
    fn svg_raster_cache_key_includes_fit() {
        let source = SvgSource::new("same", Arc::<[u8]>::from(&b"<svg/>"[..]));
        let contain = SvgRasterKey {
            source: source.key().clone(),
            width: 100,
            height: 50,
            color: [255, 255, 255, 255],
            fit: IconFit::Contain,
        };
        let stretch = SvgRasterKey {
            fit: IconFit::Stretch,
            ..contain.clone()
        };

        assert_ne!(contain, stretch);
    }

    #[test]
    fn raster_transform_uses_contain_fit() {
        let transform = raster_transform(10.0, 20.0, TextureSize::new(100, 100), IconFit::Contain);
        let mut origin = resvg::tiny_skia::Point::from_xy(0.0, 0.0);
        let mut far = resvg::tiny_skia::Point::from_xy(10.0, 20.0);

        transform.map_point(&mut origin);
        transform.map_point(&mut far);

        assert_eq!(origin.x, 25.0);
        assert_eq!(origin.y, 0.0);
        assert_eq!(far.x, 75.0);
        assert_eq!(far.y, 100.0);
    }

    #[test]
    fn raster_transform_uses_stretch_fit() {
        let transform = raster_transform(10.0, 20.0, TextureSize::new(100, 100), IconFit::Stretch);
        let mut origin = resvg::tiny_skia::Point::from_xy(0.0, 0.0);
        let mut far = resvg::tiny_skia::Point::from_xy(10.0, 20.0);

        transform.map_point(&mut origin);
        transform.map_point(&mut far);

        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
        assert_eq!(far.x, 100.0);
        assert_eq!(far.y, 100.0);
    }
}
