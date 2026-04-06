use types::value::Image;

pub struct CpuExecutor;

impl CpuExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn load_image(
        &self,
        path: &str,
    ) -> Result<Image, Box<dyn std::error::Error + Send + Sync>> {
        let img = image::open(path)?;
        Ok(Image::from_cpu(img))
    }

    pub fn save_image(
        &self,
        image: &Image,
        path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let cpu_img = image.as_cpu(device, queue);
        cpu_img.save(path)?;
        Ok(())
    }
}

impl Default for CpuExecutor {
    fn default() -> Self {
        Self::new()
    }
}
