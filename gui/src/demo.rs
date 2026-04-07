use std::sync::Arc;
use crate::renderer::{Border, Color, Point, Rect, RectStyle, Renderer, Shadow, TextStyle};
use crate::renderer::svg::SvgCache;
use crate::shell::{App, AppContext, AppEvent};

pub struct DemoApp {
    test_image: Arc<wgpu::TextureView>,
    #[allow(dead_code)]
    svg_cache: SvgCache,
    play_icon: Arc<wgpu::TextureView>,
}

impl App for DemoApp {
    fn init(ctx: &mut AppContext) -> Self {
        let img = image::open("assets/test/test.png").expect("failed to load test.png");
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_image"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture, mip_level: 0,
                origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );

        let test_image = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));

        let svg_data = std::fs::read("assets/icons/play.svg").expect("failed to load play.svg");
        let mut svg_cache = SvgCache::new();
        let play_icon = svg_cache.load(&ctx.device, &ctx.queue, &svg_data, 96, Color::WHITE);

        Self { test_image, svg_cache, play_icon }
    }

    fn event(&mut self, _event: AppEvent, _ctx: &mut AppContext) {}
    fn update(&mut self, _ctx: &mut AppContext) {}

    fn render(&mut self, renderer: &mut Renderer, _ctx: &AppContext) {
        // 白色圆角矩形
        renderer.draw_rect(
            Rect { x: 50.0, y: 50.0, w: 400.0, h: 240.0 },
            &RectStyle {
                color: Color::WHITE,
                border: None,
                radius: [32.0; 4],
                shadow: None,
            },
        );
        renderer.draw_text(
            Point { x: 170.0, y: 140.0 },
            "Hello",
            &TextStyle { color: Color::BLACK, size: 48.0 },
        );

        // 蓝色带边框圆角矩形
        renderer.draw_rect(
            Rect { x: 500.0, y: 50.0, w: 400.0, h: 240.0 },
            &RectStyle {
                color: Color { r: 0.2, g: 0.4, b: 0.8, a: 1.0 },
                border: Some(Border { width: 4.0, color: Color::WHITE }),
                radius: [24.0; 4],
                shadow: None,
            },
        );
        renderer.draw_text(
            Point { x: 620.0, y: 140.0 },
            "World",
            &TextStyle { color: Color::WHITE, size: 48.0 },
        );

        // S 形贝塞尔曲线
        renderer.draw_curve(
            [
                Point { x: 450.0, y: 170.0 },
                Point { x: 470.0, y: 80.0 },
                Point { x: 480.0, y: 260.0 },
                Point { x: 500.0, y: 170.0 },
            ],
            4.0,
            Color { r: 0.6, g: 0.8, b: 0.4, a: 1.0 },
        );

        // 测试图片
        renderer.draw_image(
            Rect { x: 50.0, y: 350.0, w: 400.0, h: 300.0 },
            self.test_image.clone(),
        );
        renderer.draw_text(
            Point { x: 190.0, y: 660.0 },
            "test.png",
            &TextStyle { color: Color::WHITE, size: 32.0 },
        );

        // SVG 图标
        renderer.draw_image(
            Rect { x: 500.0, y: 400.0, w: 96.0, h: 96.0 },
            self.play_icon.clone(),
        );
        renderer.draw_text(
            Point { x: 500.0, y: 510.0 },
            "play.svg",
            &TextStyle { color: Color::WHITE, size: 32.0 },
        );

        // 裁剪测试：圆角裁剪图片 + 白色描边 + 阴影
        let clip_rect = Rect { x: 650.0, y: 350.0, w: 400.0, h: 300.0 };
        renderer.draw_rect(
            clip_rect,
            &RectStyle {
                color: Color::TRANSPARENT,
                border: None,
                radius: [32.0; 4],
                shadow: Some(Shadow {
                    color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 },
                    offset: [0.0, 4.0],
                    blur: 16.0,
                    spread: 0.0,
                }),
            },
        );
        renderer.push_clip(clip_rect, 32.0);
        renderer.draw_image(clip_rect, self.test_image.clone());
        renderer.pop_clip();
        renderer.draw_rect(
            clip_rect,
            &RectStyle {
                color: Color::TRANSPARENT,
                border: Some(Border { width: 4.0, color: Color::WHITE }),
                radius: [32.0; 4],
                shadow: None,
            },
        );
        renderer.draw_text(
            Point { x: 790.0, y: 660.0 },
            "clipped",
            &TextStyle { color: Color::WHITE, size: 32.0 },
        );

        // 带阴影的圆角矩形
        renderer.draw_rect(
            Rect { x: 650.0, y: 50.0, w: 300.0, h: 200.0 },
            &RectStyle {
                color: Color::WHITE,
                border: None,
                radius: [24.0; 4],
                shadow: Some(Shadow {
                    color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 },
                    offset: [4.0, 8.0],
                    blur: 24.0,
                    spread: 0.0,
                }),
            },
        );
        renderer.draw_text(
            Point { x: 740.0, y: 130.0 },
            "Shadow",
            &TextStyle { color: Color::BLACK, size: 36.0 },
        );
    }
}
